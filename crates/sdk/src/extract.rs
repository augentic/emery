//! Mines source seams and combines their claims.
//!
//! An adapter divides its input into [seams](crate#vocabulary). [`extract`]
//! settles every seam against the input, puts each to the model as one gated
//! turn — largest first, a bounded number pending, an upstream failure put
//! once more — and returns one evidence document in seam order.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{Evidence, SourceContent, SourceInput};
use emery_prose::Doc;
use futures::stream::{self, StreamExt as _};
use futures::{FutureExt as _, TryFutureExt as _};
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, bad_request, server_error};

use crate::{CLAIMS, Context, RUNTIME, beneath, prompt, reference};

// The adapter's prompt among its documents: the system of every mining turn.
const PROMPT: &str = "extract.md";

/// The most turns one [`extract`] call holds pending at once.
///
/// The cap is on the turns in flight, not on how finely a survey cuts: a seam
/// past this many waits for a slot, and a small seam fills the slot a large
/// one leaves as soon as it answers, so finer cuts pack the slots better even
/// though no more than this many are answered at once.
pub const CONCURRENT: usize = 4;

/// Mines each seam and combines accepted claims into one [`Evidence`] document.
///
/// `docs` must contain `extract.md`. It becomes the system prompt for every
/// request, with the shared `claims.md` of [`RUNTIME`] appended, so each turn
/// carries the id grammar and the gate without a `read_doc` call for them.
/// The model receives the adapter identifier, source name, seam description,
/// and access to the remaining embedded references. Responses are checked
/// with [`Evidence::findings`]; rejected responses may be corrected until the
/// host's round limit is reached.
///
/// Up to [`CONCURRENT`] requests run concurrently, largest first: a
/// [`Seam::Files`] by its file count, a [`Seam::Whole`] or [`Seam::Note`],
/// whose size is not known, before them, and ties in seam order. A request
/// that fails upstream — the model or a tool transport — is put once more; a
/// refusal is not. All requests are awaited, and claims retain the order of
/// `seams`. Every workspace seam uses the same source root, so claim paths
/// share one root-relative namespace.
///
/// When several seams fail, the returned error describes each failure and
/// carries the class and code of the first failed seam.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when `seams` is empty, a
///   [`Seam::Files`] path is invalid, the model rejects the request, or no
///   valid response is produced within the available rounds.
/// - Returns [`Error::ServerError`] when [`Seam::Files`] is used with inline
///   input or `docs` does not contain `extract.md`.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails on
///   the retried request too.
pub async fn extract<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], seams: &[Seam],
) -> Result<Evidence, Error> {
    let source = &ctx.input.name;
    if seams.is_empty() {
        return Err(bad_request!("`{source}`: nothing to extract"));
    }

    // settle every seam and the question before the first turn is spent
    let plans =
        seams.iter().map(|seam| Plan::of(seam, ctx.input)).collect::<Result<Vec<_>, _>>()?;
    let system = format!("{}\n\n---\n\n{}", prompt(docs, PROMPT)?, prompt(RUNTIME, CLAIMS)?);
    let mut question =
        Question::<Evidence>::new("evidence").system(system).tools(reference::tools());
    if let SourceContent::Workspace(root) = &ctx.input.content {
        question = question.workspace(root);
    }

    // one gated turn per seam, largest first, at most CONCURRENT pending
    let mut order: Vec<_> = plans.iter().enumerate().collect();
    order.sort_by_key(|(_, plan)| plan.size().map(Reverse));
    let outcomes = stream::iter(order)
        .map(|(index, plan)| {
            turn(&question, ctx, docs, index, plan).map(move |outcome| (index, outcome))
        })
        .buffer_unordered(CONCURRENT)
        .collect()
        .await;

    // join the accepted claims in seam order
    let partials = join(source, outcomes)?;
    let claims: Vec<_> = partials.into_iter().flat_map(|partial| partial.claims).collect();

    Ok(Evidence { claims })
}

