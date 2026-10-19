//! Provides transport-independent operations for creating, reading, and building Emery revisions.
//!
//! [`specify`] extracts source claims, reconciles requirements by authority,
//! synthesises a specification, design, and build plan, and commits them as
//! one content-addressed revision. [`show`] renders any one document from the
//! current revision. [`build`] builds every slice of the current plan through
//! a target adapter in dependency waves.
//!
//! Every operation uses a [`Provider`] of model, adapter, storage,
//! version-control, file, and plugin capabilities. Command-line parsing and
//! presentation are handled outside this crate.

mod adapter;
mod authority;
mod brief;
pub mod build;
mod revision;
pub mod show;
pub mod specify;
mod store;
pub mod vcs;

use std::future::Future;
use std::path::{Component, Path, PathBuf};

pub use adapter::{AdapterRef, Axis};
pub use authority::Rank;
use emery_adapter::source::Source;
use emery_adapter::target::Target;
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, Vcs, bad_request};
pub use store::{CONTAINER, REVISION_KEY};

// How many times a question to the model, a slice's build, or a wave's
// verification is put when it fails upstream or runs out of the backend's
// time: the first failure is put once more, the second is the run's.
const RETRIES: usize = 2;

// The heading the backend's correction turn opens with, which a question
// that spent its rounds on a rejected candidate carries whole.
const REJECTED: &str = "## Previous answer (rejected)";

/// Returns whether a failed model turn is one the engine puts once more.
///
/// An upstream failure is, and so is a turn the model backend's time budget
/// ended: the model's work is not the same twice, and a turn put again over
/// the same tree starts from what the first left. A question refused on a
/// rejected candidate is not, since its description carries that candidate,
/// whose text may spell a timeout of its own. The engine reads this once per
/// retried dispatch and brief, and the CLI once more to hint at the resume.
#[must_use]
pub fn is_retryable(error: &Error) -> bool {
    match error {
        Error::BadGateway { .. } => true,
        Error::BadRequest { description, .. } => {
            description.contains("budget exhausted")
                && description.contains("timed out")
                && !description.contains(REJECTED)
        }
        Error::NotFound { .. } | Error::ServerError { .. } => false,
    }
}

/// Reads and writes what a commit would carry at a path beneath a working copy the engine cut.
///
/// `at` is a working copy relative to the project root, as the engine names
/// it to [`Vcs`], and `path` a path relative to that copy, as
/// [`Vcs::pending`] lists it. A build reads what it is about to seal, and
/// what the tree it starts from holds, through this capability alone, and
/// writes through it alone: the one file its own repository starts from, and
/// the files the model resolves a slice's merge conflicts to.
pub trait Files: Send + Sync {
    /// Reads what a commit would carry at `path` beneath `at`.
    ///
    /// A regular file reads as its bytes. A symbolic link reads as the
    /// target it names, never followed, since that text is what a commit
    /// stores of it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when `path` holds neither, a directory
    /// or nothing at all, or what it holds cannot be read.
    fn read(&self, at: &str, path: &str) -> impl Future<Output = Result<Vec<u8>, Error>> + Send;

    /// Lists what the directory at `path` beneath `at` holds, each entry by
    /// name and kind, in no particular order.
    ///
    /// A symbolic link is listed as a link whatever it leads to, never
    /// followed, since a link is what a commit stores of it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when `path` is no directory or cannot
    /// be read.
    fn list(&self, at: &str, path: &str) -> impl Future<Output = Result<Vec<Entry>, Error>> + Send;

    /// Writes `bytes` as the whole file at `path` beneath `at`, creating or replacing it.
    ///
    /// The directories on the way are created where they are missing. No
    /// symbolic link is followed: one standing at the path or on the way to
    /// it refuses the write, so what the write puts is what the commit will
    /// carry at `path` and nothing outside the copy is reached.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when a link stands at the path or on
    /// the way to it, a file stands where a directory is needed, or the
    /// file cannot be written.
    fn write(
        &self, at: &str, path: &str, bytes: &[u8],
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

/// One entry of a directory a build reads through [`Files::list`].
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

/// Normalises an operator path to a path beneath the `.` project preopen.
///
/// `.` components are dropped and `..` steps back over the segment before it.
/// An empty result is `.`, the project root itself.
///
/// This is the operator's rule, over a path typed at the command line or in
/// `emery.toml`, where `..` is a step back as a shell reads it. The
/// adapter's rule for a path a claim or a tool names,
/// [`emery_adapter::beneath`], refuses `..` outright, since a model's path is
/// held to the spelling the tree holds.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use emery_engine::preopen_path;
///
/// assert_eq!(preopen_path(Path::new("./docs/../src"))?, Path::new("src"));
/// assert_eq!(preopen_path(Path::new("."))?, Path::new("."));
/// assert!(preopen_path(Path::new("../outside")).is_err());
/// # Ok::<(), omnia_sdk::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`Error::BadRequest`] for an absolute path, or a relative path that
/// escapes above the project root.
pub fn preopen_path(path: &Path) -> Result<PathBuf, Error> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir if normalized.pop() => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(bad_request!(
                    "path `{}` must be relative to the project root",
                    path.display()
                ));
            }
        }
    }

    Ok(if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized })
}

/// Normalises `relative` beneath the project preopen, resolved from `base`.
///
/// # Errors
///
/// Returns [`Error::BadRequest`] when the joined path escapes the project root.
pub fn preopen_join(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative))
}

/// A bundle of every capability an engine operation may require.
///
/// Any type implementing the required model, source, target, storage,
/// version-control, file, and plugin capabilities implements this trait
/// automatically.
pub trait Provider:
    Model + Source + Target + StateStore + BlobStore + Plugins + Vcs + Files + Send + Sync + 'static
{
}

impl<P> Provider for P where
    P: Model
        + Source
        + Target
        + StateStore
        + BlobStore
        + Plugins
        + Vcs
        + Files
        + Send
        + Sync
        + 'static
{
}
