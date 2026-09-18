//! Mines source seams and combines their claims.
//!
//! An adapter divides its input into [seams](crate#vocabulary). [`extract`]
//! settles every seam against the input, puts each to the model as one gated
//! turn, and returns one evidence document in seam order.

use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{Evidence, SourceContent, SourceInput};
use emery_prose::Doc;
use futures::stream::{self, StreamExt as _};
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, bad_request, server_error};

use crate::{Context, beneath, question};

/// The most turns one [`extract`] call holds pending at once.
///
/// Seams past this many wait for an earlier turn to answer, so a survey that
/// cuts finer than this gains nothing in wall-clock time.
pub const CONCURRENT: usize = 4;

/// Mines each seam and combines accepted claims into one [`Evidence`] document.
///
/// `docs` must contain `extract.md`, which becomes the system prompt for
/// every request. The model receives the adapter identifier, source key,
/// seam description, and access to embedded references. Responses are
/// checked with [`Evidence::findings`]; rejected responses may be corrected
/// until the host's round limit is reached.
///
/// Up to [`CONCURRENT`] requests run concurrently. All requests are awaited,
/// while claims retain the order of `seams`. Every workspace seam uses the
/// same source root, so claim paths share one root-relative namespace.
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
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn extract<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], seams: &[Seam],
) -> Result<Evidence, Error> {
    let key = &ctx.input.key;
    if seams.is_empty() {
        return Err(bad_request!("`{key}`: nothing to extract"));
    }

    // settle every seam and the question before the first turn is spent
    let plans =
        seams.iter().map(|seam| Plan::of(seam, ctx.input)).collect::<Result<Vec<_>, _>>()?;
    let mut question = question::of::<Evidence>("evidence", docs, "extract.md")?;
    if let SourceContent::Workspace(root) = &ctx.input.content {
        question = question.workspace(root);
    }

    // one gated turn per seam, at most CONCURRENT pending
    let outcomes: Vec<_> = stream::iter(&plans)
        .map(|plan| turn(&question, ctx, docs, plan))
        .buffered(CONCURRENT)
        .collect()
        .await;

    // join the accepted claims in seam order
    let partials = join(key, outcomes)?;
    Ok(Evidence {
        claims: partials.into_iter().flat_map(|partial| partial.claims).collect(),
    })
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
    Note(String),
}

// What one seam puts to the model, settled against the input before any turn
// is spent.
#[derive(Debug)]
enum Plan<'a> {
    // The adapter's own note, standing where the input's rendering would.
    Note(&'a str),
    // The files to mine beneath the lent root: sorted, once each, all beneath it.
    Files { root: &'a str, files: Vec<String> },
    // The whole tree, lent.
    Tree(&'a str),
    // The inline value, which rides the turn; nothing is lent.
    Value(&'a str),
}

impl<'a> Plan<'a> {
    // A `Files` path that escapes the root, or a set naming no file, is
    // `bad_request`; `Files` over an inline value is the adapter's own
    // defect, so `server_error`.
    fn of(seam: &'a Seam, input: &'a SourceInput) -> Result<Self, Error> {
        let key = &input.key;
        match (seam, &input.content) {
            (Seam::Note(note), _) => Ok(Self::Note(note)),
            (Seam::Whole, SourceContent::Workspace(root)) => Ok(Self::Tree(root)),
            (Seam::Whole, SourceContent::Value(value)) => Ok(Self::Value(value)),
            (Seam::Files(_), SourceContent::Value(_)) => Err(server_error!(
                "`{key}`: a `Files` seam needs a workspace input, not an inline value"
            )),
            (Seam::Files(named), SourceContent::Workspace(root)) => {
                let mut files = named
                    .iter()
                    .map(|path| {
                        beneath(path).map_err(|reason| bad_request!("`{key}`: `{path}` {reason}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                files.sort();
                files.dedup();
                if files.is_empty() {
                    return Err(bad_request!("`{key}`: a `Files` seam names no file"));
                }
                Ok(Self::Files { root, files })
            }
        }
    }
}

// The user turn of one seam: which source is bound, what the seam is lent,
// and where the model's work stops.
struct Brief<'a> {
    adapter_id: &'a str,
    key: &'a str,
    plan: &'a Plan<'a>,
}

impl Display for Brief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Extract the claim set of the source bound to adapter `{id}` (source key `{key}`).\n\n",
            id = self.adapter_id,
            key = self.key,
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
            "\n\nThe prompt's references are available through this call's `read_doc` tool \
             (`list_docs` enumerates them); load referenced bodies on demand.\n\n\
             Answer with one JSON object matching the gated claims schema. The caller persists \
             the document; do not write it yourself.",
        )
    }
}

// One seam's turn: the question asked with the seam's brief, the reference
// tools answered from `docs`, and the claim gate as the check the backend
// loops on until the answer is clean or its rounds are spent.
async fn turn<P: Model>(
    question: &Question<Evidence>, ctx: &Context<'_, P>, docs: &'static [Doc], plan: &Plan<'_>,
) -> Result<Evidence, Error> {
    let brief = Brief {
        adapter_id: ctx.adapter_id,
        key: &ctx.input.key,
        plan,
    };
    question
        .ask(ctx.model, brief.to_string(), Some(question::answering(docs)), |answer| {
            question::gate(answer.findings())
        })
        .await
        .map_err(Error::from)
}

// The accepted documents in seam order, or the source's failure: a lone
// seam's as it stands, several seams' reported together under the first
// one's class and code.
fn join(key: &str, outcomes: Vec<Result<Evidence, Error>>) -> Result<Vec<Evidence>, Error> {
    let count = outcomes.len();
    let mut accepted = Vec::with_capacity(count);
    let mut failed = Vec::new();
    for (index, outcome) in outcomes.into_iter().enumerate() {
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
                    "`{key}`: {} of {count} seams failed:\n{}",
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
