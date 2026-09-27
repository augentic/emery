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

/// The prompt and reference document embedded in the adapter.
pub static PROSE: &[Doc] = emery_sdk::prose!["prose/extract.md", "prose/references/greeting.md"];

/// Returns one mining seam for the greeting source.
///
/// Inline text is mined as a whole. A workspace seam instructs extraction to
/// use `references/greeting.md` when the source tree contains no greeting.
///
/// # Errors
///
/// Returns [`Error::BadRequest`] when the bound brief is empty.
pub fn survey(input: &SourceInput) -> Result<Vec<Seam>, Error> {
    let seam = match &input.content {
        SourceContent::Value(value) if value.trim().is_empty() => {
            return Err(bad_request!("the bound greeting brief is empty"));
        }
        SourceContent::Value(_) => Seam::Whole,
        SourceContent::Workspace(_) => Seam::Note(
            "`$SOURCE_DIR` is the bound greeting tree, lent read-only: the root of every file \
             you can read, and the root every `path` is relative to. Prefer the bound tree; fall \
             back to `references/greeting.md` when the tree does not state a greeting. Nothing \
             outside it is reachable; extract mines only this source."
                .to_owned(),
        ),
    };
    Ok(vec![seam])
}
