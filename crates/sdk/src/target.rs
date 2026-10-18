//! Provides the types and functions a target adapter is written with.
//!
//! A target adapter runs one [`Task`] at a time in a tree the engine lends
//! it and answers an [`Outcome`]: an init lays the layout's initial files
//! the tree lacks, once per repository a build lands in and before any
//! slice, with no model turn; a build lays one [`Slice`] of the plan and
//! answers a [`Report`]; a verify runs the checks over the integrated tree
//! once a wave of slices has merged and answers a [`Verdict`]. [`build`]
//! puts the one gated build turn: the slice's documents under the adapter's
//! `build.md`, the workspace lent and written through the turn's
//! `write_files` and `edit_files` tools, and the answered report held to
//! the slice by [`Report::findings`], to the tree, and to the adapter's own
//! check until it passes, its `written` filled in from what the tools
//! wrote. [`verify`] puts the one gated verify turn: the integrated tree
//! lent under the adapter's `verify.md`, the checks it names run through
//! the model's shell, what they find repaired through the same two tools,
//! and the answered verdict held to [`Verdict::findings`] and the adapter's
//! check. Both turns carry the adapter's [`Worker`], what the model's worker
//! reaches beyond the tree for the toolchain the prompts run.
//! [`TargetAdapter`] is what a target adapter implements, and
//! [`export_target!`](crate::export_target) exports a type implementing it,
//! answering the component's `metadata` through [`metadata`] over the
//! type's [`MERGE_RULES`](TargetAdapter::MERGE_RULES) and
//! [`BASE`](TargetAdapter::BASE).
//!
//! # Examples
//!
//! This adapter initialises every repository with one manifest, builds every
//! slice, and verifies every wave through the SDK's turns, holding each
//! report to one rule of its layout's own:
//!
//! ```
//! use std::fs::OpenOptions;
//! use std::io::{ErrorKind, Write as _};
//! use std::path::Path;
//!
//! use emery_sdk::target::{Base, Outcome, TargetAdapter, Task, Worker};
//! use emery_sdk::{Doc, Error, Model, server_error};
//!
//! pub struct Adapter;
//!
//! impl TargetAdapter for Adapter {
//!     const BASE: Base = Base::Fresh;
//!     const WORKER: Worker = Worker {
//!         egress: &["index.crates.io", "static.crates.io"],
//!         read: &["~/.rustup", "~/.cargo/bin"],
//!         env: &[("RUSTUP_HOME", "~/.rustup")],
//!     };
//!
//!     async fn run<P: Model>(task: Task<'_, P>) -> Result<Outcome, Error> {
//!         match task {
//!             Task::Init(ctx) => {
//!                 // a held manifest stands; the tree lacking one gets the layout's
//!                 let manifest = Path::new(ctx.workspace).join("Cargo.toml");
//!                 match OpenOptions::new().write(true).create_new(true).open(manifest) {
//!                     Ok(mut file) => file
//!                         .write_all(b"[workspace]\nmembers = [\"crates/*\"]\n")
//!                         .map_err(|err| server_error!("laying `Cargo.toml`: {err}"))?,
//!                     Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
//!                     Err(err) => return Err(server_error!("laying `Cargo.toml`: {err}")),
//!                 }
//!                 Ok(Outcome::Initialised)
//!             }
//!             Task::Build(ctx) => emery_sdk::target::build(&ctx, PROSE, |report| {
//!                 let stray = report.written.iter().filter(|path| !path.starts_with("crates/"));
//!                 Ok(stray.map(|path| format!("`{path}` lies outside `crates/`")).collect())
//!             })
//!             .await
//!             .map(Outcome::Built),
//!             Task::Verify(ctx) => emery_sdk::target::verify(&ctx, PROSE, |_| Ok(Vec::new()))
//!                 .await
//!                 .map(Outcome::Verified),
//!         }
//!     }
//! }
//!
//! pub static PROSE: &[Doc] = &[
//!     Doc {
//!         path: "build.md",
//!         body: "Implement each requirement as a Rust module under `src/`.",
//!     },
//!     Doc {
//!         path: "verify.md",
//!         body: "Run `cargo test` in `$WORKSPACE`.",
//!     },
//! ];
//!
//! #[cfg(target_arch = "wasm32")]
//! emery_sdk::export_target!(Adapter);
//! # fn main() {}
//! ```

mod write;

use std::fmt::{self, Display, Formatter};
use std::path::Path;

