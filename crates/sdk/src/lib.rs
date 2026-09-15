//! The SDK an Emery source adapter is written against.
//!
//! A source adapter is a WebAssembly component that reads one kind of source
//! — a document tree, a codebase, a written brief — and returns typed claims
//! about it. An adapter implements [`SourceAdapter`] and invokes [`source!`];
//! this crate supplies the rest: the contract types, the model call and its
//! claim gate, the error vocabulary, and the component export. Adapter code is
//! left with what is specific to its source.
//!
//! The contract types come from `emery-adapter` and are re-exported here, so
//! an adapter never sees the WIT bindings. [`Source`], the capability the
//! engine calls adapters through, is re-exported for a program that drives an
//! adapter the way the engine does; an adapter implements [`SourceAdapter`]
//! and never `Source`.
//!
//! # Examples
//!
//! The smallest complete adapter declares the kind of source it reads, embeds
//! its prompt, and leaves the survey at its default of one material:
//!
//! ```
//! use emery_prose::registry::Doc;
//! use emery_sdk::{SourceAdapter, SourceKind};
//!
//! static DOCS: &[Doc] = &[Doc {
//!     path: "prompts/extract.md",
//!     body: "Extract every requirement the brief states as a `requirement` claim.",
//! }];
//!
//! struct Adapter;
//!
//! impl SourceAdapter for Adapter {
//!     const KIND: SourceKind = SourceKind::Intent;
//!
//!     fn docs() -> &'static [Doc] {
//!         DOCS
//!     }
//! }
//!
//! emery_sdk::source!(crate::Adapter);
//! # fn main() {}
//! ```
//!
//! A shipped adapter embeds its prompt with `emery_prose::emit` and
//! `emery_prose::registry!` rather than a hand-written table, and a tree
//! adapter overrides [`SourceAdapter::survey`] to cut its input with the
//! [`survey`] helpers.
//!
//! # Vocabulary
//!
//! - **Source**: what one adapter is asked to read — a directory or an inline
//!   value — under the key the specification cites it by.
//! - **Claim**, **evidence**: one typed statement about the source, and the
//!   document of claims an adapter returns. The **claim gate**
//!   ([`Evidence::findings`]) is the set of rules every claim must satisfy.
//! - **Material**: the part of a source one model call is asked about. The
//!   **survey** ([`SourceAdapter::survey`]) decides the materials before any
//!   call is made; a material is **mined** when the model is asked about it.
//! - **Lend**: the directory the model may read during a call — the source
//!   root, or a material's own directory.
//! - **Findings**, **rounds**: the claim gate's report on an answer, sent back
//!   to the model so it can answer again; the host bounds how many rounds a
//!   call gets.
//!
//! Every failure is omnia's [`Error`]. An adapter refuses input it cannot use
//! with [`bad_request!`] and reports anything else with the sibling macros;
//! there is no adapter error type.

mod references;
mod source;

pub use emery_adapter::source::{
    AdapterMetadata, Backing, Claim, ClaimKind, Evidence, Source, SourceContent, SourceInput,
    SourceKind,
};
pub use omnia_guest::{Error, Model, bad_gateway, bad_request, model, not_found, server_error};
// The export shim the `source!` macro expands against; no adapter names it.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use source::export;
pub use source::{Context, Material, SourceAdapter, survey};

/// Exports a [`SourceAdapter`] as the component the engine loads.
///
/// Invoke it once at the crate root with the path to the implementing type.
/// The expansion is a `wasm32`-only module, so the crate carries no `cfg` of
/// its own and still builds natively for its tests.
///
/// # Examples
///
/// ```
/// # use emery_prose::registry::Doc;
/// # use emery_sdk::{SourceAdapter, SourceKind};
/// # struct Adapter;
/// # impl SourceAdapter for Adapter {
/// #     const KIND: SourceKind = SourceKind::Intent;
/// #     fn docs() -> &'static [Doc] {
/// #         &[]
/// #     }
/// # }
/// emery_sdk::source!(crate::Adapter);
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! source {
    ($adapter:ty) => {
        #[cfg(target_arch = "wasm32")]
        mod guest {
            use $crate::export;

            struct Adapter;
            export::export!(Adapter with_types_in export);

            impl export::Guest for Adapter {
                fn metadata(_id: export::AdapterId) -> export::AdapterMetadata {
                    export::metadata::<$adapter>()
                }

                async fn extract(
                    id: export::AdapterId,
                    input: export::Input,
                ) -> Result<export::Evidence, export::Error> {
                    export::extract::<$adapter>(id, input).await
                }
            }
        }
    };
}
