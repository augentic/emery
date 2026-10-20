//! Builds the current plan through a target adapter into a labelled commit.
//!
//! [`build`] initialises the target repository when needed, builds ready
//! slices in waves across isolated working copies, merges their commits under
//! the adapter's rules (resolving conflicts with the model), verifies each
//! wave, and labels the verified head `emery/<revision>`. Interrupted runs
//! resume from the existing label and integration working copy.

pub(crate) mod merge;

use std::collections::BTreeSet;
use std::fmt::{self, Display};
use std::num::NonZeroUsize;

use emery_adapter::target::{DesignType, Outcome, Slice as SliceInput, Target, Task};
use emery_adapter::{beneath, is_kebab};
use futures::{FutureExt as _, StreamExt as _, stream};
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::vcs::{Change, ChangeKind, Rule, Strategy};
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, Vcs, bad_request, server_error};
use serde::Serialize;

use self::merge::{Conflict, MergeBrief};
use crate::adapter::{self, AdapterRef, Loaded};
use crate::brief::Brief as _;
use crate::revision::{Design, Plan, Slice, Spec};
pub use crate::revision::{ReqId, SliceId, Waves};
use crate::vcs::{BuildRepo, INTEGRATION, Integration, Message, WorkingCopy};
use crate::{EntryKind, Files, RETRIES, is_retryable, secrets, store, vcs};

