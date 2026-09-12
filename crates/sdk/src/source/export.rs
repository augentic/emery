//! Component export
//!
//! Turns a [`SourceAdapter`] implementation into the `source-adapter` wasm
//! world the engine loads: the bindings the [`crate::source!`] macro's
//! `guest` module wires into, and the two answers it gives over them. An
//! adapter crate invokes the macro once and gains a complete component export
//! without touching the generated bindings.
//!
//! This is the only wasm-specific code an adapter carries, which keeps the
//! rest of its logic portable and testable natively.

use emery_adapter::source::SourceInput;
pub use emery_adapter::source::export::*;
use omnia_guest::model::WasiModel;

use super::{Context, SourceAdapter};

/// Answers `metadata` for adapter `A`: its record, lowered onto the WIT bindings.
#[must_use]
pub fn metadata<A: SourceAdapter>() -> AdapterMetadata {
    A::metadata().into()
}

/// Answers `extract` for adapter `A`: its evidence, or its failure lowered
/// onto the WIT bindings variant.
///
/// # Errors
///
/// Returns the adapter's failure lowered onto the WIT bindings variant.
pub async fn extract<A: SourceAdapter>(id: AdapterId, input: Input) -> Result<Evidence, Error> {
    let input = SourceInput::from(input);
    let ctx = Context {
        adapter_id: &id,
        input: &input,
    };

    Ok(A::extract(&WasiModel, &ctx).await?.into())
}
