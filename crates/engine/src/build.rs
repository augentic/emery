//! Builds the current plan through a target adapter, into a labelled commit.
//!
//! [`build`] reads the current revision, loads the one target adapter the run
//! names, settles the repository and the base — the default branch of the
//! repository the target names, cloned under the target's name; the root
//! commit of the one the engine lays under that name for an adapter that
//! builds into a fresh tree; or the project checkout's sealed head — and
//! reads what the label `emery/<revision>` already holds over it, so a run
//! resumes where the last verified wave left off. A base no label stands
//! over is held to the tree the adapter builds over: one that lays a whole
//! tree is refused a base whose root holds a directory. The slices still to
//! build go in waves: every slice
//! whose dependencies are merged is built at once, each in a working copy of
//! its own cut at the wave's head, with its plan entry, its cut of the
//! specification, and the whole design. What each changed is sealed as its
//! commit, once its copy is found still sitting where it was cut, and merged
//! into the integration working copy in id order under the adapter's merge
//! rules. The adapter verifies the integrated tree, repairing what its
//! checks find, the wave is labelled, and the next wave is cut from it. The
//! engine writes no state of its own: the labelled history is the output,
//! standing in the repository the build landed in.
//!
//! A slice whose build fails upstream is built once more in a fresh copy,
//! and a wave whose verification fails upstream is verified once more over
//! the same one. A slice whose merge conflicts is left for the next wave and built again
//! over the merged head; its [`ATTEMPTS`]th conflict ends the run. A wave the
//! adapter does not verify ends the run with the label where the last
//! verified wave left it. Either way the integration working copy is left
//! for inspection and removed by the next run.
//!
//! Nothing is sealed unread: every path a slice's build or a wave's
//! verification left changed is held to the secret gate first, and a
//! credential among them ends the run with the copy left as it was written.

mod secrets;

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;

use emery_adapter::target::{Base, Slice as SliceInput, Target};
use emery_adapter::{SKIP_DIRS, is_kebab};
use futures::{StreamExt as _, TryFutureExt as _, TryStreamExt as _, stream};
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::vcs::{Rule, Strategy};
use omnia_sdk::{BlobStore, Error, Plugins, StateStore, Vcs, bad_request, server_error};
use serde::Serialize;

use crate::adapter::{self, AdapterRef, Loaded};
pub use crate::revision::{ReqId, SliceId, Waves};
use crate::revision::{Slice, Spec};
use crate::vcs::{INTEGRATION, Message, Repo, WorkingCopy};
use crate::{EntryKind, Files, store, vcs};

/// How many times a slice is built before a conflict at its merge ends the run.
pub const ATTEMPTS: usize = 3;

// How many times a slice's build, each in a fresh working copy, or a
// wave's verification, over the same copy, is put to the adapter when it
// fails upstream.
const RETRIES: usize = 2;

