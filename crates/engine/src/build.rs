//! Builds the current plan through a target adapter into a labelled commit.
//!
//! [`build`] reads the revision from the target repository, has the adapter
//! initialise the tree when no slice of it is merged yet, builds ready
//! slices in waves across isolated working copies, merges their commits under
//! the adapter's rules (resolving conflicts with the model), verifies each
//! wave, and labels the verified head `emery/<revision>`. Interrupted runs
//! resume from the existing label and integration working copy.

pub(crate) mod merge;

use std::collections::BTreeSet;
use std::num::NonZeroUsize;

use emery_adapter::target::{DesignType, Outcome, Slice as SliceInput, Target, Task};
use emery_adapter::{BadPath, SPECIFICATION, beneath};
use futures::{FutureExt as _, StreamExt as _, stream};
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::vcs::{Change, ChangeKind, Rule, Strategy};
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, Vcs, server_error};
use serde::Serialize;

use self::merge::UnresolvedMerge;
use crate::adapter::{self, AdapterRef, Loaded};
use crate::revision::{Design, Plan, Slice, Spec};
pub use crate::revision::{ReqId, SliceId, Waves};
use crate::vcs::{BuildRepo, INTEGRATION, Integration, Message, WorkingCopy};
use crate::{EntryKind, Files, RETRIES, describe, is_retryable, secrets, store, vcs};

/// Builds every slice of the current plan through the target adapter `input` names.
///
/// The revision is read from the repository the target names, where
/// [`apply`](crate::apply::apply) sealed it under its label; the base is
/// that repository's head. The slices the label `emery/<revision>` already
/// holds are not built again, and neither are those the integration working
/// copy holds merged over the labelled head from a run that ended before
/// their wave was verified: they are verified in their wave instead. A
/// build in which no slice stands merged yet first has the adapter
/// initialise the tree — scaffold the layout's initial files it lacks, with
/// no model turn — and seals what it scaffolded as the `Initialise` commit
/// the slices build over, a `.gitignore` of the engine's own directory
/// beside it in a repository the engine created whose layout holds none.
/// The rest go in waves:
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
///   - with code `spec-not-generated`, when the repository holds no sealed
///     revision, naming a label that fell behind the head and a staged
///     candidate where either stands;
///   - with code `revision-not-found`, when the repository the target names
///     does not exist or has no default branch, or what the run already
///     holds under the target's name was cloned or created under another
///     origin.
/// - Returns [`Error::BadRequest`] when the run is refused:
///   - when the target's name is not kebab-case;
///   - with code `spec-outdated`, when the stored revision uses another
///     grammar;
///   - with code `spec-mismatch`, when `spec/` in the repository is not the
///     three documents of a revision, or does not match the revision its
///     label names;
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
///   - with code `spec-rewritten`, when a path the initialisation, a
///     slice's build, a conflict's resolution, or a wave's verification
///     changed lies under `spec/`, where the sealed specification lives,
///     each listed; nothing of the change set is sealed, and the copy stays
///     as it was written;
///   - with code `secret-detected`, when such a path carries a credential,
///     each listed with what it carries; nothing of the change set is
///     sealed, and the copy stays as it was written;
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

    // the repository the build lands in, which holds the revision it builds
    let repo = BuildRepo::settle(&input.name, input.repository.as_deref())?;
    let mut standing = store::current(provider, &repo).await?;
    let Some(current) = standing.current.take() else {
        return Err(store::not_generated(provider, &standing).await);
    };
    let revision_id = current.id;
    let revision = current.revision?;
    let candidate = store::staged_id(provider).await?.filter(|staged| *staged != revision_id);

    let Loaded {
        id: adapter,
        metadata,
    } = adapter::load_target(provider, &input.adapter, input.digest.as_ref()).await?;

    // the base, and what the label already holds over it
    let head = standing.head.as_deref().unwrap_or(&current.commit);
    let base = repo.base(provider, head).await?;
    let label = store::label(&revision_id);
    let labelled = repo.merged_under(provider, &label, &base, &revision_id).await?;
    let mut progress = Progress::start(&base, labelled);
    let Integration {
        copy: integration,
        head,
        staged,
    } = repo.cut_integration(provider, &progress.head, &revision_id).await?;
    progress.stage(head, staged);
    let initialised = progress.initialise(provider, &repo, &integration, &adapter).await?;

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
    /// builds into the repository [`apply`](crate::apply::apply) created
    /// under the name and keeps.
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
    /// `None` for a build resumed from a slice the label's chain names or
    /// one staged in the integration copy, and for a tree the
    /// initialisation added nothing to.
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

