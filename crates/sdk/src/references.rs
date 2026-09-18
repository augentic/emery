//! Provides model tools for reading embedded reference documents.
//!
//! The tools list available paths and retrieve complete document bodies from
//! an adapter's in-memory [`Doc`] table and the SDK's [`RUNTIME`] table. They
//! require no network service.

use std::future::ready;

use emery_prose::Doc;
use omnia_sdk::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::prose::RUNTIME;

/// The empty argument object accepted by `list_docs`.
// A braced struct derives the empty `object` schema a tool's parameters must
// be; a unit struct would derive `null`.
#[derive(Debug, Deserialize, JsonSchema)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "the schema derive needs the braced form for an object"
)]
struct ListDocs {}

/// The document selector accepted by `read_doc`.
#[derive(Debug, Deserialize, JsonSchema)]
struct ReadDoc {
    /// The document path as `list_docs` lists it, such as `claims.md`.
    path: String,
}

/// Returns the declarations of the `list_docs` and `read_doc` tools.
#[must_use]
pub fn tools() -> Vec<Tool> {
    vec![
        Tool::Function(Function::of::<ListDocs>(
            "list_docs",
            "List every reference document path this call can read: the adapter's own, then \
             Emery's shared `claims.md` and `reconciliation.md`.",
        )),
        Tool::Function(Function::of::<ReadDoc>(
            "read_doc",
            "Read one embedded reference document in full by its path.",
        )),
    ]
}

/// Returns the handler that answers those tools from `docs` and then [`RUNTIME`].
#[must_use]
pub fn answering(docs: &'static [Doc]) -> Tools {
    Box::new(move |call: ToolCall| -> ToolFuture { Box::pin(ready(answer(docs, &call))) })
}

fn answer(docs: &[Doc], call: &ToolCall) -> Result<String, String> {
    match call.name.as_str() {
        "list_docs" => {
            let paths: Vec<&str> = docs.iter().chain(RUNTIME).map(|doc| doc.path).collect();
            Ok(json!({ "paths": paths }).to_string())
        }
        "read_doc" => {
            let ReadDoc { path } = call.arguments().map_err(|err| format!("read_doc: {err}"))?;
            let doc = emery_prose::find(docs, &path)
                .or_else(|| emery_prose::find(RUNTIME, &path))
                .ok_or_else(|| format!("no document `{path}`"))?;
            Ok(json!({ "path": path, "body": doc.body }).to_string())
        }
        other => Err(format!("unknown tool `{other}`")),
    }
}
