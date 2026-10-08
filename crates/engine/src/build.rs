//! Builds the current plan through a target adapter, into a labelled commit.
//!
//! [`build`] reads the current revision, loads the one target adapter the run
//! names, settles the base — the project checkout's sealed head, or a branch
//! of the repository the target names — and reads what the label
//! `emery/<revision>` already holds over it, so a run resumes where the last
//! verified wave left off. The slices still to build go in waves: every slice
//! whose dependencies are merged is built at once, each in a working copy of
//! its own cut at the wave's head, with its plan entry, its cut of the
//! specification, and the whole design. What each wrote is sealed as its
//! commit and merged into the integration working copy in id order under the
//! adapter's merge rules. The adapter verifies the integrated tree, the wave
//! is labelled, and the next wave is cut from it. The engine writes no state
//! of its own: the labelled history is the output, pushed when the target
//! names a remote.
//!
//! A slice whose merge conflicts is left for the next wave and built again
//! over the merged head; a second conflict ends the run. A wave the adapter
//! does not verify ends the run with the label where the last verified wave
//! left it. Either way the integration working copy is left for inspection
//! and removed by the next run.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;

use emery_adapter::target::{Slice as SliceInput, Target};
use futures::{StreamExt as _, TryFutureExt as _, TryStreamExt as _, stream};
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::vcs::Rule;
use omnia_sdk::{BlobStore, Error, Plugins, StateStore, Vcs, server_error};
use serde::{Deserialize, Serialize};

use crate::adapter::{self, AdapterRef, Loaded};
pub use crate::revision::{ReqId, SliceId, Waves};
use crate::revision::{Slice, Spec};
use crate::vcs::{INTEGRATION, Message, Repo, Repository, WorkingCopy};
use crate::{store, vcs};

/// How many times a slice is built before a conflict at its merge ends the run.
pub const ATTEMPTS: usize = 2;

