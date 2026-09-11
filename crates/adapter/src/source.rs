//! Component export
//!
//! Turns a [`crate::SourceAdapter`] implementation into the `source-adapter`
//! wasm world the engine loads. An adapter crate invokes
//! [`crate::source!`] once and gains a complete component export without
//! touching the generated bindings.
//!
//! This is the only wasm-specific code an adapter carries, which keeps the
//! rest of its logic portable and testable natively.

pub use emery_source::wire::export::*;

use crate::types::{Context, SourceContent, SourceInput};
use crate::{SourceAdapter, WasiModel};

/// Answers `metadata` for adapter `A`: its record, lowered onto the wire.
#[must_use]
pub fn metadata<A: SourceAdapter>() -> AdapterMetadata {
    A::metadata().into()
}

/// Answers `extract` for adapter `A`: its evidence, or its failure lowered
/// onto the wire variant.
///
/// # Errors
///
/// Returns the adapter's failure lowered onto the wire variant.
pub async fn extract<A: SourceAdapter>(id: AdapterId, input: Input) -> Result<Evidence, Error> {
    let input = SourceInput::from(input);
    // A bound tree is lent to the model; an inline value rides the prompt.
    let lend = match &input.content {
        SourceContent::Workspace(root) => Some(root.clone()),
        SourceContent::Value(_) => None,
    };
    let ctx = Context {
        adapter_id: &id,
        docs: A::docs(),
        lend,
    };

    A::extract(&WasiModel, &ctx, &input).await.map(Into::into).map_err(Into::into)
}

/// Wires a [`crate::SourceAdapter`] into component exports.
///
/// ```ignore
/// emery_adapter::source!(crate::Captures);
/// ```
#[macro_export]
macro_rules! source {
    ($adapter:ty) => {
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
    };
}