// The file a created repository's `Initialise` commit holds beside what the
// adapter's init scaffolded, when the layout holds no ignore file of its
// own, so that the engine's own directory is ignored wherever the result is
// checked out.
const IGNORE: (&str, &[u8]) = (".gitignore", b".emery/\n");

// What every wave of one build shares.
pub(crate) struct Run<'a> {
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
pub(crate) struct Progress {
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
        let (head, labelled, merged) = match labelled {
            Some((head, merged)) => (head.clone(), Some(head), merged),
            None => (base.to_owned(), None, BTreeSet::new()),
        };
        Self {
            resumed: merged.iter().copied().collect(),
            head,
            labelled,
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

    // A build no slice of the revision stands merged in, by the label's
    // chain or staged in the copy, is initialised with the adapter's layout,
    // what it scaffolded sealed in the integration copy as the commit the
    // slices build over; a repository the engine created is given a
    // `.gitignore` of the engine's own directory beside it where the layout
    // holds none, so a checkout of the result ignores `.emery/`. One
    // resumed from a slice was initialised by the run that merged it.
    async fn initialise<P: Target + Vcs + Files>(
        &mut self, provider: &P, repo: &BuildRepo, integration: &WorkingCopy, adapter: &str,
    ) -> Result<Option<Initialised>, Error> {
        if !self.merged.is_empty() || !self.staged.is_empty() {
            return Ok(None);
        }
        let mut changes = initialise(provider, adapter, integration.path()).await?;
        let (path, content) = IGNORE;
        if repo.is_created()
            && !changes.iter().any(|change| change.path == path)
            && kind_at(provider, integration.path(), path).await?.is_none()
        {
            provider.write(integration.path(), path, content).await?;
            changes.push(Change {
                path: path.to_owned(),
                kind: ChangeKind::Added,
            });
        }
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
pub(crate) struct Built<'a> {
    slice: &'a Slice,
    worktree: WorkingCopy,
    covered: Vec<ReqId>,
    uncovered: Vec<ReqId>,
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
        // report lists it, held to the seal gates before any of it is sealed
        let pending = built.worktree.pending(provider).await?;
        gated(provider, built.worktree.path(), "the build", &pending).await?;
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

        // what the checks and their repairs left behind, held to the seal
        // gates and sealed as the wave's own commit
        let pending = self.integration.pending(provider).await?;
        if !pending.is_empty() {
            gated(provider, self.integration.path(), "the verification", &pending).await?;
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
// it left pending, held to the seal gates. Not put again on a failure
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
    gated(provider, at, "the initialisation", &pending).await?;
    tracing::info!(scaffolded = pending.len(), "initialised");
    Ok(pending)
}

// Holds what `wrote` left pending at `at` to the two gates every seal of a
// build passes: no path under `spec/`, since the sealed specification is the
// store's alone to write and a commit carrying one would rewrite the revision
// its label names, then the secret gate. The specification guard reads
// nothing, so it runs first and a refusal reads no file.
async fn gated<P: Files>(
    provider: &P, at: &str, wrote: &str, pending: &[Change],
) -> Result<(), Error> {
    let rewritten: Vec<String> = pending
        .iter()
        .filter(|change| matches!(beneath(&change.path), Err(BadPath::Specification)))
        .map(|change| format!("- {}", change.path))
        .collect();
    if !rewritten.is_empty() {
        return Err(Error::BadRequest {
            code: "spec-rewritten".into(),
            description: format!(
                "what {wrote} wrote touches `{SPECIFICATION}/`, where the sealed specification \
                 lives, so none of it is sealed:\n{}",
                rewritten.join("\n")
            ),
        });
    }
    let leaks = secrets::findings(provider, at, pending).await?;
    if !leaks.is_empty() {
        return Err(secrets::refusal(wrote, &leaks));
    }
    Ok(())
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
