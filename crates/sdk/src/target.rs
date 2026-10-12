//! Provides the types and functions a target adapter is written with.
//!
//! A target adapter receives a [`Slice`] of the build plan and the workspace
//! to build into, and returns a [`Report`] of what it built; once a wave of
//! slices has merged, it receives the integrated tree and returns a
//! [`Verdict`] over it. [`build`] puts the one gated build turn: the
//! adapter's [`Seed`] laid where the tree lacks it, the slice's documents
//! under the adapter's `build.md`, the workspace lent and written through
//! the turn's `write_files` and `edit_files` tools, and the answered report
//! held to the slice by [`Report::findings`] and to the tree until it
//! passes, its `written` filled in from what the seed and the tools wrote.
//! [`verify`] puts the one gated verify turn: the integrated tree lent under
//! the adapter's `verify.md`, the checks it names run through the model's
//! shell, what they find repaired through the same two tools, and the
//! answered verdict held to [`Verdict::findings`]. [`TargetAdapter`] is what
//! a target adapter implements, and [`export_target!`](crate::export_target)
//! exports a type implementing it, answering the component's `metadata`
//! through [`metadata`] over the type's
//! [`MERGE_RULES`](TargetAdapter::MERGE_RULES).
//!
//! # Examples
//!
//! This adapter seeds every tree with one manifest, builds every slice, and
//! verifies every wave through the SDK's turns:
//!
//! ```
//! use emery_sdk::Doc;
//! use emery_sdk::target::Seed;
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
//! pub static SEED: &[Seed] =
//!     &[Seed::new("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n")];
//!
//! #[cfg(target_arch = "wasm32")]
//! mod guest {
//!     use emery_sdk::target::{Context, Report, TargetAdapter, Verdict, VerifyContext};
//!     use emery_sdk::{Error, Model};
//!
//!     struct Adapter;
//!
//!     emery_sdk::export_target!(Adapter);
//!
//!     impl TargetAdapter for Adapter {
//!         async fn build<P: Model>(ctx: &Context<'_, P>) -> Result<Report, Error> {
//!             emery_sdk::target::build(ctx, super::PROSE, super::SEED).await
//!         }
//!
//!         async fn verify<P: Model>(ctx: &VerifyContext<'_, P>) -> Result<Verdict, Error> {
//!             emery_sdk::target::verify(ctx, super::PROSE).await
//!         }
//!     }
//! }
//! # fn main() {}
//! ```

mod write;

use std::fmt::{self, Display, Formatter};
use std::path::Path;

#[cfg(target_arch = "wasm32")]
#[doc(inline)]
pub use emery_adapter::target::export;
pub use emery_adapter::target::{
    MergeRule, MergeStrategy, Report, Slice, Target, TargetMetadata, Verdict,
};
use emery_prose::Doc;
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model};

pub use self::write::Seed;
use self::write::{Writer, Written};
use crate::{BUILD, VERIFY, prompt, reference};

const VERIFY_TURN: &str = "verify";

/// The adapter addressed, the slice it builds, the tree it builds into, and the model.
///
/// [`build`] takes it, so the one turn of a call is put to the one model the
/// host bound.
#[derive(Debug)]
pub struct Context<'a, P> {
    /// The identifier used to address the adapter.
    pub adapter_id: &'a str,
    /// The [`Slice`] to build.
    pub slice: &'a Slice,
    /// The deployment-local path of the tree root, which the turn lends to
    /// the model and writes beneath through `write_files` and `edit_files`.
    /// The deployment's grant decides whether it may be written.
    pub workspace: &'a str,
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
    /// The [`Model`] the turn is put to.
    pub model: &'a P,
}

/// A target adapter: how it builds one slice of the plan into the project tree, and verifies the tree.
///
/// Implement it on a unit struct and hand that type to
/// [`export_target!`](crate::export_target), which exports the
/// `target-adapter` world over it.
pub trait TargetAdapter {
    /// The rules the engine merges every slice into the integrated tree
    /// under, since the adapter alone knows which files its builds
    /// regenerate and which they extend.
    ///
    /// The first rule matching a conflicting path applies; none leaves every
    /// conflict unresolved, which the engine reports and rebuilds over the
    /// merged head.
    const MERGE_RULES: &'static [MergeRule] = &[];