/// A portion of a source assigned to one model request.
///
/// An adapter's survey chooses one or more seams before any call is made;
/// see the [vocabulary](crate#vocabulary).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Seam {
    /// The complete workspace or inline value.
    Whole,
    /// Selected files beneath a workspace root.
    ///
    /// Paths are sorted, deduplicated, and interpreted relative to the root.
    /// A path that escapes the root is rejected.
    Files(Vec<String>),
    /// Adapter-defined instructions describing what to mine.
    ///
    /// For workspace input, the complete root remains available to the model.
    /// The note's first non-blank line is the seam's `label` on the events
    /// [`extract`] logs for it, so lead with what the seam covers — the
    /// surface and its entry — and put the standing instructions after.
    Note(String),
}

impl Display for Seam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Whole => write!(f, "whole"),
            Self::Files(files) => write!(f, "files: {}", files.join(", ")),
            Self::Note(note) => write!(f, "note: {note}"),
        }
    }
}

// A seam settled against the input before any turn is spent.
#[derive(Debug)]
enum Plan<'a> {
    Note(&'a str),
    Files { root: &'a str, files: Vec<String> },
    Tree(&'a str),
    Value(&'a str),
}

impl<'a> Plan<'a> {
    // A `Files` seam over an inline value is the adapter's own defect, so
    // `server_error`; the rest is the operator's input.
    fn of(seam: &'a Seam, input: &'a SourceInput) -> Result<Self, Error> {
        let source = &input.name;
        match (seam, &input.content) {
            (Seam::Note(note), _) => Ok(Self::Note(note)),
            (Seam::Whole, SourceContent::Workspace(root)) => Ok(Self::Tree(root)),
            (Seam::Whole, SourceContent::Value(value)) => Ok(Self::Value(value)),
            (Seam::Files(_), SourceContent::Value(_)) => Err(server_error!(
                "`{source}`: a `Files` seam needs a workspace input, not an inline value"
            )),
            (Seam::Files(named), SourceContent::Workspace(root)) => {
                let mut files = named
                    .iter()
                    .map(|path| {
                        beneath(path)
                            .map_err(|reason| bad_request!("`{source}`: `{path}` {reason}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                files.sort();
                files.dedup();
                if files.is_empty() {
                    return Err(bad_request!("`{source}`: a `Files` seam names no file"));
                }
                Ok(Self::Files { root, files })
            }
        }
    }

    const fn size(&self) -> Option<usize> {
        match self {
            Self::Files { files, .. } => Some(files.len()),
            Self::Note(_) | Self::Tree(_) | Self::Value(_) => None,
        }
    }

    // What names the seam in a log line: a note's first line, a file list's
    // first file and how many more, or the shape of a whole input
    fn label(&self) -> String {
        const WIDTH: usize = 72;
        match self {
            Self::Note(note) => {
                let line = note.lines().find(|line| !line.trim().is_empty()).unwrap_or_default();
                if line.chars().count() > WIDTH {
                    format!("{}…", line.chars().take(WIDTH).collect::<String>())
                } else {
                    line.to_owned()
                }
            }
            Self::Files { files, .. } => match files.as_slice() {
                [only] => only.clone(),
                [first, rest @ ..] => format!("{first} (+{})", rest.len()),
                [] => String::new(),
            },
            Self::Tree(_) => "tree".to_owned(),
            Self::Value(_) => "value".to_owned(),
        }
    }
}

// The turn is one of several in flight, so its events name the seam
// themselves; the failure's description is `join`'s to report once, so the
// event carries the class alone.
#[tracing::instrument(skip_all, fields(source = %ctx.input.name, seam = index))]
async fn turn<P: Model>(
    question: &Question<Evidence>, ctx: &Context<'_, P>, docs: &'static [Doc], index: usize,
    plan: &Plan<'_>,
) -> Result<Evidence, Error> {
    let source = &ctx.input.name;
    let brief = Brief {
        adapter_id: ctx.adapter_id,
        source,
        plan,
    };
    let ask = || {
        question
            .ask(
                ctx.model,
                brief.to_string(),
                Some(reference::serve(docs, source, Some(index), &[PROMPT, CLAIMS])),
                |answer| {
                    let findings = answer.findings();
                    if findings.is_empty() {
                        return Ok(());
                    }
                    tracing::debug!(%source, seam = index, ?findings, "candidate rejected");
                    Err(findings)
                },
            )
            .map_err(Error::from)
    };

    let label = plan.label();
    tracing::info!(%source, seam = index, label, files = plan.size(), "mining");
    let outcome = match ask().await {
        Err(error @ Error::BadGateway { .. }) => {
            tracing::warn!(
                %source,
                seam = index,
                label,
                %error,
                "failed upstream; putting the turn once more"
            );
            ask().await
        }
        outcome => outcome,
    };
    match &outcome {
        Ok(evidence) => {
            tracing::info!(%source, seam = index, label, claims = evidence.claims.len(), "mined");
        }
        Err(error) => {
            tracing::warn!(%source, seam = index, label, code = %error.code(), "failed");
        }
    }

    outcome
}

// The user turn of one seam.
struct Brief<'a> {
    adapter_id: &'a str,
    source: &'a str,
    plan: &'a Plan<'a>,
}

impl Display for Brief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Extract the claim set of the source `{source}` bound to adapter `{id}`.\n\n",
            id = self.adapter_id,
            source = self.source,
        )?;

        match self.plan {
            Plan::Note(note) => f.write_str(note)?,
            Plan::Files { root, files } => {
                writeln!(
                    f,
                    "`$SOURCE_DIR` is the read-only view at `{root}` — the source tree. Mine these \
                     files beneath it and nothing else:"
                )?;
                for file in files {
                    write!(f, "\n- `{file}`")?;
                }
                f.write_str(
                    "\n\nAnchor every `path` relative to `$SOURCE_DIR`. Nothing outside it is \
                     reachable; extract mines only this source.",
                )?;
            }
            Plan::Tree(root) => write!(
                f,
                "`$SOURCE_DIR` is the read-only view at `{root}` — the source tree the prompt \
                 walks. Nothing outside it is reachable; extract mines only this source."
            )?,
            Plan::Value(value) => write!(
                f,
                "The bound seam is this inline value; no `$SOURCE_DIR` is lent:\n\n{value}\n\n\
                 Nothing else is reachable; extract mines only this source."
            )?,
        }

        f.write_str(
            "\n\nThe claim rules (`claims.md`) are already in the system prompt; the prompt's \
             further references are available through this call's `read_doc` tool (`list_docs` \
             enumerates them); load referenced bodies on demand.\n\n\
             Answer with one JSON object matching the gated claims schema. The caller persists \
             the document; do not write it yourself.",
        )
    }
}

fn join(
    source: &str, outcomes: BTreeMap<usize, Result<Evidence, Error>>,
) -> Result<Vec<Evidence>, Error> {
    let count = outcomes.len();
    let mut accepted = Vec::with_capacity(count);
    let mut failed = Vec::new();
    for (index, outcome) in outcomes {
        match outcome {
            Ok(evidence) => accepted.push(evidence),
            Err(error) => failed.push((index, error)),
        }
    }

    match failed.as_slice() {
        [] => Ok(accepted),
        [(_, only)] if count == 1 => Err(only.clone()),
        [(_, first), ..] => {
            let report: Vec<String> = failed
                .iter()
                .map(|(index, error)| format!("- seam {index}: {}", error.description()))
                .collect();
            Err(described(
                first,
                format!(
                    "`{source}`: {} of {count} seams failed:\n{}",
                    failed.len(),
                    report.join("\n")
                ),
            ))
        }
    }
}

// `error` with `description` in place of its own; the class and code carry.
fn described(error: &Error, description: String) -> Error {
    let mut described = error.clone();
    match &mut described {
        Error::BadRequest { description: own, .. }
        | Error::NotFound { description: own, .. }
        | Error::ServerError { description: own, .. }
        | Error::BadGateway { description: own, .. } => *own = description,
    }
    described
}
