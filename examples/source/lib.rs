//! Demonstrates a minimal Emery source adapter.
//!
//! The adapter extracts greeting claims from inline text or a workspace. It
//! shows the three parts of an adapter:
//!
//! - An embedded prompt and its references.
//! - A survey that selects mining seams.
//! - A WebAssembly guest: an `emery_sdk::SourceAdapter` exported with
//!   `emery_sdk::export_source!`.

use emery_sdk::{
    Context, Doc, Error, Evidence, Model, Seam, SourceAdapter, SourceContent, SourceInput,
    SourceKind, bad_request,
};

/// The adapter implementation.
pub struct Adapter;

impl SourceAdapter for Adapter {
    const KIND: SourceKind = SourceKind::Documentation;

    async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
        let seams = survey(ctx.input)?;
        emery_sdk::extract(ctx, PROSE, &seams).await
    }
}

/// The prompt and its reference embedded in the adapter.
pub static PROSE: &[Doc] = emery_sdk::prose!["prose/extract.md", "prose/references/greeting.md"];

// The greeting source is one seam however it arrives; an empty brief is
// refused before a turn is spent.
fn survey(input: &SourceInput) -> Result<Vec<Seam>, Error> {
    if let SourceContent::Value(value) = &input.content
        && value.trim().is_empty()
    {
        return Err(bad_request!("the bound greeting brief is empty"));
    }
    Ok(vec![Seam::whole()])
}

#[cfg(target_arch = "wasm32")]
emery_sdk::export_source!(Adapter);
