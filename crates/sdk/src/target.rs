//! Provides the types and functions a target adapter is written with.
//!
//! A target adapter implements [`TargetAdapter`] and exports it with
//! [`export_target!`](crate::export_target). Each [`Task`] answers an
//! [`Outcome`]:
//!
//! - [`Task::Init`] scaffolds the layout's initial files without a model turn.
//! - [`Task::Build`] runs [`build`] to implement one [`Slice`] and return a
//!   validated [`Report`].
//! - [`Task::Verify`] runs [`verify`] to check and repair the integrated tree
//!   and return a validated [`Verdict`].
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
//! use emery_sdk::target::{Grant, Metadata, Outcome, TargetAdapter, Task};
//! use emery_sdk::{Doc, Error, Model, server_error};
//!
//! // cargo fetches from crates.io's sparse index and its download host
//! const CARGO: Grant = Grant {
//!     egress: &["index.crates.io", "static.crates.io"],
//!     read: &[],
//!     env: &[],
//! };
//!
//! // rustup finds its toolchains under `RUSTUP_HOME`, which the worker's
//! // private `HOME` would otherwise hide
//! const RUSTUP: Grant = Grant {
//!     egress: &[],
//!     read: &["~/.rustup", "~/.cargo/bin"],
//!     env: &[("RUSTUP_HOME", "~/.rustup")],
//! };
//!
//! const GRANTS: &[Grant] = &[CARGO, RUSTUP];
//!
//! pub struct Adapter;
//!
//! impl TargetAdapter for Adapter {
//!     fn metadata() -> Metadata {
//!         Metadata {
//!             grants: GRANTS,
//!             ..Metadata::default()
//!         }
//!     }
//!
//!     async fn run<P: Model>(task: Task<'_, P>) -> Result<Outcome, Error> {
//!         match task {
//!             Task::Init(ctx) => {
//!                 // a held manifest stands; the tree lacking one gets the layout's
//!                 let manifest = Path::new(ctx.workspace).join("Cargo.toml");
//!                 match OpenOptions::new().write(true).create_new(true).open(manifest) {
//!                     Ok(mut file) => file
//!                         .write_all(b"[workspace]\nmembers = [\"crates/*\"]\n")
//!                         .map_err(|err| server_error!("scaffolding `Cargo.toml`: {err}"))?,
//!                     Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
//!                     Err(err) => return Err(server_error!("scaffolding `Cargo.toml`: {err}")),
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

mod design;
mod write;

use std::fmt::{self, Display, Formatter};
use std::path::Path;

#[cfg(target_arch = "wasm32")]
#[doc(inline)]
pub use emery_adapter::target::export;
pub use emery_adapter::target::{
    DesignType, MergeRule, MergeStrategy, Outcome, Report, Requirement, Scenario, Slice,
    TargetMetadata, Verdict,
};
use emery_prose::Doc;
use omnia_sdk::model::{Question, WorkerGrant};
use omnia_sdk::{Error, Model, server_error};

use self::write::{Writer, Written};
use crate::{BUILD, VERIFY, prompt, reference, unique};

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

/// What one program of the worker's toolchain is granted beyond the lent tree.
///
/// A target adapter spells one per program its prompts run, as that
/// program's own documentation states its reach, and lists them as its
/// [`Metadata::grants`]. [`build`] and [`verify`] carry the list with every
/// turn as the completion's worker grant, their [`union`](Self::union), each
/// entry once however many grants spell it. The host holds each entry to
/// its shape and the backend applies what exists on its machine. The adapter
/// is trusted by its digest, so what it spells is what the deployment
/// granted when it pinned it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grant {
    /// Host names the program's commands may reach through the backend's
    /// egress proxy: a name whole, or `*.suffix` for a domain and every
    /// host beneath it. Never an IP literal, a port, a scheme, or a path.
    pub egress: &'static [&'static str],
    /// Directories the program may read beside the lend and the system,
    /// each `~/<path>`: beneath the operator's home, never `~/` itself and
    /// never holding `..`. One the machine lacks grants nothing.
    pub read: &'static [&'static str],
    /// Environment variables set for the program, each a name of the shape
    /// `[A-Za-z_][A-Za-z0-9_]*` and a value whose leading `~/` the backend
    /// expands to the operator's home.
    pub env: &'static [(&'static str, &'static str)],
}

