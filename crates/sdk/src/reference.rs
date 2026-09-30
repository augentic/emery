//! Serves an adapter's reference documents to the model as tools.
//!
//! [`tools`] declares `list_docs` and `read_doc`; [`serve`] fields each call
//! the model makes to them, answering from an adapter's [`Doc`] table and
//! then [`RUNTIME`], and reports every call at DEBUG.

use std::future::ready;

use emery_prose::Doc;
use omnia_sdk::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::{CLAIMS, EXTRACT, RUNTIME, SURVEY};

const LIST_DOCS: &str = "list_docs";
const READ_DOC: &str = "read_doc";

// The documents a turn's system prompt is built from: a mining turn's, and
// the survey turn's.
const SYSTEM: &[&str] = &[EXTRACT, CLAIMS, SURVEY];

/// Returns the reference tools declared to the model on every turn.
#[must_use]
pub fn tools() -> Vec<Tool> {
    vec![
        Tool::Function(Function::of::<ListDocs>(
            LIST_DOCS,
            "List the further reference documents this call can read: the adapter's own, then \
             Emery's shared ones.",
        )),
        Tool::Function(Function::of::<ReadDoc>(
            READ_DOC,
            "Read one embedded reference document in full by its path.",
        )),
    ]
}

/// Returns the handler that serves the reference tools from `docs` and then [`RUNTIME`].
///
/// `list_docs` lists the adapter's references and the runtime references. It
/// never lists a system document (`extract.md`, `claims.md`, `survey.md`),
/// since a turn either carries it already or has nothing to learn from it.
/// `read_doc` still answers every document, so a followed link never fails.
///
/// Each call is reported at DEBUG with its arguments as the model sent them,
/// under the `source` name and, for a mining turn, its `seam`.
#[must_use]
pub fn serve(docs: &'static [Doc], source: &str, seam: Option<usize>) -> Tools {
    let source = source.to_owned();

    Box::new(move |call: ToolCall| -> ToolFuture {
        let response = match call.name.as_str() {
            LIST_DOCS => {
                let paths: Vec<&str> = docs
                    .iter()
                    .chain(RUNTIME)
                    .map(|doc| doc.path)
                    .filter(|path| !SYSTEM.contains(path))
                    .collect();
                Ok(json!({ "paths": paths }).to_string())
            }
            READ_DOC => call.arguments().map_err(|err| format!("{READ_DOC}: {err}")).and_then(
                |ReadDoc { path }| {
                    emery_prose::find(docs, &path)
                        .or_else(|| emery_prose::find(RUNTIME, &path))
                        .map(|doc| json!({ "path": path, "body": doc.body }).to_string())
                        .ok_or_else(|| format!("no document `{path}`"))
                },
            ),
            other => Err(format!("unknown tool `{other}`")),
        };

        tracing::debug!(
            %source,
            seam,
            tool = %call.name,
            arguments = %call.arguments,
            error = response.as_deref().err(),
            "responded"
        );

        Box::pin(ready(response))
    })
}

/// The empty argument object accepted by `list_docs`.
// The `///` is the `JsonSchema` description the model reads. A braced struct
// derives the empty `object` schema a tool's parameters must be; a unit struct
// would derive `null`.
#[derive(Debug, Deserialize, JsonSchema)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "the schema derive needs the braced form for an object"
)]
struct ListDocs {}

/// The document selector accepted by `read_doc`.
// The `///` lines are the `JsonSchema` descriptions the model reads.
#[derive(Debug, Deserialize, JsonSchema)]
struct ReadDoc {
    /// The document path as `list_docs` lists it, such as `reconciliation.md`.
    path: String,
}
