//! Builds the current plan through a target adapter, into a labelled commit.
//!
//! [`build`] reads the current revision, loads the one target adapter the run
//! names, settles the base — the project checkout's sealed head, or a branch
//! of the repository the target names — and cuts an integration working copy
//! from it. Every slice of the plan is dispatched to the adapter in
//! dependency order, each with its plan entry, its cut of the specification,
//! and the whole design, and what it wrote is sealed as one commit. The
//! integrated head is labelled `emery/<revision>`, pushed when the target
//! names a remote, and the working copy is removed. The engine writes no
//! state of its own: the labelled history is the output.
//!
//! The first slice that fails ends the run; the slices built before it stay
//! committed in the integration working copy, which is left for inspection
//! and removed by the next run.

use emery_adapter::target::{Slice as SliceInput, Target};
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::{BlobStore, Error, Plugins, StateStore, Vcs, server_error};
use serde::{Deserialize, Serialize};

use crate::adapter::{self, AdapterRef, Loaded};
pub use crate::revision::{ReqId, SliceId, Waves};
use crate::revision::{Slice, Spec};
use crate::vcs::{INTEGRATION, Repo, Repository, WorkingCopy};
use crate::{store, vcs};

/// Builds every slice of the current plan through the target adapter `input` names.
///
/// Slices are built one at a time, each after the slices it depends on, and
/// the first failure ends the run; what was built before it stays committed
/// in the integration working copy. Each slice's report is held to the
/// slice by [`Report::findings`](emery_adapter::target::Report::findings)
/// before its commit, and the next is built.
///
/// # Errors
///
/// - Returns [`Error::NotFound`]:
///   - with code `spec-not-generated`, when no revision has been committed;
///   - with code `revision-not-found`, when the repository the target names,
///     its branch, or its remote does not exist.
/// - Returns [`Error::BadRequest`] when the run is refused:
///   - with code `spec-outdated`, when the stored revision uses another
///     grammar;
///   - an adapter [`specify`](crate::specify::specify) would refuse: one that
///     is not a target adapter ([`Axis::Target`](crate::Axis)), an
///     incompatible one (code `unsupported-version`), a release the store
///     lacks whose namespace the deployment routes nowhere, a pre-compiled
///     artifact, or one resolving to other bytes than its pin (code
///     `refused`);
///   - with code `repository-required`, when the target names no repository
///     and the project is none;
///   - with code `base-not-sealed`, when the project checkout holds pending
///     changes outside `.emery/`, listed, or no commit;
///   - a slice the adapter refuses, or answers no acceptable report for.
/// - Returns [`Error::ServerError`] when a report breaks the report gate, or
///   storage or version control fails.
/// - Returns [`Error::BadGateway`] when the adapter, its acquisition, the
///   model, or the repository's remote fails upstream.
///
/// A failure at a slice keeps the adapter's class and code; its description
/// names the slice and the slices built before it.
pub async fn build<P: Target + StateStore + BlobStore + Plugins + Vcs>(
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
        adapter::load_target(provider, &input.adapter, input.digest.as_ref()).await?;

    // the base, and the working copy the slices integrate in
    let (repo, base) = match &input.repository {
        Some(target) => {
            let repository = Repository::new(&target.url);
            repository.ensure(provider).await?;
            let base = repository.resolve(provider, &target.branch).await?;
            (Repo::Clone(repository), base)
        }
        None => (Repo::Project, vcs::project_base(provider).await?),
    };
    let integration = WorkingCopy::cut(provider, &repo.path(), INTEGRATION, &base).await?;

    let waves = revision.plan.waves();
    let order = revision.plan.order();
    tracing::info!(
        revision = %revision_id,
        adapter = %adapter,
        %base,
        slices = order.len(),
        waves = waves.len(),
        widest = waves.widest(),
        "building"
    );

    // every slice carries the whole design, rendered once
    let design = revision.design.to_string();
    let slicing = Slicing {
        adapter: &adapter,
        reference: &input.adapter,
        revision: &revision_id,
        base: &base,
        spec: &revision.spec,
        design: &design,
        integration: &integration,
    };
    let mut built = Vec::with_capacity(order.len());
    for slice in order {
        match slicing.build(provider, slice).await {
            Ok(outcome) => built.push(outcome),
            Err(error) => return Err(at_slice(error, slice, &built)),
        }
    }

    // the integrated head, labelled for the revision
    let head = integration.head(provider).await?;
    let label = format!("emery/{revision_id}");
    repo.publish(provider, &label, &head, input.remote.as_deref()).await?;
    integration.remove(provider).await?;

    Ok(BuildOutput {
        revision: revision_id,
        waves,
        base,
        slices: built,
        head,
        label,
        pushed: input.remote,
    })
}

