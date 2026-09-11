//! Source adapter SDK
//!
//! Everything an adapter author needs to build an Emery source adapter: the
//! [`SourceAdapter`] trait to implement, the [`EvidenceTurn`] envelope and
//! [`evidence`] call that ask the extract question, and the export macro that
//! turns an implementation into a wasm component.
//!
//! The contract itself lives in `emery-source` and is re-exported here, so an
//! adapter depends on one crate and never sees the wire bindings directly.
//! Failures are omnia's [`Error`]: an adapter refuses its input with
//! [`bad_request!`] and reports anything else with the sibling macros.

mod evidence;
mod operations;
pub mod references;
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod source;

pub use emery_source::Source;
pub use evidence::{EvidenceTurn, content_note, evidence};
#[cfg(target_arch = "wasm32")]
pub use omnia_guest::model::WasiModel;
pub use omnia_guest::model::{
    Findings, Format, Function, Message, Question, Reply, Request, Role, SchemaFormat, Tool,
    ToolCall, ToolFuture, Tools,
};
pub use omnia_guest::{Error, Model, bad_gateway, bad_request, not_found, server_error};
pub use operations::SourceAdapter;

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
            struct Adapter;
            $crate::source::export!(Adapter with_types_in $crate::source);

            impl $crate::source::Guest for Adapter {
                fn metadata(
                    _id: $crate::source::AdapterId,
                ) -> $crate::source::AdapterMetadata {
                    $crate::source::metadata::<$adapter>()
                }

                async fn extract(
                    id: $crate::source::AdapterId,
                    input: $crate::source::Input,
                ) -> Result<$crate::source::Evidence, $crate::source::Error> {
                    $crate::source::extract::<$adapter>(id, input).await
                }
            }
        }
    };
}
