//! Defines the [`Target`] capability and the records supplied to an adapter.
//!
//! The engine uses [`Target`] to query a loaded adapter's metadata, build a
//! slice into a workspace, and verify the integrated tree. WebAssembly builds
//! dispatch through the imported adapter interface; native builds require an
//! implementation.

use std::borrow::Cow;
use std::future::Future;

use omnia_sdk::Error;

use crate::target::{Report, Verdict};

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
/// use emery_adapter::target::{Report, Slice, Target, TargetMetadata, Verdict};
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
///     fn verify(
///         &self, _id: &str, _workspace: &str,
///     ) -> impl Future<Output = Result<Verdict, Error>> + Send {
///         ready(Ok(Verdict {
///             passed: true,
///             failures: Vec::new(),
///         }))
///     }
///
///     fn metadata(&self, _id: &str) -> TargetMetadata {
///         TargetMetadata {
///             emery_version: None,
///             merge_rules: Vec::new(),
///         }
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

    /// Verifies the integrated tree at `workspace` using the adapter registered as `id`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] for any other adapter failure.
    #[cfg(not(target_arch = "wasm32"))]
    fn verify(
        &self, id: &str, workspace: &str,
    ) -> impl Future<Output = Result<Verdict, Error>> + Send;

    /// Verifies the integrated tree at `workspace` using the adapter registered as `id`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] for any other adapter failure.
    #[cfg(target_arch = "wasm32")]
    fn verify(
        &self, id: &str, workspace: &str,
    ) -> impl Future<Output = Result<Verdict, Error>> + Send {
        crate::target::bindings::import::verify(id, workspace)
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
    /// The commit the lent tree sits on: the integrated head the slice
    /// builds over.
    pub base: String,
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
    /// The rules every slice is merged into the integrated tree under; the
    /// first rule matching a conflicting path applies.
    pub merge_rules: Vec<MergeRule>,
}

/// One rule for merging a slice into the integrated tree.
///
/// An adapter spells its rules as a `const` slice, so `paths` borrows a
/// literal there and owns what the contract carries across.
///
/// # Examples
///
/// ```
/// use std::borrow::Cow;
///
/// use emery_adapter::target::{MergeRule, MergeStrategy};
///
/// const RULES: &[MergeRule] = &[MergeRule {
///     paths: Cow::Borrowed("src/*/index.ts"),
///     strategy: MergeStrategy::Union,
/// }];
/// assert_eq!(RULES[0].paths, "src/*/index.ts");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeRule {
    /// A glob over paths relative to the tree root.
    pub paths: Cow<'static, str>,
    /// How a conflict at a matching path is resolved.
    pub strategy: MergeStrategy,
}

/// How a merge resolves a conflict at a path a [`MergeRule`] matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Both sides' lines kept, each once: declaration and import lists.
    Union,
    /// The integrated tree's side kept whole, for the build to regenerate:
    /// lockfiles.
    Ours,
    /// The slice's side kept whole.
    Theirs,
}
