//! Computes the typed change between a built ancestor revision and the current revision.
//!
//! [`ChangeSet::between`] pairs requirements, design types, design sections,
//! and plan parts across one lineage using [`Diff::within`](crate::specify::Diff::within).
//! It derives the engine's minimum part scope, formats the [`TargetChange`]
//! record passed to [`Task::Scope`](emery_adapter::target::Task::Scope), and
//! builds the [`SliceChange`] carried by each slice of a change build.

use std::collections::{BTreeMap, BTreeSet};

use emery_adapter::target::{
    Change as TargetChange, Departure, PartReach as TargetPartReach,
    Requirement as TargetRequirement, SectionChange, SliceChange, Standing, TypeChange,
};
use serde::{Deserialize, Serialize};
use strum::VariantArray as _;

use crate::revision::{
    Design, Diff, Part, PartId, Plan, ReqId, Revision, SectionKind, SliceId, Spec,
};

/// What a change build rebuilt, carried, and retired against the built ancestor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Change {
    /// The built ancestor revision identifier.
    pub from: String,
    /// The commit the built ancestor's label stands at.
    pub commit: String,
    /// The parts of the current plan in scope to rebuild, in id order.
    pub rebuilt: Vec<PartReach>,
    /// The parts of the current plan carried from the built ancestor without
    /// rebuilding, in id order.
    pub carried: Vec<PartId>,
    /// The retiring slices cut for parts removed since the built ancestor,
    /// in slice-id order.
    pub retiring: Vec<SliceId>,
}

impl Change {
    /// Summarises `change` for the resolved part `scope` and `retiring` slices.
    ///
    /// Parts in `scope` whose minimum [`PartReach::reasons`] are empty carry
    /// [`Reason::Adapter`]. Parts outside `scope` are listed in
    /// [`carried`](Self::carried).
    #[must_use]
    pub fn of(change: &ChangeSet, scope: &BTreeSet<PartId>, retiring: Vec<SliceId>) -> Self {
        let mut rebuilt = Vec::new();
        let mut carried = Vec::new();
        for part in &change.parts {
            if scope.contains(&part.id) {
                let reasons = if part.reasons.is_empty() {
                    vec![Reason::Adapter]
                } else {
                    part.reasons.clone()
                };
                rebuilt.push(PartReach {
                    id: part.id,
                    name: part.name.clone(),
                    reasons,
                });
            } else {
                carried.push(part.id);
            }
        }
        Self {
            from: change.built.clone(),
            commit: change.commit.clone(),
            rebuilt,
            carried,
            retiring,
        }
    }
}

/// A plan part reached by a change build and why it is rebuilt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PartReach {
    /// The part identifier.
    pub id: PartId,
    /// The part's name.
    pub name: String,
    /// Why the change reaches the part, in declaration order.
    pub reasons: Vec<Reason>,
}

/// Why a change build reaches a part of the plan.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Reason {
    /// The part places an added or changed requirement.
    Requirements,
    /// A requirement the built part held left the part.
    Departed,
    /// A design type the part owns or uses changed signature.
    Types,
    /// The part's plan entry changed beyond added, departed, or changed types.
    Plan,
    /// The target adapter widened the scope to include the part.
    Adapter,
}

/// Why a build ran as a whole build rather than a change build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Whole {
    /// The reason every part of the plan was built from the base.
    pub reason: WholeReason,
}

impl Whole {
    /// Wraps `reason` as a whole-build summary.
    #[must_use]
    pub const fn new(reason: WholeReason) -> Self {
        Self { reason }
    }
}

impl From<WholeReason> for Whole {
    fn from(reason: WholeReason) -> Self {
        Self::new(reason)
    }
}

/// The reason a build ran as a whole build.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::AsRefStr,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum WholeReason {
    /// The caller asked for a whole build (`--whole`).
    Requested,
    /// The current revision has no parent in its lineage.
    NoParent,
    /// No ancestor in the lineage has a complete verified build.
    ParentUnbuilt,
}

