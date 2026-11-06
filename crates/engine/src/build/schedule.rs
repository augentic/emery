//! Cuts the parts of a plan into budget-sized build slices.
//!
//! `Schedule::new` cuts each part into one or more serial slices under the
//! run's requirement budget: remaining requirements are grouped by sub-stem,
//! groups sharing a trigger are merged, untriggered groups pack before
//! triggered ones, and each part's slices build in sequence while independent
//! parts build in parallel waves.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroUsize;

use emery_adapter::target::Target;
use omnia_sdk::plugins::Digest;
use omnia_sdk::{Error, Plugins};
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

use crate::adapter::{self, AdapterRef};
use crate::revision::{
    Part, PartId, Plan, ReqId, Requirement, Revision, SliceId, Spec, Waves, ranks,
};
use crate::specify::Floors;
use crate::vcs::StagedSlice;

/// The fallback requirement budget when neither `--budget`, `[target] budget`, nor the adapter's metadata sets one.
pub const DEFAULT_BUDGET: NonZeroUsize = NonZeroUsize::new(12).expect("positive budget");

/// Where a build's requirement budget came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AsRefStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum BudgetSource {
    /// Set by `--budget` or `EMERY_BUDGET`.
    Flag,
    /// Set by `[target] budget` in `emery.toml`.
    Config,
    /// Declared by the target adapter's metadata.
    Adapter,
    /// The engine fallback, [`DEFAULT_BUDGET`].
    Default,
}

impl BudgetSource {
    /// Resolves the requirement budget from the caller's choice and the adapter's default.
    #[must_use]
    pub fn resolve(
        budget: Option<NonZeroUsize>, source: Option<Self>, metadata: Option<u32>,
    ) -> (NonZeroUsize, Self) {
        if let Some(budget) = budget {
            return (budget, source.unwrap_or(Self::Flag));
        }
        if let Some(budget) =
            metadata.and_then(|n| usize::try_from(n).ok()).and_then(NonZeroUsize::new)
        {
            return (budget, Self::Adapter);
        }
        (DEFAULT_BUDGET, Self::Default)
    }

    // Resolves the requirement budget, loading `adapter`'s metadata only when
    // `budget` is `None`.
    pub(crate) async fn resolve_with<P: Target + Plugins>(
        provider: &P, budget: Option<NonZeroUsize>, source: Option<Self>,
        adapter: Option<&AdapterRef>, digest: Option<&Digest>,
    ) -> Result<(NonZeroUsize, Self), Error> {
        let metadata = match (budget, adapter) {
            (None, Some(adapter)) => {
                adapter::load_target(provider, adapter, digest).await?.metadata.budget
            }
            _ => None,
        };
        Ok(Self::resolve(budget, source, metadata))
    }
}

/// One slice of a build schedule, cut from one part of the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScheduledSlice {
    /// The slice identifier, numbered in schedule order.
    pub id: SliceId,
    /// The part this slice was cut from.
    pub part: PartId,
    /// The part's name.
    pub name: String,
    /// The 1-based position of this slice within its part's cut (`1..=count`).
    pub index: u32,
    /// How many slices the part was cut into.
    pub count: u32,
    /// The requirements this slice builds, in id order.
    pub requirements: Vec<ReqId>,
    /// The design type keys this slice lays: the part's owned types on its
    /// first slice when nothing of the part was built yet, empty otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<String>,
    /// Whether this slice's requirements exceed the budget because a single
    /// sub-stem or trigger group holds more requirements than the budget.
    pub over_budget: bool,
}

impl ScheduledSlice {
    // Whether every requirement of this slice is in `done`.
    fn is_done(&self, done: &BTreeSet<ReqId>) -> bool {
        self.requirements.iter().all(|id| done.contains(id))
    }
}

// A plan's parts cut into budget-sized slices for one build.
#[derive(Debug, Clone)]
pub struct Schedule<'a> {
    plan: &'a Plan,
    slices: Vec<ScheduledSlice>,
    resumed: Vec<SliceId>,
}

