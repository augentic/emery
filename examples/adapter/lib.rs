//! The mock source adapter the live journey runs against.
//!
//! The smallest complete source adapter: it reads a greeting fixture and asks
//! the model to describe it as claims. It exists so the engine can be
//! exercised end to end without depending on a first-party adapter from the
//! adapters repository.
//!
//! It has the shape of a real adapter: the kind of source it reads, the prose
//! tree `emery_sdk::include_prose!` embeds, a survey, and a `wasm32`-only
//! guest exporting the `source-adapter` world over `emery_sdk::mine`.

#[cfg(target_arch = "wasm32")]
mod guest {
    use emery_sdk::export::{self, AdapterId, AdapterMetadata, Error, Evidence, Guest, Input};
    use emery_sdk::{Doc, SourceKind};

    // The extraction prompt and its reference, from the tree beside this file.
    static DOCS: &[Doc] = emery_sdk::include_prose!("prose");

    struct Adapter;
    export::export!(Adapter with_types_in export);

    impl Guest for Adapter {
        fn metadata(_id: AdapterId) -> AdapterMetadata {
            emery_sdk::metadata(SourceKind::Documentation)
        }

        async fn extract(id: AdapterId, input: Input) -> Result<Evidence, Error> {
            emery_sdk::extract(id, input, DOCS, async |ctx| super::survey(&ctx.input.content)).await
        }
    }
}

use emery_sdk::{Error, Seam, SourceContent, bad_request};

/// Returns the one seam to mine: a bound brief whole, or a tree with the fallback noted.
///
/// A workspace is pointed at `references/greeting.md` as the fallback when
/// the tree states no greeting.
///
/// # Errors
///
/// Returns [`Error::BadRequest`] when the bound brief is empty.
pub fn survey(content: &SourceContent) -> Result<Vec<Seam>, Error> {
    let seam = match content {
        SourceContent::Value(value) if value.trim().is_empty() => {
            return Err(bad_request!("the bound greeting brief is empty"));
        }
        SourceContent::Value(_) => Seam::Whole,
        SourceContent::Workspace(root) => Seam::Note(format!(
            "`$SOURCE_DIR` is the read-only view at `{root}` — the greeting tree the prompt \
             walks. Prefer the bound tree; fall back to `references/greeting.md` when the tree \
             does not state a greeting. Nothing outside it is reachable; extract mines only this \
             source."
        )),
    };
    Ok(vec![seam])
}
