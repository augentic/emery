//! Lists files in a workspace source under an adapter's entry policy.
//!
//! [`list`] walks regular files beneath a source root, asking the adapter's
//! filter about each [`Entry`]. The engine's own `.omnia/` directories and
//! `spec.md` / `design.md` files are never offered.

use std::path::Path;

use anyhow::Context as _;
use omnia_sdk::{Error, bad_request};

/// A workspace entry offered to an adapter's filter by its root-relative path.
///
/// The path is `/`-separated and UTF-8. An entry whose name is not UTF-8 is
/// refused before any entry is offered.
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

    /// Returns the entry's own name: the last segment of its path.
    #[must_use]
    pub fn name(self) -> &'a str {
        let path = self.path();
        path.rsplit_once('/').map_or(path, |(_, name)| name)
    }

    /// Returns the part of the name after its last dot; a leading dot is not one.
    #[must_use]
    pub fn extension(self) -> Option<&'a str> {
        let (stem, extension) = self.name().rsplit_once('.')?;
        (!stem.is_empty()).then_some(extension)
    }

    /// Returns `true` when the entry's name begins with a dot.
    #[must_use]
    pub fn hidden(self) -> bool {
        self.name().starts_with('.')
    }
}

/// Lists the files beneath `root`, sorted, as `/`-separated paths relative to it.
///
/// `keep` is asked about every entry; a refused directory is not entered.
/// The engine's own `.omnia/` directories and `spec.md` / `design.md` files
/// are never offered, wherever they appear. Symlinks are not followed.
///
/// # Examples
///
/// ```
/// use emery_sdk::workspace;
///
/// # let scratch = tempfile::tempdir()?;
/// # for file in ["README.md", "api/orders.md", "api/users.md", "notes/todo.md", ".git/HEAD"] {
/// #     let path = scratch.path().join(file);
/// #     std::fs::create_dir_all(path.parent().unwrap())?;
/// #     std::fs::write(path, "")?;
/// # }
/// # let root = scratch.path().to_str().unwrap();
/// let files = workspace::list(root, |entry| !entry.hidden())?;
///
/// assert_eq!(files, ["README.md", "api/orders.md", "api/users.md", "notes/todo.md"]);
/// # anyhow::Ok(())
/// ```
///
/// # Errors
///
/// Returns [`Error::ServerError`] when a directory cannot be read, and
/// [`Error::BadRequest`] for an entry whose name is not UTF-8, which no
/// `path` anchor could cite.
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

// The engine's own files: output, never input, wherever they sit in a tree.
const SKIP_DIRS: &[&str] = &[".omnia"];
const SKIP_FILES: &[&str] = &["spec.md", "design.md"];

// `dir`'s kept files as `prefix`-relative paths, descending into each kept
// directory.
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
