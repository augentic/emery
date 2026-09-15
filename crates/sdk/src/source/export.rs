//! Exports a [`SourceAdapter`] as the `source-adapter` wasm world.
//!
//! The [`crate::source!`] macro's `guest` module wires into the bindings
//! re-exported here and answers the world's two calls with [`metadata`] and
//! [`extract`]. An adapter invokes the macro once and gains a complete
//! component export without touching the generated bindings; this is the only
//! wasm-specific code it carries.

use emery_adapter::source::SourceInput;
pub use emery_adapter::source::export::*;
use omnia_guest::model::WasiModel;

use super::{Context, SourceAdapter};

/// Answers the world's `metadata` call for adapter `A`.
#[must_use]
pub fn metadata<A: SourceAdapter>() -> AdapterMetadata {
    A::metadata().into()
}

/// Answers the world's `extract` call for adapter `A`, against the host model.
///
/// # Errors
///
/// Returns the adapter's failure, lowered onto the WIT `error` variant.
pub async fn extract<A: SourceAdapter>(id: AdapterId, input: Input) -> Result<Evidence, Error> {
    let input = SourceInput::from(input);
    let ctx = Context {
        adapter_id: &id,
        input: &input,
    };

    Ok(A::extract(&WasiModel, &ctx).await?.into())
}
