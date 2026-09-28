//! Mines source seams and combines their claims.
//!
//! An adapter divides its input into [seams](crate#vocabulary). [`extract`]
//! settles every seam against the input, puts each to the model as one gated
//! turn, and returns one evidence document in seam order. Turns run largest
//! first with a bounded number pending, and a turn that fails upstream is put
//! once more.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::path::Path;

use emery_adapter::is_kebab;
use emery_adapter::source::{ClaimKind, Evidence, SourceContent, SourceInput};
use emery_prose::Doc;
use futures::stream::{self, StreamExt as _};
use futures::{FutureExt as _, TryFutureExt as _};
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, bad_request, server_error};

use crate::{CLAIMS, Context, EXTRACT, RUNTIME, beneath, prompt, reference};

/// The most turns one [`extract`] call holds pending at once.
///
/// The cap is on the turns in flight, not on how finely a survey cuts: a seam
/// past this many waits for a slot, and a small seam fills the slot a large
/// one leaves as soon as it answers, so finer cuts pack the slots better even
/// though no more than this many are answered at once.
pub const CONCURRENT: usize = 4;

/// The most bytes of a seam's files that [`extract`] lays into its turn whole.
///
/// A [`Seam::Files`] whose files fit within this together is put with every
/// file's lines in the brief, numbered, so the model cites `path` anchors
/// without reading the lend; past it, or when a file is not UTF-8 text, the
/// files are listed by path instead. An adapter that cuts a tree by size reads
/// it as its threshold for one seam.
pub const INLINE_BYTES: u64 = 64 * 1024;

