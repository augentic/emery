//! Adapter SDK
//!
//! The one crate an Emery adapter depends on. An adapter is a WebAssembly
//! component that reads one kind of source — a document tree, a codebase, a
//! written brief — and returns typed claims about it. This crate carries what
//! every such adapter needs and would otherwise write again: the contract
//! types, the model call and its claim gate, the error vocabulary, and the
//! component export.
//!
//! An adapter implements [`SourceAdapter`] for its source kind: it says which
//! [`Material`]s the model should read and refuses input it cannot use. The
//! SDK does the rest — reports the kind in the adapter's metadata, asks the
//! model for each material's claims-only [`Evidence`], checks it against the
//! claim gate, and joins the results into one document. A tree adapter lists
//! its files with [`survey`], which prunes the engine's own files for it,
//! and cuts them mechanically by directory or, through one model call under
//! its own survey prompt, by what they serve. The [`source!`] macro then
//! turns the implementation into a component. Adapter code is left with what
//! is specific to its source, and nothing else.
//!
//! The contract lives in `emery-adapter` and is re-exported here, so an
//! adapter never sees the WIT bindings. Failures are omnia's [`Error`]: an
//! adapter refuses its input with [`bad_request!`] and reports anything else
//! with the sibling macros.

mod references;
mod source;

pub use emery_adapter::source::{
    AdapterMetadata, Backing, Claim, ClaimKind, Evidence, SourceContent, SourceInput, SourceKind,
};
pub use omnia_guest::{Error, Model, bad_gateway, bad_request, model, not_found, server_error};
// The export shim the `source!` macro expands against; no adapter names it.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use source::export;
pub use source::{Context, Material, SourceAdapter, survey};

/// Wires a [`SourceAdapter`] into component exports.
///
/// One invocation at the crate root declares the `wasm32`-only `guest`
/// module, so an adapter carries no `cfg` of its own and still builds
/// natively.
///
/// ```ignore
/// emery_sdk::source!(crate::Captures);
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
