//! Settles the basis of a build: a change build against a built ancestor, or a whole build.
//!
//! `Basis::settle` walks the current revision's lineage for the nearest
//! ancestor whose label holds a complete verified build, computes the
//! `ChangeSet` against it, and asks the target adapter to widen the
//! minimum part scope through `Task::Scope` in a working copy cut at the
//! built head.

use std::collections::BTreeSet;
use std::num::NonZeroUsize;

use emery_adapter::target::{Outcome, Target, Task};
use omnia_sdk::{Error, Vcs, server_error};

use super::Schedule;
use super::change::{Change, ChangeSet, Whole, WholeReason};
use crate::Files;
use crate::revision::{Part, PartId, ReqId, Revision, SliceId};
use crate::store::{self, Current};
use crate::vcs::{BuildRepo, Labelled, MergedIds, SCOPE, WorkingCopy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Basis {
    Change { change: Box<ChangeSet>, scope: BTreeSet<PartId> },
    Whole(WholeReason),
}

impl Basis {
    // Walks `current`'s lineage in `repo` for the nearest complete built
    // ancestor and resolves the part scope with `adapter`.
    pub async fn settle<P: Target + Vcs + Files>(
        provider: &P, repo: &BuildRepo, adapter: Option<&str>, base: &str, current: &Revision,
        budget: NonZeroUsize, requested_whole: bool,
    ) -> Result<Self, Error> {
        if requested_whole {
            return Ok(Self::Whole(WholeReason::Requested));
        }
        if current.spec.parent.is_none() {
            return Ok(Self::Whole(WholeReason::NoParent));
        }

        let settler = Settler {
            provider,
            repo,
            adapter,
            base,
            budget,
        };
        let Some(ancestor) = settler.find_ancestor(current).await? else {
            return Ok(Self::Whole(WholeReason::ParentUnbuilt));
        };
        let change = ChangeSet::between(ancestor.id, ancestor.commit, &ancestor.revision, current);
        let scope = settler.scope(&change).await?;
        Ok(Self::Change {
            change: Box::new(change),
            scope,
        })
    }

    #[must_use]
    pub const fn change_set(&self) -> Option<&ChangeSet> {
        match self {
            Self::Change { change, .. } => Some(change),
            Self::Whole(_) => None,
        }
    }

    #[must_use]
    pub fn carried(&self) -> BTreeSet<ReqId> {
        match self {
            Self::Change { change, scope } => change.carried(scope),
            Self::Whole(_) => BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn removed_parts(&self) -> &[Part] {
        match self {
            Self::Change { change, .. } => change.removed_parts(),
            Self::Whole(_) => &[],
        }
    }

    #[must_use]
    pub fn outcome(&self, retiring: Vec<SliceId>) -> (Option<Change>, Option<Whole>) {
        match self {
            Self::Change { change, scope } => (Some(change.describe(scope, retiring)), None),
            Self::Whole(reason) => (None, Some(Whole::from(*reason))),
        }
    }
}

struct Settler<'a, P> {
    provider: &'a P,
    repo: &'a BuildRepo,
    adapter: Option<&'a str>,
    base: &'a str,
    budget: NonZeroUsize,
}

struct Ancestor {
    id: String,
    commit: String,
    revision: Revision,
    merged: MergedIds,
}

impl Ancestor {
    fn is_complete(
        &self, budget: NonZeroUsize, carried: &BTreeSet<ReqId>, retiring: &[Part],
    ) -> bool {
        let mut done = self.merged.clone();
        done.requirements.extend(carried.iter().copied());
        Schedule::new(&self.revision, budget, &self.merged, &[], carried, retiring)
            .unbuilt(&done)
            .next()
            .is_none()
    }
}

impl<P: Target + Vcs + Files> Settler<'_, P> {
    // Finds the nearest ancestor of `start` that holds slice commits on its
    // label and verifies that its build is complete.
    async fn find_ancestor(&self, start: &Revision) -> Result<Option<Ancestor>, Error> {
        let empty = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut chain: Vec<Ancestor> = Vec::new();
        let mut parent = start.spec.parent.as_deref();
        loop {
            let Some(ancestor) = self.next_built(parent, &mut visited).await? else {
                return Ok(None);
            };
            let complete = ancestor.is_complete(self.budget, &empty, &[]);
            chain.push(ancestor);
            if complete {
                break;
            }
            parent = chain.last().and_then(|last| last.revision.spec.parent.as_deref());
        }

        // a change-built ancestor's label holds only the slices it rebuilt;
        // walk forward from the whole-built root of the chain, recomputing each
        // hop's carried requirements to verify every hop is complete
        for window in chain.windows(2).rev() {
            let [current, earlier] = window else { continue };
            let change = ChangeSet::between(
                &earlier.id,
                &earlier.commit,
                &earlier.revision,
                &current.revision,
            );
            let scope = self.scope(&change).await?;
            let carried = change.carried(&scope);
            if !current.is_complete(self.budget, &carried, change.removed_parts()) {
                return Ok(None);
            }
        }

        Ok(chain.into_iter().next())
    }

    // Steps along `parent` past any revision that was sealed and never built
    // (`merged` empty) to the nearest ancestor whose label holds slice commits.
    async fn next_built(
        &self, parent: Option<&str>, visited: &mut BTreeSet<String>,
    ) -> Result<Option<Ancestor>, Error> {
        let mut next = parent.map(str::to_owned);
        while let Some(id) = next.take() {
            if !visited.insert(id.clone()) {
                return Ok(None);
            }
            let label = store::label(&id);
            let Some(Labelled { head: commit, merged }) =
                self.repo.merged_under(self.provider, &label, self.base, &id).await?
            else {
                return Ok(None);
            };
            let Some(current) =
                Current::read_at(self.provider, self.repo, &commit, Some(&id)).await?
            else {
                return Ok(None);
            };
            let revision = current.revision?;
            if merged.is_empty() {
                next.clone_from(&revision.spec.parent);
                continue;
            }
            return Ok(Some(Ancestor {
                id,
                commit,
                revision,
                merged,
            }));
        }
        Ok(None)
    }

    // Puts `Task::Scope` in a read-only working copy cut at the built commit
    // and unions the adapter's answer with the engine's minimum scope.
    async fn scope(&self, change: &ChangeSet) -> Result<BTreeSet<PartId>, Error> {
        let mut scoped = change.minimum();
        let Some(adapter) = self.adapter else {
            return Ok(scoped);
        };

        let copy =
            WorkingCopy::cut(self.provider, &self.repo.path(), SCOPE, change.commit()).await?;
        let task = Task::Scope(change.as_wit());
        let ran = async {
            let outcome = Target::run(self.provider, adapter, copy.path(), &task).await?;
            let pending = copy.pending(self.provider).await?;
            Ok::<_, Error>((outcome, pending))
        }
        .await;
        copy.remove(self.provider).await?;
        let (outcome, pending) = ran?;

        let (input, parts) = match (&task, outcome) {
            (Task::Scope(input), Outcome::Scoped(parts)) => (input, parts),
            (_, outcome) => return Err(server_error!("`{adapter}` answered {outcome} to {task}")),
        };
        if !pending.is_empty() {
            let paths: Vec<String> = pending.into_iter().map(|change| change.path).collect();
            return Err(server_error!(
                "`{adapter}` modified the scope working copy: {}",
                paths.join(", ")
            ));
        }
        let findings = input.findings(&parts);
        if !findings.is_empty() {
            return Err(server_error!(
                "`{adapter}` returned an invalid scope:\n{}",
                findings.join("\n")
            ));
        }
        for id in parts {
            let part: PartId = id.parse().map_err(|err| {
                server_error!("`{adapter}` returned invalid part id `{id}`: {err}")
            })?;
            scoped.insert(part);
        }
        Ok(scoped)
    }
}
