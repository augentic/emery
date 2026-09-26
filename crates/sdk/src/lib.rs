#![warn(missing_docs, clippy::missing_errors_doc)]

//! Provides the types and functions a source adapter is written with.
//!
//! A source adapter reads a [`SourceInput`] and returns typed [`Evidence`].
//! The crate root holds what every adapter uses; the modules hold what some
//! adapters use:
//!
//! - [`source_adapter!`] exports an adapter's metadata and extraction
//!   functions as a WebAssembly component, and [`metadata`] answers the
//!   first of them.
//! - [`Context`], [`Seam`], and [`extract`](fn@extract) run extraction over the
//!   boundaries selected by an adapter, at most [`CONCURRENT`] at a time.
//! - [`Doc`], [`prose!`], [`body`], and [`find`] embed and read adapter
//!   guidance; [`RUNTIME`] is the guidance every adapter shares, and
//!   [`check`] holds an adapter's list to its tree.
//! - [`workspace::list`] traverses workspace input under an adapter-defined
//!   filter.
//! - [`survey::surfaces`] optionally discovers caller-facing entry points.
//!
//! Contract types and [`Error`] are re-exported, allowing an adapter to depend
//! on this crate alone. [`Source`] is among them for a host program that calls
//! an adapter the way the engine does; an adapter implements the world's guest
//! interface through [`source_adapter!`] and never [`Source`].
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
//!     path: "extract.md",
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
//! # Vocabulary
//!
//! - **Seam**: the portion of a source handled by one model request. See
//!   [`Seam`].
//! - **Survey**: the adapter-specific step that divides an input into seams
//!   before extraction.
//! - **Mine**: to put one seam to the model under the adapter's `extract.md`
//!   and gate its answer. [`extract`](fn@extract) mines every seam of a source.
//! - **Context**: the adapter identifier, source input, and model available to
//!   one extraction call. See [`Context`].
//! - **Lend**: the workspace directory made readable to the model for a seam.
//! - **Finding**: a validation problem returned to the model for correction.
//!   The host limits how many correction rounds are available.
//!
//! Fallible APIs return [`Error`]. Use [`bad_request!`] when an adapter rejects
//! unusable input.
//!
//! Progress is emitted through `tracing`; every event names the source key
//! and, within a seam, its index. An adapter opens at its guest environment's
//! `RUST_LOG`, which the Omnia runtime sets from the run's one tracing level:
//! `info` on a bare `emery` run, so this crate's progress reaches stderr,
//! one step up per `-v` and down per `-q`, and an operator's own `RUST_LOG`
//! (`emery_sdk=debug`, `off`) kept when no flag is passed.

mod extract;
mod reference;
pub mod survey;
pub mod workspace;

#[cfg(target_arch = "wasm32")]
#[doc(inline)]
pub use emery_adapter::source::export;
pub use emery_adapter::source::{
    AdapterMetadata, Backing, Claim, ClaimKind, Evidence, Source, SourceContent, SourceInput,
    SourceKind,
};
pub use emery_prose::{Doc, body, check, find, prose};
pub use omnia_sdk::{Error, Model, bad_gateway, bad_request, not_found, server_error};

pub use self::extract::{CONCURRENT, Seam, extract};

/// The runtime references every adapter prompt may link.
///
/// `claims.md` defines claims and their gate; `reconciliation.md` explains
/// where accepted claims land. Prompts link these paths without listing them
/// in their own table; pass [`RUNTIME`] to [`check`] as imports.
pub static RUNTIME: &[Doc] = prose!["../prose/claims.md", "../prose/reconciliation.md"];

/// Exports an adapter's metadata and extraction functions as a component.
///
/// The arguments must identify functions with these signatures:
///
/// - `fn() -> AdapterMetadata`
/// - `async fn<P: Model>(&Context<'_, P>) -> Result<Evidence, Error>`
///
/// The macro supplies a [`Context`] containing the imported source input and
/// host model. It then converts the returned evidence or error into the
/// `source-adapter` WIT records.
///
/// Invoke this macro inside a `#[cfg(target_arch = "wasm32")]` module because
/// the export interface exists only on WebAssembly targets. Adapters needing
/// custom guest behaviour may implement `export::Guest` directly.
#[macro_export]
macro_rules! source_adapter {
    ($metadata:path, $extract:path $(,)?) => {
        const _: () = {
            struct Adapter;
    $crate::export::export!(Adapter with_types_in $crate::export);

            impl $crate::export::Guest for Adapter {
                fn metadata(_id: $crate::export::AdapterId) -> $crate::export::AdapterMetadata {
                    $crate::export::AdapterMetadata::from($metadata())
                }

                async fn extract(
                    id: $crate::export::AdapterId, input: $crate::export::Input,
                ) -> Result<$crate::export::Evidence, $crate::export::Error> {
                    $crate::call($extract, id, input).await
                }
            }
        };
    };
}

/// The host model an extraction is put to on WebAssembly.
///
/// [`source_adapter!`] binds it into every [`Context`] it builds; the empty
/// [`Model`] impl delegates each request through Omnia's WASI interface.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Default)]
pub struct Provider;

#[cfg(target_arch = "wasm32")]
impl Model for Provider {}

/// The `extract` arm of [`source_adapter!`], reached through the macro alone.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[omnia_wasi_otel::instrument(name = "source_adapter_extract")]
pub async fn call(
    extract: impl AsyncFnOnce(&Context<'_, Provider>) -> Result<Evidence, Error>,
    id: export::AdapterId, input: export::Input,
) -> Result<export::Evidence, export::Error> {
    let input = SourceInput::from(input);
    let ctx = Context {
        adapter_id: &id,
        input: &input,
        model: &Provider,
    };
    Ok(extract(&ctx).await?.into())
}

/// Returns the `metadata` answer for an adapter reading `kind` sources.
///
/// The `emery-version` pin is this SDK's own version, identifying the contract
/// the adapter compiled against. Build an [`AdapterMetadata`] directly only
/// when the adapter must loosen or tighten that pin.
#[must_use]
pub fn metadata(kind: SourceKind) -> AdapterMetadata {
    AdapterMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        kind,
    }
}

/// The adapter addressed, its input, and the model available to one extraction call.
///
/// [`extract`](fn@extract) and [`survey::surfaces`] both take it, so an adapter's survey
/// and its extraction put their turns to the same model.
#[derive(Debug)]
pub struct Context<'a, P> {
    /// The identifier used to address the adapter.
    pub adapter_id: &'a str,
    /// The [`SourceInput`] identifying the source and its content.
    pub input: &'a SourceInput,
    /// The [`Model`] used for survey and extraction requests.
    pub model: &'a P,
}

// A missing prompt is the adapter build's own defect, reported before a turn
// is spent.
fn prompt(docs: &[Doc], path: &str) -> Result<&'static str, Error> {
    body(docs, path).ok_or_else(|| server_error!("`{path}` is not embedded"))
}

fn beneath(path: &str) -> Result<String, &'static str> {
    if path.starts_with('/') || path.split('/').any(|segment| segment == "..") {
        return Err("escapes the source root");
    }

    let segments: Vec<&str> =
        path.split('/').filter(|segment| !segment.is_empty() && *segment != ".").collect();
    if segments.is_empty() {
        return Err("names no file");
    }

    Ok(segments.join("/"))
}
