//! Emery's engine: the operations that write and read a specification revision.
//!
//! [`specify`] extracts typed claims from a run's sources, derives the
//! requirements under authority precedence, synthesises a specification and a
//! design, and commits the pair as one content-addressed revision. [`show`]
//! reads a document of the current revision back as Markdown. Both are typed
//! operations over a [`Provider`] of capabilities; argument parsing, terminal
//! text, and exit codes belong to whichever front end drives them.
//!
//! # Vocabulary
//!
//! - **Revision**: the specification and design one run commits, identified
//!   by the digest of their canonical JSON. The revision is the truth; the
//!   Markdown an operator reads is a projection of it.
//! - **Brief**: one typed question put to the model during synthesis, with
//!   the checks its answer must pass before it is accepted. A run puts up to
//!   three: how the requirement claims group, the draft of `spec.md`, and the
//!   draft of `design.md`.
//! - **Basis**: what one requirement is built on before any prose is drafted —
//!   its contributing claims grouped into agreeing classes, ranked by the
//!   authority of their sources.
//! - **Rounds**: an answer that fails its checks goes back to the model with
//!   the findings; the host bounds how many rounds a brief gets.

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

/// Every capability an operation may need, as one bound.
///
/// A transport names the provider it binds with this single trait. Any type
/// carrying all of the capabilities implements it.
pub trait Provider:
    Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static
{
}

impl<P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static> Provider for P {}

// The link-checked synthesis corpus, embedded at compile time.
static DOCS: &[emery_prose::Doc] = emery_prose::include_prose!("../prose");
