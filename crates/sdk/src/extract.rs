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
use emery_adapter::source::{Anchor, ClaimKind, Evidence, SourceContent, SourceInput};
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
/// A [`Seam`]'s `files` are laid into the brief in order, every line numbered
/// so the model cites `path` anchors without reading the lend, for as long as
/// they fit within this together; the first that does not, or that is not
/// UTF-8 text, and every file after it are listed by path instead. An adapter
/// that cuts a tree by size reads it as its threshold for one seam, and one
/// that lays a closure puts the entry first.
pub const INLINE_BYTES: u64 = 64 * 1024;

// How many of a seam's anchors in a claim's file a finding names.
const NEAREST: usize = 16;

/// Mines each seam and combines accepted claims into one [`Evidence`] document.
///
/// `docs` must contain `extract.md`. It becomes the system prompt for every
/// request, with the shared `claims.md` of [`RUNTIME`] appended, so each turn
/// carries the id grammar and the gate without a `read_doc` call for them.
/// The model receives the adapter identifier, source name, the seam's `text`,
/// its `files` or the whole input, its `stems`, and access to the remaining
/// embedded references. Responses are checked with [`Evidence::findings`] and
/// then held to the seam: a `path` names a regular file under the lent root,
/// within the seam's `files` when it names any, with a line range the file
/// holds; on an inline value no claim carries a `path`; a `requirement` or
/// `criterion` id leads with one of the seam's `stems` when it has them; a
/// `requirement`'s `path` overlaps one of the seam's `anchors` when it has
/// them. Rejected responses may be corrected until the host's round limit is
/// reached.
///
/// Up to [`CONCURRENT`] requests run concurrently, largest first. A seam
/// naming files is sized by their count; one over the whole input has no
/// known size and goes before them. Ties keep seam order. A request that
/// fails upstream, in the model or a tool transport, is put once more; a
/// refusal is not, and neither is a request the backend's time budget ended,
/// which the backend reports as a budget exhausted — the same request put
/// again takes as long. All requests are awaited, and claims retain the order
/// of `seams`. Every workspace seam uses the same source root, so claim paths
/// share one root-relative namespace.
///
/// When several seams fail, the returned error describes each failure and
/// carries the class and code of the first failed seam.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when `seams` is empty, a seam's file path
///   is invalid, the model rejects the request, or no valid response is
///   produced within the available rounds or the backend's time budget.
/// - Returns [`Error::ServerError`] when a seam names files or anchors over an
///   inline value, a stem is not kebab-case, an anchor is outside the `path`
///   grammar, or `docs` does not contain `extract.md`.
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
/// An adapter's survey chooses one or more seams before any call is made; see
/// the [vocabulary](crate#vocabulary). Each field narrows the turn and an
/// empty one leaves it open, so [`Seam::whole`] is the input as it stands:
///
/// - `text` leads the turn. Its first non-blank line is the seam's `label` on
///   the events [`extract`] logs for it, so lead with what the seam covers,
///   such as the surface and its entry, and put the standing instructions
///   after.
/// - `files` are the files mined beneath a workspace root, relative to it;
///   empty mines the whole lend. They keep their order, deduplicated, and the
///   leading ones are laid into the turn whole for as long as they fit within
///   [`INLINE_BYTES`] together, the rest listed, so an entry comes first; a
///   claim's `path` must name one of them. A path that escapes the root is
///   rejected; files over an inline value are the adapter's own defect.
/// - `stems` are the first dotted segments the seam's `requirement` and
///   `criterion` ids lead with, each lowercase kebab-case; a claim under
///   another stem is refused. Empty leaves the ids to the model.
/// - `anchors` are the lines a `requirement` may anchor at, each in the
///   `path` grammar (`<path>#L<n>`, `<path>#L<start>-L<end>`, or a bare
///   `<path>` for the whole file) relative to the workspace root — where an
///   adapter's survey found a behaviour can start; a `requirement` whose
///   `path` overlaps none of them is refused with the nearest anchors in its
///   file named, so the model re-anchors it or leaves it out. Other kinds
///   are not held to them. Empty leaves the anchors to the model; anchors
///   over an inline value are the adapter's own defect.
///
/// # Examples
///
/// ```
/// use emery_sdk::Seam;
///
/// let brief = Seam::whole();
/// let module = Seam::files(["src/orders.ts", "src/lib/pricing.ts"]);
/// let anchored = Seam::anchors(["src/orders.ts#L4-L9"]);
/// let surface = Seam {
///     text: "Surface `POST /orders` — entry `src/routes.ts` — stem `orders`.".to_owned(),
///     files: vec!["src/routes.ts".to_owned(), "src/lib/pricing.ts".to_owned()],
///     stems: vec!["orders".to_owned()],
///     anchors: vec!["src/routes.ts#L6-L9".to_owned(), "src/lib/pricing.ts#L12".to_owned()],
/// };
/// # let _ = (brief, module, anchored, surface);
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Seam {
    /// The seam's instructions, leading the turn; empty for none.
    pub text: String,
    /// The files mined, relative to the workspace root; empty for the whole lend.
    pub files: Vec<String>,
    /// The stems the seam's `requirement` and `criterion` ids lead with; empty for any.
    pub stems: Vec<String>,
    /// The lines a `requirement` anchors at, in the `path` grammar; empty for any.
    pub anchors: Vec<String>,
}