    /// Builds the slice `ctx` carries into its workspace and reports what it
    /// covered and wrote.
    ///
    /// The one turn goes through [`build`](fn@build).
    ///
    /// # Errors
    ///
    /// Returns what [`build`](fn@build) returns.
    fn build<P: Model>(ctx: &Context<'_, P>) -> impl Future<Output = Result<Report, Error>>;

    /// Verifies the integrated tree `ctx` carries and reports what failed.
    ///
    /// The one turn goes through [`verify`](fn@verify).
    ///
    /// # Errors
    ///
    /// Returns what [`verify`](fn@verify) returns.
    fn verify<P: Model>(ctx: &VerifyContext<'_, P>)
    -> impl Future<Output = Result<Verdict, Error>>;
}

/// Returns the `metadata` answer for a target adapter merging under `merge_rules`.
///
/// [`export_target!`](crate::export_target) answers the component's
/// `metadata` with it over the adapter's
/// [`MERGE_RULES`](TargetAdapter::MERGE_RULES). The `emery-version` pin is
/// this SDK's own version, identifying the contract the adapter compiled
/// against. Build a [`TargetMetadata`] directly only when the adapter must
/// loosen or tighten that pin.
#[must_use]
pub fn metadata(merge_rules: &[MergeRule]) -> TargetMetadata {
    TargetMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        merge_rules: merge_rules.to_vec(),
    }
}

/// Builds the slice of `ctx` into its workspace through one gated turn.
///
/// `docs` must contain `build.md`, which becomes the system prompt. Before
/// the turn, every file of `seed` the workspace lacks is laid at its path
/// and counts as written; one the workspace holds is left as it stands,
/// and a symlink or directory standing at one refuses the slice. The turn
/// carries the slice's plan entry, its cut of the specification, and the
/// whole design, names the seed files it laid, lends the workspace, and
/// offers the embedded references through the reference tools. It writes
/// through two tools. `write_files` writes one or more files beneath the
/// workspace per call, listed or mapped by path, each created or replaced
/// whole, at a path [`beneath`](crate::beneath) accepts and no symlink
/// stands on, and removes the files its `delete` names; an entry that would
/// leave a held file empty or under half its size is refused unless it sets
/// `shrink`. `edit_files` replaces the one place each edit's `find` matches
/// within a file the workspace holds. A call either tool refuses anywhere
/// changes nothing. The answered [`Report`] is held to the slice by
/// [`Report::findings`] and to the tree — a `written` path names a regular
/// file under the workspace; every finding is returned to the model for one
/// correction round, until the host's round limit is reached. The accepted
/// report's `written` is filled in: every path it lists, spelled
/// root-relative, and every file the seed or a tool wrote that the tree
/// still holds, once each and sorted.
///
/// A failure upstream is not put again here: the turn writes the tree as it
/// goes, so a second turn would start over what the first left. The caller
/// that owns the tree may cut a fresh one and ask again.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when a symlink or directory stands at a
///   seed path, when the model rejects the request, or when no valid report
///   is produced within the available rounds or the backend's time budget.
/// - Returns [`Error::ServerError`] when `docs` does not contain `build.md`,
///   when a seed path is one [`beneath`](crate::beneath) refuses or is
///   declared twice, or when a seed file cannot be written.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn build<P: Model>(
    ctx: &Context<'_, P>, docs: &'static [Doc], seed: &[Seed],
) -> Result<Report, Error> {
    let slice = ctx.slice;
    let id = &slice.id;
    let root = Path::new(ctx.workspace);
    let writer = Writer::new(ctx.workspace, id);
    let written = writer.written();
    let mut tools = reference::tools();
    tools.extend(Writer::tools());
    let question = Question::<Report>::new(&format!("build-{id}"))
        .system(prompt(docs, BUILD)?)
        .tools(tools)
        .workspace(ctx.workspace);
    let seeded = writer.seed(seed)?;
    if !seeded.is_empty() {
        tracing::info!(slice = %id, seeded = seeded.len(), "seeded");
        tracing::debug!(slice = %id, ?seeded, "the files the tree lacked");
    }
    let brief = Brief {
        adapter_id: ctx.adapter_id,
        slice,
        seeded: &seeded,
    };

    tracing::info!(
        slice = %id,
        name = %slice.name,
        requirements = slice.requirements.len(),
        "building"
    );
    let tools = writer.serve(reference::serve(docs, id, None));
    let mut report = question
        .ask(ctx.model, brief.to_string(), Some(tools), |answer| {
            let mut findings = answer.findings(slice);
            findings.extend(Written::findings(root, answer));
            if findings.is_empty() {
                return Ok(());
            }
            tracing::debug!(slice = %id, ?findings, "candidate rejected");
            Err(findings)
        })
        .await
        .map_err(Error::from)?;
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
/// lends the tree and offers the embedded references through the reference
/// tools and the same `write_files` and `edit_files` a build has: the model
/// runs the checks the prompt names through its shell, repairs what they
/// find in the tree, runs them again, and answers what they finally found.
/// The answered
/// [`Verdict`] is held to [`Verdict::findings`]; every finding is returned
/// to the model for one correction round, until the host's round limit is
/// reached. A verdict that fails is an answer, not an error: the caller
/// decides what a failed wave means, and what the repairs left in the tree
/// is the caller's to seal.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the model rejects the request or no
///   valid verdict is produced within the available rounds or the backend's
///   time budget.
/// - Returns [`Error::ServerError`] when `docs` does not contain `verify.md`.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn verify<P: Model>(
    ctx: &VerifyContext<'_, P>, docs: &'static [Doc],
) -> Result<Verdict, Error> {
    let writer = Writer::new(ctx.workspace, VERIFY_TURN);
    let mut tools = reference::tools();
    tools.extend(Writer::tools());
    let question = Question::<Verdict>::new(VERIFY_TURN)
        .system(prompt(docs, VERIFY)?)
        .tools(tools)
        .workspace(ctx.workspace);
    let brief = VerifyBrief {
        adapter_id: ctx.adapter_id,
    };

    tracing::info!("verifying");
    let tools = writer.serve(reference::serve(docs, VERIFY_TURN, None));
    let verdict = question
        .ask(ctx.model, brief.to_string(), Some(tools), |answer| {
            let findings = answer.findings();
            if findings.is_empty() {
                return Ok(());
            }
            tracing::debug!(?findings, "candidate rejected");
            Err(findings)
        })
        .await
        .map_err(Error::from)?;

    tracing::info!(passed = verdict.passed, failures = verdict.failures.len(), "verified");
    tracing::debug!(failures = ?verdict.failures, "the verdict");

    Ok(verdict)
}

