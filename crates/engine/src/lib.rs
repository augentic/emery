//! Provides transport-independent operations for creating, reading, and building Emery revisions.
//!
//! [`specify`] extracts source claims, reconciles requirements by authority,
//! synthesises a specification, design, and build plan, and stages them as
//! one content-addressed candidate. [`apply`] seals the staged candidate as
//! the current revision. [`amend`] matches change sources against the
//! current revision and stages the amendment. [`show`] renders any one
//! document from the current revision, or from the staged candidate, or the
//! staged amendment. [`build`] builds every slice of the current plan
//! through a target adapter in dependency waves.
//!
//! Command-line parsing and presentation are handled outside this crate.

mod adapter;
pub mod amend;
pub mod apply;
mod authority;
mod brief;
pub mod build;
pub mod files;
mod revision;
mod secrets;
pub mod show;
pub mod specify;
mod store;
mod text;
pub mod vcs;

use std::path::{Component, Path, PathBuf};

pub use adapter::{AdapterRef, Axis};
pub use authority::Rank;
pub use files::{Entry, EntryKind};
use omnia_sdk::{Error, ErrorKind, bad_request};
pub use store::{REVISION_KEY, STAGED_CONTAINER, STAGED_KEY};

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
    match error.kind() {
        ErrorKind::BadGateway => true,
        ErrorKind::BadRequest => {
            let description = error.description();
            description.contains("budget exhausted")
                && description.contains("timed out")
                && !description.contains(REJECTED)
        }
        ErrorKind::NotFound | ErrorKind::ServerError | ErrorKind::Conflict => false,
    }
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
/// assert_eq!(preopen_path("./docs/../src")?, Path::new("src"));
/// assert_eq!(preopen_path(".")?, Path::new("."));
/// assert!(preopen_path("../outside").is_err());
/// # Ok::<(), omnia_sdk::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`Error::BadRequest`] for an absolute path, or a relative path that
/// escapes above the project root.
pub fn preopen_path(path: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let path = path.as_ref();
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
pub fn preopen_join(base: impl AsRef<Path>, relative: impl AsRef<Path>) -> Result<PathBuf, Error> {
    preopen_path(base.as_ref().join(relative))
}

// The error with its description rewritten and its class and code kept, so
// a failure gains where it happened without losing what it was.
pub(crate) fn describe(error: Error, rewrite: impl FnOnce(String) -> String) -> Error {
    Error::new(error.kind(), error.code(), rewrite(error.description().into_owned()))
}
