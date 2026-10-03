//! Provides the types and functions a target adapter is written with.
//!
//! A target adapter receives a [`Slice`] of the build plan and the workspace
//! to build into, and returns a [`Report`] of what it built. [`build`] puts
//! the one gated turn: the slice's documents under the adapter's `build.md`,
//! the workspace lent, and the answered report held to the slice by
//! [`Report::findings`] until it passes. [`metadata`] answers the `metadata`
//! export, and [`target_adapter!`](crate::target_adapter) exports both over
//! an adapter's two plain fns.
//!
//! # Examples
//!
//! This adapter builds every slice through the SDK's one turn:
//!
//! ```
//! use emery_sdk::Doc;
//!
//! pub static PROSE: &[Doc] = &[Doc {
//!     path: "build.md",
//!     body: "Implement each requirement as a Rust module under `src/`.",
//! }];
//!
//! #[cfg(target_arch = "wasm32")]
//! mod guest {
//!     use emery_sdk::target::{Context, Report, TargetMetadata};
//!     use emery_sdk::{Error, Model};
//!
//!     emery_sdk::target_adapter!(metadata, build);
//!
//!     fn metadata() -> TargetMetadata {
//!         emery_sdk::target::metadata()
//!     }
//!
//!     async fn build<P: Model>(ctx: &Context<'_, P>) -> Result<Report, Error> {
//!         emery_sdk::target::build(ctx, super::PROSE).await
//!     }
//! }
//! # fn main() {}
//! ```

use std::fmt::{self, Display, Formatter};

#[cfg(target_arch = "wasm32")]
#[doc(inline)]
pub use emery_adapter::target::export;
pub use emery_adapter::target::{Report, Slice, Target, TargetMetadata};
use emery_prose::Doc;
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model};

use crate::{BUILD, prompt, reference};

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
    /// the model. The deployment's grant decides what may be written beneath
    /// it.
    pub workspace: &'a str,
    /// The [`Model`] the turn is put to.
    pub model: &'a P,
}

/// Returns the `metadata` answer for a target adapter.
///
/// The `emery-version` pin is this SDK's own version, identifying the contract
/// the adapter compiled against. Build a [`TargetMetadata`] directly only
/// when the adapter must loosen or tighten that pin.
#[must_use]
pub fn metadata() -> TargetMetadata {
    TargetMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

/// Builds the slice of `ctx` into its workspace through one gated turn.
///
/// `docs` must contain `build.md`, which becomes the system prompt. The turn
/// carries the slice's plan entry, its cut of the specification, and the
/// whole design, lends the workspace, and offers the embedded references
/// through the reference tools. The answered [`Report`] is held
/// to the slice by [`Report::findings`]; every finding is returned to the
/// model for one correction round, until the host's round limit is reached.
///
/// A failure upstream is not put again: the turn writes the tree as it
/// goes, so a second turn would start over what the first left.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the model rejects the request or no
///   valid report is produced within the available rounds or the backend's
///   time budget.
/// - Returns [`Error::ServerError`] when `docs` does not contain `build.md`.
/// - Returns [`Error::BadGateway`] when a model tool or transport fails.
pub async fn build<P: Model>(ctx: &Context<'_, P>, docs: &'static [Doc]) -> Result<Report, Error> {
    let slice = ctx.slice;
    let id = &slice.id;
    let question = Question::<Report>::new(&format!("build-{id}"))
        .system(prompt(docs, BUILD)?)
        .tools(reference::tools())
        .workspace(ctx.workspace);
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
    let report = question
        .ask(ctx.model, brief.to_string(), Some(reference::serve(docs, id, None)), |answer| {
            let findings = answer.findings(slice);
            if findings.is_empty() {
                return Ok(());
            }
            tracing::debug!(slice = %id, ?findings, "candidate rejected");
            Err(findings)
        })
        .await
        .map_err(Error::from)?;

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

/// The `build` arm of [`target_adapter!`](crate::target_adapter), reached through the macro alone.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[omnia_wasi_otel::instrument(name = "target_adapter_build")]
pub async fn call(
    build: impl AsyncFnOnce(&Context<'_, crate::Provider>) -> Result<Report, Error>,
    id: export::AdapterId, slice: export::Slice, workspace: String,
) -> Result<export::Report, export::Error> {
    let slice = Slice::from(slice);
    let ctx = Context {
        adapter_id: &id,
        slice: &slice,
        workspace: &workspace,
        model: &crate::Provider,
    };
    Ok(build(&ctx).await?.into())
}

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
             read and write, and the root every `written` path is relative to. Build the slice \
             into it as the prompt describes, and change nothing outside it.\n\n\
             The slice's entry in the plan:\n\n{plan}\n\n\
             The specification, cut to the slice's requirements:\n\n{spec}\n\n\
             The design, whole:\n\n{design}\n\n",
            name = slice.name,
            id = slice.id,
            adapter = self.adapter_id,
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
             `written` lists each file you wrote or changed, once each, as a `/`-separated path \
             relative to `$WORKSPACE`. Leave a requirement out of `covered` rather than claim \
             what the tree does not hold.",
            ids = ids.join(", "),
        )
    }
}
