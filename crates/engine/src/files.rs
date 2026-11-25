//! Reads and writes what a commit would carry beneath a working copy.
//!
//! `at` is a working copy relative to the project root, as the engine names
//! it to version control, and `path` a path relative to that copy. A build
//! reads what it is about to seal through these functions alone, and writes
//! through them alone.

use std::path::Path;
use std::{fs, io};

use emery_adapter::{standing, write_whole};
use omnia_sdk::{Error, server_error};

/// One entry of a directory a build lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The entry's name within its directory.
    pub name: String,
    /// What stands at the entry.
    pub kind: EntryKind,
}

/// What stands at an [`Entry`] of a directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A symbolic link, wherever it leads.
    Link,
}

/// Reads what a commit would carry at `path` beneath `at`.
///
/// # Errors
///
/// Returns [`omnia_sdk::ErrorKind::ServerError`] when `path` holds neither, a directory
/// or nothing at all, or what it holds cannot be read.
pub async fn read(at: &str, path: &str) -> Result<Vec<u8>, Error> {
    let outcome = carried(&Path::new(at).join(path))
        .map_err(|error| server_error!("reading `{path}` under `{at}`: {error}"));
    async move { outcome }.await
}

/// Lists what the directory at `path` beneath `at` holds.
///
/// # Errors
///
/// Returns [`omnia_sdk::ErrorKind::ServerError`] when `path` is no directory or cannot
/// be read.
pub async fn list(at: &str, path: &str) -> Result<Vec<Entry>, Error> {
    let outcome = listed(&Path::new(at).join(path))
        .map_err(|error| server_error!("listing `{path}` under `{at}`: {error}"));
    async move { outcome }.await
}

/// Writes `bytes` as the whole file at `path` beneath `at`.
///
/// # Errors
///
/// Returns [`omnia_sdk::ErrorKind::ServerError`] when a link stands at the path or on
/// the way to it, a file stands where a directory is needed, or the
/// file cannot be written.
pub async fn write(at: &str, path: &str, bytes: &[u8]) -> Result<(), Error> {
    let outcome = written(Path::new(at), path, bytes)
        .map_err(|error| server_error!("writing `{path}` under `{at}`: {error}"));
    async move { outcome }.await
}

fn written(root: &Path, path: &str, bytes: &[u8]) -> io::Result<()> {
    let standing = standing(root, path).map_err(io::Error::other)?;
    write_whole(root, path, standing, bytes)
}

fn listed(full: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(full)? {
        let entry = entry?;
        let kind = fs::symlink_metadata(entry.path())?.file_type();
        let kind = if kind.is_symlink() {
            EntryKind::Link
        } else if kind.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::File
        };
        entries.push(Entry {
            name: entry.file_name().to_string_lossy().into_owned(),
            kind,
        });
    }
    Ok(entries)
}

fn carried(full: &Path) -> io::Result<Vec<u8>> {
    let kind = fs::symlink_metadata(full)?.file_type();
    if kind.is_symlink() {
        Ok(fs::read_link(full)?.into_os_string().into_encoded_bytes())
    } else if kind.is_file() {
        fs::read(full)
    } else {
        Err(io::Error::other("neither a file nor a symbolic link stands there"))
    }
}
