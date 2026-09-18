//! Discovers caller-facing surfaces in workspace input.
//!
//! [`surfaces`] asks the model to identify boundaries such as routes,
//! commands, jobs, and exported APIs. Each result names the module where a
//! caller enters that surface. The adapter decides how results become mining
//! [seams](crate#vocabulary).

use std::collections::BTreeSet;

use emery_adapter::source::SourceContent;
use emery_prose::Doc;
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::workspace::Entry;
use crate::{Context, path, references, workspace};

/// Returns the surfaces discovered by the model in a workspace source.
///
/// `docs` must contain `survey.md`, which becomes the system prompt.
/// The model may read the workspace and the embedded reference documents.
///
/// Every surface must have a unique, nonempty name and a root-relative entry
/// path. The entry must be a regular file accepted by `keep`, as must each
/// directory leading to it. Emery's `.omnia/` directories and generated
/// documents are always rejected.
///
/// Results preserve model order. Several surfaces may share an entry module,
/// and an empty inventory is valid.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the request is invalid or the model
///   cannot produce a valid inventory within the available rounds.
/// - Returns [`Error::ServerError`] when `docs` does not contain `survey.md`
///   or the source contains inline text instead of a workspace.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn surfaces<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], mut keep: impl FnMut(Entry<'_>) -> bool + Send,
) -> Result<Vec<Surface>, Error> {
    let key = &ctx.input.key;
    let system = emery_prose::body(docs, "survey.md")
        .ok_or_else(|| server_error!("`survey.md` is not embedded"))?;
    let SourceContent::Workspace(root) = &ctx.input.content else {
        return Err(server_error!(
            "`{key}`: a survey by model needs a workspace input, not an inline value"
        ));
    };

    let inventory = Question::<Inventory>::new("survey")
        .system(system)
        .tools(references::tools())
        .workspace(root)
        .ask(ctx.model, turn(ctx, root), Some(references::answering(docs)), |answer| {
            let findings = answer.findings(root, &mut keep);
            if findings.is_empty() { Ok(()) } else { Err(findings) }
        })
        .await
        .map_err(Error::from)?;

    Ok(inventory
        .surfaces
        .into_iter()
        .map(|surface| Surface {
            // The check accepted the entry, so it is a path beneath the root.
            entry: path::beneath(&surface.entry).unwrap_or(surface.entry),
            name: surface.name,
        })
        .collect())
}

/// The complete set of surfaces reported by the model.
///
/// An empty inventory is valid. [`surfaces`] validates names and entry paths
/// before returning the surfaces to an adapter.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery survey answer")]
pub struct Inventory {
    /// The [`Surface`] values in discovery order.
    pub surfaces: Vec<Surface>,
}

/// A caller-facing capability and the module where it is entered.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// A description of the exposed capability.
    pub name: String,
    /// The entry module as a path relative to the workspace root.
    pub entry: String,
}

impl Inventory {
    // What the check holds against a candidate: every surface named once, and
    // entered at a module of the tree that `keep` accepts.
    fn findings(&self, root: &str, keep: &mut impl FnMut(Entry<'_>) -> bool) -> Vec<String> {
        let mut findings = Vec::new();
        let mut names = BTreeSet::new();
        for surface in &self.surfaces {
            if surface.name.trim().is_empty() {
                findings.push(format!("the surface entered at `{}` has no name", surface.entry));
            } else if !names.insert(surface.name.as_str()) {
                findings.push(format!("surface `{}` is listed twice", surface.name));
            }
            if let Err(finding) = module(root, &surface.entry, keep) {
                findings.push(finding);
            }
        }
        findings
    }
}

// `named` as a path beneath `root` when it is a regular file there that
// `keep` accepts — asked about each directory on the way and the file itself,
// as the walk would offer them — and none of the engine's own; otherwise the
// finding.
fn module(
    root: &str, named: &str, keep: &mut impl FnMut(Entry<'_>) -> bool,
) -> Result<String, String> {
    let entry = path::beneath(named).map_err(|reason| format!("`{named}` {reason}"))?;
    workspace::offered_file(root, &entry, keep)?;
    Ok(entry)
}

// The survey turn: which source is being surveyed, the root lent, how an
// entry is named, and where the model's work stops.
fn turn<P>(ctx: &Context<'_, P>, root: &str) -> String {
    format!(
        "Survey the source bound to adapter `{id}` (source key `{key}`) before it is mined.\n\n\
         `$SOURCE_DIR` is the read-only view at `{root}` — the source tree. List the surfaces it \
         exposes as the prompt describes them, each named for what a caller outside the source \
         reaches, with the module the caller enters it at. Name an entry as a `/`-separated path \
         relative to `$SOURCE_DIR`, to a module of the kind the prompt says this adapter mines; \
         a module may be the entry of several surfaces, and a module no surface enters is not \
         named.\n\n\
         Read under `$SOURCE_DIR` to decide; nothing outside it is reachable. The caller mines \
         each surface from its entry, following what it reaches through the whole tree — you \
         follow nothing and group nothing. When the tree declares no surface, answer none rather \
         than inventing one.\n\n\
         The prompt's references are available through this call's `read_doc` tool (`list_docs` \
         enumerates them); load referenced bodies on demand.\n\n\
         Answer with one JSON object matching the survey schema. The caller mines the surfaces; \
         extract nothing yourself.",
        id = ctx.adapter_id,
        key = ctx.input.key,
    )
}
