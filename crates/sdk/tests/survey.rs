//! The mechanical survey
//!
//! What `files` and `by_directory` decide so a tree adapter does not
//! rewrite the walk: skip roots, the keep filter, the grain floor, and
//! that a symlink is not a file to mine.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use emery_sdk::survey::{self, Entry};

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(path, body).expect("write");
}

fn owned(files: &[&str]) -> Vec<String> {
    files.iter().copied().map(str::to_string).collect()
}

// Every regular file beneath the root is listed relative to it, sorted, with
// `/` separators — the path space a claim's `path` anchor cites.
#[test]
fn lists_relative() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "b.md", "");
    write(tmp.path(), "a/y.md", "");
    write(tmp.path(), "a/x.md", "");

    let found = survey::files(tmp.path(), |_, _| true).expect("walk");

    assert_eq!(found, ["a/x.md", "a/y.md", "b.md"]);
}

// The engine's own files are never offered, wherever they sit: a projection
// of the last revision is output, not a source to mine.
#[test]
fn skip_roots() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "readme.md", "");
    write(tmp.path(), "spec.md", "");
    write(tmp.path(), "design.md", "");
    write(tmp.path(), ".omnia/store.json", "");
    write(tmp.path(), "nested/spec.md", "");
    write(tmp.path(), "nested/design.md", "");
    write(tmp.path(), "nested/.omnia/x", "");
    write(tmp.path(), "nested/keep.md", "");

    let found = survey::files(tmp.path(), |_, _| true).expect("walk");

    assert_eq!(found, ["nested/keep.md", "readme.md"]);
}

// A refused directory is not entered; a refused file is omitted. Every other
// entry is the adapter's.
#[test]
fn keep_filter() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "keep.md", "");
    write(tmp.path(), "skip.lock", "");
    write(tmp.path(), "vendor/lib.md", "");
    write(tmp.path(), "src/main.rs", "");

    let found = survey::files(tmp.path(), |path, kind| match kind {
        Entry::Dir => path != Path::new("vendor"),
        Entry::File => path.extension().is_none_or(|ext| ext != "lock"),
    })
    .expect("walk");

    assert_eq!(found, ["keep.md", "src/main.rs"]);
}

// A symlink is not a regular file or a directory to enter, so a link at the
// root — even to a real file beside it — is not listed.
#[test]
fn skips_symlinks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "real.md", "");
    symlink(tmp.path().join("real.md"), tmp.path().join("link.md")).expect("symlink");

    let found = survey::files(tmp.path(), |_, _| true).expect("walk");

    assert_eq!(found, ["real.md"]);
}

// A root the walk cannot open is the adapter host's defect, not the
// operator's input.
#[test]
fn missing_root() {
    let error =
        survey::files(Path::new("/no/such/emery-survey-root"), |_, _| true).expect_err("missing");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("reading"), "{error}");
}

// Directories holding at least `floor` files are their own group, in
// directory-name order; smaller directories and the root's own files fold
// into one sorted remainder.
#[test]
fn grain_floor() {
    assert_eq!(
        survey::by_directory(owned(&["a/1.md", "a/2.md", "b/1.md", "root.md"]), 2),
        [owned(&["a/1.md", "a/2.md"]), owned(&["b/1.md", "root.md"])]
    );
}

// When every directory meets the floor, the remainder is only the root's
// own files; when there are none, it is dropped.
#[test]
fn no_empty_remainder() {
    assert_eq!(
        survey::by_directory(owned(&["a/1.md", "a/2.md", "b/1.md"]), 1),
        [owned(&["a/1.md", "a/2.md"]), owned(&["b/1.md"])]
    );
}
