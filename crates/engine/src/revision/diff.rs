//! Describes changes between two specification revisions.
//!
//! Comparisons read typed revision data rather than rendered Markdown:
//!
//! - Requirements pair by identifier within one history, and by shared source
//!   anchors then identifier across histories.
//! - Design sections pair by kind.
//! - Plan slices pair by identifier.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{Design, Plan, ReqId, Requirement, Revision, SectionKind, Slice, SliceId, Spec};

/// Changes from a displaced revision to a newly committed revision.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diff {
    /// The identifier of the displaced revision.
    pub from: String,
    /// Changes to the behavioural specification.
    pub spec: SpecDiff,
    /// Changes to the rebuild design.
    pub design: DesignDiff,
    /// Changes to the build plan.
    pub plan: PlanDiff,
}

impl Diff {
    /// Returns the changes from `outgoing` to `incoming`.
    ///
    /// `from` must identify `outgoing`. When `incoming`'s specification names
    /// `from` as its `parent`, requirements pair by identifier alone and any
    /// `outgoing` requirement left out is [`retired`](SpecDiff::retired).
    /// Across histories they pair by stem and shared anchors before
    /// identifier, and any left out is [`removed`](SpecDiff::removed).
    #[must_use]
    pub fn between(from: &str, outgoing: &Revision, incoming: &Revision) -> Self {
        Self {
            from: from.to_string(),
            spec: SpecDiff::between(from, &outgoing.spec, &incoming.spec),
            design: DesignDiff::between(&outgoing.design, &incoming.design),
            plan: PlanDiff::between(&outgoing.plan, &incoming.plan),
        }
    }

    /// Returns whether the two revisions differ in nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.spec.is_empty() && self.design.is_empty() && self.plan.is_empty()
    }
}

/// Changes to the specification portion of a revision.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecDiff {
    /// Whether the preamble changed.
    pub preamble: bool,
    /// Requirements present only in the new revision.
    pub added: Vec<Entry>,
    /// Requirements present only in the displaced revision, where the new
    /// one was drafted from the sources alone.
    pub removed: Vec<Entry>,
    /// Requirements the new revision withdraws from the displaced one it
    /// amends, each with the statement withdrawn.
    pub retired: Vec<Retired>,
    /// Requirements present in both revisions with differing content.
    pub changed: Vec<Changed>,
}

impl SpecDiff {
    /// Returns whether the specifications differ in nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.preamble
            && self.added.is_empty()
            && self.removed.is_empty()
            && self.retired.is_empty()
            && self.changed.is_empty()
    }

    // A requirement paired under another id reads as a change of `id`, with
    // `was` naming the one it carried.
    fn between(from: &str, outgoing: &Spec, incoming: &Spec) -> Self {
        let pairing = Pairing::between(from, outgoing, incoming);
        Self {
            preamble: outgoing.preamble != incoming.preamble,
            added: pairing.added.into_iter().map(Entry::from).collect(),
            removed: pairing.removed.into_iter().map(Entry::from).collect(),
            retired: pairing.retired.into_iter().map(Retired::from).collect(),
            changed: pairing
                .paired
                .into_iter()
                .filter_map(|(before, after)| {
                    let fields = differences(before, after);
                    (!fields.is_empty()).then(|| Changed {
                        entry: Entry {
                            was: (before.id != after.id).then_some(before.id),
                            ..Entry::from(after)
                        },
                        fields,
                    })
                })
                .collect(),
        }
    }
}

struct Pairing<'a> {
    paired: Vec<(&'a Requirement, &'a Requirement)>,
    added: Vec<&'a Requirement>,
    removed: Vec<&'a Requirement>,
    retired: Vec<&'a Requirement>,
}

