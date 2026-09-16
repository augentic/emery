//! The `list_docs` and `read_doc` tools a model call offers.
//!
//! The model consults the adapter's embedded documents on demand instead of
//! receiving the whole corpus in the prompt. Calls are answered in-process
//! from the embedded [`Doc`] table; there is no server behind them, so an
//! adapter needs no network access to expose its references.

use std::future::ready;

use emery_prose::registry::{self, Doc};
use omnia_sdk::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

/// The `list_docs` arguments: none.
// A braced struct derives the empty `object` schema a tool's parameters must
// be; a unit struct would derive `null`.
#[derive(Debug, Deserialize, JsonSchema)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "the schema derive needs the braced form for an object"
)]
struct ListDocs {}

/// The `read_doc` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
struct ReadDoc {
    /// The adapter-relative document path, such as `prompts/extract.md`.
    path: String,
}

/// Returns the declarations of the `list_docs` and `read_doc` tools.
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

/// Returns the handler that answers those tools from `docs`.
#[must_use]
pub fn answering(docs: &'static [Doc]) -> Tools {
    Box::new(move |call: ToolCall| -> ToolFuture { Box::pin(ready(answer(docs, &call))) })
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