/// The typed differences between a built ancestor revision and the current revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeSet {
    built: String,
    commit: String,
    parts: Vec<PartReach>,
    removed: Vec<Part>,
    deltas: BTreeMap<PartId, PartDelta>,
    standings: BTreeMap<ReqId, RequirementStanding>,
    retired: Vec<Departure>,
    types: Vec<TypeChange>,
    sections: Vec<SectionChange>,
    vanished: Vec<String>,
}

impl ChangeSet {
    /// Computes the typed change from `outgoing` (identified by `built` at `commit`) to `incoming`.
    #[must_use]
    pub fn between(
        built: impl Into<String>, commit: impl Into<String>, outgoing: &Revision,
        incoming: &Revision,
    ) -> Self {
        let built = built.into();
        let commit = commit.into();
        let diff = Diff::within(&built, outgoing, incoming);

        let standings = requirement_standings(&outgoing.spec, &incoming.spec, &diff);
        let retired = retired_requirements(&outgoing.spec, &diff);
        let vanished = outgoing.spec.stems().difference(&incoming.spec.stems()).cloned().collect();
        let types = type_changes(&outgoing.design, &incoming.design);
        let sections = section_changes(&outgoing.design, &incoming.design);

        // removed parts and their departures and types
        let mut deltas = BTreeMap::new();
        let removed: Vec<Part> = outgoing
            .plan
            .parts
            .iter()
            .filter(|part| incoming.plan.part(part.id).is_none())
            .cloned()
            .collect();
        for before in &removed {
            deltas.insert(
                before.id,
                PartDelta::removed(before, &outgoing.spec, &incoming.plan, &types),
            );
        }

        // current parts and their minimum reach reasons
        let mut parts = Vec::with_capacity(incoming.plan.parts.len());
        for part in &incoming.plan.parts {
            let before = outgoing.plan.part(part.id);
            let delta = PartDelta::current(before, part, &outgoing.spec, &incoming.plan, &types);
            let reasons = delta.reasons(before, part, &standings);
            parts.push(PartReach {
                id: part.id,
                name: part.name.clone(),
                reasons,
            });
            deltas.insert(part.id, delta);
        }

        Self {
            built,
            commit,
            parts,
            removed,
            deltas,
            standings,
            retired,
            types,
            sections,
            vanished,
        }
    }

    /// Returns the built ancestor revision identifier.
    #[must_use]
    pub fn built(&self) -> &str {
        &self.built
    }

    /// Returns the commit the built ancestor's label stands at.
    #[must_use]
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// Returns every part of the current plan with its minimum scope reasons, in id order.
    #[must_use]
    pub fn parts(&self) -> &[PartReach] {
        &self.parts
    }

    /// Returns the parts of the built plan that the current plan removed, in id order.
    #[must_use]
    pub fn removed_parts(&self) -> &[Part] {
        &self.removed
    }

    /// Returns the part ids the engine's minimum scope reaches from typed facts alone.
    #[must_use]
    pub fn minimum(&self) -> BTreeSet<PartId> {
        self.parts.iter().filter(|part| !part.reasons.is_empty()).map(|part| part.id).collect()
    }

    /// Returns the contract [`TargetChange`] record passed to [`Task::Scope`](emery_adapter::target::Task::Scope).
    #[must_use]
    pub fn as_wit(&self) -> TargetChange {
        TargetChange {
            built: self.built.clone(),
            commit: self.commit.clone(),
            parts: self
                .parts
                .iter()
                .map(|part| TargetPartReach {
                    id: part.id.to_string(),
                    name: part.name.clone(),
                    requirements: self
                        .deltas
                        .get(&part.id)
                        .into_iter()
                        .flat_map(|delta| &delta.requirements)
                        .map(ToString::to_string)
                        .collect(),
                    reasons: part.reasons.iter().map(ToString::to_string).collect(),
                })
                .collect(),
            retired: self.retired.clone(),
            types: self.types.clone(),
            sections: self.sections.clone(),
        }
    }

    /// Returns the requirements of every current plan part outside `scope`.
    #[must_use]
    pub fn carried(&self, scope: &BTreeSet<PartId>) -> BTreeSet<ReqId> {
        self.parts
            .iter()
            .filter(|part| !scope.contains(&part.id))
            .filter_map(|part| self.deltas.get(&part.id))
            .flat_map(|delta| delta.requirements.iter().copied())
            .collect()
    }