/// Builds every slice of the current plan through the target adapter `input` names.
///
/// The slices the label `emery/<revision>` already holds are not built
/// again, and neither are those the integration working copy holds merged
/// over the labelled head from a run that ended before their wave was
/// verified: they are verified in their wave instead. A build neither
/// resumes first has the adapter initialise the tree — scaffold the
/// layout's initial files it lacks, with no model turn — and seals what it
/// scaffolded as the `Initialise` commit the slices build over; a repository
/// the engine creates holds that commit as its root. The rest go in waves:
/// every slice whose dependencies are merged is built at once, up to `jobs`
/// concurrently, in a working copy cut at the wave's head; each report is
/// held to its slice by
/// [`Report::findings`](emery_adapter::target::Report::findings), what each
/// copy changed sealed as its commit and merged in id order into the
/// integration working copy under the adapter's merge rules, a path the
/// rules leave conflicted resolved with the model and merged in its place,
/// and the wave verified by the adapter and labelled, what its checks
/// changed sealed first. A build that fails upstream or runs out of the
/// backend's time is put once more in a fresh copy, a conflict's resolution
/// or a verification once more over the same one. The first failure ends
/// the run once the slices built beside it are merged, and what was merged
/// before it stays committed in the integration working copy, the verified
/// waves labelled, for the next run to resume from.
///
/// # Errors
///
/// - Returns [`Error::NotFound`]:
///   - with code `spec-not-generated`, when no revision has been committed,
///     naming a staged candidate where one stands;
///   - with code `revision-not-found`, when the repository the target names
///     does not exist or has no default branch, or what the run already
///     holds under the target's name was cloned or created under another
///     origin.
/// - Returns [`Error::BadRequest`] when the run is refused:
///   - when the target's name is not kebab-case;
///   - with code `spec-outdated`, when the stored revision uses another
///     grammar;
///   - an adapter [`specify`](crate::specify::specify) would refuse: one that
///     is not a target adapter ([`Axis::Target`](crate::Axis)), an
///     incompatible one (code `unsupported-version`), a release the store
///     lacks whose namespace the deployment routes nowhere, a pre-compiled
///     artifact, or one resolving to other bytes than its pin (code
///     `refused`);
///   - with code `repository-required`, when the target names the project
///     checkout as its repository ([`vcs::PROJECT`]) and the project is no
///     repository;
///   - with code `base-not-sealed`, when the project checkout holds pending
///     changes outside `.emery/`, listed, or no commit;
///   - with code `verify-failed`, when the adapter does not verify a wave,
///     each failing check listed;
///   - with code `secret-detected`, when a path the initialisation, a
///     slice's build, a conflict's resolution, or a wave's verification
///     changed carries a credential, each listed with what it carries;
///     nothing of the change set is sealed, and the copy stays as it was
///     written;
///   - the initialisation, when the adapter refuses the tree, as when a
///     directory stands where a file of its layout goes;
///   - with code `merge-unresolved`, when a path the adapter's rules leave
///     conflicted cannot be put to the model — one side does not hold it
///     as a text file, or the path is one the engine never writes — or the
///     model's resolution is refused within the available rounds; the
///     slice's copy and the integration copy stay for inspection;
///   - a slice the adapter refuses or answers no acceptable report for.
/// - Returns [`Error::ServerError`] when the adapter answers a task with
///   an outcome of another kind, a report breaks the report gate, a
///   verdict breaks [`Verdict::findings`](emery_adapter::target::Verdict::findings),
///   a slice's working copy no longer sits on the head it was cut at, a
///   slice merged again with its resolutions still conflicts, the
///   initialisation's or a verification's changes seal as no commit, the
///   slices left to build wait on one another, or storage or version
///   control fails.
/// - Returns [`Error::BadGateway`] when the adapter, its acquisition, the
///   model, or the repository's remote fails upstream; a slice's build, a
///   conflict's resolution, or a wave's verification is put twice before it
///   does, as one the backend's time budget ended is; the initialisation,
///   which puts no turn, once.
///
/// A failure at a slice keeps the adapter's class and code; its description
/// names the slice, its wave, and the slices merged before it, those built
/// beside it in its wave included. A failure at a wave's verification names
/// the wave, the slices merged in it, and where the label stands.
pub async fn build<P: Model + Target + StateStore + BlobStore + Plugins + Vcs + Files>(
    input: BuildInput, context: Context<P>,
) -> Result<BuildOutput, Error> {
    let provider = context.provider();
    let name = &input.name;
    if !is_kebab(name) {
        return Err(bad_request!("target `{name}` is not a kebab-case name"));
    }

    let Some((revision_id, revision)) = store::current(provider).await? else {
        return Err(store::not_generated(provider).await);
    };
    let candidate = store::staged_id(provider).await?.filter(|staged| *staged != revision_id);

    let Loaded {
        id: adapter,
        metadata,
    } = adapter::load_target(provider, &input.adapter, input.digest.as_ref()).await?;

    // the repository the build lands in: the project's for `.`, else the
    // target's own, a clone of the URL or the one the engine creates; then
    // the base, the adapter's layout scaffolded into a repository the engine
    // creates, and what the label already holds over it
    let repo = match input.repository.as_deref() {
        Some(vcs::PROJECT) => BuildRepo::Project,
        url => BuildRepo::Target {
            name: input.name,
            url: url.map(ToOwned::to_owned),
        },
    };
    let created = matches!(repo, BuildRepo::Target { url: None, .. });
    let base = repo.base(provider, async |at| initialise(provider, &adapter, at).await).await?;
    let label = format!("emery/{revision_id}");
    let labelled = repo.merged_under(provider, &label, &base, &revision_id).await?;
    let mut progress = Progress::start(&base, labelled);
    let Integration {
        copy: integration,
        head,
        staged,
    } = repo.cut_integration(provider, &progress.head, &revision_id).await?;
    progress.stage(head, staged);

    // a created repository's base is its initialisation; any other base is
    // initialised on a fresh build
    let initialised =
        if created { None } else { progress.initialise(provider, &integration, &adapter).await? };

    let waves = revision.plan.waves();
    tracing::info!(
        revision = %revision_id,
        adapter = %adapter,
        %base,
        head = %progress.head,
        slices = revision.plan.slices.len(),
        waves = waves.len(),
        widest = waves.widest(),
        resumed = progress.resumed.len(),
        staged = progress.staged.len(),
        "building"
    );

    // every slice carries the whole design, rendered once, and every merge
    // the adapter's rules
    let rendered = revision.design.to_string();
    let policy = vcs::rules(&metadata.merge_rules);
    let run = Run {
        adapter: &adapter,
        reference: &input.adapter,
        revision: &revision_id,
        label: &label,
        spec: &revision.spec,
        design: &revision.design,
        rendered: &rendered,
        plan: &revision.plan,
        repo: &repo,
        integration: &integration,
        policy: &policy,
        jobs: input.jobs,
    };

    run.waves(provider, &mut progress).await?;
    integration.remove(provider).await?;

    Ok(BuildOutput {
        revision: revision_id,
        candidate,
        waves,
        base,
        repository: repo.path(),
        initialised,
        resumed: progress.resumed,
        slices: progress.built,
        verified: progress.verified,
        sealed: progress.sealed,
        repaired: progress.repaired,
        head: progress.head,
        label,
    })
}