/// Builds every slice of the current plan through the target adapter `input` names.
///
/// The slices the label `emery/<revision>` already holds are not built
/// again. The rest go in waves: every slice whose dependencies are merged is
/// built at once, up to `jobs` concurrently, in a working copy cut at the
/// wave's head; each report is held to its slice by
/// [`Report::findings`](emery_adapter::target::Report::findings), what each
/// copy changed sealed as its commit and merged in id order under the
/// adapter's merge rules, and the wave verified by the adapter and labelled,
/// what its checks changed sealed first. A build that fails upstream is put
/// once more in a fresh copy, a verification once more over the same one. A slice whose merge conflicts is built again in
/// the next wave, over the merged head; the first failure ends the run, and
/// what was merged before it stays committed in the integration working
/// copy, the verified waves labelled.
///
/// # Errors
///
/// - Returns [`Error::NotFound`]:
///   - with code `spec-not-generated`, when no revision has been committed;
///   - with code `revision-not-found`, when the repository the target names
///     does not exist or has no default branch, or what the run already
///     holds under the target's name was cloned or laid under another
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
///   - with code `repository-required`, when the target names no
///     repository, the adapter builds over any base ([`Base::Any`]), and
///     the project is none;
///   - with code `base-not-sealed`, when the project checkout holds pending
///     changes outside `.emery/`, listed, or no commit;
///   - with code `base-not-fresh`, when the adapter builds into a fresh
///     tree ([`Base::Fresh`]) and the root of a base no label stands over
///     holds a directory outside `.emery/` and `.git/`, each listed; such
///     an adapter named with no repository builds into the one the engine
///     lays, whose root holds a `.gitignore` alone;
///   - with code `slice-conflict`, when a slice's merge conflicts at paths
///     no rule resolves on its [`ATTEMPTS`]th build, the paths listed;
///   - with code `verify-failed`, when the adapter does not verify a wave,
///     each failing check listed;
///   - with code `secret-detected`, when a path a slice's build or a wave's
///     verification changed carries a credential, each listed with what it
///     carries; nothing of the change set is sealed, and the copy stays as
///     it was written;
///   - a slice the adapter refuses, or answers no acceptable report for.
/// - Returns [`Error::ServerError`] when a report breaks the report gate, a
///   verdict breaks [`Verdict::findings`](emery_adapter::target::Verdict::findings),
///   a slice's working copy no longer sits on the head it was cut at, a
///   verification's changes seal as no commit, the slices left to build
///   wait on one another, or storage or version control fails.
/// - Returns [`Error::BadGateway`] when the adapter, its acquisition, the
///   model, or the repository's remote fails upstream; a slice's build, or
///   a wave's verification, is put twice before it does.
///
/// A failure at a slice keeps the adapter's class and code; its description
/// names the slice, its wave, and the slices merged before it. A failure at a
/// wave's verification names the wave, the slices merged in it, and where
/// the label stands.
pub async fn build<P: Target + StateStore + BlobStore + Plugins + Vcs + Files>(
    input: BuildInput, context: Context<P>,
) -> Result<BuildOutput, Error> {
    let provider = context.provider();
    let name = &input.name;
    if !is_kebab(name) {
        return Err(bad_request!("target `{name}` is not a kebab-case name"));
    }

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

    // the repository the build lands in: the one the target names, the
    // engine's own for an adapter that lays a whole tree, else the project's;
    // then the base, and what the label already holds over it
    let repo = match (input.repository, metadata.base) {
        (Some(url), _) => Repo::Target {
            name: input.name,
            url: Some(url),
        },
        (None, Base::Fresh) => Repo::Target {
            name: input.name,
            url: None,
        },
        (None, Base::Any) => Repo::Project,
    };
    let base = repo.base(provider).await?;
    let label = format!("emery/{revision_id}");
    let labelled = repo.merged_under(provider, &label, &base, &revision_id).await?;
    let mut progress = Progress::start(&base, labelled);
    let integration = WorkingCopy::cut(provider, &repo.path(), INTEGRATION, &progress.head).await?;

    // a fresh build is held to the base the adapter builds over; a resumed
    // one was admitted by the run that set the label
    if progress.labelled.is_none() && metadata.base == Base::Fresh {
        require_fresh(provider, &integration, &base, &adapter).await?;
    }

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

    integration.remove(provider).await?;

    Ok(BuildOutput {
        revision: revision_id,
        waves,
        base,
        repository: repo.path(),
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
    /// build clones or lays sits at [`vcs::target_path`] of it.
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
    /// The URL of the repository the plan is built into.
    ///
    /// The build starts from its default branch as the remote holds it.
    /// `None` builds into the project's own repository, from the sealed
    /// commit its checkout sits on, or, through an adapter that builds into
    /// a fresh tree, into the one the engine lays under the name.
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
    /// The plan's slices grouped into the sets ready to build at once.
    ///
    /// The plan's own projection: what the waves would be were every slice
    /// built from the base. A conflicted slice moves to a later wave of the
    /// run, which each [`BuiltSlice::wave`] records.
    pub waves: Waves,
    /// The commit the build started from.
    pub base: String,
    /// The repository the label stands in, as a path beneath the project
    /// root: `.` for the project's own, or [`vcs::target_path`] of the
    /// target's name for the one it names or the engine laid.
    pub repository: String,
    /// The slices the label already held, not built again, in id order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resumed: Vec<SliceId>,
    /// Every slice this run built, in build order, as its build reported it.
    pub slices: Vec<BuiltSlice>,
    /// The integrated head each wave was verified and labelled at, in wave order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub verified: Vec<String>,
    /// What each wave's verification left in the integrated tree, sealed as
    /// `Wave <k> verified` before the head was labelled, in wave order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sealed: Vec<Seal>,
    /// The waves whose verification repaired the integrated tree, from one:
    /// those whose seal carries a path the adapter's `ours` rules do not
    /// cover, since what a check regenerates under one, a lockfile, is no
    /// repair.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub repaired: Vec<usize>,
    /// The integrated commit the label points at.
    pub head: String,
    /// The label set on the head, `emery/<revision>`.
    pub label: String,
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
    /// The merge commit that brought what the slice wrote into the
    /// integrated head; `None` when it changed nothing.
    pub commit: Option<String>,
    /// The paths an earlier build of the slice conflicted at, before it was
    /// built again over the merged head.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
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
            built: Vec::new(),
            tries: BTreeMap::new(),
            conflicts: BTreeMap::new(),
            verified: Vec::new(),
            sealed: Vec::new(),
            repaired: Vec::new(),
        }
    }
}

