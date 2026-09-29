//! Asks the model which surfaces a source exposes, from what an adapter's
//! code read of it.
//!
//! An adapter's survey is a plain fn over the input; this module is the one
//! way a survey may put a turn to the model instead. The adapter renders the
//! facts its code found — the modules, the manifest, the bootstrap, the
//! packages and the calls made through them — as [`Facts`], and [`surfaces`]
//! asks the model to name each surface, the anchor where it is registered or
//! declared, and the stem its ids lead with. The answer is held to the tree
//! before it is returned, so what the adapter derives from it — the closure
//! each entry reaches, the anchors within it — rests on modules the tree
//! holds.

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use emery_adapter::is_kebab;
use emery_adapter::source::{Anchor, BadAnchor, SourceContent};
use emery_prose::Doc;
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::extract::{Laid, lay, line_count};
use crate::{Context, SURVEY, beneath, prompt, reference};

/// What an adapter's code read of a tree, for the model to name its surfaces from.
///
/// The model is told how to read what code found, never how to find it: the
/// adapter lists every production module, renders what locates the surfaces
/// among them as `text`, and names the files worth laying into the turn whole.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Facts<'a> {
    /// Every production module of the tree, root-relative; an anchor names one of these.
    pub modules: &'a [String],
    /// What the adapter's code read: the manifest, the bootstrap, the registration sites.
    pub text: &'a str,
    /// The files laid into the turn whole, in order, for as long as they fit
    /// within [`INLINE_BYTES`](crate::INLINE_BYTES) together; the rest are left to the lend.
    pub lay: &'a [String],
}

/// Returns the surfaces the model names in a workspace source.
///
/// `docs` must contain `survey.md`, which becomes the system prompt. The turn
/// carries the adapter's `facts`, lends the tree read-only, and offers the
/// embedded references through the reference tools.
///
/// Each answered surface is held to the tree before it is returned: its
/// `anchor` parses under the claim `path` grammar, names one of
/// `facts.modules`, and cites lines the file holds; its `stem` is lowercase
/// kebab-case; its `name` is non-empty and unique. Each `unreached` module is
/// one of `facts.modules` and no surface's entry. `check` then holds the
/// answer to what the adapter alone can know — which modules the named
/// entries reach — and returns its own findings, empty for none. Every
/// finding is returned to the model together for one correction round, until
/// the host's round limit is reached. Accepted anchors come back with their
/// path normalised root-relative.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the model rejects the request or no
///   valid answer is produced within the available rounds or the backend's
///   time budget.
/// - Returns [`Error::ServerError`] when `docs` does not contain `survey.md`
///   or the source is an inline value with no tree to survey.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn surfaces<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], facts: &Facts<'_>,
    mut check: impl FnMut(&Inventory) -> Vec<String> + Send,
) -> Result<Inventory, Error> {
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
    let laid = lay(root, facts.lay);
    let brief = Brief {
        adapter_id: ctx.adapter_id,
        source,
        facts,
        laid: &laid,
    };

    tracing::info!(%source, modules = facts.modules.len(), "surveying by model");
    let mut inventory = question
        .ask(ctx.model, brief.to_string(), Some(reference::serve(docs, source, None)), |answer| {
            let mut findings = answer.findings(root, facts.modules);
            findings.extend(check(answer));
            if findings.is_empty() {
                return Ok(());
            }
            tracing::debug!(%source, ?findings, "candidate rejected");
            Err(findings)
        })
        .await
        .map_err(Error::from)?;

    // normalise the accepted anchors and unreached paths
    for surface in &mut inventory.surfaces {
        if let Ok(anchor) = Anchor::parse(&surface.anchor) {
            let path = beneath(anchor.path).unwrap_or_else(|_| anchor.path.to_owned());
            surface.anchor = match anchor.lines {
                None => path,
                Some((start, end)) if start == end => format!("{path}#L{start}"),
                Some((start, end)) => format!("{path}#L{start}-L{end}"),
            };
        }
    }
    for path in &mut inventory.unreached {
        if let Ok(normalised) = beneath(path) {
            *path = normalised;
        }
    }
    let named: Vec<String> = inventory
        .surfaces
        .iter()
        .map(|surface| format!("{} @ {} as {}", surface.name, surface.anchor, surface.stem))
        .collect();
    tracing::info!(
        %source,
        surfaces = named.len(),
        unreached = inventory.unreached.len(),
        "surveyed by model"
    );
    tracing::debug!(%source, ?named, "the model's surfaces");

    Ok(inventory)
}

/// The surfaces the model names in a source, and the modules none reaches.
///
/// An empty inventory is a valid answer: the tree declares no surface.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery survey answer")]
pub struct Inventory {
    /// The surfaces in the order the model names them.
    pub surfaces: Vec<Surface>,
    /// The production modules no surface reaches — dead code, or a module the
    /// model could not place — each root-relative; empty for none.
    #[serde(default)]
    pub unreached: Vec<String>,
}

/// One surface the model names: what a caller does, where, and under which stem.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// What a caller outside the source does through the surface, as a
    /// reviewer would name it: `POST /orders`, `import command`, `orders consumer`.
    pub name: String,
    /// Where the surface is registered or declared, in the claim `path`
    /// grammar: a production module and the lines of the registration, as
    /// `src/routes/orders.ts#L12-L40`, or the module alone for the whole file.
    pub anchor: String,
    /// The stem every `requirement` and `criterion` of the surface leads
    /// with, lowercase kebab-case, under the prompt's convention.
    pub stem: String,
}