impl<'a> Pairing<'a> {
    // Within one history an amendment keeps each requirement's id and numbers
    // additions past the base, so shared anchors say nothing. Across
    // histories ids are positional in source order, so the pair sharing the
    // most anchors under one stem is taken first, ties broken in id order.
    fn between(from: &str, outgoing: &'a Spec, incoming: &'a Spec) -> Self {
        let amends = incoming.parent.as_deref() == Some(from);
        let anchored = if amends { Vec::new() } else { Self::anchored(outgoing, incoming) };
        let by_id = incoming
            .requirements
            .iter()
            .filter_map(|after| outgoing.requirement(after.id).map(|before| (before, after)));

        let mut taken = BTreeSet::new();
        let mut paired = BTreeMap::new();
        for (before, after) in anchored.into_iter().chain(by_id) {
            if !taken.contains(&before.id) && !paired.contains_key(&after.id) {
                taken.insert(before.id);
                paired.insert(after.id, (before, after));
            }
        }

        let unpaired = outgoing.requirements.iter().filter(|before| !taken.contains(&before.id));
        let (removed, retired) = if amends {
            (Vec::new(), unpaired.collect())
        } else {
            (unpaired.collect(), Vec::new())
        };

        Self {
            added: incoming
                .requirements
                .iter()
                .filter(|after| !paired.contains_key(&after.id))
                .collect(),
            removed,
            retired,
            paired: paired.into_values().collect(),
        }
    }

    fn anchored(outgoing: &'a Spec, incoming: &'a Spec) -> Vec<(&'a Requirement, &'a Requirement)> {
        let mut pairs: Vec<(usize, &Requirement, &Requirement)> = outgoing
            .requirements
            .iter()
            .flat_map(|before| incoming.requirements.iter().map(move |after| (before, after)))
            .filter(|(before, after)| before.stem() == after.stem())
            .map(|(before, after)| (before.shared_anchors(after), before, after))
            .filter(|&(shared, ..)| shared > 0)
            .collect();
        pairs.sort_by_key(|&(shared, before, after)| (Reverse(shared), before.id, after.id));
        pairs.into_iter().map(|(_, before, after)| (before, after)).collect()
    }
}

/// A requirement the new revision withdraws from the one it amends.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Retired {
    /// The requirement's identity in the displaced revision.
    #[serde(flatten)]
    pub entry: Entry,
    /// The statement withdrawn: the displaced revision's body paragraphs
    /// joined, or `None` where it held the requirement in conflict.
    pub statement: Option<String>,
}

impl From<&Requirement> for Retired {
    fn from(requirement: &Requirement) -> Self {
        Self {
            entry: Entry::from(requirement),
            statement: requirement.statement(),
        }
    }
}

/// A requirement identified in a revision diff.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Entry {
    /// The stable requirement identifier.
    pub id: ReqId,
    /// The requirement subject.
    pub subject: String,
    /// The identifier the requirement carried in the displaced revision, when
    /// its anchors matched it there under another one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub was: Option<ReqId>,
}

impl From<&Requirement> for Entry {
    fn from(requirement: &Requirement) -> Self {
        Self {
            id: requirement.id,
            subject: requirement.subject.clone(),
            was: None,
        }
    }
}

/// A record changed between revisions.
///
/// `E` identifies the record: a requirement [`Entry`] or a [`SliceEntry`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Changed<E = Entry> {
    /// The record's identity in the new revision.
    #[serde(flatten)]
    pub entry: E,
    /// Names of the differing fields, in declaration order.
    pub fields: Vec<&'static str>,
}

/// Changes to the design portion of a revision.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DesignDiff {
    /// Whether the preamble changed.
    pub preamble: bool,
    /// Sections present only in the new revision.
    pub added: Vec<SectionKind>,
    /// Sections present only in the displaced revision.
    pub removed: Vec<SectionKind>,
    /// Sections present in both revisions with differing blocks.
    pub changed: Vec<SectionKind>,
}

