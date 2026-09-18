//! Shapes the questions the SDK puts to the model.
//!
//! Every question the SDK asks — one seam's extraction, a survey — has the
//! same shape: an embedded prompt as the system, the reference tools declared
//! by [`of`] and answered by [`answering`] from an adapter's [`Doc`] table and
//! then [`RUNTIME`], and a check that [`gate`] builds from the findings
//! against a candidate.

use std::future::ready;

use emery_prose::Doc;
use omnia_sdk::model::{Findings, Function, Question, Tool, ToolCall, ToolFuture, Tools};
use omnia_sdk::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::RUNTIME;

// The tool names as the model calls them.
const LIST_DOCS: &str = "list_docs";
const READ_DOC: &str = "read_doc";

/// Returns the question named `name`, put under `docs`' `prompt` with the reference tools declared.
///
/// # Errors
///
/// Returns [`Error::ServerError`] when `docs` does not embed `prompt`: the
/// adapter build's own defect, reported before a turn is spent.
pub fn of<T>(name: &str, docs: &'static [Doc], prompt: &str) -> Result<Question<T>, Error>
where
    T: JsonSchema + DeserializeOwned + Send,
{
    let system = emery_prose::body(docs, prompt)
        .ok_or_else(|| server_error!("`{prompt}` is not embedded"))?;
    Ok(Question::new(name).system(system).tools(tools()))
}

/// Returns the check's verdict on a candidate: clean, or the findings to correct.
pub fn gate(findings: Findings) -> Result<(), Findings> {
    if findings.is_empty() { Ok(()) } else { Err(findings) }
}

/// Returns the handler that answers the reference tools from `docs` and then [`RUNTIME`].
#[must_use]
pub fn answering(docs: &'static [Doc]) -> Tools {
    Box::new(move |call: ToolCall| -> ToolFuture { Box::pin(ready(answer(docs, &call))) })
}

// The doc comments on these argument types reach the model: `JsonSchema`
// carries each as the `description` of the tool's parameters.

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

fn tools() -> Vec<Tool> {
    vec![
        Tool::Function(Function::of::<ListDocs>(
            LIST_DOCS,
            "List every reference document path this call can read: the adapter's own, then \
             Emery's shared `claims.md` and `reconciliation.md`.",
        )),
        Tool::Function(Function::of::<ReadDoc>(
            READ_DOC,
            "Read one embedded reference document in full by its path.",
        )),
    ]
}

fn answer(docs: &[Doc], call: &ToolCall) -> Result<String, String> {
    match call.name.as_str() {
        LIST_DOCS => {
            let paths: Vec<&str> = docs.iter().chain(RUNTIME).map(|doc| doc.path).collect();
            Ok(json!({ "paths": paths }).to_string())
        }
        READ_DOC => {
            let ReadDoc { path } = call.arguments().map_err(|err| format!("{READ_DOC}: {err}"))?;
            let doc = emery_prose::find(docs, &path)
                .or_else(|| emery_prose::find(RUNTIME, &path))
                .ok_or_else(|| format!("no document `{path}`"))?;
            Ok(json!({ "path": path, "body": doc.body }).to_string())
        }
        other => Err(format!("unknown tool `{other}`")),
    }
}
