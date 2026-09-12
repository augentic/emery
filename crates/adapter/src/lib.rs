//! Source adapter SDK
//!
//! Everything an adapter author needs to build an Emery source adapter: the
//! [`SourceAdapter`] trait to implement, the [`Material`] an extraction hands
//! the model, and the export macro that turns an implementation into a wasm
//! component.
//!
//! The contract itself lives in `emery-source` and is re-exported here, so an
//! adapter depends on one crate and never sees the WIT bindings directly.
//! Failures are omnia's [`Error`]: an adapter refuses its input with
//! [`bad_request!`] and reports anything else with the sibling macros.

mod brief;
mod references;
mod source;

// The `source!` macro expands against this; no adapter names it.
pub use brief::Material;
pub use emery_source::{
    AdapterMetadata, Authority, Backing, Claim, ClaimKind, Evidence, SourceContent, SourceInput,
};
pub use omnia_guest::{Error, Model, bad_gateway, bad_request, model, not_found, server_error};
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use source::export;
pub use source::{Context, SourceAdapter};

/// Wires a [`SourceAdapter`] into component exports.
///
/// One invocation at the crate root declares the `wasm32`-only `guest`
/// module, so an adapter carries no `cfg` of its own and still builds
/// natively.
///
/// ```ignore
/// emery_adapter::source!(crate::Captures);
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