impl Grant {
    /// Returns the worker grant a turn carries for `grants`: `None` when
    /// they unite to nothing, every entry once otherwise, in
    /// first-occurrence order.
    #[must_use]
    pub fn union(grants: &[Self]) -> Option<WorkerGrant> {
        let union = WorkerGrant {
            egress: unique(
                grants.iter().flat_map(|grant| grant.egress.iter().map(|host| (*host).to_owned())),
            ),
            read: unique(
                grants.iter().flat_map(|grant| grant.read.iter().map(|root| (*root).to_owned())),
            ),
            env: unique(grants.iter().flat_map(|grant| {
                grant.env.iter().map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            })),
        };
        (union != WorkerGrant::default()).then_some(union)
    }

    /// Returns the entries of `grants` that violate the grant grammar.
    ///
    /// An empty vector means `grants` is valid. [`build`] and [`verify`]
    /// reject invalid grants before starting a turn.
    ///
    /// Every grant must satisfy four rules:
    ///
    /// - Each `egress` entry is a host name (`example.com` or `*.example.com`).
    /// - Each `read` entry is `~/<path>` over a plain relative path.
    /// - Each `env` name matches `[A-Za-z_][A-Za-z0-9_]*`.
    /// - No environment variable is assigned two different values.
    #[must_use]
    pub fn findings(grants: &[Self]) -> Vec<String> {
        let mut findings = Vec::new();
        for grant in grants {
            for host in grant.egress {
                if !is_host_name(host) {
                    findings.push(format!(
                        "egress `{host}` is not a host name: a name whole or `*.suffix`, never a \
                         scheme, a port, or a path"
                    ));
                }
            }
            for root in grant.read {
                if !is_home_root(root) {
                    findings.push(format!(
                        "read root `{root}` is not `~/<path>` over a plain relative path"
                    ));
                }
            }
            for (name, _) in grant.env {
                if !is_variable_name(name) {
                    findings.push(format!(
                        "variable `{name}` is not a name of the shape `[A-Za-z_][A-Za-z0-9_]*`"
                    ));
                }
            }
        }

        // one value per variable across the grants
        let env = unique(grants.iter().flat_map(|grant| grant.env.iter().copied()));
        for (index, (name, value)) in env.iter().enumerate() {
            let earlier = env[..index].iter().find(|(other, _)| other == name);
            if let Some((_, other)) = earlier {
                findings.push(format!("variable `{name}` is set to `{other}` and to `{value}`"));
            }
        }
        unique(findings)
    }
}

/// Static configuration a target adapter declares for merge rules and worker grants.
///
/// Returned by [`TargetAdapter::metadata`]. [`export_target!`](crate::export_target)
/// exposes [`rules`](Self::rules) through the component's `metadata` export,
/// and [`build`] and [`verify`] attach [`grants`](Self::grants) to every turn.
/// The default declares no rules and no grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Metadata {
    /// Merge rules applied when integrating each slice into the tree.
    ///
    /// The first rule matching a conflicting path applies; paths matched by no
    /// rule are resolved by the engine through a merge turn.
    pub rules: &'static [MergeRule],
    /// Toolchain access granted to the model's worker in [`build`] and
    /// [`verify`] turns, one [`Grant`] per program the prompts run.
    pub grants: &'static [Grant],
}

fn is_host_name(host: &str) -> bool {
    !host.is_empty() && !host.contains(['/', ':']) && !host.chars().any(char::is_whitespace)
}

fn is_home_root(root: &str) -> bool {
    root.strip_prefix("~/").is_some_and(|path| {
        !path.is_empty()
            && !path.contains('\\')
            && path.split('/').all(|segment| !matches!(segment, "" | "." | ".."))
    })
}

fn is_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The adapter addressed and the tree it initialises.
///
/// An init carries no model: it is the adapter's own code scaffolding the
/// layout's initial files where the tree lacks them, so two inits over one
/// tree scaffold the same files.
#[derive(Debug)]
pub struct InitContext<'a> {
    /// The identifier used to address the adapter.
    pub adapter_id: &'a str,
    /// The deployment-local path of the tree root the layout is scaffolded
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
    /// [`Metadata::grants`].
    pub grants: &'a [Grant],
    /// The [`Model`] the turn is put to.
    pub model: &'a P,
    /// What the adapter's own code scaffolded into the workspace for the
    /// slice before the turn, where it scaffolded anything. The host lifts
    /// the context with `None`; an adapter that scaffolds a skeleton sets it
    /// through [`with_scaffold`](Self::with_scaffold).
    pub scaffold: Option<Scaffold<'a>>,
}

impl<'a, P> BuildContext<'a, P> {
    /// Returns the context with [`scaffold`](Self::scaffold) set.
    #[must_use]
    pub const fn with_scaffold(mut self, scaffold: Scaffold<'a>) -> Self {
        self.scaffold = Some(scaffold);
        self
    }
}

