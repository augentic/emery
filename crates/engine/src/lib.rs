//! The specification engine
//!
//! Emery's core: the operations that generate a specification revision from
//! sources ([`specify`]) and read one back for review ([`show`]),
//! together with the source rules and adapter references those
//! operations accept.
//!
//! The engine is transport-neutral. It speaks in typed operations and
//! results over a [`Provider`] of capabilities, and leaves argument parsing,
//! terminal text, and exit codes to whichever front end drives it.

mod adapter;
mod artifact;
pub mod show;
pub mod specify;
mod store;

use std::path::{Component, Path, PathBuf};

pub use adapter::AdapterRef;
use emery_source::Source;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore, bad_request};
pub use store::{CONTAINER, CURRENT};

/// Normalizes an operator path inside the `.` project preopen.
///
/// # Errors
///
/// Returns a `BadRequest` for an absolute path or a relative path that
/// escapes above the project root.
pub fn preopen_path(path: &Path) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        return Err(bad_request!("path `{}` must be relative to the project root", path.display()));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            // `..` steps back over the segment it follows; with nothing to
            // pop it would escape the root and falls through to the refusal.
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

/// Every capability an operation may need, gathered into one bound so a
/// transport can name the provider it binds with a single trait.
pub trait Provider:
    Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static
{
}

impl<P: Model + Source + StateStore + BlobStore + Plugins + Send + Sync + 'static> Provider for P {}

// Generated from the link-checked synthesis corpus at build time.
mod prose {
    emery_prose::registry!();
}
