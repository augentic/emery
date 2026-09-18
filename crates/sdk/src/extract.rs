//! Mines source seams and combines their claims.
//!
//! An adapter divides its input into [seams](crate#vocabulary). [`mine`]
//! submits each seam to the model, validates every response, and returns one
//! evidence document in seam order.

use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{Evidence, SourceContent, SourceInput};
use emery_prose::Doc;
use futures::stream::{self, StreamExt as _};
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, bad_gateway, bad_request, not_found, server_error};

use crate::{path, references};

// Turns one adapter holds pending at once.
const CONCURRENT: usize = 4;

/// Mines each seam and combines accepted claims into one [`Evidence`] document.
///
/// `docs` must contain `extract.md`, which becomes the system prompt for
/// every request. The model receives the adapter identifier, source key,
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
///   input or `docs` does not contain `extract.md`.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn extract<P: Model>(
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

// What one seam is lent: the root the model receives, and the files to mine
// relative to it.
#[derive(Debug)]
struct Lend {
    // The source root, lent through the request's workspace grant; none for
    // an inline value.
    workspace: Option<String>,
    // For `Files`, the files to mine relative to the root, sorted and
    // deduped; empty otherwise.
    files: Vec<String>,
}

impl Lend {
    // What `seam` is lent of `input`. A `Files` path that escapes the
    // root, or a set naming no file, is `bad_request`; `Files` over an
    // inline value is the adapter's own defect, so `server_error`.
    fn of(seam: &Seam, input: &SourceInput) -> Result<Self, Error> {
        let key = &input.key;
        let root = match (&input.content, seam) {
            (SourceContent::Workspace(root), _) => root,
            (SourceContent::Value(_), Seam::Files(_)) => {
                return Err(server_error!(
                    "`{key}`: a `Files` seam needs a workspace input, not an inline value"
                ));
            }
            (SourceContent::Value(_), _) => {
                return Ok(Self {
                    workspace: None,
                    files: Vec::new(),
                });
            }
        };

        let Seam::Files(paths) = seam else {
            return Ok(Self {
                workspace: Some(root.clone()),
                files: Vec::new(),
            });
        };

        let mut files = Vec::with_capacity(paths.len());
        for named in paths {
            let file = path::beneath(named)
                .map_err(|reason| bad_request!("`{key}`: `{named}` {reason}"))?;
            files.push(file);
        }
        files.sort();
        files.dedup();
        if files.is_empty() {
            return Err(bad_request!("`{key}`: a `Files` seam names no file"));
        }

        Ok(Self {
            workspace: Some(root.clone()),
            files,
        })
    }
}

// The brief: the call's context, the seam and what it is lent; rendered
// as the user turn.
struct Brief<'a, P> {
    ctx: &'a Context<'a, P>,
    seam: &'a Seam,
    lend: &'a Lend,
}

impl<P> Display for Brief<'_, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let input = self.ctx.input;
        write!(
            f,
            "Extract the claim set of the source bound to adapter `{id}` (source key `{key}`).\n\n",
            id = self.ctx.adapter_id,
            key = input.key,
        )?;

        match (self.seam, &input.content) {
            (Seam::Note(note), _) => f.write_str(note)?,
            (Seam::Files(_), _) => {
                writeln!(
                    f,
                    "`$SOURCE_DIR` is the read-only view at `{workspace}` — the source tree. Mine \
                     these files beneath it and nothing else:",
                    workspace = self.lend.workspace.as_deref().unwrap_or_default(),
                )?;
                for file in &self.lend.files {
                    write!(f, "\n- `{file}`")?;
                }
                f.write_str(
                    "\n\nAnchor every `path` relative to `$SOURCE_DIR`. Nothing outside it is \
                     reachable; extract mines only this source.",
                )?;
            }
            (Seam::Whole, SourceContent::Workspace(root)) => write!(
                f,
                "`$SOURCE_DIR` is the read-only view at `{root}` — the source tree the prompt \
                 walks. Nothing outside it is reachable; extract mines only this source."
            )?,
            (Seam::Whole, SourceContent::Value(value)) => write!(
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

// One seam's turn: the embedded prompt as the system, the brief as the user
// turn, the seam's lend, and the claim gate as the check the backend loops
// on until the answer is clean or its rounds are spent.
async fn evidence<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], seam: &Seam, lend: &Lend,
) -> Result<Evidence, Error> {
    let system = emery_prose::body(docs, "extract.md")
        .ok_or_else(|| server_error!("`extract.md` is not embedded"))?;
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