/// The `build` arm of [`export_target!`](crate::export_target), reached through the macro alone.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[omnia_wasi_otel::instrument(name = "target_adapter_build")]
pub async fn call<A: TargetAdapter>(
    id: export::AdapterId, slice: export::Slice, workspace: String,
) -> Result<export::Report, export::Error> {
    let slice = Slice::from(slice);
    let ctx = Context {
        adapter_id: &id,
        slice: &slice,
        workspace: &workspace,
        model: &crate::Provider,
    };
    Ok(A::build(&ctx).await?.into())
}

/// The `verify` arm of [`export_target!`](crate::export_target), reached through the macro alone.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[omnia_wasi_otel::instrument(name = "target_adapter_verify")]
pub async fn verify_call<A: TargetAdapter>(
    id: export::AdapterId, workspace: String,
) -> Result<export::Verdict, export::Error> {
    let ctx = VerifyContext {
        adapter_id: &id,
        workspace: &workspace,
        model: &crate::Provider,
    };
    Ok(A::verify(&ctx).await?.into())
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
    seeded: &'a [String],
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

        // the seed this call laid, when the tree lacked any of it
        if !self.seeded.is_empty() {
            let seeded = self.seeded.iter().map(|file| format!("`{file}`")).collect::<Vec<_>>();
            write!(
                f,
                "The tree lacked these files of the layout, laid before this turn and counted \
                 as written by this slice: {seeded}.\n\n",
                seeded = seeded.join(", "),
            )?;
        }

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