/// The target a build runs through, and the repository it builds into.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildInput {
    /// The target adapter every slice is built through.
    pub adapter: AdapterRef,
    /// The `sha256:` digest the adapter's component must resolve to.
    ///
    /// The run passes it on the load, and the loader holds the resolved
    /// bytes to it, stored or fetched. `None` trusts whatever the load
    /// resolves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
    /// The repository the plan is built into, at a branch.
    ///
    /// `None` builds into the project's own repository, from the sealed
    /// commit its checkout sits on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<TargetRepository>,
    /// The remote the label is pushed to once the build integrates.
    ///
    /// `None` leaves the label local.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
}

/// A repository a plan is built into, at the branch the build starts from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TargetRepository {
    /// The URL the repository is cloned from.
    pub url: String,
    /// The branch whose commit the build starts from, resolved each run.
    pub branch: String,
}

/// What a successful [`build`] built.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildOutput {
    /// The identifier of the revision whose plan was built.
    pub revision: String,
    /// The plan's slices grouped into the sets ready to build at once.
    ///
    /// The slices were built one at a time, in the order the waves flatten
    /// to; the waves are what a build could have run concurrently.
    pub waves: Waves,
    /// The commit the build started from.
    pub base: String,
    /// Every slice, in build order, as its build reported it.
    pub slices: Vec<BuiltSlice>,
    /// The integrated commit the label points at.
    pub head: String,
    /// The label set on the head, `emery/<revision>`.
    pub label: String,
    /// The remote the label was pushed to, when the target names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pushed: Option<String>,
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
    /// working copy's root.
    pub written: Vec<String>,
    /// The commit that sealed what the slice wrote; `None` when it changed
    /// nothing.
    pub commit: Option<String>,
}

// What every slice of one build shares.
struct Slicing<'a> {
    adapter: &'a str,
    reference: &'a AdapterRef,
    revision: &'a str,
    base: &'a str,
    spec: &'a Spec,
    design: &'a str,
    integration: &'a WorkingCopy,
}

impl Slicing<'_> {
    #[tracing::instrument(skip_all, fields(slice = %slice.id, name = %slice.name))]
    async fn build<P: Target + Vcs>(
        &self, provider: &P, slice: &Slice,
    ) -> Result<BuiltSlice, Error> {
        let input = SliceInput {
            id: slice.id.to_string(),
            name: slice.name.clone(),
            requirements: slice.requirements.iter().map(ToString::to_string).collect(),
            spec: self.spec.cut(&slice.requirements),
            design: self.design.to_owned(),
            plan: slice.to_string(),
        };
        tracing::info!(requirements = input.requirements.len(), "building slice");
        let report = Target::build(provider, self.adapter, &input, self.integration.path()).await?;

        let findings = report.findings(&input);
        if !findings.is_empty() {
            return Err(server_error!(
                "`{}` returned an invalid report:\n{}",
                self.adapter,
                findings.join("\n")
            ));
        }

        // the gate held every covered id to the slice, so each is one of these
        let (covered, uncovered): (Vec<ReqId>, Vec<ReqId>) =
            slice.requirements.iter().partition(|id| report.covers(&id.to_string()));

        // what the slice wrote, sealed as its commit
        let pending = self.integration.pending(provider).await?;
        let commit = self.integration.commit(provider, &self.message(slice, &covered)).await?;
        tracing::info!(
            covered = covered.len(),
            uncovered = uncovered.len(),
            written = report.written.len(),
            changed = pending.len(),
            commit = commit.as_deref().unwrap_or("none"),
            "slice built"
        );

        Ok(BuiltSlice {
            id: slice.id,
            name: slice.name.clone(),
            covered,
            uncovered,
            written: report.written,
            commit,
        })
    }

    // `<id> <name>`, then the trailers that tie the commit to its revision.
    fn message(&self, slice: &Slice, covered: &[ReqId]) -> String {
        let list =
            |ids: &[ReqId]| ids.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
        format!(
            "{} {}\n\nRevision: {}\nRequirements: {}\nCovered: {}\nAdapter: {}\nBase: {}",
            slice.id,
            slice.name,
            self.revision,
            list(&slice.requirements),
            list(covered),
            self.reference,
            self.base
        )
    }
}

// A slice's failure keeps its class and code; the description gains the
// slice it failed at and the slices built before it, which stay committed
// in the integration working copy.
fn at_slice(error: Error, slice: &Slice, built: &[BuiltSlice]) -> Error {
    let ids: Vec<String> = built.iter().map(|built| built.id.to_string()).collect();
    let before = match ids.as_slice() {
        [] => String::new(),
        [one] => format!("; {one} built before it stays committed in `{INTEGRATION}`"),
        many => {
            format!("; {} built before it stay committed in `{INTEGRATION}`", many.join(", "))
        }
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
