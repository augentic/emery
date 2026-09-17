//! Asserts what `prose!` embeds and what `check` holds a table to.
//!
//! The list names each document by tree-relative path and embeds its body
//! verbatim from the tree beside the invoking file. The check walks that tree
//! — through a symlinked directory, never through a cycle — and reports a
//! document the list leaves out, a listed document the tree lacks, a path
//! listed twice, and a relative link no listed document answers; a link in
//! fenced code is not a link, and neither is a URL.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use emery_prose::{Doc, body, check};

static DOCS: &[Doc] = emery_prose::prose!("fixtures", ["prompts/extract.md", "references/ids.md"]);

// The fixtures agree with their list: the one place the list, the embed, and
// the check are seen together over a real tree.
#[test]
fn fixtures() {
    let paths: Vec<&str> = DOCS.iter().map(|doc| doc.path).collect();
    assert_eq!(paths, ["prompts/extract.md", "references/ids.md"]);
    assert_eq!(body(DOCS, "references/ids.md"), Some(include_str!("fixtures/references/ids.md")));
    assert_eq!(body(DOCS, "prompts/extract.md"), Some(include_str!("fixtures/prompts/extract.md")));

    let findings = check(DOCS, &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    assert!(findings.is_empty(), "{}", findings.join("\n"));
}

// A document added to the tree but not to the list is the one drift the
// compiler cannot see; a listed document the tree lacks is caught here too,
// for a table written by hand.
#[test]
fn unlisted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "a.md", "# A\n");
    write(tmp.path(), "b.md", "# B\n");
    write(tmp.path(), "notes.txt", "not a document\n");

    let table = [doc("a.md", "# A\n"), doc("c.md", "# C\n")];
    assert_eq!(
        check(&table, tmp.path()),
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
    assert_eq!(check(&table, tmp.path()), ["`a.md` is listed twice"]);
}

// A symlinked directory is part of the tree, and a `](` inside fenced code
// is not a link; the live adapter trees share their runtime references by
// symlink, so this is where that is proved.
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
    let findings = check(&table, &tree);
    assert!(findings.is_empty(), "{}", findings.join("\n"));
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
        check(&table, tmp.path()),
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

    let absent = check(&[], &tmp.path().join("absent"));
    assert_eq!(absent.len(), 1, "{absent:?}");
    assert!(absent[0].contains("cannot be read"), "{}", absent[0]);

    let cycle = tmp.path().join("cycle");
    write(&cycle, "intro.md", "# Intro\n");
    symlink(Path::new("."), cycle.join("loop")).expect("symlink");
    let looped = check(&[doc("intro.md", "# Intro\n")], &cycle);
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
