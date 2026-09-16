//! The SDK an Emery source adapter is written against.
//!
//! A source adapter is a WebAssembly component that reads one kind of source
//! — a document tree, a codebase, a written brief — and returns typed claims
//! about it. It is a guest of the `source-adapter` world: it implements the
//! world's `Guest` from the `export` module and answers the two calls with
//! what this crate supplies — `metadata` for the kind of source it reads, and
//! `extract`, which runs [`mine`] over the [seams](#vocabulary) the adapter's
//! own survey chose. Adapter code is left with what is specific to its
//! source: the kind it reads, the documents it embeds, and how its input
//! cuts.
//!
//! The contract types come from `emery-adapter` and are re-exported here;
//! on `wasm32`, so is the world the adapter exports through (`export`).
//! [`Source`], the capability the engine calls adapters through, is
//! re-exported for a program that drives an adapter the way the engine does;
//! an adapter implements the world's `Guest`, never `Source`. The embedded
//! documents come from `emery-prose` and are re-exported too — [`Doc`],
//! [`include_prose!`], and the lookups in [`mod@prose`] — so an adapter's
//! `[dependencies]` is this crate alone.
//!
//! # Examples
//!
//! The smallest complete adapter declares the kind of source it reads, embeds
//! its prompt, keeps a brief whole, and exports the world on `wasm32` alone
//! — so the crate builds natively and its survey is tested there:
//!
//! ```
//! use emery_sdk::{Context, Doc, Error, Seam, SourceKind};
//!
//! pub const KIND: SourceKind = SourceKind::Intent;
//!
//! pub static DOCS: &[Doc] = &[Doc {
//!     path: "prompts/extract.md",
//!     body: "Extract every requirement the brief states as a `requirement` claim.",
//! }];
//!
//! /// Returns the seams to mine: a brief is never split.
//! pub fn survey(_ctx: &Context<'_>) -> Result<Vec<Seam>, Error> {
//!     Ok(vec![Seam::Whole])
//! }
//!
//! #[cfg(target_arch = "wasm32")]
//! mod guest {
//!     use emery_sdk::export::{self, AdapterId, AdapterMetadata, Error, Evidence, Guest, Input};
//!
//!     struct Adapter;
//!     export::export!(Adapter with_types_in export);
//!
//!     impl Guest for Adapter {
//!         fn metadata(_id: AdapterId) -> AdapterMetadata {
//!             emery_sdk::metadata(super::KIND)
//!         }
//!
//!         async fn extract(id: AdapterId, input: Input) -> Result<Evidence, Error> {
//!             emery_sdk::extract(id, input, super::DOCS, async |ctx| super::survey(ctx)).await
//!         }
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! A shipped adapter embeds its `prose/` tree with
//! `include_prose!("../prose")` in its guest module rather than writing the
//! table by hand, and a tree adapter cuts its input with the [`survey`]
//! helpers.
//!
//! # Vocabulary
//!
//! - **Source**: what one adapter is asked to read — a directory or an inline
//!   value — under the key the specification cites it by.
//! - **Claim**, **evidence**: one typed statement about the source, and the
//!   document of claims an adapter returns. The **claim gate**
//!   ([`Evidence::findings`]) is the set of rules every claim must satisfy.
//! - **Seam**: the part of a source one model call is asked about. The
//!   **survey** is the adapter's own choice of seams, made before any call;
//!   a seam is **mined** ([`mine`]) when the model is asked about it.
//! - **Lend**: the directory the model may read during a call — the source
//!   root, or a seam's own directory.
//! - **Findings**, **rounds**: the claim gate's report on an answer, sent back
//!   to the model so it can answer again; the host bounds how many rounds a
//!   call gets.
//!
//! Every failure is omnia's [`Error`]. An adapter refuses input it cannot use
//! with [`bad_request!`] and reports anything else with the sibling macros;
//! there is no adapter error type.

mod mine;
mod references;
pub mod survey;

#[cfg(target_arch = "wasm32")]
#[doc(inline)]
pub use emery_adapter::source::export;
pub use emery_adapter::source::{
    AdapterMetadata, Backing, Claim, ClaimKind, Evidence, Source, SourceContent, SourceInput,
    SourceKind,
};
pub use emery_prose::{Doc, include_prose};
pub use omnia_sdk::{Error, Model, bad_gateway, bad_request, model, not_found, server_error};

/// Lookups over an adapter's embedded documents, by tree-relative path.
pub mod prose {
    pub use emery_prose::{body, find};
}

pub use self::mine::{Context, Seam, mine};

/// Returns the `metadata` answer for an adapter reading `kind` sources.
///
/// The `emery-version` pin is this SDK's own version: the contract the
/// adapter compiled against.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn metadata(kind: SourceKind) -> export::AdapterMetadata {
    AdapterMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        kind,
    }
    .into()
}

/// Answers `extract`: lifts `input`, surveys it, and mines the seams over the host model.
///
/// `survey` is the adapter's own choice of [seams](crate#vocabulary), given
/// the call's [`Context`]; everything else is [`mine`] under the
/// `prompts/extract.md` among `docs`, over the host's model. The outcome is
/// lowered onto the world's `evidence` and `error`. A mechanical survey is a
/// plain fn, passed as `async |ctx| survey::survey(ctx)`; a survey that asks
/// the model is already async, so the guest passes `survey::survey`.
///
/// # Errors
///
/// Whatever `survey` or [`mine`] returns, lowered onto the WIT `error`.
#[cfg(target_arch = "wasm32")]
pub async fn extract(
    id: export::AdapterId, input: export::Input, docs: &'static [Doc],
    survey: impl AsyncFnOnce(&Context<'_>) -> Result<Vec<Seam>, Error>,
) -> Result<export::Evidence, export::Error> {
    let input = SourceInput::from(input);
    let ctx = Context {
        adapter_id: &id,
        input: &input,
    };
    let seams = survey(&ctx).await?;
    Ok(mine(&omnia_sdk::model::WasiModel, &ctx, docs, &seams).await?.into())
}