/// The target a build runs through, and the repository it builds into.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildInput {
    /// The kebab-case name the target's repository is kept under.
    ///
    /// The caller's own name for the target, or the adapter's
    /// ([`AdapterRef::name`]) when the caller gives none. The repository a
    /// build clones or creates sits at [`vcs::target_path`] of it.
    pub name: String,
    /// The target adapter every slice is built through.
    pub adapter: AdapterRef,
    /// The `sha256:` digest the adapter's component must resolve to.
    ///
    /// The run passes it on the load, and the loader holds the resolved
    /// bytes to it, stored or fetched. `None` trusts whatever the load
    /// resolves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
    /// The repository the plan is built into.
    ///
    /// A URL builds into a clone of it, from its default branch as the
    /// remote holds it. [`vcs::PROJECT`] builds into the project's own
    /// repository, from the sealed commit its checkout sits on. `None`
    /// builds into the repository the engine creates under the name and keeps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// How many slices of a wave are built at once.
    ///
    /// `None` builds every slice of a wave at once. The cap bounds
    /// concurrency alone: every slice of a wave builds over the wave's head
    /// whatever it is, so a cap of one yields the same history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jobs: Option<NonZeroUsize>,
}

/// What a successful [`build`] built.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct BuildOutput {
    /// The identifier of the revision whose plan was built.
    pub revision: String,
    /// A candidate staged and not sealed, by id, which the build did not
    /// read: `emery apply` seals it for the next build.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<String>,
    /// The plan's slices grouped into the sets ready to build at once.
    ///
    /// The plan's own projection: what the waves would be were every slice
    /// built from the base.
    pub waves: Waves,
    /// The commit the build started from.
    pub base: String,
    /// The repository the label stands in, as a path beneath the project
    /// root: `.` for the project's own, or [`vcs::target_path`] of the
    /// target's name for the one it names or the engine created.
    pub repository: String,
    /// The `Initialise` commit this run sealed over the base with the
    /// layout the adapter scaffolded, when it scaffolded any.
    ///
    /// `None` for a build resumed from the label or from staged slices,
    /// for a repository the engine created, whose base is that commit, and
    /// for a tree the initialisation added nothing to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialised: Option<Initialised>,
    /// The slices not built again, in id order.
    ///
    /// Those the label already held, and those the integration working
    /// copy held merged over the labelled head from a run that ended before
    /// their wave was verified, which this run verified in their wave; each
    /// [`Verified`] names the latter among its slices.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resumed: Vec<SliceId>,
    /// Every slice this run built, in build order, as its build reported it.
    pub slices: Vec<BuiltSlice>,
    /// Each wave this run verified and labelled, in wave order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub verified: Vec<Verified>,
    /// What each wave's verification left in the integrated tree, sealed as
    /// `Wave <k> verified` before the head was labelled, in wave order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sealed: Vec<Seal>,
    /// The waves whose verification repaired the integrated tree, from one:
    /// those whose seal carries a path the adapter's `ours` rules do not
    /// cover, so not a lockfile the checks regenerated.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub repaired: Vec<usize>,
    /// The integrated commit the label points at.
    pub head: String,
    /// The label set on the head, `emery/<revision>`.
    pub label: String,
}

/// The layout the adapter scaffolded into the tree before any slice, sealed as the `Initialise` commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Initialised {
    /// The commit the scaffolded files were sealed as, which the slices build over.
    pub commit: String,
    /// The paths the initialisation scaffolded, relative to the tree root and sorted.
    pub paths: Vec<String>,
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
    /// The paths the build changed in the slice's working copy — written,
    /// edited, or removed — relative to its root and sorted: what its commit
    /// records, read from the copy rather than the adapter's report.
    pub written: Vec<String>,
    /// The paths the adapter's merge rules left conflicted and the model
    /// resolved, sorted; empty when the rules left none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resolved: Vec<String>,
    /// The commit that brought what the slice wrote into the integrated
    /// head; `None` when it changed nothing.
    pub commit: Option<String>,
}

/// One wave verified and labelled, with the slices it merged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Verified {
    /// The wave, from one.
    pub wave: usize,
    /// The slices merged in the wave, in id order: those this run built,
    /// and those an earlier run staged in the integration working copy,
    /// which [`BuildOutput::resumed`] lists and [`BuildOutput::slices`]
    /// does not.
    pub slices: Vec<SliceId>,
    /// The integrated head the wave was verified and labelled at.
    pub head: String,
}

/// What one wave's verification left in the integrated tree, sealed as the wave's commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Seal {
    /// The wave, from one.
    pub wave: usize,
    /// The `Wave <k> verified` commit the changes were sealed as.
    pub commit: String,
    /// The paths the verification changed, relative to the tree root and sorted.
    pub paths: Vec<String>,
}

