//! Reference tools
//!
//! The `list_docs` and `read_doc` tools a judgment offers the model, so it
//! can consult the adapter's embedded reference documents on demand instead
//! of receiving the whole corpus in the prompt.
//!
//! Tool calls are answered in-process from the embedded [`Doc`] table. There
//! is no server behind them, so an adapter needs no network access and no
//! external endpoint to expose its references.

use std::future::ready;

use emery_prose::registry::{self, Doc};
use omnia_guest::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

/// The `list_docs` arguments: none. The braces stay — a braced struct derives
/// the empty `object` schema a tool's parameters must be, where a unit struct
/// would derive `null`.
#[derive(Debug, Deserialize, JsonSchema)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "the schema derive needs the braced form for an object"
)]
struct ListDocs {}

/// The `read_doc` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
struct ReadDoc {
    /// Adapter-relative document path, e.g. `prompts/build.md`.
    path: String,
}

/// Declares the `list_docs` and `read_doc` tools for a judgment that carries
/// reference documents.
#[must_use]
pub fn tools() -> Vec<Tool> {
    vec![
        Tool::Function(Function::of::<ListDocs>(
            "list_docs",
            "List every reference document path this adapter embeds.",
        )),
        Tool::Function(Function::of::<ReadDoc>(
            "read_doc",
            "Read one embedded reference document in full by its path.",
        )),
    ]
}

/// Builds the tool handler a question passes to `ask`: [`answer`] over
/// `docs`, or `None` when the adapter embeds nothing to consult.
#[must_use]
pub fn answering(docs: &'static [Doc]) -> Option<Tools> {
    (!docs.is_empty()).then(|| {
        Box::new(move |call: ToolCall| -> ToolFuture { Box::pin(ready(answer(docs, &call))) })
            as Tools
    })
}

fn answer(docs: &[Doc], call: &ToolCall) -> Result<String, String> {
    match call.name.as_str() {
        "list_docs" => {
            let paths: Vec<&str> = docs.iter().map(|doc| doc.path).collect();
            Ok(json!({ "paths": paths }).to_string())
        }
        "read_doc" => {
            let ReadDoc { path } = call.arguments().map_err(|err| format!("read_doc: {err}"))?;
            let doc = registry::find(docs, &path).ok_or_else(|| format!("no document `{path}`"))?;
            Ok(json!({ "path": path, "body": doc.body }).to_string())
        }
        other => Err(format!("unknown tool `{other}`")),
    }
}