impl Surface {
    /// Parses the surface's `anchor` under the claim `path` grammar.
    ///
    /// # Errors
    ///
    /// Returns [`BadAnchor`] when the anchor is outside the grammar, which an
    /// accepted inventory's never is.
    pub fn anchor(&self) -> Result<Anchor<'_>, BadAnchor> {
        Anchor::parse(&self.anchor)
    }
}

impl Inventory {
    // What the tree alone can hold the answer to: each anchor a module of the
    // list with lines its file holds, each stem in the grammar, each name
    // once, each unreached module of the list and no surface's entry.
    fn findings(&self, root: &str, modules: &[String]) -> Vec<String> {
        let mut findings = Vec::new();
        let mut names = BTreeSet::new();
        let mut entries = BTreeSet::new();
        for (index, surface) in self.surfaces.iter().enumerate() {
            let label = if surface.name.trim().is_empty() {
                findings.push(format!("- surface {index}: has no name"));
                format!("surface {index}")
            } else {
                if !names.insert(surface.name.as_str()) {
                    findings.push(format!("- surface `{}` is listed twice", surface.name));
                }
                format!("surface `{}`", surface.name)
            };

            match Anchor::parse(&surface.anchor) {
                Err(reason) => {
                    findings.push(format!("- {label}: anchor `{}` {reason}", surface.anchor));
                }
                Ok(anchor) => {
                    let path = beneath(anchor.path).unwrap_or_else(|_| anchor.path.to_owned());
                    let held = modules.contains(&path).then(|| line_count(root, &path));
                    match (held, anchor.lines) {
                        (None, _) => findings.push(format!(
                            "- {label}: anchor `{}` names no module of this source; anchor within \
                             the modules listed",
                            surface.anchor
                        )),
                        (Some(None), _) => findings.push(format!(
                            "- {label}: anchor `{}` names no regular file under the lent tree",
                            surface.anchor
                        )),
                        (Some(Some(total)), Some((_, end))) if end > total => {
                            findings.push(format!(
                                "- {label}: anchor `{}` cites line {end}, but `{path}` has {total} \
                                 lines",
                                surface.anchor
                            ));
                        }
                        _ => {
                            entries.insert(path);
                        }
                    }
                }
            }

            if !is_kebab(&surface.stem) {
                findings.push(format!(
                    "- {label}: stem `{}` is not lowercase kebab-case",
                    surface.stem
                ));
            }
        }

        for path in &self.unreached {
            let normalised = beneath(path).unwrap_or_else(|_| path.clone());
            if !modules.contains(&normalised) {
                findings.push(format!("- unreached `{path}` names no module of this source"));
            } else if entries.contains(&normalised) {
                findings.push(format!(
                    "- unreached `{path}` is a surface's entry; a module is one or the other"
                ));
            }
        }

        findings
    }
}

// The user turn of the survey. The lend carries the root, so the brief never
// names it.
struct Brief<'a> {
    adapter_id: &'a str,
    source: &'a str,
    facts: &'a Facts<'a>,
    laid: &'a [String],
}

impl Display for Brief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Survey the source `{source}` bound to adapter `{id}` before it is mined.\n\n",
            id = self.adapter_id,
            source = self.source,
        )?;

        // the adapter's facts
        if !self.facts.text.trim().is_empty() {
            f.write_str(self.facts.text.trim_end())?;
            f.write_str("\n\n")?;
        }

        // the modules
        f.write_str(
            "`$SOURCE_DIR` is the bound source tree, lent read-only: the root every `anchor` is \
             relative to. The production modules this adapter mines are these, and no other \
             file is a module:\n",
        )?;
        for module in self.facts.modules {
            write!(f, "\n- `{module}`")?;
        }

        // the laid files
        if !self.laid.is_empty() {
            let (these, them) = match self.laid.len() {
                1 => ("This file is", "it"),
                _ => ("These files are", "them"),
            };
            write!(
                f,
                "\n\n{these} laid out here whole, every line led by its number, so cite `#L<n>` \
                 from the numbers shown rather than reading {them} again:\n\n"
            )?;
            Laid(&self.facts.lay[..self.laid.len()], self.laid).fmt(f)?;
        }

        f.write_str(
            "\n\nName each surface the source exposes as the prompt describes: what a caller \
             outside the source does through it; its `anchor` — the module of the list above \
             where it is registered or declared, and the lines of that registration or \
             declaration, as `<path>#L<n>-L<n>`; and the `stem` its `requirement` and \
             `criterion` ids lead with, lowercase kebab-case, under the prompt's convention. \
             Several surfaces may anchor in one module. List under `unreached` each module of \
             the list no surface reaches — dead code, or a module you cannot place — rather than \
             inventing a surface for it. When the tree declares no surface, answer none.\n\n\
             Nothing outside `$SOURCE_DIR` is reachable. The caller derives what each surface \
             reaches from the anchor you name — follow nothing, group nothing, and extract \
             nothing yourself.\n\n\
             The prompt's references are available through this call's `read_doc` tool \
             (`list_docs` enumerates them); load referenced bodies on demand.\n\n\
             Answer with one JSON object matching the survey schema.",
        )
    }
}