// What every wave of one build shares.
struct Run<'a> {
    adapter: &'a str,
    reference: &'a AdapterRef,
    revision: &'a str,
    label: &'a str,
    spec: &'a Spec,
    design: &'a Design,
    // the design as every slice carries it
    rendered: &'a str,
    plan: &'a Plan,
    repo: &'a BuildRepo,
    integration: &'a WorkingCopy,
    // the adapter's merge rules, in the capability's reading
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
    // merged over the head by an earlier run and never verified: each is
    // this run's in the wave it is ready in, with no build
    staged: BTreeSet<SliceId>,
    resumed: Vec<SliceId>,
    built: Vec<BuiltSlice>,
    verified: Vec<Verified>,
    sealed: Vec<Seal>,
    repaired: Vec<usize>,
}

impl Progress {
    // `labelled` is the head the label stands at and the slices its chain names;
    // `None` starts a fresh build from the base.
    fn start(base: &str, labelled: Option<(String, BTreeSet<SliceId>)>) -> Self {
        let (head, merged) = labelled.clone().unwrap_or_else(|| (base.to_owned(), BTreeSet::new()));
        Self {
            resumed: merged.iter().copied().collect(),
            head,
            labelled: labelled.map(|(head, _)| head),
            merged,
            staged: BTreeSet::new(),
            built: Vec::new(),
            verified: Vec::new(),
            sealed: Vec::new(),
            repaired: Vec::new(),
        }
    }

    // `head` is where the integration copy was cut and `staged` the slices
    // an earlier run merged over the labelled head beneath it; the build
    // runs over `head` and verifies them in their wave.
    fn stage(&mut self, head: String, staged: BTreeSet<SliceId>) {
        if staged.is_empty() {
            return;
        }
        self.head = head;
        self.resumed.extend(staged.iter().copied());
        self.resumed.sort_unstable();
        self.resumed.dedup();
        self.staged = staged;
    }

    // A fresh build is initialised with the adapter's layout, what it
    // scaffolded sealed in the integration copy as the commit the slices
    // build over.
    // One resumed from the label, or from what a run staged over it, was
    // initialised by that run.
    async fn initialise<P: Target + Vcs + Files>(
        &mut self, provider: &P, integration: &WorkingCopy, adapter: &str,
    ) -> Result<Option<Initialised>, Error> {
        if self.labelled.is_some() || !self.staged.is_empty() {
            return Ok(None);
        }
        let changes = initialise(provider, adapter, integration.path()).await?;
        if changes.is_empty() {
            return Ok(None);
        }
        let Some(commit) = integration.commit(provider, vcs::INITIALISE).await? else {
            return Err(server_error!(
                "the initialisation left {} paths pending, yet sealing them made no commit",
                changes.len()
            ));
        };
        let mut paths: Vec<String> = changes.into_iter().map(|change| change.path).collect();
        paths.sort_unstable();
        tracing::info!(scaffolded = paths.len(), %commit, "initialisation sealed");
        self.head.clone_from(&commit);
        Ok(Some(Initialised { commit, paths }))
    }
}

// One slice built in its working copy, not yet merged.
struct Built<'a> {
    slice: &'a Slice,
    worktree: WorkingCopy,
    covered: Vec<ReqId>,
    uncovered: Vec<ReqId>,
}

// A slice's merge under the adapter's rules, left at the paths no rule
// resolved: what this slice and one merged before it in the wave both
// rewrote.
struct UnresolvedMerge<'a> {
    built: &'a Built<'a>,
    // what the slice's build changed, as its working copy holds it
    pending: &'a [Change],
    // the slice's sealed commit, and the message it and its merge carry
    commit: String,
    message: &'a str,
    paths: Vec<String>,
}