    /// Returns the [`SliceChange`] for a slice of `part` building `requirements`.
    ///
    /// `built_before` lists the requirement ids of `part` that earlier slices
    /// of this build already rebuilt.
    #[must_use]
    pub fn slice_change(
        &self, part: PartId, requirements: &[ReqId], built_before: &[ReqId],
    ) -> SliceChange {
        let standing: Vec<Standing> = requirements
            .iter()
            .map(|id| self.standings.get(id).map_or(Standing::Added, RequirementStanding::standing))
            .collect();
        let slice_ids: BTreeSet<ReqId> = requirements.iter().copied().collect();
        let prior: Vec<TargetRequirement> = slice_ids
            .into_iter()
            .filter_map(|id| self.standings.get(&id)?.prior().cloned())
            .collect();
        let built_before: BTreeSet<ReqId> = built_before.iter().copied().collect();
        let delta = self.deltas.get(&part);

        SliceChange {
            built: self.built.clone(),
            commit: self.commit.clone(),
            standing,
            prior,
            departed: delta.map_or_else(Vec::new, |delta| delta.departed.clone()),
            built_before: built_before.into_iter().map(|id| id.to_string()).collect(),
            vanished: self.vanished.clone(),
            types: delta.map_or_else(Vec::new, |delta| delta.types.clone()),
            sections: self.sections.clone(),
        }
    }

    /// Returns the build output [`Change`] summary for `scope` and `retiring` slices.
    #[must_use]
    pub fn describe(&self, scope: &BTreeSet<PartId>, retiring: Vec<SliceId>) -> Change {
        Change::of(self, scope, retiring)
    }
}

// A current requirement's standing against the built ancestor, carrying the
// built form when the requirement changed.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RequirementStanding {
    Added,
    Changed(TargetRequirement),
    Unchanged,
}

impl RequirementStanding {
    const fn standing(&self) -> Standing {
        match self {
            Self::Added => Standing::Added,
            Self::Changed(_) => Standing::Changed,
            Self::Unchanged => Standing::Unchanged,
        }
    }

    const fn is_touched(&self) -> bool {
        matches!(self, Self::Added | Self::Changed(_))
    }

    const fn prior(&self) -> Option<&TargetRequirement> {
        match self {
            Self::Changed(prior) => Some(prior),
            Self::Added | Self::Unchanged => None,
        }
    }
}

// The requirements, departures, and changed design types of one part.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct PartDelta {
    requirements: Vec<ReqId>,
    departed: Vec<Departure>,
    types: Vec<TypeChange>,
}

impl PartDelta {
    fn removed(before: &Part, outgoing: &Spec, incoming: &Plan, types: &[TypeChange]) -> Self {
        Self {
            requirements: Vec::new(),
            departed: departures(before, None, outgoing, incoming),
            types: types_for(Some(before), None, types),
        }
    }

    fn current(
        before: Option<&Part>, after: &Part, outgoing: &Spec, incoming: &Plan, types: &[TypeChange],
    ) -> Self {
        Self {
            requirements: after.requirements.clone(),
            departed: before.map_or_else(Vec::new, |before| {
                departures(before, Some(after), outgoing, incoming)
            }),
            types: types_for(before, Some(after), types),
        }
    }

    fn reasons(
        &self, before: Option<&Part>, after: &Part,
        standings: &BTreeMap<ReqId, RequirementStanding>,
    ) -> Vec<Reason> {
        let mut reasons = Vec::new();
        if self
            .requirements
            .iter()
            .any(|id| standings.get(id).is_some_and(RequirementStanding::is_touched))
        {
            reasons.push(Reason::Requirements);
        }
        if !self.departed.is_empty() {
            reasons.push(Reason::Departed);
        }
        if !self.types.is_empty() {
            reasons.push(Reason::Types);
        }
        if self.plan_changed(before, after, standings, reasons.is_empty()) {
            reasons.push(Reason::Plan);
        }
        reasons
    }