impl Seam {
    /// Returns the seam over the complete workspace or inline value.
    #[must_use]
    pub fn whole() -> Self {
        Self::default()
    }

    /// Returns the seam over `files` beneath the workspace root, with no text and no stems.
    pub fn files(files: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            files: files.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Returns the seam over the whole input led by `text`.
    pub fn note(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// Returns the seam over the whole workspace whose `requirement`s anchor at
    /// `anchors`, each in the `path` grammar.
    pub fn anchors(anchors: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            anchors: anchors.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }
}

// A seam settled against the input before any turn is spent. The lend carries
// the root, so no plan names it.
#[derive(Debug)]
struct Plan<'a> {
    text: &'a str,
    stems: Vec<&'a str>,
    lend: Lend<'a>,
    anchors: Vec<Anchored>,
}

// One line span a `requirement` may anchor at, root-relative; no lines is
// the whole file.
#[derive(Debug)]
struct Anchored {
    path: String,
    lines: Option<(u64, u64)>,
}

impl Anchored {
    // Whether a claim's anchor at `path` over `lines` shares a line with this
    // one. A whole-file span here covers every line of its file; a claim
    // citing a whole file against listed lines cites none of them.
    fn overlaps(&self, path: &str, lines: Option<(u64, u64)>) -> bool {
        self.path == path
            && match (self.lines, lines) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some((start, end)), Some((from, to))) => from <= end && start <= to,
            }
    }
}

// What the turn is put over: the lent tree, files settled beneath it, or the
// inline value itself.
#[derive(Debug)]
enum Lend<'a> {
    Tree,
    Files(Scope),
    Value(&'a str),
}

impl<'a> Plan<'a> {
    // Files over an inline value, or a malformed stem, is the adapter's own
    // defect, so `server_error`; the rest is the operator's input.
    fn of(seam: &'a Seam, input: &'a SourceInput) -> Result<Self, Error> {
        let source = &input.name;

        // the stems
        let mut stems: Vec<&str> = seam.stems.iter().map(String::as_str).collect();
        if let Some(stem) = stems.iter().find(|stem| !is_kebab(stem)) {
            return Err(server_error!(
                "`{source}`: a seam's stem `{stem}` is not lowercase kebab-case"
            ));
        }
        stems.sort_unstable();
        stems.dedup();

        // the lend
        let lend = match (&input.content, seam.files.is_empty()) {
            (SourceContent::Workspace(_), true) => Lend::Tree,
            (SourceContent::Workspace(root), false) => {
                Lend::Files(Scope::settle(source, root, &seam.files)?)
            }
            (SourceContent::Value(value), true) => Lend::Value(value),
            (SourceContent::Value(_), false) => {
                return Err(server_error!(
                    "`{source}`: a seam names files, but the source is an inline value with no \
                     tree to lend"
                ));
            }
        };

        // the anchors
        if matches!(lend, Lend::Value(_)) && !seam.anchors.is_empty() {
            return Err(server_error!(
                "`{source}`: a seam names anchors, but the source is an inline value with no tree \
                 to anchor in"
            ));
        }
        let mut anchors = Vec::with_capacity(seam.anchors.len());
        for anchor in &seam.anchors {
            let parsed = Anchor::parse(anchor).map_err(|reason| {
                server_error!("`{source}`: a seam's anchor `{anchor}` {reason}")
            })?;
            let path = beneath(parsed.path).unwrap_or_else(|_| parsed.path.to_owned());
            anchors.push(Anchored {
                path,
                lines: parsed.lines,
            });
        }

        Ok(Self {
            text: &seam.text,
            stems,
            lend,
            anchors,
        })
    }

    const fn size(&self) -> Option<usize> {
        match &self.lend {
            Lend::Files(scope) => Some(scope.files.len()),
            Lend::Tree | Lend::Value(_) => None,
        }
    }

