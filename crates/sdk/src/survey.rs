//! Discovers caller-facing surfaces in workspace input.
//!
//! [`surfaces`] asks the model to identify boundaries such as routes,
//! commands, jobs, and exported APIs. Each result names the module where a
//! caller enters that surface. The adapter decides how results become mining
//! [seams](crate#vocabulary).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::SourceContent;
use emery_prose::Doc;
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::workspace::{Entry, Unoffered};
use crate::{Context, SURVEY, beneath, prompt, reference, workspace};

/// The most modules one survey turn lists before it collapses them to directory counts.
pub const MODULE_CAP: usize = 200;

/// Returns the surfaces discovered by the model in a workspace source.
///
/// `docs` must contain `survey.md`, which becomes the system prompt. The turn
/// lists the modules `keep` accepts, so the model reads the manifest and the
/// bootstrap among them rather than globbing the tree. Up to [`MODULE_CAP`]
/// modules are listed by path; past that, the root's own files are listed and
/// each top-level directory stands for its modules with a count. The model may
/// read the workspace and the embedded reference documents.
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
/// - Returns [`Error::BadRequest`] when a workspace name is not UTF-8, the
///   request is invalid, or the model cannot produce a valid inventory within
///   the available rounds.
/// - Returns [`Error::ServerError`] when `docs` does not contain `survey.md`,
///   the source contains inline text instead of a workspace, or the workspace
///   cannot be read.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn surfaces<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], mut keep: impl FnMut(Entry<'_>) -> bool + Send,
) -> Result<Vec<Surface>, Error> {
    let source = &ctx.input.name;
    let SourceContent::Workspace(root) = &ctx.input.content else {
        return Err(server_error!(
            "`{source}`: a survey by model needs a workspace input, not an inline value"
        ));
    };
    let question = Question::<Inventory>::new(&format!("survey-{source}"))
        .system(prompt(docs, SURVEY)?)
        .tools(reference::tools())
        .workspace(root);
    let modules = workspace::list(root, &mut keep)?;
    let brief = Brief {
        adapter_id: ctx.adapter_id,
        source,
        modules: &modules,
    };

    tracing::info!(%source, "surveying");
    let inventory = question
        .ask(ctx.model, brief.to_string(), Some(reference::serve(docs, source, None)), |answer| {
            let findings = answer.findings(root, &mut keep);
            if findings.is_empty() {
                return Ok(());
            }
            tracing::debug!(%source, ?findings, "candidate rejected");
            Err(findings)
        })
        .await?;

    let surfaces: Vec<_> = inventory
        .surfaces
        .iter()
        .map(|surface| format!("{} @ {}", surface.name, surface.entry))
        .collect();
    tracing::info!(%source, ?surfaces, "surveyed");

    // normalise the accepted entries
    Ok(inventory
        .surfaces
        .into_iter()
        .map(|surface| Surface {
            entry: beneath(&surface.entry).unwrap_or(surface.entry),
            name: surface.name,
        })
        .collect())
}

/// The complete set of surfaces reported by the model.
///
/// An empty inventory is valid. [`surfaces`] validates names and entry paths
/// before returning the surfaces to an adapter.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq)]
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

fn module(root: &str, named: &str, keep: &mut impl FnMut(Entry<'_>) -> bool) -> Result<(), String> {
    let entry = beneath(named).map_err(|reason| format!("`{named}` {reason}"))?;
    workspace::offered_file(root, &entry, keep).map_err(|unoffered| match unoffered {
        Unoffered::NoFile => format!("no file at `{entry}`"),
        Unoffered::Refused => format!("`{entry}` is not a module this adapter mines"),
    })
}

// The user turn of the survey. The lend carries the root, so the brief never
// names it.
struct Brief<'a> {
    adapter_id: &'a str,
    source: &'a str,
    modules: &'a [String],
}

impl Display for Brief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Survey the source `{source}` bound to adapter `{id}` before it is mined.\n\n\
             `$SOURCE_DIR` is the bound source tree, lent read-only: the root of every file you \
             can read. List the surfaces it exposes as the prompt describes them, each named for \
             what a caller outside the source reaches, with the module the caller enters it at. \
             Name an entry as a `/`-separated path relative to `$SOURCE_DIR`, to a module of the \
             kind the prompt says this adapter mines; a module may be the entry of several \
             surfaces, and a module no surface enters is not named.\n\n\
             {modules}\
             Nothing outside `$SOURCE_DIR` is reachable. The caller mines each surface from its \
             entry, following what it reaches through the whole tree — you follow nothing and \
             group nothing. When the tree declares no surface, answer none rather than \
             inventing one.\n\n\
             The prompt's references are available through this call's `read_doc` tool \
             (`list_docs` enumerates them); load referenced bodies on demand.\n\n\
             Answer with one JSON object matching the survey schema. The caller mines the \
             surfaces; extract nothing yourself.",
            id = self.adapter_id,
            source = self.source,
            modules = Modules(self.modules),
        )
    }
}

// The `## Modules` section of the turn, with how to read it.
struct Modules<'a>(&'a [String]);

impl Display for Modules<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("## Modules\n\n")?;
        if self.0.len() <= MODULE_CAP {
            for module in self.0 {
                writeln!(f, "- `{module}`")?;
            }
            return f.write_str(
                "\nRead the manifest and the bootstrap among the modules listed; do not glob or \
                 list the tree yourself.\n\n",
            );
        }

        let mut dirs: BTreeMap<&str, usize> = BTreeMap::new();
        for module in self.0 {
            match module.split_once('/') {
                Some((dir, _)) => *dirs.entry(dir).or_default() += 1,
                None => writeln!(f, "- `{module}`")?,
            }
        }
        for (dir, count) in dirs {
            let noun = if count == 1 { "module" } else { "modules" };
            writeln!(f, "- `{dir}/` ({count} {noun})")?;
        }
        f.write_str(
            "\nThe tree has too many modules to list, so each top-level directory stands for the \
             modules beneath it. Read the manifest and the bootstrap among the files listed and \
             within those directories; list a directory only to reach them.\n\n",
        )
    }
}