    fn plan_changed(
        &self, before: Option<&Part>, after: &Part,
        standings: &BTreeMap<ReqId, RequirementStanding>, otherwise_unreached: bool,
    ) -> bool {
        let Some(before) = before else {
            return otherwise_unreached;
        };
        before.name != after.name
            || before.depends_on != after.depends_on
            || before.brief != after.brief
            || after.requirements.iter().any(|id| {
                !before.requirements.contains(id)
                    && !standings.get(id).is_some_and(RequirementStanding::is_touched)
            })
            || self.types_reassigned(&before.types, &after.types)
            || self.types_reassigned(&before.uses, &after.uses)
    }

    fn types_reassigned(&self, before: &[String], after: &[String]) -> bool {
        before
            .iter()
            .filter(|key| !after.contains(key))
            .chain(after.iter().filter(|key| !before.contains(key)))
            .any(|key| !self.types.iter().any(|change| change.key == *key))
    }
}

fn requirement_standings(
    outgoing: &Spec, incoming: &Spec, diff: &Diff,
) -> BTreeMap<ReqId, RequirementStanding> {
    let added: BTreeSet<ReqId> = diff.spec.added.iter().map(|entry| entry.id).collect();
    let changed: BTreeSet<ReqId> =
        diff.spec.changed.iter().map(|changed| changed.entry.id).collect();
    incoming
        .requirements
        .iter()
        .map(|requirement| {
            let id = requirement.id;
            let standing = if added.contains(&id) {
                RequirementStanding::Added
            } else if changed.contains(&id)
                && let Some(before) = outgoing.requirement(id)
            {
                RequirementStanding::Changed(TargetRequirement::from(before))
            } else {
                RequirementStanding::Unchanged
            };
            (id, standing)
        })
        .collect()
}

fn retired_requirements(outgoing: &Spec, diff: &Diff) -> Vec<Departure> {
    let mut ids: Vec<ReqId> = diff.spec.retired.iter().map(|retired| retired.entry.id).collect();
    ids.sort_unstable();
    ids.into_iter()
        .filter_map(|id| outgoing.requirement(id))
        .map(|requirement| Departure {
            requirement: TargetRequirement::from(requirement),
            moved_to: None,
        })
        .collect()
}

fn type_changes(outgoing: &Design, incoming: &Design) -> Vec<TypeChange> {
    outgoing
        .type_keys()
        .union(&incoming.type_keys())
        .filter_map(|key| {
            let before = outgoing.signature(key).map(str::to_owned);
            let after = incoming.signature(key).map(str::to_owned);
            (before != after).then(|| TypeChange {
                key: key.clone(),
                before,
                after,
            })
        })
        .collect()
}

fn section_changes(outgoing: &Design, incoming: &Design) -> Vec<SectionChange> {
    SectionKind::VARIANTS
        .iter()
        .copied()
        .filter_map(|kind| {
            let before = outgoing.prose(kind);
            let after = incoming.prose(kind);
            (before != after).then(|| SectionChange {
                kind: kind.as_ref().to_owned(),
                before,
                after,
            })
        })
        .collect()
}

fn departures(
    before: &Part, after: Option<&Part>, outgoing: &Spec, incoming: &Plan,
) -> Vec<Departure> {
    let kept: BTreeSet<ReqId> =
        after.into_iter().flat_map(|part| part.requirements.iter().copied()).collect();
    let mut left: Vec<ReqId> =
        before.requirements.iter().copied().filter(|id| !kept.contains(id)).collect();
    left.sort_unstable();
    left.into_iter()
        .filter_map(|id| {
            let requirement = TargetRequirement::from(outgoing.requirement(id)?);
            let moved_to = incoming.part_for(id).map(|part| part.id.to_string());
            Some(Departure {
                requirement,
                moved_to,
            })
        })
        .collect()
}

fn types_for(
    before: Option<&Part>, after: Option<&Part>, changes: &[TypeChange],
) -> Vec<TypeChange> {
    changes
        .iter()
        .filter(|change| {
            before.is_some_and(|part| part.references_type(&change.key))
                || after.is_some_and(|part| part.references_type(&change.key))
        })
        .cloned()
        .collect()
}
