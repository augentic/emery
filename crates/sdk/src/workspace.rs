//! Lists workspace files under an adapter-defined filter.
//!
//! [`list`] visits directories and regular files beneath a source root. The
//! filter receives each [`Entry`] and may prune directories or omit files.
//! Emery's `.omnia/` directories and generated documents are always excluded.

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use omnia_sdk::{Error, bad_request};

// The engine's own files: output, never input, wherever they sit in a tree.
const SKIP_DIRS: &[&str] = &[".omnia"];
const SKIP_FILES: &[&str] = &["spec.md", "design.md"];

/// A workspace entry passed to an adapter's filter.
///
/// Paths are UTF-8 and use `/` separators. Encountering a non-UTF-8 name
/// aborts the listing.
///
/// # Examples
///
/// ```
/// use emery_sdk::workspace::Entry;
///
/// let entry = Entry::File("src/lib.rs");
/// assert_eq!(entry.path(), "src/lib.rs");
/// assert_eq!(entry.name(), "lib.rs");
/// assert_eq!(entry.extension(), Some("rs"));
/// assert!(!entry.hidden());
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Entry<'a> {
    /// A directory; refusing it prunes everything beneath.
    Dir(&'a str),
    /// A regular file; refusing it leaves it out of the listing.
    File(&'a str),
}

impl<'a> Entry<'a> {
    /// Returns the `/`-separated path relative to the root.
    #[must_use]
    pub const fn path(self) -> &'a str {
        match self {
            Self::Dir(path) | Self::File(path) => path,
        }
    }

    /// Returns the final segment of the entry's path.
    #[must_use]
    pub fn name(self) -> &'a str {
        let path = self.path();
        path.rsplit_once('/').map_or(path, |(_, name)| name)
    }

    /// Returns the part of the name after its final dot.
    ///
    /// A leading dot does not introduce an extension.
    #[must_use]
    pub fn extension(self) -> Option<&'a str> {
        let (stem, extension) = self.name().rsplit_once('.')?;
        (!stem.is_empty()).then_some(extension)
    }

    /// Returns whether the entry's name begins with a dot.
    #[must_use]
    pub fn hidden(self) -> bool {
        self.name().starts_with('.')
    }
}

/// Returns sorted, root-relative paths for files accepted by `keep`.
///
/// `keep` is asked about every entry; a refused directory is not entered.
/// `.omnia/` directories and generated `spec.md` and `design.md` files are
/// never offered, wherever they occur. Symlinks and non-regular files are not
/// followed or returned.
///
/// # Examples
///
/// ```
/// use emery_sdk::workspace;
///
/// # let scratch = tempfile::tempdir()?;
/// # for file in ["README.md", "api/orders.md", "api/users.md", "notes/todo.md", ".git/HEAD"] {
/// #     let path = scratch.path().join(file);
/// #     let parent = path.parent().ok_or("file has no parent")?;
/// #     std::fs::create_dir_all(parent)?;
/// #     std::fs::write(path, "")?;
/// # }
/// # let root = scratch.path().to_str().ok_or("temporary path is not UTF-8")?;
/// let files = workspace::list(root, |entry| !entry.hidden())?;
///
/// assert_eq!(files, ["README.md", "api/orders.md", "api/users.md", "notes/todo.md"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when an entry name is not UTF-8.
/// - Returns [`Error::ServerError`] when the workspace cannot be read.
pub fn list(root: &str, mut keep: impl FnMut(Entry<'_>) -> bool) -> Result<Vec<String>, Error> {
    let mut files = walk(Path::new(root), "", &mut keep)?;
    files.sort();
    Ok(files)
}

pub(crate) fn excluded(entry: Entry<'_>) -> bool {
    match entry {
        Entry::Dir(_) => SKIP_DIRS.contains(&entry.name()),
        Entry::File(_) => SKIP_FILES.contains(&entry.name()),
    }
}

#[derive(Debug)]
pub(crate) enum Unoffered {
    // A segment is missing, of the wrong kind, or a symlink the walk never follows.
    NoFile,
    Refused,
}

// Reads and offers each step as `list` does, so a path is held to the listing.
pub(crate) fn offered_file(
    root: &str, relative: &str, keep: &mut impl FnMut(Entry<'_>) -> bool,
) -> Result<(), Unoffered> {
    let mut dir = PathBuf::from(root);
    for offered in steps(relative) {
        let found = find_entry(&dir, offered.name()).ok_or(Unoffered::NoFile)?;
        let placed = match offered {
            Entry::Dir(_) => found.file_type().is_ok_and(|kind| kind.is_dir()),
            Entry::File(_) => found.file_type().is_ok_and(|kind| kind.is_file()),
        };
        if !placed {
            return Err(Unoffered::NoFile);
        }
        if excluded(offered) || !keep(offered) {
            return Err(Unoffered::Refused);
        }
        dir = found.path();
    }
    Ok(())
}

fn steps(relative: &str) -> impl Iterator<Item = Entry<'_>> {
    relative
        .match_indices('/')
        .map(|(end, _)| Entry::Dir(&relative[..end]))
        .chain(std::iter::once(Entry::File(relative)))
}

// A `DirEntry` reports a symlink as a symlink; the metadata of the joined
// path would have followed it.
fn find_entry(dir: &Path, name: &str) -> Option<std::fs::DirEntry> {
    let reading = std::fs::read_dir(dir).ok()?;
    reading.filter_map(Result::ok).find(|entry| entry.file_name().to_str() == Some(name))
}

fn walk(
    dir: &Path, prefix: &str, keep: &mut impl FnMut(Entry<'_>) -> bool,
) -> Result<Vec<String>, Error> {
    let reading = || format!("reading `{}`", dir.display());
    let mut found = Vec::new();
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
            let offered = Entry::Dir(&relative);
            if excluded(offered) || !keep(offered) {
                continue;
            }
            found.extend(walk(&entry.path(), &relative, keep)?);
        } else if file_type.is_file() {
            let offered = Entry::File(&relative);
            if !excluded(offered) && keep(offered) {
                found.push(relative);
            }
        }
    }
    Ok(found)
}
