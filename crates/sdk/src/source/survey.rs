//! The mechanical survey
//!
//! What a tree adapter does before its first model call: list the files
//! beneath its root and cut them into the materials it will mine. The walk
//! honours the engine's own skip roots — `spec.md`, `design.md`, `.omnia/` —
//! wherever they appear, so no adapter can mine a projection of the last
//! revision back into evidence; every other choice of entry is the
//! adapter's, asked per entry. The cut is by top-level directory under a
//! grain floor: a directory too small to be worth its own model call folds,
//! with the root's own files, into one remainder.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context as _;
use omnia_guest::{Error, bad_request};

/// A directory entry the walk asks an adapter about.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Entry {
    /// A directory; refusing it prunes everything beneath.
    Dir,
    /// A regular file; refusing it leaves it out of the survey.
    File,
}

/// The files beneath `root`, sorted and named relative to it with `/`.
///
/// `keep` is asked for every entry with its root-relative path and kind; a
/// refused directory is not entered. The engine's skip roots are never
/// offered: `.omnia/` directories and `spec.md` / `design.md` files are
/// pruned wherever they appear. Symlinks are not followed.
///
/// # Errors
///
/// `ServerError` when a directory cannot be read; `BadRequest` for an entry
/// whose name is not UTF-8, which no `path` anchor could cite.
pub fn files(
    root: &Path, mut keep: impl FnMut(&Path, Entry) -> bool,
) -> Result<Vec<String>, Error> {
    let mut found = Vec::new();
    walk(root, "", &mut keep, &mut found)?;
    found.sort();
    Ok(found)
}

/// `files` cut by top-level directory under a grain `floor`.
///
/// One group per directory holding at least `floor` files, in lexicographic
/// order of directory name, then one remainder holding the root's own files
/// and every smaller directory's, sorted; an empty remainder is dropped.
/// Files keep the order they arrived in within a group, so sorted input
/// yields sorted groups.
#[must_use]
pub fn by_directory(files: Vec<String>, floor: usize) -> Vec<Vec<String>> {
    let mut directories: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut remainder = Vec::new();
    for file in files {
        match file.split_once('/') {
            Some((directory, _)) => directories.entry(directory.to_owned()).or_default().push(file),
            None => remainder.push(file),
        }
    }

    let mut groups = Vec::with_capacity(directories.len() + 1);
    for group in directories.into_values() {
        if group.len() >= floor {
            groups.push(group);
        } else {
            remainder.extend(group);
        }
    }
    if !remainder.is_empty() {
        remainder.sort();
        groups.push(remainder);
    }
    groups
}

// The engine's own files: output, never input, wherever they sit in a tree.
const SKIP_DIRS: &[&str] = &[".omnia"];
const SKIP_FILES: &[&str] = &["spec.md", "design.md"];

// `dir`'s entries into `found` as `prefix`-relative paths, descending into
// each kept directory.
fn walk(
    dir: &Path, prefix: &str, keep: &mut impl FnMut(&Path, Entry) -> bool, found: &mut Vec<String>,
) -> Result<(), Error> {
    let reading = || format!("reading `{}`", dir.display());
    for entry in std::fs::read_dir(dir).with_context(reading)? {
        let entry = entry.with_context(reading)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(bad_request!(
                "`{}` is not UTF-8; no `path` anchor could cite it",
                entry.path().display()
            ));
        };
        let file_type = entry.file_type().with_context(reading)?;
        let relative = if prefix.is_empty() { name.to_owned() } else { format!("{prefix}/{name}") };

        if file_type.is_dir() {
            if SKIP_DIRS.contains(&name) || !keep(Path::new(&relative), Entry::Dir) {
                continue;
            }
            walk(&entry.path(), &relative, keep, found)?;
        } else if file_type.is_file()
            && !SKIP_FILES.contains(&name)
            && keep(Path::new(&relative), Entry::File)
        {
            found.push(relative);
        }
    }
    Ok(())
}