impl Run<'_> {
    // Wave by wave, until every slice is merged.
    async fn waves<P: Model + Target + Vcs + Files>(
        &self, provider: &P, progress: &mut Progress,
    ) -> Result<(), Error> {
        let plan = self.plan;
        let mut wave = 0;
        while progress.merged.len() < plan.slices.len() {
            let ready = plan.ready(&progress.merged);
            if ready.is_empty() {
                let left: Vec<String> = plan
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
            self.wave(provider, progress, wave, &ready).await?;
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(wave))]
    async fn wave<P: Model + Target + Vcs + Files>(
        &self, provider: &P, progress: &mut Progress, wave: usize, ready: &[&Slice],
    ) -> Result<(), Error> {
        // a slice an earlier run staged is merged already; the rest build
        let mut merged = Vec::with_capacity(ready.len());
        let mut pending = Vec::with_capacity(ready.len());
        for slice in ready.iter().copied() {
            if progress.staged.remove(&slice.id) {
                tracing::info!(slice = %slice.id, "slice staged by an earlier run; not built again");
                progress.merged.insert(slice.id);
                merged.push(slice.id);
            } else {
                pending.push(slice);
            }
        }
        let jobs = self.jobs.map_or(pending.len(), NonZeroUsize::get).min(pending.len());
        tracing::info!(
            slices = ready.len(),
            staged = merged.len(),
            jobs,
            head = %progress.head,
            "building wave"
        );

        // every slice still to build, built at once in a working copy of its
        // own, every outcome waited for; a buffer of none would never poll
        let head = progress.head.clone();
        let mut builds = Vec::with_capacity(pending.len());
        for slice in pending.iter().copied() {
            builds.push(self.build(provider, slice, &head, wave).map(move |built| (slice, built)));
        }
        let mut outcomes: Vec<(&Slice, Result<Built<'_>, Error>)> = if builds.is_empty() {
            Vec::new()
        } else {
            stream::iter(builds).buffer_unordered(jobs).collect().await
        };
        outcomes.sort_by_key(|(slice, _)| slice.id);

        // what built is merged in id order, whatever order the builds
        // finished in, before a failed build ends the run, so it stays for
        // the next run to resume from
        let mut failed = None;
        for (slice, outcome) in outcomes {
            match outcome {
                Ok(built) => match self.integrate(provider, progress, built, wave).await {
                    Ok(()) => merged.push(slice.id),
                    Err(error) => return Err(at_slice(error, slice, wave, &progress.built)),
                },
                Err(error) => {
                    failed.get_or_insert((slice, error));
                }
            }
        }
        if let Some((slice, error)) = failed {
            return Err(at_slice(error, slice, wave, &progress.built));
        }
        if merged.is_empty() {
            tracing::info!("nothing merged; the wave leaves the integrated head as it was");
            return Ok(());
        }

        // the integrated tree, verified by the adapter and labelled
        merged.sort_unstable();
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
        let input = SliceInput {
            id: slice.id.to_string(),
            name: slice.name.clone(),
            requirements: slice
                .requirements
                .iter()
                .filter_map(|id| self.spec.requirement(*id))
                .map(Into::into)
                .collect(),
            types: slice
                .types
                .iter()
                .filter_map(|key| {
                    self.design.signature(key).map(|signature| DesignType {
                        key: key.clone(),
                        signature: signature.to_owned(),
                    })
                })
                .collect(),
            brief: slice.brief.clone(),
            spec: self.spec.cut(&slice.requirements),
            design: self.rendered.to_owned(),
            plan: slice.to_string(),
            base: head.to_owned(),
        };
        tracing::info!(requirements = input.requirements.len(), wave, "building slice");

        // a build that fails upstream or runs out of time is put again in a
        // fresh copy, since the first may have written part of the tree
        let task = Task::Build(input);
        let mut attempt = 0;
        let (worktree, outcome) = loop {
            attempt += 1;
            let worktree = WorkingCopy::cut(provider, &self.repo.path(), &at, head).await?;
            match Target::run(provider, self.adapter, worktree.path(), &task).await {
                Ok(outcome) => break (worktree, outcome),
                Err(error) if attempt < RETRIES && is_retryable(&error) => {
                    tracing::warn!(
                        attempt,
                        description = %error.description(),
                        "build failed upstream or timed out; building again"
                    );
                    worktree.remove(provider).await?;
                }
                Err(error) => return Err(error),
            }
        };
        let (input, report) = match (&task, outcome) {
            (Task::Build(input), Outcome::Built(report)) => (input, report),
            (_, outcome) => {
                return Err(server_error!("`{}` answered {outcome} to {task}", self.adapter));
            }
        };

        let findings = report.findings(input);
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
            reported = report.written.len(),
            "slice built"
        );
        Ok(Built {
            slice,
            worktree,
            covered,
            uncovered,
        })
    }

    // Seals what the slice changed and merges it into the integrated head
    // under the adapter's rules, the paths they leave conflicted resolved
    // with the model and merged in their place. The slice's working copy is
    // removed once merged, and left for inspection when the slice fails.
    async fn integrate<P: Model + Vcs + Files>(
        &self, provider: &P, progress: &mut Progress, built: Built<'_>, wave: usize,
    ) -> Result<(), Error> {
        let slice = built.slice;

        // the copy still sits where it was cut
        let head = built.worktree.head(provider).await?;
        if head != progress.head {
            return Err(server_error!(
                "its working copy sits on `{head}`, not the `{}` it was cut at: the build sealed \
                 or moved commits of its own",
                progress.head
            ));
        }

        // what the build changed, as the copy holds it rather than as the
        // report lists it, held to the secret gate before any of it is sealed
        let pending = built.worktree.pending(provider).await?;
        let leaks = secrets::findings(provider, built.worktree.path(), &pending).await?;
        if !leaks.is_empty() {
            return Err(secret_detected("the build", &leaks));
        }
        let mut written: Vec<String> = pending.iter().map(|change| change.path.clone()).collect();
        written.sort_unstable();

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

        // the slice's commit merged under the rules; what they leave
        // conflicted is resolved with the model and merged in its place
        let (commit, resolved) = match built.worktree.commit(provider, &message).await? {
            None => (None, Vec::new()),
            Some(commit) => {
                let merged =
                    self.integration.merge(provider, &commit, &message, self.policy).await?;
                if let Some(commit) = merged.commit {
                    (Some(commit), Vec::new())
                } else {
                    let unresolved = UnresolvedMerge {
                        built: &built,
                        pending: &pending,
                        commit,
                        message: &message,
                        paths: merged.conflicts,
                    };
                    let (commit, resolved) = unresolved.resolve(self, provider, progress).await?;
                    (Some(commit), resolved)
                }
            }
        };
        built.worktree.remove(provider).await?;

        tracing::info!(
            slice = %slice.id,
            commit = commit.as_deref().unwrap_or("none"),
            resolved = resolved.len(),
            "slice merged"
        );
        progress.merged.insert(slice.id);
        progress.built.push(BuiltSlice {
            id: slice.id,
            name: slice.name.clone(),
            wave,
            covered: built.covered,
            uncovered: built.uncovered,
            written,
            resolved,
            commit,
        });
        Ok(())
    }

    // Has the adapter verify the integrated tree, seals what its checks and
    // repairs left behind, and labels the head the wave reached.
    async fn verify<P: Target + Vcs + Files>(
        &self, provider: &P, progress: &mut Progress, wave: usize, merged: &[SliceId],
    ) -> Result<(), Error> {
        // a verify that fails upstream or runs out of time is put again over
        // the same copy, since nothing of it is sealed until the verdict and
        // a repair it left stands as the shell would have left it
        let mut attempt = 0;
        let outcome = loop {
            attempt += 1;
            match Target::run(provider, self.adapter, self.integration.path(), &Task::Verify).await
            {
                Ok(outcome) => break outcome,
                Err(error) if attempt < RETRIES && is_retryable(&error) => {
                    tracing::warn!(
                        attempt,
                        description = %error.description(),
                        "verify failed upstream or timed out; verifying again"
                    );
                }
                Err(error) => return Err(error),
            }
        };
        let Outcome::Verified(verdict) = outcome else {
            return Err(server_error!("`{}` answered {outcome} to {}", self.adapter, Task::Verify));
        };
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

        // what the checks and their repairs left behind, held to the secret
        // gate and sealed as the wave's own commit
        let pending = self.integration.pending(provider).await?;
        if !pending.is_empty() {
            let leaks = secrets::findings(provider, self.integration.path(), &pending).await?;
            if !leaks.is_empty() {
                return Err(secret_detected("the verification", &leaks));
            }
            let ids: Vec<String> = merged.iter().map(ToString::to_string).collect();
            let message = format!(
                "Wave {wave} verified\n\nRevision: {}\nAdapter: {}\nSlices: {}",
                self.revision,
                self.reference,
                ids.join(", ")
            );
            let Some(commit) = self.integration.commit(provider, &message).await? else {
                return Err(server_error!(
                    "the verification left {} paths pending, yet sealing them made no commit",
                    pending.len()
                ));
            };
            let mut paths: Vec<String> = pending.into_iter().map(|change| change.path).collect();
            paths.sort_unstable();

            // a path a check regenerates under `ours`, a lockfile, is no repair
            let repaired = paths
                .iter()
                .any(|path| vcs::strategy_for(self.policy, path) != Some(Strategy::Ours));
            tracing::info!(changed = paths.len(), %commit, repaired, "verification changes sealed");
            if repaired {
                progress.repaired.push(wave);
            }
            progress.sealed.push(Seal { wave, commit, paths });
        }

        let head = self.integration.head(provider).await?;
        self.repo.label(provider, self.label, &head).await?;
        tracing::info!(merged = merged.len(), %head, "wave verified");
        progress.verified.push(Verified {
            wave,
            slices: merged.to_vec(),
            head: head.clone(),
        });
        progress.labelled = Some(head.clone());
        progress.head = head;
        Ok(())
    }
}

