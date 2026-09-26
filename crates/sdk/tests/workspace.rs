//! Verifies workspace listing and adapter-defined entry filters.
//!
//! The listing carries root-relative regular files in order, omits engine
//! output and symlinks, and applies the adapter's filter while walking.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use emery_sdk::workspace::{self, Entry};

fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(path, "").expect("write");
}

fn utf8(root: &Path) -> &str {
    root.to_str().expect("a UTF-8 scratch root")
}

fn tree<'a>(root: &'a Path, files: &[&str]) -> &'a str {
    for file in files {
        write(root, file);
    }
    utf8(root)
}

// The listing is the path space a claim's `path` anchor cites.
#[test]
fn lists_relative() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), &["b.md", "a/y.md", "a/x.md"]);

    let files = workspace::list(root, |_| true).expect("walk");

    assert_eq!(files, ["a/x.md", "a/y.md", "b.md"]);
}

// A projection of the last revision is output, not a source to mine.
#[test]
fn skip_roots() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(
        tmp.path(),
        &[
            "readme.md",
            "spec.md",
            "design.md",
            ".omnia/store.json",
            "nested/spec.md",
            "nested/design.md",
            "nested/.omnia/x",
            "nested/keep.md",
        ],
    );

    let files = workspace::list(root, |_| true).expect("walk");

    assert_eq!(files, ["nested/keep.md", "readme.md"]);
}

#[test]
fn keep_filter() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), &["keep.md", "skip.lock", "vendor/lib.md", "src/main.rs"]);

    let files = workspace::list(root, |entry| match entry {
        Entry::Dir(path) => path != "vendor",
        Entry::File(_) => entry.extension().is_none_or(|ext| ext != "lock"),
    })
    .expect("walk");

    assert_eq!(files, ["keep.md", "src/main.rs"]);
}

// A leading dot is hidden, not an extension, so an adapter states its policy
// without unpicking the path.
#[test]
fn entry_readers() {
    let file = Entry::File("api/orders.test.ts");
    assert_eq!(file.path(), "api/orders.test.ts");
    assert_eq!(file.name(), "orders.test.ts");
    assert_eq!(file.extension(), Some("ts"));
    assert!(!file.hidden());

    let dir = Entry::Dir(".github");
    assert_eq!(dir.name(), ".github");
    assert_eq!(dir.extension(), None, "a leading dot is not an extension");
    assert!(dir.hidden());

    assert_eq!(Entry::File("README").extension(), None);
    assert_eq!(Entry::File("src/.env.local").extension(), Some("local"));
    assert!(Entry::File("src/.env.local").hidden());
}

// Even a link to a real file beside it is not listed.
#[test]
fn skips_symlinks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), &["real.md"]);
    symlink(tmp.path().join("real.md"), tmp.path().join("link.md")).expect("symlink");

    let files = workspace::list(root, |_| true).expect("walk");

    assert_eq!(files, ["real.md"]);
}

#[test]
fn skips_symlink_dirs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), &["real/nested/file.md"]);
    symlink(tmp.path().join("real"), tmp.path().join("link")).expect("symlink");

    let files = workspace::list(root, |_| true).expect("walk");

    assert_eq!(files, ["real/nested/file.md"]);
}

// A root the walk cannot open is the adapter host's defect, not the operator's.
#[test]
fn missing_root() {
    let error = workspace::list("/no/such/emery-workspace-root", |_| true).expect_err("missing");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("reading"), "{error}");
}