/// Files and prompt context an adapter scaffolds into the workspace before a [`build`] turn.
///
/// Attach a scaffold to [`BuildContext`] with [`BuildContext::with_scaffold`]
/// when the adapter generates skeleton files from the typed [`Slice`] for the
/// model to fill in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scaffold<'a> {
    /// Markdown rendered after the design under `What the adapter scaffolded
    /// for this slice before this turn:`.
    ///
    /// Names the scaffolded files and the markers to fill. An empty string
    /// omits the section.
    pub brief: &'a str,
    /// Reference passages appended verbatim after the scaffold section.
    ///
    /// Ignored when cutting the design to the slice, so types mentioned only
    /// in quoted examples are not retained. An empty string appends nothing.
    pub passages: &'a str,
    /// Guarded files that `write_files` may not delete or overwrite without
    /// their [`Guarded::keeps`] tokens.
    ///
    /// When non-empty, [`build`] also exposes `restore_files` so the model can
    /// reset a guarded file to its initial state.
    pub files: &'a [Guarded],
    /// Keys of design `Type:` blocks already laid into the scaffolded files.
    ///
    /// [`build`] replaces each matching data-type block in the brief's design
    /// with a pointer to the scaffold section.
    pub types: &'a [String],
}

/// A scaffolded file protected against accidental overwrite or deletion during [`build`].
///
/// `write_files` rejects any `delete` of `path`, and rejects any whole-file
/// write to `path` while it exists in the workspace unless the new content
/// contains every token in [`keeps`](Self::keeps). An empty `keeps` list
/// allows any whole-file write while still preventing deletion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guarded {
    /// Root-relative path of the scaffolded file.
    pub path: String,
    /// Substrings that any whole-file write to `path` must preserve, such as
    /// requirement markers, item declarations, or anchored list lines.
    pub keeps: Vec<String>,
}

impl Guarded {
    /// Returns a guard over `path` keeping `keeps`.
    pub fn new(
        path: impl Into<String>, keeps: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            path: path.into(),
            keeps: keeps.into_iter().map(Into::into).collect(),
        }
    }
}

impl Scaffold<'_> {
    fn section(&self) -> Option<&str> {
        Some(self.brief.trim_end()).filter(|brief| !brief.is_empty())
    }

    fn quoted(&self) -> Option<&str> {
        Some(self.passages.trim_end()).filter(|passages| !passages.is_empty())
    }
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
    /// [`Metadata::grants`].
    pub grants: &'a [Grant],
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
    /// Scaffold the layout's initial files the tree lacks, a held file left
    /// as it stands. Put once per repository a build lands in, before any
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
    /// Returns what the adapter declares about itself: the rules it merges
    /// under and the grants its worker carries.
    ///
    /// Answered from compiled-in constants, with no model call and no
    /// filesystem access. [`Metadata::default`] by default, for an adapter
    /// that merges under no rule and whose turns run nothing that reaches
    /// beyond the tree.
    #[must_use]
    fn metadata() -> Metadata {
        Metadata::default()
    }

    /// Runs `task` in the tree its context names and returns the matching [`Outcome`].
    ///
    /// Returning an [`Outcome`] that does not match `task` is an adapter defect
    /// and is rejected by the component export as `internal`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the tree refuses [`Task::Init`], such
    ///   as when a directory or symlink occupies a layout file's path.
    /// - Returns [`Error::ServerError`] when [`Task::Init`] cannot write the tree.
    /// - Returns what [`build`](fn@build) or [`verify`](fn@verify) returns.
    fn run<P: Model>(task: Task<'_, P>) -> impl Future<Output = Result<Outcome, Error>>;
}

/// Returns the `metadata` answer for a target adapter merging under `rules`.
///
/// [`export_target!`](crate::export_target) answers the component's
/// `metadata` with it over the adapter's [`Metadata::rules`]. The engine
/// merges every slice under the rules first and resolves what they leave
/// with the model. The `emery-version` pin is this SDK's own version,
/// identifying the contract the adapter compiled against. Build a
/// [`TargetMetadata`] directly only when the adapter must loosen or tighten
/// that pin.
#[must_use]
pub fn metadata(rules: &[MergeRule]) -> TargetMetadata {
    TargetMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        merge_rules: rules.to_vec(),
    }
}

