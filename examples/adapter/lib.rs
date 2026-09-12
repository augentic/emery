//! Mock source adapter
//!
//! The smallest complete source adapter: it reads a greeting fixture and
//! asks the model to describe it as claims. It exists so the engine can be
//! exercised end to end on the live journey without depending on a
//! first-party adapter from the adapters repository, and so the SDK's wasm32
//! export side is linted by `make wasm`.
//!
//! It is also the reference shape for adapter authors: one `SourceAdapter`
//! implementation, an embedded prose tree, and a single `source!` export.

emery_adapter::source!(crate::Adapter);

use emery_adapter::types::{Context, Evidence, SourceContent, SourceInput};
use emery_adapter::{
    Error, EvidenceTurn, Model, SourceAdapter, bad_request, content_note, evidence, server_error,
};
use emery_prose::registry::{self, Doc};

static DOCS: &[Doc] = &[
    Doc {
        path: "prompts/extract.md",
        body: include_str!("prose/prompts/extract.md"),
    },
    Doc {
        path: "references/greeting.md",
        body: include_str!("prose/references/greeting.md"),
    },
];

/// Extracts the greeting fixture into structured claims.
#[derive(Clone, Copy, Debug)]
pub struct Adapter;

impl SourceAdapter for Adapter {
    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        let system = registry::body(DOCS, "prompts/extract.md")
            .ok_or_else(|| server_error!("`prompts/extract.md` is not embedded"))?;
        let turn = EvidenceTurn::prepared("greeting", greeting_note(input)?);
        evidence(model, ctx, input, system, turn).await
    }
}

// Builds the prompt's content note: refuses an empty inline brief, passes an
// inline value through, and points a workspace at `references/greeting.md`
// as the fallback when the tree states no greeting.
fn greeting_note(input: &SourceInput) -> Result<String, Error> {
    match &input.content {
        SourceContent::Value(value) if value.trim().is_empty() => {
            Err(bad_request!("the bound greeting brief is empty"))
        }
        SourceContent::Value(_) => Ok(content_note(input, "")),
        SourceContent::Workspace(_) => Ok(format!(
            "{} Prefer the bound tree; fall back to `references/greeting.md` when the tree \
             does not state a greeting.",
            content_note(input, "the greeting tree")
        )),
    }
}