impl UnresolvedMerge<'_> {
    // Resolves every path with the model, seals the resolutions in the
    // slice's working copy over its commit, and merges the slice again with
    // each resolution taken whole: the merge commit, and the paths resolved.
    async fn resolve<P: Model + Vcs + Files>(
        self, run: &Run<'_>, provider: &P, progress: &Progress,
    ) -> Result<(String, Vec<String>), Error> {
        let slice = self.built.slice;
        let worktree = &self.built.worktree;
        tracing::info!(
            slice = %slice.id,
            conflicts = self.paths.join(", "),
            "the rules left conflicts; resolving them with the model"
        );

        // each path a text file on both sides, read before any turn is spent
        let mut conflicts = Vec::with_capacity(self.paths.len());
        for path in &self.paths {
            let integrated = self.side(provider, run.integration, path, Side::Integrated).await?;
            let slice = self.side(provider, worktree, path, Side::Slice).await?;
            conflicts.push(Conflict {
                path: path.clone(),
                integrated,
                slice,
            });
        }

        // the plan wave's other merged slices that wrote a conflicted path,
        // whose plan entries say what their side is for; staged slices are
        // merged already and sit outside `built`, and may sit after this one
        // in id order, so the wave comes from the plan rather than this run's
        // wave counter, which restarts on a resume
        let waves = run.plan.waves();
        let wave_slices = waves.iter().find(|wave| wave.contains(&slice.id)).unwrap_or(&[]);
        let merged: Vec<&Slice> = wave_slices
            .iter()
            .filter(|&&id| id != slice.id)
            .filter(|id| progress.merged.contains(id))
            .filter_map(|id| {
                let plan_slice = run.plan.slice(*id)?;
                let touches =
                    progress.built.iter().find(|built| built.id == *id).is_none_or(|built| {
                        built.written.iter().any(|path| self.paths.contains(path))
                    });
                touches.then_some(plan_slice)
            })
            .collect();
        let brief = MergeBrief {
            slice,
            merged,
            conflicts,
        };
        let resolutions = match brief.judge(provider).await {
            Ok(resolutions) => resolutions,
            Err(error @ Error::BadRequest { .. }) if !is_retryable(&error) => {
                return Err(self.unresolved(format_args!(
                    "the model's resolution was refused within the available rounds: {}",
                    findings_of(&error.description())
                )));
            }
            Err(error) => return Err(error),
        };

        // the resolutions written in the slice's copy, held to the secret gate,
        // and sealed over its commit; one that keeps the slice's own side
        // changes nothing, and the slice's commit stands
        for file in &resolutions {
            let path = beneath(&file.path)
                .map_err(|bad| server_error!("the resolution at `{}` {bad}", file.path))?;
            provider.write(worktree.path(), &path, file.content.as_bytes()).await?;
        }
        let pending = worktree.pending(provider).await?;
        let leaks = secrets::findings(provider, worktree.path(), &pending).await?;
        if !leaks.is_empty() {
            return Err(secret_detected("the merge's resolution", &leaks));
        }
        let mut resolved: Vec<String> = resolutions.into_iter().map(|file| file.path).collect();
        resolved.sort_unstable();
        let subject = format!("{} {} resolved at {}", slice.id, slice.name, resolved.join(", "));
        let commit = worktree.commit(provider, &subject).await?.unwrap_or(self.commit);

        // merged again, each resolution taken whole and the rules as before
        let policy: Vec<Rule> = resolved
            .iter()
            .map(|path| Rule {
                paths: format!("/{path}"),
                strategy: Strategy::Theirs,
            })
            .chain(run.policy.iter().cloned())
            .collect();
        let merged = run.integration.merge(provider, &commit, self.message, &policy).await?;
        match merged.commit {
            Some(commit) => Ok((commit, resolved)),
            None => Err(server_error!(
                "its merge still conflicts at {} once the model resolved {}",
                merged.conflicts.join(", "),
                resolved.join(", ")
            )),
        }
    }

    // One side of a conflicted path as text, or the refusal: a path the
    // engine never writes, one the slice removed, one `side` does not hold
    // as a regular file, or one that is not text is nothing the model can
    // resolve, so no turn is spent on it.
    async fn side<P: Files>(
        &self, provider: &P, copy: &WorkingCopy, path: &str, side: Side,
    ) -> Result<String, Error> {
        if let Err(bad) = beneath(path) {
            return Err(self.unresolved(format_args!("`{path}` {bad}")));
        }
        let removed = side == Side::Slice
            && self.pending.iter().any(|c| c.path == path && c.kind == ChangeKind::Deleted);
        if removed {
            return Err(self.unresolved(format_args!("the slice removed `{path}`")));
        }
        match kind_at(provider, copy.path(), path).await? {
            Some(EntryKind::File) => {}
            Some(EntryKind::Link) => {
                return Err(
                    self.unresolved(format_args!("a symbolic link stands at `{path}` in {side}"))
                );
            }
            Some(EntryKind::Directory) => {
                return Err(
                    self.unresolved(format_args!("a directory stands at `{path}` in {side}"))
                );
            }
            None => {
                return Err(self.unresolved(format_args!("{side} does not hold `{path}`")));
            }
        }
        String::from_utf8(provider.read(copy.path(), path).await?)
            .map_err(|_bytes| self.unresolved(format_args!("`{path}` is not text in {side}")))
    }

    fn unresolved(&self, why: impl Display) -> Error {
        Error::BadRequest {
            code: "merge-unresolved".into(),
            description: format!(
                "its merge conflicts at {}, and the conflict cannot be resolved: {why}",
                self.paths.join(", ")
            ),
        }
    }
}

