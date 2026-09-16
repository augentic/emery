//! Mines the seams of one source and joins the claims into one document.
//!
//! An adapter decides the [seams](crate#vocabulary); [`mine`] does the rest
//! of an `extract` — one model turn per seam under the adapter's embedded
//! prompt, the claim gate on every answer, and the join of the seams' claims
//! in seam order.

mod brief;

use emery_adapter::source::{Evidence, SourceInput};
use emery_prose::Doc;
use futures::stream::{self, StreamExt as _};
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, bad_gateway, bad_request, not_found, server_error};

use self::brief::{Brief, Lend};
use crate::references;

// Turns one adapter holds pending at once.
const CONCURRENT: usize = 4;

/// Mines `seams` through the model and joins the claims into one [`Evidence`] document.
///
/// Each seam is one turn. `prompts/extract.md` among `docs` is the system
/// prompt; the turn names the adapter and the source key from `ctx`,
/// describes the seam, and lends the model the source root; the `list_docs`
/// and `read_doc` tools answer from `docs`. The answer is checked against
/// the claim gate ([`Evidence::findings`]), and findings go back to the
/// model for another round until it answers clean or the host's rounds are
/// spent. The engine runs the same gate again on receipt.
///
/// At most four turns are pending at once. The claims join in seam order.
/// Every seam of a workspace is lent the root, so every `path` anchor is
/// relative to it and the document cites one path space however the source
/// was cut. Every seam is waited for; when more than one fails, the error
/// names them all and takes the class of the first.
///
/// # Errors
///
/// - [`Error::BadRequest`] when `seams` is empty, a [`Seam::Files`] path
///   escapes the root or names no file, the host refuses a request, or the
///   model's answer still fails the claim gate once its rounds are spent.
/// - [`Error::ServerError`] for a [`Seam::Files`] over an inline value, or a
///   corpus without `prompts/extract.md`.
/// - [`Error::BadGateway`] for a tool or transport failure.
pub async fn mine<P: Model>(
    model: &P, ctx: &Context<'_>, docs: &'static [Doc], seams: &[Seam],
) -> Result<Evidence, Error> {
    let key = &ctx.input.key;
    if seams.is_empty() {
        return Err(bad_request!("`{key}`: nothing to mine"));
    }

    let lends = seams.iter().map(|seam| Lend::of(seam, ctx)).collect::<Result<Vec<_>, _>>()?;
    let outcomes: Vec<_> = stream::iter(seams.iter().zip(&lends))
        .map(|(seam, lend)| evidence(model, ctx, docs, seam, lend))
        .buffered(CONCURRENT)
        .collect()
        .await;
    let partials = collect(key, outcomes)?;

    Ok(Evidence {
        claims: partials.into_iter().flat_map(|partial| partial.claims).collect(),
    })
}

/// The part of a source one model call is asked about.
///
/// An adapter's survey chooses one or more seams before any call is made;
/// see the [vocabulary](crate#vocabulary).
#[derive(Debug, Eq, PartialEq)]
pub enum Seam {
    /// The whole input: a workspace described as the source tree, or an
    /// inline value quoted into the turn.
    Whole,
    /// Files beneath the input's root, named relative to it.
    ///
    /// The whole root is lent, and the turn lists the files relative to it,
    /// sorted and deduplicated, as the only ones to mine. One that escapes
    /// the root is refused.
    Files(Vec<String>),
    /// A note the adapter wrote for a source that needs its own handling.
    ///
    /// The whole root is lent, and the note stands in the turn where the
    /// SDK's description of the input would be.
    Note(String),
}

/// What one call knows: which adapter was addressed, and with what input.
#[derive(Debug)]
pub struct Context<'a> {
    /// The id the call addressed the adapter by.
    pub adapter_id: &'a str,
    /// The source key and the workspace or inline value to read.
    pub input: &'a SourceInput,
}

// One seam's turn: the embedded prompt as the system, the brief as the user
// turn, the seam's lend, and the claim gate as the check the backend loops
// on until the answer is clean or its rounds are spent.
async fn evidence<P: Model>(
    model: &P, ctx: &Context<'_>, docs: &'static [Doc], seam: &Seam, lend: &Lend,
) -> Result<Evidence, Error> {
    let system = emery_prose::body(docs, "prompts/extract.md")
        .ok_or_else(|| server_error!("`prompts/extract.md` is not embedded"))?;
    let brief = Brief { ctx, seam, lend };

    let mut question =
        Question::<Evidence>::new("evidence").system(system).tools(references::tools());
    if let Some(workspace) = &lend.workspace {
        question = question.workspace(workspace);
    }

    question
        .ask(model, brief.to_string(), Some(references::answering(docs)), |answer| {
            let findings = answer.findings();
            if findings.is_empty() { Ok(()) } else { Err(findings) }
        })
        .await
        .map_err(Error::from)
}

// A lone seam's failure is the source's as it stands; several are reported
// together, under the first one's class.
fn collect(key: &str, outcomes: Vec<Result<Evidence, Error>>) -> Result<Vec<Evidence>, Error> {
    let count = outcomes.len();
    let mut partials = Vec::with_capacity(count);
    let mut failures = Vec::new();
    for (index, outcome) in outcomes.into_iter().enumerate() {
        match outcome {
            Ok(evidence) => partials.push(evidence),
            Err(error) if count == 1 => return Err(error),
            Err(error) => failures.push((index, error)),
        }
    }

    let Some((_, first)) = failures.first() else {
        return Ok(partials);
    };
    let report: Vec<String> = failures
        .iter()
        .map(|(index, error)| format!("- seam {index}: {}", error.description()))
        .collect();

    Err(reclass(
        first,
        &format!("`{key}`: {} of {count} seams failed:\n{}", failures.len(), report.join("\n")),
    ))
}

// The first failed seam decides the class; the report names them all.
fn reclass(class: &Error, description: &str) -> Error {
    match class {
        Error::BadRequest { .. } => bad_request!("{description}"),
        Error::NotFound { .. } => not_found!("{description}"),
        Error::ServerError { .. } => server_error!("{description}"),
        Error::BadGateway { .. } => bad_gateway!("{description}"),
    }
}
