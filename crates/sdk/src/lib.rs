//! Builds Emery source adapters.
//!
//! A source adapter reads a [`SourceInput`] and returns typed [`Evidence`].
//! This crate provides the pieces shared by adapters:
//!
//! - [`source_adapter!`] exports an adapter's metadata and extraction
//!   functions as a WebAssembly component.
//! - [`Context`], [`Seam`], and [`mine`] run extraction over the boundaries
//!   selected by an adapter.
//! - [`workspace::list`] traverses workspace input under an adapter-defined
//!   filter.
//! - [`survey::surfaces`] optionally discovers caller-facing entry points.
//! - [`Doc`], [`prose!`], and [`mod@prose`] embed and inspect adapter guidance;
//!   [`prose::RUNTIME`] is the guidance every adapter shares.
//!
//! Contract types and [`Error`] are re-exported, allowing an adapter to depend
//! on this crate alone.
//!
//! # Examples
//!
//! This adapter treats its input as a single mining seam:
//!
//! ```
//! use emery_sdk::{Doc, Error, Seam, SourceInput, SourceKind};
//!
//! pub const KIND: SourceKind = SourceKind::Intent;
//!
//! pub static PROSE: &[Doc] = &[Doc {
//!     path: "prompts/extract.md",
//!     body: "Extract every requirement the brief states as a `requirement` claim.",
//! }];
//!
//! /// Returns the input as a single mining seam.
//! pub fn survey(_input: &SourceInput) -> Result<Vec<Seam>, Error> {
//!     Ok(vec![Seam::Whole])
//! }
//!
//! #[cfg(target_arch = "wasm32")]
//! mod guest {
//!     use emery_sdk::{AdapterMetadata, Context, Error, Evidence, Model};
//!
//!     emery_sdk::source_adapter!(metadata, extract);
//!
//!     fn metadata() -> AdapterMetadata {
//!         emery_sdk::metadata(super::KIND)
//!     }
//!
//!     async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
//!         let seams = super::survey(ctx.input)?;
//!         emery_sdk::extract(ctx, super::PROSE, &seams).await
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! An adapter lists its `prose/` directory with [`prose!`]
//! (`prose!["../prose/prompts/extract.md", ..]` from `src/lib.rs`) and holds
//! the list to the tree with [`prose::check`], [`prose::RUNTIME`] as the
//! imports. Its prompts link those shared references as `../emery/claims.md`
//! without listing them.
//!
//! # Vocabulary
//!
//! - **Seam**: the portion of a source handled by one model request. See
//!   [`Seam`].
//! - **Survey**: the adapter-specific step that divides an input into seams
//!   before extraction.
//! - **Context**: the adapter identifier, source input, and model available to
//!   one extraction call. See [`Context`].
//! - **Lend**: the workspace directory made readable to the model for a seam.
//! - **Finding**: a validation problem returned to the model for correction.
//!   The host limits how many correction rounds are available.
//!
//! Fallible APIs return [`Error`]. Use [`bad_request!`] when an adapter rejects
//! unusable input.

mod extract;
#[doc(hidden)]
pub mod guest;
mod path;
pub mod prose;
mod references;
pub mod survey;
pub mod workspace;

#[cfg(target_arch = "wasm32")]
#[doc(inline)]
pub use emery_adapter::source::export;
pub use emery_adapter::source::{
    AdapterMetadata, Backing, Claim, ClaimKind, Evidence, Source, SourceContent, SourceInput,
    SourceKind,
};
pub use emery_prose::{Doc, prose};
pub use omnia_sdk::{Error, Model, bad_gateway, bad_request, model, not_found, server_error};

pub use self::extract::{Context, Seam, extract};
#[cfg(target_arch = "wasm32")]
pub use self::guest::Provider;

/// Returns the `metadata` answer for an adapter reading `kind` sources.
///
/// The `emery-version` pin is this SDK's own version, identifying the contract
/// the adapter compiled against. Build an [`AdapterMetadata`] directly only
/// when the adapter must loosen or tighten that pin.
///
/// # Examples
///
/// ```
/// use emery_sdk::{SourceKind, metadata};
///
/// let metadata = metadata(SourceKind::Intent);
/// assert_eq!(metadata.kind, SourceKind::Intent);
/// assert!(metadata.emery_version.is_some());
/// ```
#[must_use]
pub fn metadata(kind: SourceKind) -> AdapterMetadata {
    AdapterMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        kind,
    }
}