// The findings a refused question's description carries, without the
// candidate they were found against, which here spells whole files; a
// description in another shape is kept whole.
fn findings_of(description: &str) -> &str {
    description.split_once("## Findings\n\n").map_or(description, |(_, findings)| {
        findings.split("\n\nProduce a ").next().unwrap_or(findings)
    })
}

// The two sides of a conflicted path.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Integrated,
    Slice,
}

impl Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Integrated => "the integrated tree",
            Self::Slice => "the slice",
        })
    }
}

// What stands at `path` beneath the working copy at `at`, walked a directory
// at a time so no link on the way is followed; `None` when nothing does, or
// something on the way is no directory.
async fn kind_at<P: Files>(provider: &P, at: &str, path: &str) -> Result<Option<EntryKind>, Error> {
    let segments: Vec<&str> = path.split('/').filter(|segment| !segment.is_empty()).collect();
    let Some((file, directories)) = segments.split_last() else {
        return Ok(None);
    };
    let mut directory = String::from(".");
    for segment in directories {
        let entries = provider.list(at, &directory).await?;
        match entries.iter().find(|entry| entry.name == *segment) {
            Some(entry) if entry.kind == EntryKind::Directory => {}
            _ => return Ok(None),
        }
        directory =
            if directory == "." { (*segment).to_owned() } else { format!("{directory}/{segment}") };
    }
    let entries = provider.list(at, &directory).await?;
    Ok(entries.into_iter().find(|entry| entry.name == *file).map(|entry| entry.kind))
}