    fn label(&self) -> String {
        const WIDTH: usize = 72;
        if let Some(line) = self.text.lines().find(|line| !line.trim().is_empty()) {
            return if line.chars().count() > WIDTH {
                format!("{}…", line.chars().take(WIDTH).collect::<String>())
            } else {
                line.to_owned()
            };
        }
        match &self.lend {
            Lend::Files(scope) => match scope.files.as_slice() {
                [only] => only.clone(),
                [first, rest @ ..] => format!("{first} (+{})", rest.len()),
                [] => String::new(),
            },
            Lend::Tree => "tree".to_owned(),
            Lend::Value(_) => "value".to_owned(),
        }
    }

    // The rules the claim gate cannot hold an answer to alone: a `path` within
    // the lend and the seam's files, with lines the file holds, an id under
    // the seam's stems, and a `requirement` at one of the seam's anchors.
    fn findings(&self, content: &SourceContent, evidence: &Evidence) -> Vec<String> {
        let mut findings = Vec::new();
        let mut lines: BTreeMap<String, Option<u64>> = BTreeMap::new();
        for (index, claim) in evidence.claims.iter().enumerate() {
            // the stem
            if !self.stems.is_empty()
                && matches!(claim.kind, ClaimKind::Requirement | ClaimKind::Criterion)
                && let Some(id) = claim.id.as_deref()
            {
                let stem = id.split('.').next().unwrap_or(id);
                if !self.stems.contains(&stem) {
                    let listed = self.stems.iter().map(|s| format!("`{s}`")).collect::<Vec<_>>();
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
            if let Lend::Files(scope) = &self.lend
                && !scope.files.contains(&path)
            {
                findings.push(format!(
                    "- claim {index}: path `{cited}` is outside this seam; anchor within the \
                     files it mines"
                ));
                continue;
            }

            let held = *lines.entry(path.clone()).or_insert_with(|| line_count(root, &path));
            match (held, anchor.lines) {
                (None, _) => {
                    findings.push(format!(
                        "- claim {index}: path `{cited}` names no regular file under the lent tree"
                    ));
                    continue;
                }
                (Some(total), Some((_, end))) if end > total => {
                    findings.push(format!(
                        "- claim {index}: path `{cited}` cites line {end}, but `{path}` has \
                         {total} lines"
                    ));
                    continue;
                }
                _ => {}
            }

            // a requirement at one of the seam's anchors, the nearest in its
            // file named so one correction can land
            if claim.kind == ClaimKind::Requirement
                && !self.anchors.is_empty()
                && !self.anchors.iter().any(|at| at.overlaps(&path, anchor.lines))
            {
                let nearest = self.nearest(&path, anchor.lines);
                if nearest.is_empty() {
                    findings.push(format!(
                        "- claim {index}: path `{cited}` is at none of the lines this seam names \
                         for a `requirement`, and it names none in `{path}`; anchor it in a file \
                         where its behaviour starts, or leave it out"
                    ));
                } else {
                    findings.push(format!(
                        "- claim {index}: path `{cited}` is at none of the lines this seam names \
                         for a `requirement`; in `{path}` it names {}; anchor it at the one where \
                         its behaviour starts, or leave it out",
                        nearest.join(", ")
                    ));
                }
            }
        }
        findings
    }

    // The seam's anchors in `path` nearest to `lines` — up to `NEAREST`, in
    // file order — each as `L<n>` or `L<n>-L<n>`.
    fn nearest(&self, path: &str, lines: Option<(u64, u64)>) -> Vec<String> {
        let mut spans: Vec<(u64, u64)> =
            self.anchors.iter().filter(|at| at.path == path).filter_map(|at| at.lines).collect();
        spans.sort_unstable();
        spans.dedup();
        if let Some((from, to)) = lines
            && spans.len() > NEAREST
        {
            let distance = |&(start, end): &(u64, u64)| {
                if end < from { from - end } else { start.saturating_sub(to) }
            };
            spans.sort_by_key(distance);
            spans.truncate(NEAREST);
            spans.sort_unstable();
        } else {
            spans.truncate(NEAREST);
        }
        spans
            .into_iter()
            .map(
                |(start, end)| {
                    if start == end { format!("L{start}") } else { format!("L{start}-L{end}") }
                },
            )
            .collect()
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

// The files a seam is held to, root-relative in the adapter's order, and the
// bodies of the leading ones that fit within `INLINE_BYTES` together.
#[derive(Debug)]
struct Scope {
    files: Vec<String>,
    laid: Vec<String>,
}

impl Scope {
    fn settle(source: &str, root: &str, named: &[String]) -> Result<Self, Error> {
        let mut files: Vec<String> = Vec::with_capacity(named.len());
        for path in named {
            let file =
                beneath(path).map_err(|reason| bad_request!("`{source}`: `{path}` {reason}"))?;
            if !files.contains(&file) {
                files.push(file);
            }
        }

        let laid = lay(root, &files);
        Ok(Self { files, laid })
    }

    // The files past the laid ones.
    fn listed(&self) -> &[String] {
        &self.files[self.laid.len()..]
    }
}

// The bodies of the leading files of `files`, in order, for as long as each
// is a UTF-8 regular file and they fit within `INLINE_BYTES` together; the
// first that is not, or does not, ends the run and is listed with the rest.
fn lay(root: &str, files: &[String]) -> Vec<String> {
    let root = Path::new(root);
    let mut total = 0u64;
    let mut laid = Vec::new();

    for file in files {
        let path = root.join(file);
        let Ok(meta) = std::fs::metadata(&path) else { break };
        if !meta.is_file() {
            break;
        }
        let Some(sum) = total.checked_add(meta.len()) else { break };
        if sum > INLINE_BYTES {
            break;
        }
        let Ok(body) = std::fs::read_to_string(path) else { break };
        total = sum;
        laid.push(body);
    }

    laid
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

        // the adapter's text
        if !self.plan.text.is_empty() {
            f.write_str(self.plan.text)?;
            f.write_str("\n\n")?;
        }

        // the lend
        match &self.plan.lend {
            Lend::Files(scope) if scope.laid.is_empty() => {
                f.write_str(
                    "`$SOURCE_DIR` is the bound source tree, lent read-only: the root of every \
                     file you can read. Mine these files beneath it and nothing else:\n",
                )?;
                for file in &scope.files {
                    write!(f, "\n- `{file}`")?;
                }
            }
            Lend::Files(scope) if scope.listed().is_empty() => {
                f.write_str(
                    "`$SOURCE_DIR` is the bound source tree, lent read-only: the root every \
                     `path` is relative to. Mine these files beneath it and nothing else. Each is \
                     laid out here whole, every line led by its number, so cite `#L<n>` from the \
                     numbers shown rather than reading it again:\n\n",
                )?;
                Laid(&scope.files, &scope.laid).fmt(f)?;
            }
            Lend::Files(scope) => {
                let (laid, those) = match scope.laid.len() {
                    1 => ("The first is".to_owned(), "it"),
                    n => (format!("The first {n} are"), "them"),
                };
                write!(
                    f,
                    "`$SOURCE_DIR` is the bound source tree, lent read-only: the root every \
                     `path` is relative to. Mine these files beneath it and nothing else. {laid} \
                     laid out here whole, every line led by its number, so cite `#L<n>` from the \
                     numbers shown rather than reading {those} again; the rest are listed after \
                     {those}, to read from `$SOURCE_DIR` as the seam reaches them:\n\n"
                )?;
                Laid(&scope.files, &scope.laid).fmt(f)?;
                f.write_str("\n\nThe rest of this seam's files:\n")?;
                for file in scope.listed() {
                    write!(f, "\n- `{file}`")?;
                }
            }
            Lend::Tree => f.write_str(
                "`$SOURCE_DIR` is the bound source tree, lent read-only: the root of every file \
                 you can read, and the root every `path` is relative to. Walk it as the prompt \
                 describes.",
            )?,
            Lend::Value(value) => write!(
                f,
                "The bound seam is this inline value; no `$SOURCE_DIR` is lent:\n\n{value}"
            )?,
        }

        // the stems
        match self.plan.stems.as_slice() {
            [] => {}
            [stem] => write!(
                f,
                "\n\nLead every `requirement` and `criterion` id with the stem `{stem}` as its \
                 first dotted segment; an id under another stem is refused."
            )?,
            stems => {
                let listed = stems.iter().map(|s| format!("`{s}`")).collect::<Vec<_>>();
                write!(
                    f,
                    "\n\nLead every `requirement` and `criterion` id with one of the stems {} as \
                     its first dotted segment; an id under another stem is refused.",
                    listed.join(", ")
                )?;
            }
        }

        // the anchors
        if !self.plan.anchors.is_empty() {
            f.write_str(
                "\n\nA `requirement` anchors at one of the lines the text above lists — where its \
                 behaviour starts; one anchored at any other line is refused.",
            )?;
        }

        // the anchor rule
        f.write_str(match &self.plan.lend {
            Lend::Files(_) => {
                "\n\nAnchor every `path` relative to `$SOURCE_DIR`, within these files. Nothing \
                 outside it is reachable; extract mines only this source."
            }
            Lend::Tree => {
                "\n\nAnchor every `path` relative to `$SOURCE_DIR`. Nothing outside it is \
                 reachable; extract mines only this source."
            }
            Lend::Value(_) => "\n\nNothing else is reachable; extract mines only this source.",
        })?;

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
            let count = match lines.len() {
                1 => "1 line".to_owned(),
                n => format!("{n} lines"),
            };
            writeln!(f, "### `{file}` ({count})\n\n{fence}")?;
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
