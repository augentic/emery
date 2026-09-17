//! Provides transport-independent operations for creating and reading Emery revisions.
//!
//! [`specify`] extracts source claims, reconciles requirements by authority,
//! synthesises a specification and design, and commits both as one
//! content-addressed revision. [`show`] renders either document from the
//! current revision.
//!
//! Both operations use a [`Provider`] of model, adapter, storage, and plugin
//! capabilities. Command-line parsing and presentation are handled outside
//! this crate.
//!
//! # Vocabulary
//!
//! - **Revision**: a typed specification and design identified by the digest
//!   of their canonical JSON. Markdown output is a projection of this data.
//! - **Brief**: a typed synthesis question and the checks its answer must
//!   satisfy.
//! - **Basis**: the reconciled claims, authority, and coverage from which a
//!   requirement is built.
//! - **Round**: one attempt to answer a brief. Rejected answers may be returned
//!   to the model for correction until the host's limit is reached.

mod adapter;
mod revision;
pub mod show;
pub mod specify;
mod store;

use std::path::{Component, Path, PathBuf};

pub use adapter::AdapterRef;
use emery_adapter::source::Source;
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, bad_request};
pub use store::{CONTAINER, CURRENT};

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
            // `..` steps back over the segment it follows; with nothing to
            // pop it would escape the root and falls through to the refusal.
            // A root or prefix component is an absolute path.
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

/// A bundle of every capability an engine operation may require.
///
/// Any type implementing the required model, source, storage, and plugin
/// capabilities implements this trait automatically.
pub trait Provider:
    Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static
{
}

impl<P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static> Provider for P {}

// The synthesis corpus, embedded at compile time; `specify::tests::corpus`
// holds the list to the tree and to the briefs that read it.
static PROSE: &[emery_prose::Doc] = emery_prose::prose!(
    "../prose",
    [
        "synthesis/authority.md",
        "synthesis/claim-landing.md",
        "synthesis/design-format.md",
        "synthesis/grouping.md",
        "synthesis/requirement-block.md",
        "synthesis/spec-format.md",
        "synthesis/synthesise.md",
        "synthesis/tags.md",
    ]
);