// One slice built in its working copy, not yet merged.
struct Built<'a> {
    slice: &'a Slice,
    worktree: WorkingCopy,
    covered: Vec<ReqId>,
    uncovered: Vec<ReqId>,
}

enum Integrated {
    Merged(Option<String>),
    Conflicted(Vec<String>),
}

impl Run<'_> {
    #[tracing::instrument(skip_all, fields(wave))]
    async fn wave<P: Target + Vcs + Files>(
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

        // a build that fails upstream is put again in a fresh copy, since the
        // first may have written part of the tree before it failed
        let mut attempt = 0;
        let (worktree, report) = loop {
            attempt += 1;
            let worktree = WorkingCopy::cut(provider, &self.repo.path(), &at, head).await?;
            match Target::build(provider, self.adapter, &input, worktree.path()).await {
                Ok(report) => break (worktree, report),
                Err(Error::BadGateway { description, .. }) if attempt < RETRIES => {
                    tracing::warn!(attempt, %description, "build failed upstream; building again");
                    worktree.remove(provider).await?;
                }
                Err(error) => return Err(error),
            }
        };

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

    // Seals what the slice changed and merges it into the integrated head;
    // `true` when it merged, `false` when a conflict leaves it for the next
    // wave. The slice's working copy is removed either way, and left for
    // inspection when the slice fails.
    async fn integrate<P: Vcs + Files>(
        &self, provider: &P, progress: &mut Progress, built: Built<'_>, wave: usize,
    ) -> Result<bool, Error> {
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
        let mut written: Vec<String> = pending.into_iter().map(|change| change.path).collect();
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
                    written,
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

    // Has the adapter verify the integrated tree, seals what its checks and
    // repairs left behind, and labels the head the wave reached.
    async fn verify<P: Target + Vcs + Files>(
        &self, provider: &P, progress: &mut Progress, wave: usize, merged: &[SliceId],
    ) -> Result<(), Error> {
        // a verify that fails upstream is put again over the same copy, since
        // nothing of it is sealed until the verdict and a repair it left
        // stands as the shell would have left it
        let mut attempt = 0;
        let verdict = loop {
            attempt += 1;
            match Target::verify(provider, self.adapter, self.integration.path()).await {
                Ok(verdict) => break verdict,
                Err(Error::BadGateway { description, .. }) if attempt < RETRIES => {
                    tracing::warn!(attempt, %description, "verify failed upstream; verifying again");
                }
                Err(error) => return Err(error),
            }
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
        progress.verified.push(head.clone());
        progress.labelled = Some(head.clone());
        progress.head = head;
        Ok(())
    }
}

// A fresh base holds files alone at its root: a directory outside the
// engine's own refuses the build before any slice is dispatched, since the
// adapter lays a whole tree and would rewrite what stands there.
async fn require_fresh<P: Files>(
    provider: &P, integration: &WorkingCopy, base: &str, adapter: &str,
) -> Result<(), Error> {
    let mut directories: Vec<String> = provider
        .list(integration.path(), ".")
        .await?
        .into_iter()
        .filter(|entry| entry.kind == EntryKind::Directory)
        .filter(|entry| !SKIP_DIRS.contains(&entry.name.as_str()))
        .map(|entry| entry.name)
        .collect();
    if directories.is_empty() {
        return Ok(());
    }
    directories.sort_unstable();
    Err(Error::BadRequest {
        code: "base-not-fresh".into(),
        description: format!(
            "`{adapter}` builds into a fresh tree, and the base `{base}` holds directories: {}",
            directories.join(", ")
        ),
    })
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
