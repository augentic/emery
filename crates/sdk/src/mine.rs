//! Mines source seams and combines their claims.
//!
//! An adapter divides its input into [seams](crate#vocabulary). [`mine`]
//! submits each seam to the model, validates every response, and returns one
//! evidence document in seam order.

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

/// Mines each seam and combines accepted claims into one [`Evidence`] document.
///
/// `docs` must contain `prompts/extract.md`, which becomes the system prompt
/// for every request. The model receives the adapter identifier, source key,
/// seam description, and access to embedded references. Responses are
/// checked with [`Evidence::findings`]; rejected responses may be corrected
/// until the host's round limit is reached.
///
/// Up to four requests run concurrently. All requests are awaited, while
/// claims retain the order of `seams`. Every workspace seam uses the same
/// source root, so claim paths share one root-relative namespace.
///
/// When several seams fail, the returned error describes each failure and
/// uses the class of the first failed seam.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when `seams` is empty, a
///   [`Seam::Files`] path is invalid, the model rejects the request, or no
///   valid response is produced within the available rounds.
/// - Returns [`Error::ServerError`] when [`Seam::Files`] is used with inline
///   input or `docs` does not contain `prompts/extract.md`.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn mine<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], seams: &[Seam],
) -> Result<Evidence, Error> {
    let key = &ctx.input.key;
    if seams.is_empty() {
        return Err(bad_request!("`{key}`: nothing to mine"));
    }

    let lends =
        seams.iter().map(|seam| Lend::of(seam, ctx.input)).collect::<Result<Vec<_>, _>>()?;
    let outcomes: Vec<_> = stream::iter(seams.iter().zip(&lends))
        .map(|(seam, lend)| evidence(ctx, docs, seam, lend))
        .buffered(CONCURRENT)
        .collect()
        .await;
    let partials = collect(key, outcomes)?;

    Ok(Evidence {
        claims: partials.into_iter().flat_map(|partial| partial.claims).collect(),
    })
}

/// A portion of a source assigned to one model request.
///
/// An adapter's survey chooses one or more seams before any call is made;
/// see the [vocabulary](crate#vocabulary).
#[derive(Debug, Eq, PartialEq)]
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

/// The input and model available during one adapter extraction.
#[derive(Debug)]
pub struct Context<'a, P> {
    /// The identifier used to address the adapter.
    pub adapter_id: &'a str,
    /// The [`SourceInput`] identifying the source and its content.
    pub input: &'a SourceInput,
    /// The [`Model`] used for survey and extraction requests.
    pub model: &'a P,
}

// One seam's turn: the embedded prompt as the system, the brief as the user
// turn, the seam's lend, and the claim gate as the check the backend loops
// on until the answer is clean or its rounds are spent.
async fn evidence<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], seam: &Seam, lend: &Lend,
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
        .ask(ctx.model, brief.to_string(), Some(references::answering(docs)), |answer| {
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