/// Mines each seam and combines accepted claims into one [`Evidence`] document.
///
/// `docs` must contain `extract.md`. It becomes the system prompt for every
/// request, with the shared `claims.md` of [`RUNTIME`] appended, so each turn
/// carries the id grammar and the gate without a `read_doc` call for them.
/// The model receives the adapter identifier, source name, seam description,
/// and access to the remaining embedded references. Responses are checked
/// with [`Evidence::findings`] and then held to the seam: a `path` names a
/// regular file under the lent root, within a [`Seam::Files`]'s files, with a
/// line range the file holds; on an inline value no claim carries a `path`; a
/// `requirement` or `criterion` id leads with one of a [`Seam::Note`]'s stems
/// when it has them. Rejected responses may be corrected until the host's
/// round limit is reached.
///
/// Up to [`CONCURRENT`] requests run concurrently, largest first. A
/// [`Seam::Files`] is sized by its file count. A [`Seam::Whole`] or
/// [`Seam::Note`] has no known size and goes before them. Ties keep seam
/// order. A request that fails upstream, in the model or a tool transport, is
/// put once more; a refusal is not, and neither is a request the backend's
/// time budget ended, which the backend reports as a budget exhausted — the
/// same request put again takes as long. All requests are awaited, and
/// claims retain the order of `seams`. Every workspace seam uses the same
/// source root, so claim paths share one root-relative namespace.
///
/// When several seams fail, the returned error describes each failure and
/// carries the class and code of the first failed seam.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when `seams` is empty, a [`Seam::Files`]
///   path is invalid, the model rejects the request, or no valid response is
///   produced within the available rounds or the backend's time budget.
/// - Returns [`Error::ServerError`] when [`Seam::Files`] is used with inline
///   input, a [`Seam::Note`] stem is not kebab-case, or `docs` does not
///   contain `extract.md`.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails on
///   the retried request too.
pub async fn extract<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], seams: &[Seam],
) -> Result<Evidence, Error> {
    let source = &ctx.input.name;
    if seams.is_empty() {
        return Err(bad_request!("`{source}`: nothing to extract"));
    }

    // settle every seam and the system before the first turn is spent
    let plans =
        seams.iter().map(|seam| Plan::of(seam, ctx.input)).collect::<Result<Vec<_>, _>>()?;
    let system = format!("{}\n\n---\n\n{}", prompt(docs, EXTRACT)?, prompt(RUNTIME, CLAIMS)?);

    // one gated turn per seam, largest first, at most CONCURRENT pending
    let mut order: Vec<_> = plans.iter().enumerate().collect();
    order.sort_by_key(|(_, plan)| plan.size().map(Reverse));
    let outcomes = stream::iter(order)
        .map(|(index, plan)| {
            turn(&system, ctx, docs, index, plan).map(move |outcome| (index, outcome))
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
    /// A path that escapes the root is rejected. The files are laid into the
    /// turn whole when they fit within [`INLINE_BYTES`] together, and listed
    /// otherwise; a claim's `path` must name one of them.
    Files(Vec<String>),
    /// Adapter-defined instructions describing what to mine.
    ///
    /// For workspace input, the complete root remains available to the model.
    /// The [`Note`] carries the text and the stems the seam's ids lead with.
    Note(Note),
}

/// The instructions of a [`Seam::Note`], with the stems the seam's ids are held to.
///
/// `text` leads the turn. Its first non-blank line is the seam's `label` on
/// the events [`extract`] logs for it, so lead with what the seam covers, such
/// as the surface and its entry, and put the standing instructions after.
///
/// `stems` are the first dotted segments the seam's `requirement` and
/// `criterion` ids lead with, each lowercase kebab-case; a claim under another
/// stem is refused. Empty leaves the ids to the model.
///
/// # Examples
///
/// ```
/// use emery_sdk::{Note, Seam};
///
/// let surface = Seam::Note(Note {
///     text: "Surface `POST /orders` — entry `src/routes.ts` — stem `orders`.".to_string(),
///     stems: vec!["orders".to_string()],
/// });
/// let plain = Seam::Note(Note::from("Mine the brief as one requirement per paragraph."));
/// # let _ = (surface, plain);
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Note {
    /// The seam's instructions, leading the turn.
    pub text: String,
    /// The stems the seam's `requirement` and `criterion` ids lead with; empty for any.
    pub stems: Vec<String>,
}

impl From<String> for Note {
    fn from(text: String) -> Self {
        Self {
            text,
            ..Self::default()
        }
    }
}

impl From<&str> for Note {
    fn from(text: &str) -> Self {
        Self::from(text.to_owned())
    }
}

// A seam settled against the input before any turn is spent. The lend carries
// the root, so no plan names it.
#[derive(Debug)]
enum Plan<'a> {
    Note { text: &'a str, stems: Vec<&'a str> },
    Files(Scope),
    Tree,
    Value(&'a str),
}

impl<'a> Plan<'a> {
    // A `Files` seam over an inline value, or a `Note` with a malformed stem,
    // is the adapter's own defect, so `server_error`; the rest is the
    // operator's input.
    fn of(seam: &'a Seam, input: &'a SourceInput) -> Result<Self, Error> {
        let source = &input.name;
        match (seam, &input.content) {
            (Seam::Note(note), _) => {
                let mut stems: Vec<&str> = note.stems.iter().map(String::as_str).collect();
                if let Some(stem) = stems.iter().find(|stem| !is_kebab(stem)) {
                    return Err(server_error!(
                        "`{source}`: a `Note` seam's stem `{stem}` is not lowercase kebab-case"
                    ));
                }
                stems.sort_unstable();
                stems.dedup();
                Ok(Self::Note {
                    text: &note.text,
                    stems,
                })
            }
            (Seam::Whole, SourceContent::Workspace(_)) => Ok(Self::Tree),
            (Seam::Whole, SourceContent::Value(value)) => Ok(Self::Value(value)),
            (Seam::Files(_), SourceContent::Value(_)) => Err(server_error!(
                "`{source}`: a `Files` seam needs a workspace input, not an inline value"
            )),
            (Seam::Files(named), SourceContent::Workspace(root)) => {
                Ok(Self::Files(Scope::settle(source, root, named)?))
            }
        }
    }

    const fn size(&self) -> Option<usize> {
        match self {
            Self::Files(scope) => Some(scope.files.len()),
            Self::Note { .. } | Self::Tree | Self::Value(_) => None,
        }
    }

    fn label(&self) -> String {
        const WIDTH: usize = 72;
        match self {
            Self::Note { text, .. } => {
                let line = text.lines().find(|line| !line.trim().is_empty()).unwrap_or_default();
                if line.chars().count() > WIDTH {
                    format!("{}…", line.chars().take(WIDTH).collect::<String>())
                } else {
                    line.to_owned()
                }
            }
            Self::Files(scope) => match scope.files.as_slice() {
                [only] => only.clone(),
                [first, rest @ ..] => format!("{first} (+{})", rest.len()),
                [] => String::new(),
            },
            Self::Tree => "tree".to_owned(),
            Self::Value(_) => "value".to_owned(),
        }
    }

    // The rules the claim gate cannot hold an answer to alone: a `path` within
    // the lend and a `Files` seam's files, with lines the file holds, and an
    // id under a `Note` seam's stems.
    fn findings(&self, content: &SourceContent, evidence: &Evidence) -> Vec<String> {
        let mut findings = Vec::new();
        let mut lines: BTreeMap<String, Option<u64>> = BTreeMap::new();
        for (index, claim) in evidence.claims.iter().enumerate() {
            // the stem
            if let Self::Note { stems, .. } = self
                && !stems.is_empty()
                && matches!(claim.kind, ClaimKind::Requirement | ClaimKind::Criterion)
                && let Some(id) = claim.id.as_deref()
            {
                let stem = id.split('.').next().unwrap_or(id);
                if !stems.contains(&stem) {
                    let listed = stems.iter().map(|s| format!("`{s}`")).collect::<Vec<_>>();
                    findings.push(format!(
                        "- claim {index}: id `{id}` leads with `{stem}`, not a stem of this seam \
                         ({})",
                        listed.join(", ")
                    ));
                }
            }

            // the anchor: a malformed one is the gate's finding already
            let Some(Ok(anchor)) = claim.anchor() else { continue };
            let cited = claim.path.as_deref().unwrap_or_default();
            let root = match content {
                SourceContent::Value(_) => {
                    findings.push(format!(
                        "- claim {index}: path `{cited}` cites a file, but the source is an \
                         inline value with no tree lent"
                    ));
                    continue;
                }
                SourceContent::Workspace(root) => root,
            };
            let path = beneath(anchor.path).unwrap_or_else(|_| anchor.path.to_owned());
            if let Self::Files(scope) = self
                && scope.files.binary_search(&path).is_err()
            {
                findings.push(format!(
                    "- claim {index}: path `{cited}` is outside this seam; anchor within the \
                     files it mines"
                ));
                continue;
            }

            let held = *lines.entry(path.clone()).or_insert_with(|| line_count(root, &path));
            match (held, anchor.lines) {
                (None, _) => findings.push(format!(
                    "- claim {index}: path `{cited}` names no regular file under the lent tree"
                )),
                (Some(total), Some((_, end))) if end > total => findings.push(format!(
                    "- claim {index}: path `{cited}` cites line {end}, but `{path}` has {total} \
                     lines"
                )),
                _ => {}
            }
        }
        findings
    }
}

// The lines a regular file holds; `None` for anything else at the path. A
// trailing newline closes the last line rather than opening another.
fn line_count(root: &str, path: &str) -> Option<u64> {
    let full = Path::new(root).join(path);
    if !std::fs::metadata(&full).ok()?.is_file() {
        return None;
    }
    let bytes = std::fs::read(full).ok()?;
    let breaks = bytes.split(|&byte| byte == b'\n').count().saturating_sub(1);
    let open = bytes.last().is_some_and(|&byte| byte != b'\n');
    Some(u64::try_from(breaks + usize::from(open)).unwrap_or(u64::MAX))
}

// The files a `Files` seam is held to, root-relative and sorted, and their
// bodies when they fit within `INLINE_BYTES` together.
#[derive(Debug)]
struct Scope {
    files: Vec<String>,
    laid: Option<Vec<String>>,
}

impl Scope {
    fn settle(source: &str, root: &str, named: &[String]) -> Result<Self, Error> {
        let mut files = named
            .iter()
            .map(|path| {
                beneath(path).map_err(|reason| bad_request!("`{source}`: `{path}` {reason}"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        files.sort();
        files.dedup();

        if files.is_empty() {
            return Err(bad_request!("`{source}`: a `Files` seam names no file"));
        }

        let laid = lay(root, &files);
        Ok(Self { files, laid })
    }
}

// The bodies of `files` when every one is a UTF-8 regular file and they fit
// within `INLINE_BYTES` together; sizes are summed before any body is read.
fn lay(root: &str, files: &[String]) -> Option<Vec<String>> {
    let root = Path::new(root);
    let mut total = 0u64;

    for file in files {
        let meta = std::fs::metadata(root.join(file)).ok()?;
        if !meta.is_file() {
            return None;
        }
        total = total.checked_add(meta.len())?;
        if total > INLINE_BYTES {
            return None;
        }
    }

    files.iter().map(|file| std::fs::read_to_string(root.join(file)).ok()).collect()
}

// The turn is one of several in flight, so its events and its question name
// the seam: the events for the run's log, the question for a backend's
// per-completion telemetry. A failure's description is `join`'s to report
// once, so the event carries the class alone.
#[tracing::instrument(skip_all, fields(source = %ctx.input.name, seam = index))]
async fn turn<P: Model>(
    system: &str, ctx: &Context<'_, P>, docs: &'static [Doc], index: usize, plan: &Plan<'_>,
) -> Result<Evidence, Error> {
    let source = &ctx.input.name;
    let mut question = Question::<Evidence>::new(&format!("evidence-{source}-{index}"))
        .system(system)
        .tools(reference::tools());
    if let SourceContent::Workspace(root) = &ctx.input.content {
        question = question.workspace(root);
    }

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
                Some(reference::serve(docs, source, Some(index))),
                |answer| {
                    let mut findings = answer.findings();
                    findings.extend(plan.findings(&ctx.input.content, answer));
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
            if tracing::enabled!(tracing::Level::TRACE) {
                let json = serde_json::to_string(evidence).unwrap_or_default();
                tracing::trace!(%source, seam = index, evidence = %json, "accepted");
            }
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
            Plan::Note { text, stems } => {
                f.write_str(text)?;
                match stems.as_slice() {
                    [] => {}
                    [stem] => write!(
                        f,
                        "\n\nLead every `requirement` and `criterion` id with the stem `{stem}` \
                         as its first dotted segment; an id under another stem is refused."
                    )?,
                    stems => {
                        let listed = stems.iter().map(|s| format!("`{s}`")).collect::<Vec<_>>();
                        write!(
                            f,
                            "\n\nLead every `requirement` and `criterion` id with one of the \
                             stems {} as its first dotted segment; an id under another stem is \
                             refused.",
                            listed.join(", ")
                        )?;
                    }
                }
            }
            Plan::Files(Scope {
                files,
                laid: Some(laid),
            }) => {
                f.write_str(
                    "`$SOURCE_DIR` is the bound source tree, lent read-only: the root every \
                     `path` is relative to. Mine these files beneath it and nothing else. Each is \
                     laid out here whole, every line led by its number, so cite `#L<n>` from the \
                     numbers shown rather than reading it again:\n\n",
                )?;
                Laid(files, laid).fmt(f)?;
                f.write_str(
                    "\n\nAnchor every `path` relative to `$SOURCE_DIR`, within these files. \
                     Nothing outside it is reachable; extract mines only this source.",
                )?;
            }
            Plan::Files(Scope { files, laid: None }) => {
                f.write_str(
                    "`$SOURCE_DIR` is the bound source tree, lent read-only: the root of every \
                     file you can read. Mine these files beneath it and nothing else:\n",
                )?;
                for file in files {
                    write!(f, "\n- `{file}`")?;
                }
                f.write_str(
                    "\n\nAnchor every `path` relative to `$SOURCE_DIR`, within these files. \
                     Nothing outside it is reachable; extract mines only this source.",
                )?;
            }
            Plan::Tree => f.write_str(
                "`$SOURCE_DIR` is the bound source tree, lent read-only: the root of every file \
                 you can read, and the root every `path` is relative to. Walk it as the prompt \
                 describes. Nothing outside it is reachable; extract mines only this source.",
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

// The files of a scope whole: a heading per file, then its lines in a fence
// no run of backticks inside can close, each led by its number.
struct Laid<'a>(&'a [String], &'a [String]);

impl Display for Laid<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (position, (file, body)) in self.0.iter().zip(self.1).enumerate() {
            if position > 0 {
                f.write_str("\n\n")?;
            }
            let lines: Vec<&str> = body.lines().collect();
            let width = lines.len().max(1).to_string().len();
            let fence = "`".repeat(longest_run(body).max(2) + 1);
            writeln!(f, "### `{file}` ({} lines)\n\n{fence}", lines.len())?;
            for (number, line) in (1..).zip(&lines) {
                writeln!(f, "{number:>width$}|{line}")?;
            }
            f.write_str(&fence)?;
        }
        Ok(())
    }
}

fn longest_run(body: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in body.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    longest
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