#[cfg(target_arch = "wasm32")]
#[doc(inline)]
pub use emery_adapter::target::export;
pub use emery_adapter::target::{
    Base, MergeRule, MergeStrategy, Outcome, Report, Slice, TargetMetadata, Verdict,
};
use emery_prose::Doc;
use omnia_sdk::model::{Question, WorkerGrant};
use omnia_sdk::{Error, Model};

use self::write::{Writer, Written};
use crate::{BUILD, VERIFY, prompt, reference};

/// The engine's side of the contract, for a guest that dispatches to a target adapter.
///
/// [`Target`](capability::Target) is the capability the engine puts a
/// [`Task`](capability::Task) through, `init`, `build(slice)`, or `verify`,
/// each answered by the [`Outcome`] of its kind. On `wasm32` its methods
/// dispatch through the component's import, so a guest implements it on a
/// unit type to call the adapter the host registered under an id.
pub mod capability {
    pub use emery_adapter::target::{Target, Task};
}

const VERIFY_TURN: &str = "verify";

/// What the model's worker reaches beyond the tree, for the toolchain the adapter's prompts run.
///
/// A target adapter spells it once as
/// [`WORKER`](TargetAdapter::WORKER), and [`build`] and [`verify`] carry it
/// with every turn as the completion's worker grant; the host holds each
/// entry to its shape and the backend lays what exists on its machine. The
/// adapter is trusted by its digest, so what it spells is what the
/// deployment granted when it pinned it. [`Worker::NONE`] reaches nothing
/// beyond the tree and puts no grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Worker {
    /// Host names the worker's commands may reach through the backend's
    /// egress proxy: a name whole, or `*.suffix` for a domain and every
    /// host beneath it. Never an IP literal, a port, a scheme, or a path.
    pub egress: &'static [&'static str],
    /// Directories the worker may read beside its lend and the system,
    /// each `~/<path>`: beneath the operator's home, never `~/` itself and
    /// never holding `..`. One the machine lacks grants nothing.
    pub read: &'static [&'static str],
    /// Environment variables set for the worker, each a name of the shape
    /// `[A-Za-z_][A-Za-z0-9_]*` and a value whose leading `~/` the backend
    /// expands to the operator's home.
    pub env: &'static [(&'static str, &'static str)],
}

impl Worker {
    /// A worker reaching nothing beyond the tree: no grant is put.
    pub const NONE: Self = Self {
        egress: &[],
        read: &[],
        env: &[],
    };

    /// Returns the grant a turn carries for this worker: `None` for
    /// [`Worker::NONE`], the lists as spelled otherwise.
    #[must_use]
    pub fn grant(&self) -> Option<WorkerGrant> {
        (*self != Self::NONE).then(|| WorkerGrant::from(*self))
    }
}

impl From<Worker> for WorkerGrant {
    fn from(worker: Worker) -> Self {
        Self {
            egress: worker.egress.iter().map(|host| (*host).to_owned()).collect(),
            read: worker.read.iter().map(|root| (*root).to_owned()).collect(),
            env: worker
                .env
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }
}

/// The adapter addressed and the tree it initialises.
///
/// An init carries no model: it is the adapter's own code laying the
/// layout's initial files where the tree lacks them, so two inits over one
/// tree lay the same files.
#[derive(Debug)]
pub struct InitContext<'a> {
    /// The identifier used to address the adapter.
    pub adapter_id: &'a str,
    /// The deployment-local path of the tree root the layout is laid
    /// beneath. The deployment's grant decides whether it may be written.
    pub workspace: &'a str,
}

/// The adapter addressed, the slice it builds, the tree it builds into, and the model.
///
/// [`build`] takes it, so the one turn of a call is put to the one model the
/// host bound.
#[derive(Debug)]
pub struct BuildContext<'a, P> {
    /// The identifier used to address the adapter.
    pub adapter_id: &'a str,
    /// The [`Slice`] to build.
    pub slice: &'a Slice,
    /// The deployment-local path of the tree root, which the turn lends to
    /// the model and writes beneath through `write_files` and `edit_files`.
    /// The deployment's grant decides whether it may be written.
    pub workspace: &'a str,
    /// What the model's worker reaches beyond the tree, the adapter's
    /// [`WORKER`](TargetAdapter::WORKER).
    pub worker: &'a Worker,
    /// The [`Model`] the turn is put to.
    pub model: &'a P,
}