impl<'a> Schedule<'a> {
    // Cuts `plan` into slices of at most `budget` requirements over `spec`.
    //
    // `merged` are the requirements already committed on the label chain, and
    // `staged` are the slice commits staged in the integration working copy
    // by an earlier run. A slice whose requirements are all in `merged` or
    // `staged` is recorded in `resumed` and not built again; a partially
    // built part has its remaining requirements cut into fresh slices.
    #[must_use]
    pub fn new(
        plan: &'a Plan, spec: &'a Spec, budget: NonZeroUsize, merged: &BTreeSet<ReqId>,
        staged: &[StagedSlice],
    ) -> Self {
        let floors = Floors::from_spec(spec);
        let cap = budget.get();
        let done: BTreeSet<ReqId> = merged
            .iter()
            .copied()
            .chain(staged.iter().flat_map(|slice| slice.requirements.iter().copied()))
            .collect();

        let mut slices = Vec::new();
        let mut resumed: BTreeSet<SliceId> = staged.iter().map(|slice| slice.id).collect();
        let mut ids = SliceId::sequence();

        for part in &plan.parts {
            let cuts = cut_part(part, spec, &floors, cap, &done, staged);
            let count = u32::try_from(cuts.len()).unwrap_or(u32::MAX);
            let untouched = part.requirements.iter().all(|id| !done.contains(id));
            for ((position, cut), id) in cuts.into_iter().enumerate().zip(&mut ids) {
                if cut.resumed {
                    resumed.insert(id);
                }
                let index = u32::try_from(position).map_or(u32::MAX, |n| n.saturating_add(1));
                let types =
                    if position == 0 && untouched { part.types.clone() } else { Vec::new() };
                slices.push(ScheduledSlice {
                    id,
                    part: part.id,
                    name: part.name.clone(),
                    index,
                    count,
                    requirements: cut.requirements,
                    types,
                    over_budget: cut.over_budget,
                });
            }
        }

        Self {
            plan,
            slices,
            resumed: resumed.into_iter().collect(),
        }
    }

    // Cuts `revision`'s plan into slices of at most `budget` requirements,
    // with nothing resumed.
    #[must_use]
    pub fn for_revision(revision: &'a Revision, budget: NonZeroUsize) -> Self {
        Self::new(&revision.plan, &revision.spec, budget, &BTreeSet::new(), &[])
    }

    #[must_use]
    pub fn slices(&self) -> &[ScheduledSlice] {
        &self.slices
    }

    #[must_use]
    pub fn into_slices(self) -> Vec<ScheduledSlice> {
        self.slices
    }

    #[must_use]
    pub fn resumed(&self) -> &[SliceId] {
        &self.resumed
    }

    pub fn over_budget(&self) -> impl Iterator<Item = &ScheduledSlice> {
        self.slices.iter().filter(|slice| slice.over_budget)
    }

    // The unbuilt slices ready to build once `done` requirements are merged:
    // the next unbuilt slice of every part whose in-plan dependencies are all
    // in `done`.
    #[must_use]
    pub fn ready(&self, done: &BTreeSet<ReqId>) -> Vec<&ScheduledSlice> {
        let part_done = |id: PartId| {
            self.plan
                .part(id)
                .is_none_or(|part| part.requirements.iter().all(|req| done.contains(req)))
        };
        let mut ready = Vec::new();
        for part in &self.plan.parts {
            if !self.plan.dependencies(part).all(part_done) {
                continue;
            }
            if let Some(next) =
                self.slices.iter().filter(|slice| slice.part == part.id).find(|s| !s.is_done(done))
            {
                ready.push(next);
            }
        }
        ready
    }

    pub fn unbuilt<'b>(
        &'b self, done: &'b BTreeSet<ReqId>,
    ) -> impl Iterator<Item = &'b ScheduledSlice> {
        self.slices.iter().filter(move |slice| !slice.is_done(done))
    }

    #[must_use]
    pub const fn plan(&self) -> &'a Plan {
        self.plan
    }

    #[must_use]
    pub fn part(&self, id: PartId) -> Option<&'a Part> {
        self.plan.part(id)
    }

    // Projects the schedule's slices into build waves from the base: slices
    // within one part depend serially on the slice before them, and a part's
    // first slice depends on the last slice of each dependency part.
    #[must_use]
    pub fn waves(&self) -> Waves<SliceId> {
        let last_of: BTreeMap<PartId, SliceId> =
            self.slices.iter().map(|slice| (slice.part, slice.id)).collect();
        let mut pending: BTreeMap<SliceId, BTreeSet<SliceId>> = BTreeMap::new();
        for part in &self.plan.parts {
            let mut prev: Option<SliceId> = None;
            for slice in self.slices.iter().filter(|slice| slice.part == part.id) {
                let mut deps = BTreeSet::new();
                if let Some(prev) = prev {
                    deps.insert(prev);
                } else {
                    for dep in self.plan.dependencies(part) {
                        if let Some(&last) = last_of.get(&dep) {
                            deps.insert(last);
                        }
                    }
                }
                pending.insert(slice.id, deps);
                prev = Some(slice.id);
            }
        }
        let (mut waves, cyclic) = ranks(pending);
        if !cyclic.is_empty() {
            waves.push(cyclic);
        }
        Waves(waves)
    }
}

