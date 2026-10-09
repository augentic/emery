//! Provides transport-independent operations for creating, reading, and building Emery revisions.
//!
//! [`specify`] extracts source claims, reconciles requirements by authority,
//! synthesises a specification and design, slices the specification into a
//! plan, and commits the three as one content-addressed revision. [`show`]
//! renders any one document from the current revision. [`build`] dispatches
//! every slice of the current plan to a target adapter, in dependency order.
//!
//! Every operation uses a [`Provider`] of model, adapter, storage,
//! version-control, and plugin capabilities. Command-line parsing and
//! presentation are handled outside this crate.
//!
//! # Vocabulary
//!
//! - **Revision**: a typed specification, design, and plan identified by the
//!   digest of their canonical JSON. Markdown output is a projection of this
//!   data.
//! - **Brief**: a typed synthesis question and the checks its answer must
//!   satisfy.
//! - **Basis**: the reconciled claims, authority, and coverage from which a
//!   requirement is built.
//! - **Plan**: the specification divided into slices, each owning the design
//!   types it defines and naming the slices built before it.
//! - **Slice**: a subset of the specification a builder can implement and
//!   verify on its own. Requirements sharing a stem — the first segment of
//!   their subject's dotted id — are never split across slices.
//! - **Round**: one attempt to answer a brief. Rejected answers may be returned
//!   to the model for correction until the host's limit is reached.

mod adapter;
mod authority;
pub mod build;
mod revision;
pub mod show;
pub mod specify;
mod store;
pub mod vcs;

use std::path::{Component, Path, PathBuf};

pub use adapter::{AdapterRef, Axis};
pub use authority::Rank;
use emery_adapter::source::Source;
use emery_adapter::target::Target;
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, Vcs, bad_request};
pub use store::{CONTAINER, REVISION_KEY};

/// Normalises an operator path to a path beneath the `.` project preopen.
///
/// `.` components are dropped and `..` steps back over the segment before it.
/// An empty result is `.`, the project root itself.
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
/// version-control, and plugin capabilities implements this trait
/// automatically.
pub trait Provider:
    Model + Source + Target + StateStore + BlobStore + Plugins + Vcs + Send + Sync + 'static
{
}

impl<P> Provider for P where
    P: Model + Source + Target + StateStore + BlobStore + Plugins + Vcs + Send + Sync + 'static
{
}
