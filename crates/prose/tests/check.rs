//! Verifies document embedding and corpus validation.
//!
//! The scenarios cover table order, verbatim bodies, missing and duplicate
//! entries, prompt reachability, relative-link resolution, and imported
//! documents. They also verify symlink traversal and ensure links in fenced
//! code or external URLs are ignored.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use emery_prose::{Doc, body, check};

static PROSE: &[Doc] = emery_prose::prose!["prose/prompts/extract.md", "prose/references/ids.md"];

// A document is named from the invoking file, as `include_str!` names one,
// so a list may climb out of its directory and back into the tree; the table
// path is what follows `prose/` either way.
static CLIMBING: &[Doc] = emery_prose::prose!["../tests/prose/references/ids.md"];

// The fixtures agree with their list: the one place the list, the embed, and
// the check are seen together over a real tree.
#[test]
fn fixtures() {
    let paths: Vec<&str> = PROSE.iter().map(|doc| doc.path).collect();
    assert_eq!(paths, ["prompts/extract.md", "references/ids.md"]);
    assert_eq!(body(PROSE, "references/ids.md"), Some(include_str!("prose/references/ids.md")));
    assert_eq!(body(PROSE, "prompts/extract.md"), Some(include_str!("prose/prompts/extract.md")));
    assert_eq!(CLIMBING[0].path, "references/ids.md");
    assert_eq!(CLIMBING[0].body, include_str!("prose/references/ids.md"));

    let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/prose");
    let findings = check(PROSE, &tree, &["prompts/extract.md"], &[]);
    assert!(findings.is_empty(), "{}", findings.join("\n"));
}

// A document added to the tree but not to the list is the one drift the
// compiler cannot see; a listed document the tree lacks is caught here too,
// for a table written by hand.
#[test]
fn unlisted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "a.md", "# A, see [c](c.md)\n");
    write(tmp.path(), "b.md", "# B\n");
    write(tmp.path(), "notes.txt", "not a document\n");

    let table = [doc("a.md", "# A, see [c](c.md)\n"), doc("c.md", "# C\n")];
    assert_eq!(
        check(&table, tmp.path(), &["a.md"], &[]),
        [
            "`b.md` is in the tree but not in the table",
            "`c.md` is in the table but not in the tree",
        ]
    );
}

// A path listed twice would make one lookup answer for two entries.
#[test]
fn repeated() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "a.md", "# A\n");

    let table = [doc("a.md", "# A\n"), doc("a.md", "# A\n")];
    assert_eq!(check(&table, tmp.path(), &["a.md"], &[]), ["`a.md` is listed twice"]);
}

// A listed document no prompt reaches is embedded and never read, and a link
// from another unreached document does not rescue it; a prompt the table
// lacks is `server_error` on every run. Both are the table's own drift, so
// the tree agrees with the list throughout.
#[test]
fn unlinked() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "prompts/extract.md", "see [ids](../references/ids.md)\n");
    write(tmp.path(), "references/ids.md", "# Ids\n");
    write(tmp.path(), "references/notes.md", "see [more](more.md)\n");
    write(tmp.path(), "references/more.md", "# More\n");

    let table = [
        doc("prompts/extract.md", "see [ids](../references/ids.md)\n"),
        doc("references/ids.md", "# Ids\n"),
        doc("references/notes.md", "see [more](more.md)\n"),
        doc("references/more.md", "# More\n"),
    ];
    assert_eq!(
        check(&table, tmp.path(), &["prompts/extract.md", "prompts/survey.md"], &[]),
        [
            "`prompts/survey.md` is a prompt the table does not hold",
            "`references/notes.md` is reached from no prompt",
            "`references/more.md` is reached from no prompt",
        ]
    );
}

// A symlinked directory is part of the tree, so a document reached through
// one is listed like any other, and a `](` inside fenced code is not a link.
#[test]
fn symlinked() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let shared = tmp.path().join("shared");
    write(&shared, "rule.md", "# Rule\n");
    let tree = tmp.path().join("prose");
    write(&tree, "b.md", "# B\n");
    write(
        &tree,
        "a.md",
        "see [b](b.md) and [the rule](runtime/rule.md#top)\n\n```swift\n[UInt8](effects)\n```\n",
    );
    symlink(&shared, tree.join("runtime")).expect("symlink");

    let table = [
        doc(
            "a.md",
            "see [b](b.md) and [the rule](runtime/rule.md#top)\n\n```swift\n[UInt8](effects)\n```\n",
        ),
        doc("b.md", "# B\n"),
        doc("runtime/rule.md", "# Rule\n"),
    ];
    let findings = check(&table, &tree, &["a.md"], &[]);
    assert!(findings.is_empty(), "{}", findings.join("\n"));
}

// A link may name a document another table embeds beside this one — the
// SDK's runtime references beside an adapter's own — which the tree need not
// hold and no prompt need reach; a listed document at an import's path would
// answer lookups meant for the import, so it is a finding even when the tree
// holds it and a prompt reaches it.
#[test]
fn imported() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "prompts/extract.md", "see [claims](../emery/claims.md)\n");
    let imports = [doc("emery/claims.md", "# Claims\n"), doc("emery/pipeline.md", "# Pipeline\n")];

    let table = [doc("prompts/extract.md", "see [claims](../emery/claims.md)\n")];
    let findings = check(&table, tmp.path(), &["prompts/extract.md"], &imports);
    assert!(findings.is_empty(), "{}", findings.join("\n"));

    write(tmp.path(), "emery/claims.md", "# Mine\n");
    let shadowing = [table[0], doc("emery/claims.md", "# Mine\n")];
    assert_eq!(
        check(&shadowing, tmp.path(), &["prompts/extract.md"], &imports),
        ["`emery/claims.md` shadows an import"]
    );
}

// A link is checked against the table, not the disk: a target the tree holds
// but the list omits is as unanswerable to `read_doc` as one that never
// existed, and a link that climbs out of the tree names nothing embeddable.
#[test]
fn dangling() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(
        tmp.path(),
        "prompts/extract.md",
        "see [ids](../references/ids.md) and [out](../../x.md)\n",
    );
    write(tmp.path(), "references/ids.md", "# Ids, see [nope](nope.md)\n");

    let table = [
        doc("prompts/extract.md", "see [ids](../references/ids.md) and [out](../../x.md)\n"),
        doc("references/ids.md", "# Ids, see [nope](nope.md)\n"),
    ];
    assert_eq!(
        check(&table, tmp.path(), &["prompts/extract.md"], &[]),
        [
            "`prompts/extract.md` links `../../x.md`, which leaves the tree",
            "`references/ids.md` links `nope.md`, and the table holds no `references/nope.md`",
        ]
    );
}

// A missing tree and a symlink cycle are reported rather than walked forever
// or mistaken for an empty tree.
#[test]
fn unwalkable() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let absent = check(&[], &tmp.path().join("absent"), &[], &[]);
    assert_eq!(absent.len(), 1, "{absent:?}");
    assert!(absent[0].contains("cannot be read"), "{}", absent[0]);

    let cycle = tmp.path().join("cycle");
    write(&cycle, "intro.md", "# Intro\n");
    symlink(Path::new("."), cycle.join("loop")).expect("symlink");
    let looped = check(&[doc("intro.md", "# Intro\n")], &cycle, &["intro.md"], &[]);
    assert_eq!(looped.len(), 1, "{looped:?}");
    assert!(looped[0].contains("symlink cycle"), "{}", looped[0]);
}

const fn doc(path: &'static str, body: &'static str) -> Doc {
    Doc { path, body }
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, body).expect("write");
}
