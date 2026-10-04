//! Defines the [`Target`] capability and the records supplied to an adapter.
//!
//! The engine uses [`Target`] to query a loaded adapter's metadata and build
//! a slice into a workspace. WebAssembly builds dispatch through the imported
//! adapter interface; native builds require an implementation.

use std::future::Future;

use omnia_sdk::Error;

use crate::target::Report;

/// The engine capability for querying and invoking target adapters.
///
/// Adapter components implement the guest interface rather than this trait.
/// The WebAssembly implementation classifies an input refusal as
/// [`Error::BadRequest`] and any other adapter failure as
/// [`Error::BadGateway`].
///
/// # Examples
///
/// A native host can provide adapters directly:
///
/// ```
/// use std::future::{Future, ready};
///
/// use emery_adapter::target::{Report, Slice, Target, TargetMetadata};
/// use omnia_sdk::Error;
///
/// struct Host;
///
/// impl Target for Host {
///     fn build(
///         &self, _id: &str, slice: &Slice, _workspace: &str,
///     ) -> impl Future<Output = Result<Report, Error>> + Send {
///         ready(Ok(Report {
///             covered: slice.requirements.clone(),
///             written: vec!["src/lib.rs".to_string()],
///         }))
///     }
///
///     fn metadata(&self, _id: &str) -> TargetMetadata {
///         TargetMetadata { emery_version: None }
///     }
/// }
/// ```
pub trait Target: Send + Sync {
    /// Builds `slice` into the tree at `workspace` using the adapter registered as `id`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] for any other adapter failure.
    #[cfg(not(target_arch = "wasm32"))]
    fn build(
        &self, id: &str, slice: &Slice, workspace: &str,
    ) -> impl Future<Output = Result<Report, Error>> + Send;

    /// Builds `slice` into the tree at `workspace` using the adapter registered as `id`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] for any other adapter failure.
    #[cfg(target_arch = "wasm32")]
    fn build(
        &self, id: &str, slice: &Slice, workspace: &str,
    ) -> impl Future<Output = Result<Report, Error>> + Send {
        crate::target::bindings::import::build(id, slice, workspace)
    }

    /// Returns the metadata the adapter registered as `id` declares.
    #[cfg(not(target_arch = "wasm32"))]
    fn metadata(&self, id: &str) -> TargetMetadata;

    /// Returns the metadata the adapter registered as `id` declares.
    #[cfg(target_arch = "wasm32")]
    fn metadata(&self, id: &str) -> TargetMetadata {
        crate::target::bindings::import::metadata(id)
    }
}

/// One slice of the build plan, as the engine renders it for a target.
///
/// The documents are the Markdown `emery show` prints, the specification cut
/// to the slice's requirements. `requirements` carries their ids typed, so a
/// [`Report`]'s `covered` is held to them without reading the Markdown back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slice {
    /// The stable slice identifier, `SLICE-001`.
    pub id: String,
    /// The drafted kebab-case name.
    pub name: String,
    /// The ids of the requirements the slice builds, `REQ-001`.
    pub requirements: Vec<String>,
    /// The specification, cut to the slice.
    pub spec: String,
    /// The whole design.
    pub design: String,
    /// The slice's own plan entry.
    pub plan: String,
}

/// Metadata declared by a target adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetMetadata {
    /// The minimum compatible Emery version, if the adapter declares one.
    pub emery_version: Option<String>,
}
