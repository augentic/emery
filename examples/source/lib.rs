//! Demonstrates a minimal Emery source adapter.
//!
//! The adapter extracts greeting claims from inline text or a workspace. It
//! shows the three parts of an adapter:
//!
//! - An embedded prompt and its references.
//! - A survey that selects mining seams.
//! - A WebAssembly guest exported with `emery_sdk::source_adapter!`.

use emery_sdk::{Doc, Error, Seam, SourceContent, SourceInput, bad_request};

#[cfg(target_arch = "wasm32")]
mod guest {
    use emery_sdk::{AdapterMetadata, Context, Error, Evidence, Model, SourceKind};

    emery_sdk::source_adapter!(metadata, extract);

    fn metadata() -> AdapterMetadata {
        emery_sdk::metadata(SourceKind::Documentation)
    }

    async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
        let seams = super::survey(ctx.input)?;
        emery_sdk::extract(ctx, super::PROSE, &seams).await
    }
}

// The crate is a `cdylib` whose one caller is the `wasm32` guest above.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "read by the guest module alone")
)]
static PROSE: &[Doc] = emery_sdk::prose!["prose/extract.md", "prose/references/greeting.md"];

// The greeting source is one seam however it arrives; an empty brief is
// refused before a turn is spent.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "called by the guest module alone")
)]
fn survey(input: &SourceInput) -> Result<Vec<Seam>, Error> {
    if let SourceContent::Value(value) = &input.content
        && value.trim().is_empty()
    {
        return Err(bad_request!("the bound greeting brief is empty"));
    }
    Ok(vec![Seam::whole()])
}
