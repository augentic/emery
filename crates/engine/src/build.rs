//! Builds the current plan through a target adapter.
//!
//! [`build`] reads the current revision, loads the one target adapter the run
//! names, and dispatches every slice of the plan to it in dependency order,
//! each with its plan entry, its cut of the specification, and the whole
//! design. The adapter builds into the project tree; the engine writes no
//! state of its own. The first slice that fails ends the run, and the slices
//! built before it stay written.

use emery_adapter::target::{Slice as SliceInput, Target};
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::{BlobStore, Error, Plugins, StateStore, server_error};
use serde::{Deserialize, Serialize};

use crate::adapter::{self, AdapterRef, Loaded, Registries};
pub use crate::revision::{ReqId, SliceId};
use crate::revision::{Revision, Slice};
use crate::store;

// The root a build writes into, as the target adapter's lend names it.
const WORKSPACE: &str = ".";

/// Builds every slice of the current plan through the target adapter `input` names.
///
/// Slices are built one at a time, each after the slices it depends on, and
/// the first failure ends the run; what was built before it stays written.
/// Each slice's report is held to the slice by
/// [`Report::findings`](emery_adapter::target::Report::findings) before the
/// next is built.
///
/// # Errors
///
/// - Returns [`Error::NotFound`] with code `spec-not-generated` when no
///   revision has been committed, and without a code when a local adapter
///   does not exist.
/// - Returns [`Error::BadRequest`] when the run is refused:
///   - with code `spec-outdated`, when the stored revision uses another
///     grammar;
///   - an adapter reference [`specify`](crate::specify::specify) would
///     refuse: a path outside the adapters root, a package no registry
///     routes, a digest on a declared guest, one naming the engine's own
///     guest ([`ENGINE`](crate::ENGINE)), an incompatible adapter (code
///     `unsupported-version`), or one resolving to other bytes than its pin
///     (code `refused`);
///   - a slice the adapter refuses, or answers no acceptable report for.
/// - Returns [`Error::ServerError`] when a report breaks the report gate, or
///   storage fails.
/// - Returns [`Error::BadGateway`] when the adapter, its acquisition, or the
///   model fails upstream.
///
/// A failure at a slice keeps the adapter's class and code; its description
/// names the slice and the slices built before it.
pub async fn build<P: Target + StateStore + BlobStore + Plugins>(
    input: BuildInput, context: Context<P>,
) -> Result<BuildOutput, Error> {
    let provider = context.provider();

    let Some((revision_id, revision)) = store::current(provider).await? else {
        return Err(Error::NotFound {
            code: "spec-not-generated".into(),
            description: "no specification revision has been committed".into(),
        });
    };

    let Loaded { id: adapter, .. } =
        adapter::load_target(provider, &input.adapter, input.digest.as_ref(), &input.registries)
            .await?;

    let order = revision.plan.order();
    tracing::info!(revision = %revision_id, adapter = %adapter, slices = order.len(), "building");

    let mut built = Vec::with_capacity(order.len());
    for slice in order {
        match build_slice(provider, &adapter, &revision, slice).await {
            Ok(outcome) => built.push(outcome),
            Err(error) => return Err(at_slice(error, slice, &built)),
        }
    }

    Ok(BuildOutput {
        revision: revision_id,
        slices: built,
    })
}

/// The target a build runs through.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildInput {
    /// The target adapter every slice is built through.
    pub adapter: AdapterRef,
    /// The `sha256:` digest the adapter's component must resolve to.
    ///
    /// The run passes it on the load, and the loader holds the resolved
    /// bytes to it. `None` trusts whatever the load resolves. A declared
    /// guest takes none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
    /// The registries a package adapter fetches from, by namespace.
    ///
    /// Empty, only the `emery` namespace routes.
    #[serde(default)]
    pub registries: Registries,
}

/// What a successful [`build`] built.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildOutput {
    /// The identifier of the revision whose plan was built.
    pub revision: String,
    /// Every slice, in build order, as its build reported it.
    pub slices: Vec<BuiltSlice>,
}

/// One slice as its build reported it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuiltSlice {
    /// The slice identifier.
    pub id: SliceId,
    /// The slice name.
    pub name: String,
    /// The slice's requirements the adapter reports implemented, in id order.
    pub covered: Vec<ReqId>,
    /// The slice's requirements the adapter did not report, in id order.
    pub uncovered: Vec<ReqId>,
    /// The files the adapter reports written or changed, relative to the
    /// project root.
    pub written: Vec<String>,
}

#[tracing::instrument(skip_all, fields(slice = %slice.id, name = %slice.name))]
async fn build_slice<P: Target>(
    provider: &P, adapter: &str, revision: &Revision, slice: &Slice,
) -> Result<BuiltSlice, Error> {
    let input = SliceInput {
        id: slice.id.to_string(),
        name: slice.name.clone(),
        requirements: slice.requirements.iter().map(ToString::to_string).collect(),
        spec: revision.spec.cut(&slice.requirements),
        design: revision.design.to_string(),
        plan: slice.to_string(),
    };
    tracing::info!(requirements = input.requirements.len(), "building slice");
    let report = Target::build(provider, adapter, &input, WORKSPACE).await?;

    let findings = report.findings(&input);
    if !findings.is_empty() {
        return Err(server_error!(
            "`{adapter}` returned an invalid report:\n{}",
            findings.join("\n")
        ));
    }

    // the gate held every covered id to the slice, so each is one of these
    let (covered, uncovered): (Vec<ReqId>, Vec<ReqId>) =
        slice.requirements.iter().partition(|id| report.covered.contains(&id.to_string()));
    tracing::info!(
        covered = covered.len(),
        uncovered = uncovered.len(),
        written = report.written.len(),
        "slice built"
    );

    Ok(BuiltSlice {
        id: slice.id,
        name: slice.name.clone(),
        covered,
        uncovered,
        written: report.written,
    })
}

// A slice's failure keeps its class and code; the description gains the
// slice it failed at and the slices built before it, which stay written.
fn at_slice(error: Error, slice: &Slice, built: &[BuiltSlice]) -> Error {
    let ids: Vec<String> = built.iter().map(|built| built.id.to_string()).collect();
    let before = match ids.as_slice() {
        [] => String::new(),
        [one] => format!("; {one} built before it stays written"),
        many => format!("; {} built before it stay written", many.join(", ")),
    };
    let describe = |description: String| {
        format!("slice `{}` ({}) failed{before}: {description}", slice.id, slice.name)
    };

    match error {
        Error::BadRequest { code, description } => Error::BadRequest {
            code,
            description: describe(description),
        },
        Error::NotFound { code, description } => Error::NotFound {
            code,
            description: describe(description),
        },
        Error::ServerError { code, description } => Error::ServerError {
            code,
            description: describe(description),
        },
        Error::BadGateway { code, description } => Error::BadGateway {
            code,
            description: describe(description),
        },
    }
}