/// The adapter addressed, the integrated tree it verifies, and the model.
///
/// [`verify`] takes it, so the one turn of a call is put to the one model
/// the host bound.
#[derive(Debug)]
pub struct VerifyContext<'a, P> {
    /// The identifier used to address the adapter.
    pub adapter_id: &'a str,
    /// The deployment-local path of the integrated tree's root, which the
    /// turn lends to the model for its checks to run in and repairs beneath
    /// through `write_files` and `edit_files`.
    pub workspace: &'a str,
    /// What the model's worker reaches beyond the tree, the adapter's
    /// [`WORKER`](TargetAdapter::WORKER).
    pub worker: &'a Worker,
    /// The [`Model`] the turn is put to.
    pub model: &'a P,
}

/// The task a target adapter is asked, each with the context its turn needs.
///
/// [`TargetAdapter::run`] takes one and answers the [`Outcome`] of its
/// kind: [`Outcome::Initialised`] to [`Init`](Self::Init),
/// [`Outcome::Built`] to [`Build`](Self::Build), [`Outcome::Verified`] to
/// [`Verify`](Self::Verify).
#[derive(Debug)]
pub enum Task<'a, P> {
    /// Lay the layout's initial files the tree lacks, a held file left as
    /// it stands. Put once per repository a build lands in, before any
    /// slice; the engine seals what the tree then holds as its
    /// `Initialise` commit.
    Init(InitContext<'a>),
    /// Build one slice of the plan into the tree, through
    /// [`build`](fn@build).
    Build(BuildContext<'a, P>),
    /// Run the checks over the integrated tree, repair what they find, and
    /// answer a verdict, through [`verify`](fn@verify).
    Verify(VerifyContext<'a, P>),
}

/// A target adapter: how it initialises a repository with its layout, builds one slice of the plan into the tree, and verifies the tree.
///
/// Implement it on a unit struct and hand that type to
/// [`export_target!`](crate::export_target), which exports the
/// `target-adapter` world over it.
pub trait TargetAdapter {
    /// The rules the engine merges every slice into the integrated tree
    /// under, since the adapter alone knows which files its builds
    /// regenerate and which they extend.
    ///
    /// The first rule matching a conflicting path applies. A conflict no
    /// rule resolves is the engine's: it resolves the file with the model
    /// and merges the resolution, so the rules spare a turn where a list or
    /// a regenerated file decides the outcome. None by default.
    const MERGE_RULES: &'static [MergeRule] = &[];

    /// What the model's worker reaches beyond the tree in every build and
    /// verify turn, since the adapter alone knows the toolchain its prompts
    /// run: the registries its installs fetch from, the `~/` roots a
    /// toolchain is installed under, the variables it reads.
    ///
    /// [`Worker::NONE`] by default, for an adapter whose turns run nothing
    /// that reaches beyond the tree.
    const WORKER: Worker = Worker::NONE;

    /// The base the engine admits a build of this adapter over, since the
    /// adapter alone knows whether its layout is laid into a fresh tree or
    /// stands beside what a project already holds.
    ///
    /// [`Base::Any`] by default. [`Base::Fresh`] has the engine refuse a
    /// base whose root holds a directory before any slice builds, so an
    /// adapter that lays a whole repository never rewrites a held one.
    const BASE: Base = Base::Any;

    /// Runs `task` in the tree its context names and answers the outcome of
    /// its kind.
    ///
    /// An init lays the layout's initial files the tree lacks through the
    /// adapter's own code and answers [`Outcome::Initialised`]; a build's
    /// one turn goes through [`build`](fn@build) and answers
    /// [`Outcome::Built`]; a verify's through [`verify`](fn@verify) and
    /// answers [`Outcome::Verified`]. An outcome of another kind is the
    /// adapter's defect, which the export refuses as `internal`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the tree refuses the init, as
    ///   when a directory or a link stands where a file of the layout goes.
    /// - Returns [`Error::ServerError`] when the init cannot write the tree.
    /// - Returns what [`build`](fn@build) or [`verify`](fn@verify) returns.
    fn run<P: Model>(task: Task<'_, P>) -> impl Future<Output = Result<Outcome, Error>>;
}

/// Returns the `metadata` answer for a target adapter merging under `merge_rules` and building over `base`.
///
/// [`export_target!`](crate::export_target) answers the component's
/// `metadata` with it over the adapter's
/// [`MERGE_RULES`](TargetAdapter::MERGE_RULES) and
/// [`BASE`](TargetAdapter::BASE). The engine merges every slice under the
/// rules first and resolves what they leave with the model. The
/// `emery-version` pin is this SDK's own version, identifying the contract
/// the adapter compiled against. Build a [`TargetMetadata`] directly only
/// when the adapter must loosen or tighten that pin.
#[must_use]
pub fn metadata(merge_rules: &[MergeRule], base: Base) -> TargetMetadata {
    TargetMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        merge_rules: merge_rules.to_vec(),
        base,
    }
}

/// Builds the slice of `ctx` into its workspace through one gated turn.
///
/// `docs` must contain `build.md`, which becomes the system prompt. The turn
/// carries the slice's plan entry, its cut of the specification, and the
/// whole design, lends the workspace with the grant of `ctx`'s [`Worker`],
/// and offers the embedded references through the reference tools. It writes
/// through two tools. `write_files` writes one or more files beneath the
/// workspace per call, listed or mapped by path, each created or replaced
/// whole, at a path [`beneath`](crate::beneath) accepts and no symlink
/// stands on, and removes the files its `delete` names; an entry that would
/// leave a held file empty or under half its size is refused unless it sets
/// `shrink`. `edit_files` replaces the one place each edit's `find` matches
/// within a file the workspace holds. A call either tool refuses anywhere
/// changes nothing. The answered [`Report`] is held to the slice by
/// [`Report::findings`], to the tree — a `written` path names a regular
/// file under the workspace — and to `check`, the adapter's own rule over
/// the report and the tree as the model left it; every finding, the
/// adapter's among the rest, is returned to the model for one correction
/// round, until the host's round limit is reached. A finding is an
/// instruction the model repairs by, so it names the repair, not the
/// difference alone. `check` runs on every candidate, so it reads what one
/// gate reads and runs nothing. An adapter with no rule passes
/// `|_| Ok(Vec::new())`. The accepted report's `written` is filled in:
/// every path it lists, spelled root-relative, and every file a tool wrote
/// that the tree still holds, once each and sorted.
///
/// A failure upstream is not put again here: the turn writes the tree as it
/// goes, so a second turn would start over what the first left. The caller
/// that owns the tree may cut a fresh one and ask again.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the model rejects the request, or
///   when no valid report is produced within the available rounds or the
///   backend's time budget.
/// - Returns [`Error::ServerError`] when `docs` does not contain `build.md`.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
/// - Returns the error `check` fails with, whatever its class: the
///   candidate it failed on ends the turn, since a tree the adapter cannot
///   read is the run's failure and not the model's to repair.
pub async fn build<P: Model>(
    ctx: &BuildContext<'_, P>, docs: &'static [Doc],
    mut check: impl FnMut(&Report) -> Result<Vec<String>, Error> + Send,
) -> Result<Report, Error> {
    let slice = ctx.slice;
    let id = &slice.id;
    let root = Path::new(ctx.workspace);
    let writer = Writer::new(ctx.workspace, id);
    let written = writer.written();
    let mut tools = reference::tools();
    tools.extend(Writer::tools());
    let question = granted(
        Question::<Report>::new(&format!("build-{id}"))
            .system(prompt(docs, BUILD)?)
            .tools(tools)
            .workspace(ctx.workspace),
        ctx.worker,
    );
    let brief = Brief {
        adapter_id: ctx.adapter_id,
        slice,
    };

    tracing::info!(
        slice = %id,
        name = %slice.name,
        requirements = slice.requirements.len(),
        "building"
    );
    let tools = writer.serve(reference::serve(docs, &[], id, None));
    let mut failed = None;
    let report = question
        .ask(ctx.model, brief.to_string(), Some(tools), |answer| {
            let mut findings = answer.findings(slice);
            findings.extend(Written::findings(root, answer));
            match check(answer) {
                Ok(more) => findings.extend(more),
                Err(error) => {
                    stash(&mut failed, error);
                    return Ok(());
                }
            }
            if findings.is_empty() {
                return Ok(());
            }
            tracing::debug!(slice = %id, ?findings, "candidate rejected");
            Err(findings)
        })
        .await
        .map_err(Error::from);
    if let Some(error) = failed {
        return Err(error);
    }
    let mut report = report?;
    written.fill(root, &mut report);

    let uncovered = report.uncovered(slice);
    tracing::info!(
        slice = %id,
        covered = report.covered.len(),
        uncovered = uncovered.len(),
        written = report.written.len(),
        "built"
    );
    tracing::debug!(slice = %id, ?uncovered, written = ?report.written, "the build's report");

    Ok(report)
}

/// Verifies the integrated tree of `ctx` through one gated turn.
///
/// `docs` must contain `verify.md`, which becomes the system prompt. The turn
/// lends the tree with the grant of `ctx`'s [`Worker`] and offers the
/// embedded references through the reference tools and the same
/// `write_files` and `edit_files` a build has: the model
/// runs the checks the prompt names through its shell, repairs what they
/// find in the tree, runs them again, and answers what they finally found.
/// The answered [`Verdict`] is held to [`Verdict::findings`] and to `check`,
/// the adapter's own rule over the verdict and the tree as the model left
/// it; every finding is returned to the model for one correction round,
/// until the host's round limit is reached, so a verdict cannot pass over a
/// tree that breaks the adapter's rule. An adapter with no rule passes
/// `|_| Ok(Vec::new())`. A verdict that fails is an answer, not an error:
/// the caller decides what a failed wave means, and what the repairs left
/// in the tree is the caller's to seal.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the model rejects the request or no
///   valid verdict is produced within the available rounds or the backend's
///   time budget.
/// - Returns [`Error::ServerError`] when `docs` does not contain `verify.md`.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
/// - Returns the error `check` fails with, whatever its class, the
///   candidate it failed on ending the turn.
pub async fn verify<P: Model>(
    ctx: &VerifyContext<'_, P>, docs: &'static [Doc],
    mut check: impl FnMut(&Verdict) -> Result<Vec<String>, Error> + Send,
) -> Result<Verdict, Error> {
    let writer = Writer::new(ctx.workspace, VERIFY_TURN);
    let mut tools = reference::tools();
    tools.extend(Writer::tools());
    let question = granted(
        Question::<Verdict>::new(VERIFY_TURN)
            .system(prompt(docs, VERIFY)?)
            .tools(tools)
            .workspace(ctx.workspace),
        ctx.worker,
    );
    let brief = VerifyBrief {
        adapter_id: ctx.adapter_id,
    };

    tracing::info!("verifying");
    let tools = writer.serve(reference::serve(docs, &[], VERIFY_TURN, None));
    let mut failed = None;
    let verdict = question
        .ask(ctx.model, brief.to_string(), Some(tools), |answer| {
            let mut findings = answer.findings();
            match check(answer) {
                Ok(more) => findings.extend(more),
                Err(error) => {
                    stash(&mut failed, error);
                    return Ok(());
                }
            }
            if findings.is_empty() {
                return Ok(());
            }
            tracing::debug!(?findings, "candidate rejected");
            Err(findings)
        })
        .await
        .map_err(Error::from);
    if let Some(error) = failed {
        return Err(error);
    }
    let verdict = verdict?;

    tracing::info!(passed = verdict.passed, failures = verdict.failures.len(), "verified");
    tracing::debug!(failures = ?verdict.failures, "the verdict");

    Ok(verdict)
}

/// The `run` arm of [`export_target!`](crate::export_target), reached through the macro alone.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[omnia_wasi_otel::instrument(name = "target_adapter_run")]
pub async fn call<A: TargetAdapter>(
    id: export::AdapterId, workspace: String, task: export::Task,
) -> Result<export::Outcome, export::Error> {
    let asked = emery_adapter::target::Task::from(task);
    let task = match &asked {
        emery_adapter::target::Task::Init => Task::Init(InitContext {
            adapter_id: &id,
            workspace: &workspace,
        }),
        emery_adapter::target::Task::Build(slice) => Task::Build(BuildContext {
            adapter_id: &id,
            slice,
            workspace: &workspace,
            worker: &A::WORKER,
            model: &crate::Provider,
        }),
        emery_adapter::target::Task::Verify => Task::Verify(VerifyContext {
            adapter_id: &id,
            workspace: &workspace,
            worker: &A::WORKER,
            model: &crate::Provider,
        }),
    };
    let outcome = A::run(task).await?;
    if !outcome.answers(&asked) {
        return Err(export::Error::Internal(format!("answered {outcome} to {asked}")));
    }
    Ok(outcome.into())
}

// A host error the adapter's check met is the run's, not the model's: the
// caller accepts the candidate to end the completion and carries the error
// out after `ask` returns.
fn stash(failed: &mut Option<Error>, error: Error) {
    tracing::warn!(%error, "the adapter's check failed; ending the turn");
    *failed = Some(error);
}

// `question` carrying the worker's grant, when it has one.
fn granted<T>(question: Question<T>, worker: &Worker) -> Question<T>
where
    T: schemars::JsonSchema + serde::de::DeserializeOwned + Send,
{
    match worker.grant() {
        Some(grant) => question.worker(grant),
        None => question,
    }
}

// The two tools' rules, stated once in both briefs: a build's and a
// verify's write under the same rules.
const TOOLS_RULE: &str = "`write_files` writes one or more files beneath `$WORKSPACE` per call, \
                          each created or replaced whole, the directories above it created, and \
                          removes the files its `delete` names; `edit_files` changes within \
                          files the tree holds, each edit replacing the one place its `find` \
                          matches. A call naming a path outside the tree, under `.emery/` or \
                          `.git/`, naming a projection, or leading through a symlink is refused \
                          whole and changes nothing; so is a `write_files` entry that would leave \
                          a file empty or under half its size unless it sets `shrink: true`, \
                          since a fragment sent as the whole file loses the rest.";

// The user turn of a build. The lend carries the root, so the brief never
// names it.
struct Brief<'a> {
    adapter_id: &'a str,
    slice: &'a Slice,
}

impl Display for Brief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let slice = self.slice;
        write!(
            f,
            "Build the slice `{name}` ({id}) of the plan, bound to adapter `{adapter}`.\n\n\
             `$WORKSPACE` is the project tree, lent writable: the root of every file you can \
             read, and the root every `written` path is relative to. The tree sits on commit \
             `{base}`, the integrated head this slice builds over. Write through this call's \
             two tools alone: {TOOLS_RULE} Lay the files you have ready together in one \
             `write_files` call, and make a change within a file through `edit_files`. Build \
             the slice into the tree as the prompt describes, and change nothing outside \
             it.\n\n",
            name = slice.name,
            id = slice.id,
            adapter = self.adapter_id,
            base = slice.base,
        )?;

        write!(
            f,
            "The slice's entry in the plan:\n\n{plan}\n\n\
             The specification, cut to the slice's requirements:\n\n{spec}\n\n\
             The design, whole:\n\n{design}\n\n",
            plan = slice.plan.trim_end(),
            spec = slice.spec.trim_end(),
            design = slice.design.trim_end(),
        )?;

        // the requirements and the report rules
        let ids = slice.requirements.iter().map(|id| format!("`{id}`")).collect::<Vec<_>>();
        write!(
            f,
            "Implement every requirement the specification above holds — {ids} — and no \
             other.\n\n\
             The prompt's references are available through this call's `read_doc` tool \
             (`list_docs` enumerates them); load referenced bodies on demand.\n\n\
             Answer with one JSON object matching the report schema: `covered` lists each of \
             those requirement ids the tree now implements, once each and no other id; \
             `written` lists the files the tools wrote, once each, as a `/`-separated path \
             relative to `$WORKSPACE`, and only a file the tree now holds; a file you wrote and \
             leave out is added for you. Leave a requirement out of `covered` rather than \
             claim what the tree does not hold.",
            ids = ids.join(", "),
        )
    }
}

// The user turn of a verification. The lend carries the root, so the brief
// never names it.
struct VerifyBrief<'a> {
    adapter_id: &'a str,
}

impl Display for VerifyBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Verify the integrated project tree, bound to adapter `{adapter}`.\n\n\
             `$WORKSPACE` is the tree every slice of the wave has merged into, lent writable \
             with the shell: run the checks the prompt names in it and read what they print. \
             Where a check fails, repair the tree through this call's two tools alone, then \
             run the checks again: {TOOLS_RULE} A repair within a file is an `edit_files` \
             call. Keep each repair to what the failure shows and the prompt allows, and \
             change nothing a passing check covers. What you write, and what the checks leave \
             behind, is sealed as the wave's own commit.\n\n\
             The prompt's references are available through this call's `read_doc` tool \
             (`list_docs` enumerates them); load referenced bodies on demand.\n\n\
             Answer with one JSON object matching the verdict schema: `passed` is true when \
             every check passed on its last run and false otherwise; `failures` names each \
             check that still failed, with the tail of its output, and is empty when `passed` \
             is true. Report what the checks found rather than what the tree should hold.",
            adapter = self.adapter_id,
        )
    }
}
