//! Answers the `source-adapter` guest interface over an adapter's functions.
//!
//! [`source_adapter!`](crate::source_adapter) lowers the adapter's metadata
//! with the contract's `From` and answers `extract` through [`call`], which
//! lifts the WIT input onto a [`Context`](crate::Context) carrying the host
//! model, runs the adapter's extraction, and lowers its outcome.

use crate::{Context, Error, Evidence, Model, SourceInput, export};

/// The default model provider supplied to adapter extraction functions.
///
/// [`source_adapter!`](crate::source_adapter) places this provider in each
/// [`Context`](crate::Context). It delegates model requests through Omnia's
/// WebAssembly interface.
#[derive(Clone, Copy, Debug, Default)]
pub struct Provider;

impl Model for Provider {}

/// Calls an adapter's extraction over the lifted input and the host model.
///
/// # Errors
///
/// Returns the adapter's error lowered onto the guest error record.
#[omnia_wasi_otel::instrument(name = "source_adapter_extract")]
pub async fn call(
    extract: impl AsyncFnOnce(&Context<'_, Provider>) -> Result<Evidence, Error>,
    id: export::AdapterId, input: export::Input,
) -> Result<export::Evidence, export::Error> {
    let input = SourceInput::from(input);
    let ctx = Context {
        adapter_id: &id,
        input: &input,
        model: &Provider,
    };
    Ok(extract(&ctx).await?.into())
}
