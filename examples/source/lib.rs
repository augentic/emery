//! Minimal example source adapter.
//!
//! Treats inline text or a workspace as a single mining seam and extracts
//! claims under the embedded `extract.md` prompt.

use emery_sdk::{
    Context, Doc, Error, Evidence, Metadata, Model, Seam, SourceAdapter, SourceContent,
    SourceInput, SourceKind, bad_request,
};

const KIND: SourceKind = SourceKind::Documentation;

/// The adapter implementation.
pub struct Adapter;

impl SourceAdapter for Adapter {
    fn metadata() -> Metadata {
        Metadata { kind: KIND }
    }

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
