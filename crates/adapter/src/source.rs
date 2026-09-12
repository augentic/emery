//! Component export
//!
//! Turns a [`crate::SourceAdapter`] implementation into the `source-adapter`
//! wasm world the engine loads: the bindings the [`crate::source!`] macro's
//! `guest` module wires into, and the two answers it gives over them. An
//! adapter crate invokes the macro once and gains a complete component export
//! without touching the generated bindings.
//!
//! This is the only wasm-specific code an adapter carries, which keeps the
//! rest of its logic portable and testable natively.

use emery_source::SourceInput;
pub use emery_source::export::*;
use omnia_guest::model::WasiModel;

use crate::{Context, SourceAdapter};

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
    let ctx = Context {
        adapter_id: &id,
        input: &input,
    };

    Ok(A::extract(&WasiModel, &ctx).await?.into())
}