// Has the adapter scaffold its layout into the tree at `at` and answers what
// it left pending, held to the secret gate. Not put again on a failure
// upstream: an init puts no turn, so a failure is the adapter's.
async fn initialise<P: Target + Vcs + Files>(
    provider: &P, adapter: &str, at: &str,
) -> Result<Vec<Change>, Error> {
    tracing::info!(tree = at, "initialising");
    let outcome = Target::run(provider, adapter, at, &Task::Init).await?;
    if outcome != Outcome::Initialised {
        return Err(server_error!("`{adapter}` answered {outcome} to {}", Task::Init));
    }
    let pending = vcs::pending(provider, at).await?;
    let leaks = secrets::findings(provider, at, &pending).await?;
    if !leaks.is_empty() {
        return Err(secret_detected("the initialisation", &leaks));
    }
    tracing::info!(scaffolded = pending.len(), "initialised");
    Ok(pending)
}

// The gate's refusal: what `wrote` left changed carries the credentials
// listed, and none of it is sealed.
fn secret_detected(wrote: &str, leaks: &[String]) -> Error {
    let listed: Vec<String> = leaks.iter().map(|leak| format!("- {leak}")).collect();
    Error::BadRequest {
        code: "secret-detected".into(),
        description: format!(
            "what {wrote} wrote carries a credential, so none of it is sealed:\n{}",
            listed.join("\n")
        ),
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