impl DesignDiff {
    /// Returns whether the designs differ in nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.preamble
            && self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
    }

    /// Returns the changes from `outgoing` to `incoming`, sections paired by kind.
    #[must_use]
    pub fn between(outgoing: &Design, incoming: &Design) -> Self {
        let mut diff = Self {
            preamble: outgoing.preamble != incoming.preamble,
            ..Self::default()
        };
        for section in &incoming.sections {
            match outgoing.section(section.kind) {
                None => diff.added.push(section.kind),
                Some(before) if before.blocks != section.blocks => diff.changed.push(section.kind),
                Some(_) => {}
            }
        }
        diff.removed.extend(
            outgoing
                .sections
                .iter()
                .filter(|section| incoming.section(section.kind).is_none())
                .map(|section| section.kind),
        );
        diff
    }
}

/// Changes to the plan portion of a revision.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlanDiff {
    /// Whether the preamble changed.
    pub preamble: bool,
    /// Slices present only in the new revision.
    pub added: Vec<SliceEntry>,
    /// Slices present only in the displaced revision.
    pub removed: Vec<SliceEntry>,
    /// Slices present in both revisions with differing content.
    pub changed: Vec<Changed<SliceEntry>>,
}

impl PlanDiff {
    /// Returns whether the plans differ in nothing.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.preamble
            && self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
    }

    /// Returns the changes from `outgoing` to `incoming`, slices paired by identifier.
    ///
    /// Slices are numbered by each slice's lowest requirement, so a merge
    /// that swallows a stem renumbers the slices after it as changes.
    #[must_use]
    pub fn between(outgoing: &Plan, incoming: &Plan) -> Self {
        let mut diff = Self {
            preamble: outgoing.preamble != incoming.preamble,
            ..Self::default()
        };
        for slice in &incoming.slices {
            match outgoing.slice(slice.id) {
                None => diff.added.push(SliceEntry::from(slice)),
                Some(before) => {
                    let fields = slice_differences(before, slice);
                    if !fields.is_empty() {
                        diff.changed.push(Changed {
                            entry: SliceEntry::from(slice),
                            fields,
                        });
                    }
                }
            }
        }
        diff.removed.extend(
            outgoing
                .slices
                .iter()
                .filter(|slice| incoming.slice(slice.id).is_none())
                .map(SliceEntry::from),
        );
        diff
    }
}

/// A slice identified in a revision diff.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SliceEntry {
    /// The stable slice identifier.
    pub id: SliceId,
    /// The slice name.
    pub name: String,
}

impl From<&Slice> for SliceEntry {
    fn from(slice: &Slice) -> Self {
        Self {
            id: slice.id,
            name: slice.name.clone(),
        }
    }
}

// Destructured whole, so a field added to the record is compared here or
// fails to compile.
fn differences(before: &Requirement, after: &Requirement) -> Vec<&'static str> {
    let Requirement {
        id,
        subject,
        status,
        covered,
        trigger,
        sources,
        body,
        losers,
        scenarios,
    } = before;
    [
        ("id", *id != after.id),
        ("subject", *subject != after.subject),
        ("status", *status != after.status),
        ("covered", *covered != after.covered),
        ("trigger", *trigger != after.trigger),
        ("sources", *sources != after.sources),
        ("body", *body != after.body),
        ("losers", *losers != after.losers),
        ("scenarios", *scenarios != after.scenarios),
    ]
    .into_iter()
    .filter_map(|(name, differs)| differs.then_some(name))
    .collect()
}

// The field name a changed slice's `fields` carries where its brief differs.
pub const BRIEF: &str = "brief";

fn slice_differences(before: &Slice, after: &Slice) -> Vec<&'static str> {
    let Slice {
        id: _,
        name,
        requirements,
        types,
        depends_on,
        brief,
    } = before;
    [
        ("name", *name != after.name),
        ("requirements", *requirements != after.requirements),
        ("types", *types != after.types),
        ("depends-on", *depends_on != after.depends_on),
        (BRIEF, *brief != after.brief),
    ]
    .into_iter()
    .filter_map(|(name, differs)| differs.then_some(name))
    .collect()
}