/// Builds every slice of the current plan through the target adapter `input` names.
///
/// The slices the label `emery/<revision>` already holds are not built
/// again. The rest go in waves: every slice whose dependencies are merged is
/// built at once, up to `jobs` concurrently, in a working copy cut at the
/// wave's head; each report is held to its slice by
/// [`Report::findings`](emery_adapter::target::Report::findings), each
/// commit merged in id order under the adapter's merge rules, and the wave
/// verified by the adapter and labelled. A slice whose merge conflicts is
/// built again in the next wave, over the merged head; the first failure
/// ends the run, and what was merged before it stays committed in the
/// integration working copy, the verified waves labelled.
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
///   - with code `slice-conflict`, when a slice's merge conflicts at paths
///     no rule resolves on its [`ATTEMPTS`]th build, the paths listed;
///   - with code `verify-failed`, when the adapter does not verify a wave,
///     each failing check listed;
///   - a slice the adapter refuses, or answers no acceptable report for.
/// - Returns [`Error::ServerError`] when a report breaks the report gate, a
///   verdict breaks [`Verdict::findings`](emery_adapter::target::Verdict::findings),
///   the slices left to build wait on one another, or storage or version
///   control fails.
/// - Returns [`Error::BadGateway`] when the adapter, its acquisition, the
///   model, or the repository's remote fails upstream.
///
/// A failure at a slice keeps the adapter's class and code; its description
/// names the slice, its wave, and the slices merged before it. A failure at a
/// wave's verification names the wave, the slices merged in it, and where
/// the label stands.
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

    let Loaded {
        id: adapter,
        metadata,
    } = adapter::load_target(provider, &input.adapter, input.digest.as_ref()).await?;

    // the base, and what the label already holds over it
    let (repo, base) = match &input.repository {
        Some(target) => {
            let repository = Repository::new(&target.url);
            repository.ensure(provider).await?;
            let base = repository.resolve(provider, &target.branch).await?;
            (Repo::Clone(repository), base)
        }
        None => (Repo::Project, vcs::project_base(provider).await?),
    };
    let label = format!("emery/{revision_id}");
    let labelled = repo.merged_under(provider, &label, &base, &revision_id).await?;
    let (head, merged) = labelled.clone().unwrap_or_else(|| (base.clone(), BTreeSet::new()));
    let integration = WorkingCopy::cut(provider, &repo.path(), INTEGRATION, &head).await?;

    let waves = revision.plan.waves();
    tracing::info!(
        revision = %revision_id,
        adapter = %adapter,
        %base,
        %head,
        slices = revision.plan.slices.len(),
        waves = waves.len(),
        widest = waves.widest(),
        resumed = merged.len(),
        "building"
    );

    // every slice carries the whole design, rendered once, and every merge
    // the adapter's rules
    let design = revision.design.to_string();
    let policy = vcs::rules(&metadata.merge_rules);
    let run = Run {
        adapter: &adapter,
        reference: &input.adapter,
        revision: &revision_id,
        label: &label,
        spec: &revision.spec,
        design: &design,
        repo: &repo,
        integration: &integration,
        policy: &policy,
        jobs: input.jobs,
    };
    let mut progress = Progress {
        resumed: merged.iter().copied().collect(),
        head,
        labelled: labelled.map(|(head, _)| head),
        merged,
        built: Vec::new(),
        tries: BTreeMap::new(),
        conflicts: BTreeMap::new(),
        verified: Vec::new(),
    };

    // wave by wave, until every slice is merged
    let mut wave = 0;
    while progress.merged.len() < revision.plan.slices.len() {
        let ready = revision.plan.ready(&progress.merged);
        if ready.is_empty() {
            let left: Vec<String> = revision
                .plan
                .slices
                .iter()
                .filter(|slice| !progress.merged.contains(&slice.id))
                .map(|slice| slice.id.to_string())
                .collect();
            return Err(server_error!(
                "the slices left to build wait on one another: {}",
                left.join(", ")
            ));
        }
        wave += 1;
        run.wave(provider, &mut progress, wave, &ready).await?;
    }

    // the label stands at the last verified head; sent on when the target names where
    if let Some(remote) = &input.remote {
        repo.push(provider, &label, remote).await?;
    }
    integration.remove(provider).await?;

    Ok(BuildOutput {
        revision: revision_id,
        waves,
        base,
        resumed: progress.resumed,
        slices: progress.built,
        verified: progress.verified,
        head: progress.head,
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
    /// How many slices of a wave are built at once.
    ///
    /// `None` builds every slice of a wave at once. The cap bounds
    /// concurrency alone: every slice of a wave builds over the wave's head
    /// whatever it is, so a cap of one yields the same history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jobs: Option<NonZeroUsize>,
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
    /// The plan's own projection: what the waves would be were every slice
    /// built from the base. A conflicted slice moves to a later wave of the
    /// run, which each [`BuiltSlice::wave`] records.
    pub waves: Waves,
    /// The commit the build started from.
    pub base: String,
    /// The slices the label already held, not built again, in id order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resumed: Vec<SliceId>,
    /// Every slice this run built, in build order, as its build reported it.
    pub slices: Vec<BuiltSlice>,
    /// The integrated head each wave was verified and labelled at, in wave order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub verified: Vec<String>,
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
    /// The wave the slice merged in, from one.
    pub wave: usize,
    /// The slice's requirements the adapter reports implemented, in id order.
    pub covered: Vec<ReqId>,
    /// The slice's requirements the adapter did not report, in id order.
    pub uncovered: Vec<ReqId>,
    /// The files the adapter reports written or changed, relative to the
    /// working copy's root.
    pub written: Vec<String>,
    /// The merge commit that brought what the slice wrote into the
    /// integrated head; `None` when it changed nothing.
    pub commit: Option<String>,
    /// The paths an earlier build of the slice conflicted at, before it was
    /// built again over the merged head.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
}

// What every wave of one build shares.
struct Run<'a> {
    adapter: &'a str,
    reference: &'a AdapterRef,
    revision: &'a str,
    label: &'a str,
    spec: &'a Spec,
    design: &'a str,
    repo: &'a Repo,
    integration: &'a WorkingCopy,
    policy: &'a [Rule],
    jobs: Option<NonZeroUsize>,
}

// Where a build stands between waves.
struct Progress {
    // the integrated head the next wave builds over
    head: String,
    // where the label stands; `None` until a wave of a fresh build verifies
    labelled: Option<String>,
    merged: BTreeSet<SliceId>,
    resumed: Vec<SliceId>,
    built: Vec<BuiltSlice>,
    tries: BTreeMap<SliceId, usize>,
    conflicts: BTreeMap<SliceId, Vec<String>>,
    verified: Vec<String>,
}

// One slice built in its working copy, not yet merged.
struct Built<'a> {
    slice: &'a Slice,
    worktree: WorkingCopy,
    covered: Vec<ReqId>,
    uncovered: Vec<ReqId>,
    written: Vec<String>,
}

