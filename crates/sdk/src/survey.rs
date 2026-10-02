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
//!
//! What a survey derives by code alone is spelled with the helpers beneath:
//! [`Lines`] for a span of a file, [`resolve`] for what an import leads to,
//! [`route`] for the stems and discriminators a route or literal spells, and
//! [`tests`] for the behaviours a tree's own tests state. A survey that
//! parses its source fills a [`code::Module`] per file and reads it through
//! the lookups there, each reading the language from the adapter's
//! [`Dialect`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::is_kebab;
use emery_adapter::source::{Anchor, BadAnchor, SourceContent};
use emery_prose::Doc;
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::extract::{Laid, lay, line_count};
use crate::{Context, SURVEY, beneath, prompt, reference};

pub mod code;
mod dialect;
pub mod resolve;
pub mod route;
pub mod tests;

pub use self::dialect::Dialect;

// First-occurrence order.
pub(crate) fn unique<T: PartialEq>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut list = Vec::new();
    for item in items {
        if !list.contains(&item) {
            list.push(item);
        }
    }
    list
}

/// A span of lines within one file, 1-based and inclusive.
///
/// The default holds no line. [`Display`] renders the span as prose, with an
/// en dash; [`Lines::anchor`] renders it in the claim `path` grammar. The
/// span an [`Anchor`] cites converts into one, a line past `u32::MAX`
/// saturating.
///
/// # Examples
///
/// ```
/// use emery_sdk::survey::Lines;
///
/// let span = Lines { start: 3, end: 5 };
/// assert!(span.holds(4));
/// assert!(span.contains(Lines { start: 4, end: 5 }));
/// assert_eq!(span.anchor(), "L3-L5");
/// assert_eq!(span.to_string(), "L3–L5");
/// assert_eq!(Lines::from((7, 7)).anchor(), "L7");
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Lines {
    /// The first line of the span.
    pub start: u32,
    /// The last line of the span, no earlier than the first.
    pub end: u32,
}

impl Lines {
    /// Returns whether `other` lies wholly within this span.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// Returns whether `line` lies within this span.
    #[must_use]
    pub const fn holds(self, line: u32) -> bool {
        self.start <= line && line <= self.end
    }

    /// Returns the span in the claim anchor grammar: `L3`, or `L3-L5`.
    #[must_use]
    pub fn anchor(self) -> String {
        if self.start == self.end {
            format!("L{}", self.start)
        } else {
            format!("L{}-L{}", self.start, self.end)
        }
    }
}

impl From<(u64, u64)> for Lines {
    fn from((start, end): (u64, u64)) -> Self {
        let line = |cited: u64| u32::try_from(cited).unwrap_or(u32::MAX);
        Self {
            start: line(start),
            end: line(end),
        }
    }
}

impl Display for Lines {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.start == self.end {
            write!(f, "L{}", self.start)
        } else {
            write!(f, "L{}–L{}", self.start, self.end)
        }
    }
}

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
    pub files: &'a [String],
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
/// the host's round limit is reached.
///
/// Every anchor and `unreached` path is spelled root-relative, as
/// `facts.modules` spells them, before `check` sees it and in what is
/// returned: `./src/a.ts#L2-L2` reads `src/a.ts#L2`, so the adapter looks a
/// path up as the tree spells it and never unpicks the model's spelling. One
/// outside the grammar, or escaping the root, is left as answered for the
/// finding that names it.
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
    let laid = lay(root, facts.files);
    let brief = Brief {
        adapter_id: ctx.adapter_id,
        source,
        facts,
        laid: &laid,
    };

    tracing::info!(%source, modules = facts.modules.len(), "surveying by model");
    let inventory = question
        .ask(ctx.model, brief.to_string(), Some(reference::serve(docs, source, None)), |answer| {
            // the gate and the adapter's check read one spelling of every path
            let answer = answer.normalised();
            let mut findings = answer.findings(root, facts.modules);
            findings.extend(check(&answer));
            if findings.is_empty() {
                return Ok(());
            }
            tracing::debug!(%source, ?findings, "candidate rejected");
            Err(findings)
        })
        .await
        .map_err(Error::from)?
        .normalised();

    tracing::info!(
        %source,
        surfaces = inventory.surfaces.len(),
        unreached = inventory.unreached.len(),
        "surveyed by model"
    );
    if tracing::enabled!(tracing::Level::DEBUG) {
        let named: Vec<String> = inventory
            .surfaces
            .iter()
            .map(|surface| format!("{} @ {} as {}", surface.name, surface.anchor, surface.stem))
            .collect();
        tracing::debug!(%source, ?named, "the model's surfaces");
    }

    Ok(inventory)
}

/// The surfaces the model names in a source, and the modules none reaches.
///
/// An empty inventory is a valid answer: the tree declares no surface.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
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
    // Every anchor and unreached path spelled root-relative, as the tree
    // spells its modules; one outside the grammar or escaping the root is
    // left as answered, for `findings` to name.
    fn normalised(&self) -> Self {
        let anchor = |spelled: &str| {
            let respelled = || {
                let anchor = Anchor::parse(spelled).ok()?;
                let path = beneath(anchor.path).ok()?;
                Some(
                    Anchor {
                        path: &path,
                        ..anchor
                    }
                    .to_string(),
                )
            };
            respelled().unwrap_or_else(|| spelled.to_owned())
        };

        let surfaces = self
            .surfaces
            .iter()
            .map(|surface| Surface {
                anchor: anchor(&surface.anchor),
                ..surface.clone()
            })
            .collect();
        let unreached = self
            .unreached
            .iter()
            .map(|path| beneath(path).unwrap_or_else(|_| path.clone()))
            .collect();

        Self { surfaces, unreached }
    }

    // What the tree alone can hold a normalised answer to: each anchor a
    // module of the list with lines its file holds, each stem in the grammar,
    // each name once, each unreached module of the list and no surface's
    // entry. A file's lines are counted once however many anchors cite it.
    fn findings(&self, root: &str, modules: &[String]) -> Vec<String> {
        let mut findings = Vec::new();
        let mut names = BTreeSet::new();
        let mut entries = BTreeSet::new();
        let mut lines: BTreeMap<&str, Option<u64>> = BTreeMap::new();
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
                Ok(Anchor { path, lines: cited }) => {
                    let held = modules
                        .iter()
                        .any(|module| module == path)
                        .then(|| *lines.entry(path).or_insert_with(|| line_count(root, path)));
                    match (held, cited) {
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
            if !modules.contains(path) {
                findings.push(format!("- unreached `{path}` names no module of this source"));
            } else if entries.contains(path.as_str()) {
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
            Laid(&self.facts.files[..self.laid.len()], self.laid).fmt(f)?;
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