struct Cut {
    requirements: Vec<ReqId>,
    over_budget: bool,
    resumed: bool,
}

struct Group {
    requirements: Vec<ReqId>,
    triggered: bool,
}

// Cuts `part` into slices. When every slice of the full cut is either wholly in
// `done` or wholly outside it, the full cut is kept so slice numbering and
// counts stay stable across resumes at the same budget. When `done` splits one
// of the full cut's slices (as when `--budget` changed mid-build), the
// already-built requirements stand as one resumed slice and the remaining
// requirements are packed fresh.
fn cut_part(
    part: &Part, spec: &Spec, floors: &Floors<'_>, cap: usize, done: &BTreeSet<ReqId>,
    staged: &[StagedSlice],
) -> Vec<Cut> {
    let mut full = pack_requirements(&part.requirements, spec, floors, cap);
    let aligned = full.iter().all(|cut| {
        cut.requirements.iter().all(|id| done.contains(id))
            || cut.requirements.iter().all(|id| !done.contains(id))
    });
    if aligned {
        for cut in &mut full {
            cut.resumed = cut.requirements.iter().all(|id| done.contains(id));
        }
        return full;
    }

    // budget changed mid-part: keep what is already built and pack the rest
    let (built, remaining): (Vec<ReqId>, Vec<ReqId>) =
        part.requirements.iter().copied().partition(|id| done.contains(id));
    let mut cuts = Vec::new();
    if !built.is_empty() {
        let over_budget =
            built.len() > cap && !staged.iter().any(|slice| slice.requirements == built);
        cuts.push(Cut {
            requirements: built,
            over_budget,
            resumed: true,
        });
    }
    cuts.extend(pack_requirements(&remaining, spec, floors, cap));
    cuts
}

fn pack_requirements(
    requirements: &[ReqId], spec: &Spec, floors: &Floors<'_>, cap: usize,
) -> Vec<Cut> {
    if requirements.is_empty() {
        return Vec::new();
    }
    if requirements.len() <= cap {
        return vec![Cut {
            requirements: requirements.to_vec(),
            over_budget: false,
            resumed: false,
        }];
    }

    // group by sub-stem in first-occurrence order
    let mut substems: Vec<(&str, Vec<ReqId>)> = Vec::new();
    for &id in requirements {
        let label = spec.requirement(id).map_or("", Requirement::substem);
        match substems.iter_mut().find(|(known, _)| *known == label) {
            Some((_, ids)) => ids.push(id),
            None => substems.push((label, vec![id])),
        }
    }

    // merge sub-stems whose requirements share a trigger handler
    let mut groups: Vec<Group> = Vec::new();
    for (_, ids) in substems {
        let triggered =
            ids.iter().any(|&id| spec.requirement(id).is_some_and(|req| req.trigger.is_some()));
        let held = groups.iter_mut().find(|group| {
            group.requirements.iter().any(|&a| ids.iter().any(|&b| floors.shares_trigger(a, b)))
        });
        if let Some(group) = held {
            group.requirements.extend(ids);
            group.triggered |= triggered;
        } else {
            groups.push(Group {
                requirements: ids,
                triggered,
            });
        }
    }

    // untriggered groups (stages and helpers) pack before triggered groups (entries)
    let ordered = groups
        .iter()
        .filter(|group| !group.triggered)
        .chain(groups.iter().filter(|group| group.triggered));

    let mut cuts: Vec<Cut> = Vec::new();
    for group in ordered {
        let ids = &group.requirements;
        if let Some(current) = cuts.last_mut()
            && !current.over_budget
            && current.requirements.len() + ids.len() <= cap
        {
            current.requirements.extend(ids.iter().copied());
        } else {
            cuts.push(Cut {
                requirements: ids.clone(),
                over_budget: ids.len() > cap,
                resumed: false,
            });
        }
    }
    for cut in &mut cuts {
        cut.requirements.sort_unstable();
    }
    cuts
}
