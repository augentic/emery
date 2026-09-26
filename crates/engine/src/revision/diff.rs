//! Describes changes between two specification revisions.
//!
//! Requirements are matched by identifier and design sections by kind.
//! Comparisons use typed revision data rather than rendered Markdown.

use serde::Serialize;

use super::{Design, ReqId, Requirement, Revision, SectionKind, Spec};

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
}

impl Diff {
    /// Returns the changes from `outgoing` to `incoming`.
    ///
    /// `from` must identify `outgoing`. Preambles are compared as complete
    /// values, requirements by identifier, and design sections by kind.
    #[must_use]
    pub fn between(from: &str, outgoing: &Revision, incoming: &Revision) -> Self {
        Self {
            from: from.to_string(),
            spec: SpecDiff::between(&outgoing.spec, &incoming.spec),
            design: DesignDiff::between(&outgoing.design, &incoming.design),
        }
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
    /// Requirements present only in the displaced revision.
    pub removed: Vec<Entry>,
    /// Requirements present in both revisions with differing content.
    pub changed: Vec<Changed>,
}

impl SpecDiff {
    // Matches requirements by id — the position each run numbers in source
    // order — so a requirement whose place moved reads as a change.
    fn between(outgoing: &Spec, incoming: &Spec) -> Self {
        let mut diff = Self {
            preamble: outgoing.preamble != incoming.preamble,
            ..Self::default()
        };
        for requirement in &incoming.requirements {
            match outgoing.requirement(requirement.id) {
                None => diff.added.push(Entry::from(requirement)),
                Some(before) => {
                    let fields = differences(before, requirement);
                    if !fields.is_empty() {
                        diff.changed.push(Changed {
                            requirement: Entry::from(requirement),
                            fields,
                        });
                    }
                }
            }
        }
        diff.removed.extend(
            outgoing
                .requirements
                .iter()
                .filter(|requirement| incoming.requirement(requirement.id).is_none())
                .map(Entry::from),
        );
        diff
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
}

impl From<&Requirement> for Entry {
    fn from(requirement: &Requirement) -> Self {
        Self {
            id: requirement.id,
            subject: requirement.subject.clone(),
        }
    }
}

/// A requirement changed between revisions.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Changed {
    /// The requirement's identifier and subject in the new revision.
    #[serde(flatten)]
    pub requirement: Entry,
    /// Names of the differing fields, in declaration order.
    pub fields: Vec<&'static str>,
}

/// Changes to the design portion of a revision.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
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
    fn between(outgoing: &Design, incoming: &Design) -> Self {
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

fn differences(before: &Requirement, after: &Requirement) -> Vec<&'static str> {
    [
        ("subject", before.subject != after.subject),
        ("status", before.status != after.status),
        ("covered", before.covered != after.covered),
        ("sources", before.sources != after.sources),
        ("body", before.body != after.body),
        ("losers", before.losers != after.losers),
        ("scenarios", before.scenarios != after.scenarios),
    ]
    .into_iter()
    .filter_map(|(name, differs)| differs.then_some(name))
    .collect()
}