enum Integrated {
    Merged(Option<String>),
    Conflicted(Vec<String>),
}

impl Run<'_> {
    #[tracing::instrument(skip_all, fields(wave))]
    async fn wave<P: Target + Vcs>(
        &self, provider: &P, progress: &mut Progress, wave: usize, ready: &[&Slice],
    ) -> Result<(), Error> {
        let jobs = self.jobs.map_or(ready.len(), NonZeroUsize::get).min(ready.len());
        tracing::info!(slices = ready.len(), jobs, head = %progress.head, "building wave");
        for slice in ready {
            *progress.tries.entry(slice.id).or_default() += 1;
        }

        // every ready slice, built at once in a working copy of its own
        let head = &progress.head;
        let before = &progress.built;
        let mut builds = Vec::with_capacity(ready.len());
        for slice in ready.iter().copied() {
            let build = self
                .build(provider, slice, head, wave)
                .map_err(move |error| at_slice(error, slice, wave, before));
            builds.push(build);
        }
        let mut built: Vec<Built<'_>> =
            stream::iter(builds).buffer_unordered(jobs).try_collect().await?;

        // merged in id order, whatever order they finished in
        built.sort_by_key(|built| built.slice.id);
        let mut merged = Vec::with_capacity(built.len());
        for built in built {
            let slice = built.slice;
            match self.integrate(provider, progress, built, wave).await {
                Ok(true) => merged.push(slice.id),
                Ok(false) => {}
                Err(error) => return Err(at_slice(error, slice, wave, &progress.built)),
            }
        }
        if merged.is_empty() {
            tracing::info!("nothing merged; the wave leaves the integrated head as it was");
            return Ok(());
        }

        // the integrated tree, verified by the adapter and labelled
        if let Err(error) = self.verify(provider, progress, wave, &merged).await {
            return Err(at_wave(error, wave, &merged, self.label, progress.labelled.as_deref()));
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(slice = %slice.id, name = %slice.name))]
    async fn build<'a, P: Target + Vcs>(
        &self, provider: &P, slice: &'a Slice, head: &str, wave: usize,
    ) -> Result<Built<'a>, Error> {
        let at = vcs::slice_worktree(slice.id);
        let worktree = WorkingCopy::cut(provider, &self.repo.path(), &at, head).await?;
        let input = SliceInput {
            id: slice.id.to_string(),
            name: slice.name.clone(),
            requirements: slice.requirements.iter().map(ToString::to_string).collect(),
            spec: self.spec.cut(&slice.requirements),
            design: self.design.to_owned(),
            plan: slice.to_string(),
            base: head.to_owned(),
        };
        tracing::info!(requirements = input.requirements.len(), wave, "building slice");
        let report = Target::build(provider, self.adapter, &input, worktree.path()).await?;

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
        tracing::info!(
            covered = covered.len(),
            uncovered = uncovered.len(),
            written = report.written.len(),
            "slice built"
        );
        Ok(Built {
            slice,
            worktree,
            covered,
            uncovered,
            written: report.written,
        })
    }

    // Seals what the slice wrote and merges it into the integrated head;
    // `true` when it merged, `false` when a conflict leaves it for the next
    // wave. The slice's working copy is removed either way.
    async fn integrate<P: Vcs>(
        &self, provider: &P, progress: &mut Progress, built: Built<'_>, wave: usize,
    ) -> Result<bool, Error> {
        let slice = built.slice;
        let message = Message {
            slice: slice.id,
            name: slice.name.clone(),
            revision: self.revision.to_owned(),
            requirements: slice.requirements.clone(),
            covered: built.covered.clone(),
            adapter: self.reference.to_string(),
            base: progress.head.clone(),
            wave,
        }
        .to_string();

        let integrated = match built.worktree.commit(provider, &message).await? {
            None => Integrated::Merged(None),
            Some(commit) => {
                let merged =
                    self.integration.merge(provider, &commit, &message, self.policy).await?;
                match merged.commit {
                    Some(commit) => Integrated::Merged(Some(commit)),
                    None => Integrated::Conflicted(merged.conflicts),
                }
            }
        };
        built.worktree.remove(provider).await?;

        match integrated {
            Integrated::Merged(commit) => {
                let conflicts = progress.conflicts.remove(&slice.id).unwrap_or_default();
                tracing::info!(
                    slice = %slice.id,
                    commit = commit.as_deref().unwrap_or("none"),
                    "slice merged"
                );
                progress.merged.insert(slice.id);
                progress.built.push(BuiltSlice {
                    id: slice.id,
                    name: slice.name.clone(),
                    wave,
                    covered: built.covered,
                    uncovered: built.uncovered,
                    written: built.written,
                    commit,
                    conflicts,
                });
                Ok(true)
            }
            Integrated::Conflicted(paths) => {
                let tries = progress.tries.get(&slice.id).copied().unwrap_or(1);
                if tries >= ATTEMPTS {
                    return Err(Error::BadRequest {
                        code: "slice-conflict".into(),
                        description: format!(
                            "its merge conflicts at {} on its build {tries}",
                            paths.join(", ")
                        ),
                    });
                }
                tracing::warn!(
                    slice = %slice.id,
                    conflicts = paths.join(", "),
                    "slice conflicted; it builds again over the merged head"
                );
                progress.conflicts.entry(slice.id).or_default().extend(paths);
                Ok(false)
            }
        }
    }

    // Has the adapter verify the integrated tree, seals what its checks left
    // behind, and labels the head the wave reached.
    async fn verify<P: Target + Vcs>(
        &self, provider: &P, progress: &mut Progress, wave: usize, merged: &[SliceId],
    ) -> Result<(), Error> {
        let verdict = Target::verify(provider, self.adapter, self.integration.path()).await?;
        let findings = verdict.findings();
        if !findings.is_empty() {
            return Err(server_error!(
                "`{}` returned an invalid verdict:\n{}",
                self.adapter,
                findings.join("\n")
            ));
        }
        if !verdict.passed {
            let failures: Vec<String> =
                verdict.failures.iter().map(|failure| format!("- {failure}")).collect();
            return Err(Error::BadRequest {
                code: "verify-failed".into(),
                description: format!("verification failed:\n{}", failures.join("\n")),
            });
        }

        // what the checks left behind, sealed as the wave's own commit
        let pending = self.integration.pending(provider).await?;
        if !pending.is_empty() {
            let ids: Vec<String> = merged.iter().map(ToString::to_string).collect();
            let message = format!(
                "Wave {wave} verified\n\nRevision: {}\nAdapter: {}\nSlices: {}",
                self.revision,
                self.reference,
                ids.join(", ")
            );
            let sealed = self.integration.commit(provider, &message).await?;
            tracing::info!(
                changed = pending.len(),
                commit = sealed.as_deref().unwrap_or("none"),
                "verification by-products sealed"
            );
        }

        let head = self.integration.head(provider).await?;
        self.repo.label(provider, self.label, &head).await?;
        tracing::info!(merged = merged.len(), %head, "wave verified");
        progress.verified.push(head.clone());
        progress.labelled = Some(head.clone());
        progress.head = head;
        Ok(())
    }
}