/// Builds the slice of `ctx` into its workspace through one gated model turn.
///
/// `docs` must contain `build.md`, which becomes the system prompt. The brief
/// carries the slice's plan entry, its specification cut, the design cut to the
/// slice (omitting data types owned by other slices that the slice's text does
/// not mention), and any [`Scaffold`] set on `ctx`. The workspace is lent with
/// the [`Grant::union`] of `ctx`'s [`grants`](BuildContext::grants).
///
/// The turn serves `docs` through `list_docs` and `read_doc`, and modifies the
/// workspace through up to three tools:
///
/// - `write_files` creates or replaces files whole at paths
///   [`beneath`](crate::beneath) accepts (following no symlink) and removes
///   files listed in `delete`. It refuses writes that empty an existing file or
///   halve its size without `shrink: true`, and refuses writes or deletions
///   that violate a [`Guarded`] file in [`Scaffold::files`].
/// - `edit_files` replaces the single occurrence of each edit's `find` string
///   in an existing file.
/// - `restore_files`, registered when [`Scaffold::files`] is non-empty, resets
///   named scaffolded files to their contents at the start of the turn.
///
/// Any tool call that fails a check is rejected as a whole without changing the
/// workspace.
///
/// Each candidate [`Report`] must pass three checks before the turn completes:
///
/// - [`Report::findings`] verifies that `covered` names only the slice's
///   requirements and `written` contains valid root-relative paths.
/// - Every `written` path must name a regular file in the workspace.
/// - `check` runs the adapter's own validation over the candidate report and
///   workspace; pass `|_| Ok(Vec::new())` when none is needed.
///
/// Findings from all three checks are returned to the model for correction
/// until a candidate passes or the round limit is reached. The accepted
/// report's `written` list is normalised and augmented with every file the
/// tools wrote that still stands in the workspace.
///
/// Upstream failures are not retried because the turn mutates the workspace in
/// place. The caller may cut a fresh working copy and call `build` again.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the model rejects the request or
///   produces no valid report within the round limit or time budget.
/// - Returns [`Error::ServerError`] before any turn when `docs` lacks
///   `build.md`, `ctx`'s [`grants`](BuildContext::grants) have
///   [`findings`](Grant::findings), or a guarded file in the workspace cannot
///   be read.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
/// - Returns the [`Error`] returned by `check`, ending the turn immediately.
pub async fn build<P: Model>(
    ctx: &BuildContext<'_, P>, docs: &'static [Doc],
    mut check: impl FnMut(&Report) -> Result<Vec<String>, Error> + Send,
) -> Result<Report, Error> {
    let slice = ctx.slice;
    let id = &slice.id;
    let root = Path::new(ctx.workspace);
    let scaffolded = ctx.scaffold.map_or(&[][..], |scaffold| scaffold.files);
    let writer = Writer::new(ctx.workspace, id).guarding(scaffolded)?;
    let written = writer.written();
    let mut tools = reference::tools();
    tools.extend(writer.tools());
    let question = granted(
        Question::<Report>::new(&format!("build-{id}"))
            .system(prompt(docs, BUILD)?)
            .tools(tools)
            .workspace(ctx.workspace),
        ctx.grants,
    )?;
    let brief = Brief {
        adapter_id: ctx.adapter_id,
        slice,
        scaffold: ctx.scaffold,
    };

    tracing::info!(
        slice = %id,
        name = %slice.name,
        requirements = slice.requirements.len(),
        scaffold = ctx.scaffold.is_some(),
        scaffolded = scaffolded.len(),
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

/// Verifies the integrated tree of `ctx` through one gated model turn.
///
/// `docs` must contain `verify.md`, which becomes the system prompt. The turn
/// lends `ctx.workspace` with the [`Grant::union`] of `ctx`'s
/// [`grants`](VerifyContext::grants), serves `docs` through `list_docs` and
/// `read_doc`, and provides `write_files` and `edit_files` so the model can
/// run the prompt's checks in its shell and repair any failures in place.
///
/// Each candidate [`Verdict`] is checked with [`Verdict::findings`] and passed
/// to `check` (use `|_| Ok(Vec::new())` when the adapter has no extra rules).
/// Findings are returned to the model for correction until a candidate passes
/// or the round limit is reached. A verdict with `passed: false` is returned
/// as `Ok`, leaving the caller to handle the failed wave and any repairs left
/// in the workspace.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the model rejects the request or
///   produces no valid verdict within the round limit or time budget.
/// - Returns [`Error::ServerError`] before any turn when `docs` lacks
///   `verify.md` or `ctx`'s [`grants`](VerifyContext::grants) have
///   [`findings`](Grant::findings).
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
/// - Returns the [`Error`] returned by `check`, ending the turn immediately.
pub async fn verify<P: Model>(
    ctx: &VerifyContext<'_, P>, docs: &'static [Doc],
    mut check: impl FnMut(&Verdict) -> Result<Vec<String>, Error> + Send,
) -> Result<Verdict, Error> {
    let writer = Writer::new(ctx.workspace, VERIFY_TURN);
    let mut tools = reference::tools();
    tools.extend(writer.tools());
    let question = granted(
        Question::<Verdict>::new(VERIFY_TURN)
            .system(prompt(docs, VERIFY)?)
            .tools(tools)
            .workspace(ctx.workspace),
        ctx.grants,
    )?;
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
    let metadata = A::metadata();
    let task = match &asked {
        emery_adapter::target::Task::Init => Task::Init(InitContext {
            adapter_id: &id,
            workspace: &workspace,
        }),
        emery_adapter::target::Task::Build(slice) => Task::Build(BuildContext {
            adapter_id: &id,
            slice,
            workspace: &workspace,
            grants: metadata.grants,
            model: &crate::Provider,
            scaffold: None,
        }),
        emery_adapter::target::Task::Verify => Task::Verify(VerifyContext {
            adapter_id: &id,
            workspace: &workspace,
            grants: metadata.grants,
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

// `question` carrying the worker grant `grants` unite to, when they unite
// to one. A grant that breaks the shape is the adapter build's own defect,
// reported before a turn is spent as a missing prompt is.
fn granted<T>(question: Question<T>, grants: &[Grant]) -> Result<Question<T>, Error>
where
    T: schemars::JsonSchema + serde::de::DeserializeOwned + Send,
{
    let findings = Grant::findings(grants);
    if !findings.is_empty() {
        return Err(server_error!(
            "the adapter's grants break the grant's shape: {}",
            findings.join("; ")
        ));
    }
    Ok(match Grant::union(grants) {
        Some(grant) => question.worker(grant),
        None => question,
    })
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
    scaffold: Option<Scaffold<'a>>,
}

impl Display for Brief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let slice = self.slice;
        let guarded = self.scaffold.is_some_and(|scaffold| !scaffold.files.is_empty());
        write!(
            f,
            "Build the slice `{name}` ({id}) of the plan, bound to adapter `{adapter}`.\n\n\
             `$WORKSPACE` is the project tree, lent writable: the root of every file you can \
             read, and the root every `written` path is relative to. The tree sits on commit \
             `{base}`, the integrated head this slice builds over. Write through this call's \
             {count} tools alone: {TOOLS_RULE} Write the files you have ready together in one \
             `write_files` call, and make a change within a file through `edit_files`. Build \
             the slice into the tree as the prompt describes, and change nothing outside \
             it.\n\n",
            name = slice.name,
            id = slice.id,
            adapter = self.adapter_id,
            base = slice.base,
            count = if guarded { "three" } else { "two" },
        )?;

        write!(
            f,
            "The slice's entry in the plan:\n\n{plan}\n\n\
             The specification, cut to the slice's requirements:\n\n{spec}\n\n\
             The design, cut to this slice:\n\n{design}\n\n",
            plan = slice.plan.trim_end(),
            spec = slice.spec.trim_end(),
            design = design::cut(slice, self.scaffold.as_ref()),
        )?;

        // what the adapter scaffolded before the turn, where it scaffolded
        // anything, and what it quotes for the turn
        if let Some(scaffold) = &self.scaffold {
            if let Some(brief) = scaffold.section() {
                write!(
                    f,
                    "What the adapter scaffolded for this slice before this turn:\n\n{brief}\n\n"
                )?;
            }
            if !scaffold.files.is_empty() {
                write!(
                    f,
                    "The {count} the adapter scaffolded for this slice {are} filled in place: \
                     change {one} through `edit_files`, or send the whole file through \
                     `write_files` with every marker and name the skeleton holds still in it; \
                     an entry that drops one is refused whole naming what it dropped, as a \
                     `delete` naming one of them is. `restore_files` puts {one} back as it was \
                     scaffolded, the turn's edits to it discarded: recovery after a refusal, \
                     not a step.\n\n",
                    count = if scaffold.files.len() == 1 { "file" } else { "files" },
                    are = if scaffold.files.len() == 1 { "is" } else { "are" },
                    one = if scaffold.files.len() == 1 { "it" } else { "one" },
                )?;
            }
            if let Some(passages) = scaffold.quoted() {
                write!(f, "{passages}\n\n")?;
            }
        }

        // the requirements and the report rules
        let ids = slice
            .requirements
            .iter()
            .map(|requirement| format!("`{}`", requirement.id))
            .collect::<Vec<_>>();
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
