//! The SDK an Emery source adapter is written against.
//!
//! A source adapter is a WebAssembly component that reads one kind of source
//! — a document tree, a codebase, a written brief — and returns typed claims
//! about it. It is a guest of the `source-adapter` world, exported through
//! [`source_adapter!`] over two plain fns of the adapter's own: `metadata`,
//! answered with [`metadata`] for the kind of source it reads, and `extract`,
//! which runs [`mine`] over the [seams](#vocabulary) the adapter's own survey
//! chose. Every call arrives as a [`Context`] — the adapter addressed, the
//! input, and the model that answers — so adapter code is left with what is
//! specific to its source: the kind it reads, the documents it embeds, and
//! how its input cuts. It names no backend: the guest's lift puts the host's
//! model in the `Context`, and a native test puts a scripted one there.
//!
//! The contract types come from `emery-adapter` and are re-exported here. On
//! `wasm32` the crate also carries the world's bindings (`export`), which the
//! macro expands against and a guest written by hand implements directly,
//! and `Provider`, the host's model on omnia's WASI defaults, which the
//! macro's lift binds into every call.
//! [`Source`], the capability the engine calls adapters through, is
//! re-exported for a program that drives an adapter the way the engine does;
//! an adapter exports the world, never implements `Source`. The embedded
//! documents come from `emery-prose` and are re-exported too — [`Doc`],
//! [`include_prose!`], and the lookups in [`mod@prose`] — so an adapter's
//! `[dependencies]` is this crate alone.
//!
//! # Examples
//!
//! The smallest complete adapter declares the kind of source it reads, embeds
//! its prompt, keeps a brief whole, and exports the world on `wasm32` alone —
//! so the crate builds natively and its survey is tested there:
//!
//! ```
//! use emery_sdk::{Doc, Error, Seam, SourceInput, SourceKind};
//!
//! pub const KIND: SourceKind = SourceKind::Intent;
//!
//! pub static DOCS: &[Doc] = &[Doc {
//!     path: "prompts/extract.md",
//!     body: "Extract every requirement the brief states as a `requirement` claim.",
//! }];
//!
//! /// Returns the seams to mine: a brief is never split.
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
//!         emery_sdk::mine(ctx, super::DOCS, &seams).await
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! A shipped adapter embeds its `prose/` tree with
//! `include_prose!("../prose")` in its guest module rather than writing the
//! table by hand, and a tree adapter lists its input through
//! [`survey::list`] or asks the model for its surfaces through
//! [`survey::surfaces`].
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
//! - **Context**: what one call knows ([`Context`]) — the adapter addressed,
//!   the input, and the model every turn of the call is put to.
//! - **Lend**: the directory the model may read during a call — the source
//!   root, for every seam of a workspace.
//! - **Findings**, **rounds**: the claim gate's report on an answer, sent back
//!   to the model so it can answer again; the host bounds how many rounds a
//!   call gets.
//!
//! Every failure is omnia's [`Error`]. An adapter refuses input it cannot use
//! with [`bad_request!`] and reports anything else with the sibling macros;
//! there is no adapter error type.

#[doc(hidden)]
pub mod guest;
mod mine;
mod path;
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

#[cfg(target_arch = "wasm32")]
pub use self::guest::Provider;

/// Lookups over an adapter's embedded documents, by tree-relative path.
pub mod prose {
    pub use emery_prose::{body, find};
}

pub use self::mine::{Context, Seam, mine};

/// Returns the `metadata` answer for an adapter reading `kind` sources.
///
/// The `emery-version` pin is this SDK's own version: the contract the
/// adapter compiled against. An adapter builds an [`AdapterMetadata`] itself
/// only to loosen or tighten that pin.
#[must_use]
pub fn metadata(kind: SourceKind) -> AdapterMetadata {
    AdapterMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        kind,
    }
}