// A slice's failure keeps its class and code; the description gains the
// slice it failed at, its wave, and the slices merged before it, which
// stay committed in the integration working copy.
fn at_slice(error: Error, slice: &Slice, wave: usize, built: &[BuiltSlice]) -> Error {
    let ids: Vec<String> = built.iter().map(|built| built.id.to_string()).collect();
    let before = match ids.as_slice() {
        [] => String::new(),
        [one] => format!("; {one} merged before it stays committed in `{INTEGRATION}`"),
        many => {
            format!("; {} merged before it stay committed in `{INTEGRATION}`", many.join(", "))
        }
    };
    describe(error, |description| {
        format!(
            "slice `{}` ({}) failed in wave {wave}{before}: {description}",
            slice.id, slice.name
        )
    })
}

// A wave's failure keeps its class and code; the description gains the
// wave, the slices merged in it, and where the label stands.
fn at_wave(
    error: Error, wave: usize, merged: &[SliceId], label: &str, labelled: Option<&str>,
) -> Error {
    let ids: Vec<String> = merged.iter().map(ToString::to_string).collect();
    let merged = match ids.as_slice() {
        [] => String::new(),
        [one] => format!("; {one} merged in it stays committed in `{INTEGRATION}`"),
        many => format!("; {} merged in it stay committed in `{INTEGRATION}`", many.join(", ")),
    };
    let standing = labelled.map_or_else(
        || format!("`{label}` is not set"),
        |head| format!("`{label}` stays at `{head}`"),
    );
    describe(error, |description| format!("wave {wave} failed{merged}; {standing}: {description}"))
}

fn describe(error: Error, describe: impl FnOnce(String) -> String) -> Error {
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
