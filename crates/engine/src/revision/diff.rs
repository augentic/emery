//! How one revision differs from the one it displaced.
//!
//! Each document's preamble, the requirements added, removed, or changed
//! (matched by id), and the design sections likewise (matched by kind). The
//! diff is typed equality over two revisions, never a comparison of their
//! projections. It is reported once, with the run that committed the incoming
//! revision, and stored nowhere.

use serde::Serialize;

use super::{Design, ReqId, Requirement, Revision, SectionKind, Spec};

/// The differences between a committed revision and the one it displaced.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diff {
    /// The id of the revision this run displaced.
    pub from: String,
    /// What changed in the specification.
    pub spec: SpecDiff,
    /// What changed in the design.
    pub design: DesignDiff,
}

impl Diff {
    /// Returns the differences between `outgoing`, which `from` names, and `incoming`.
    ///
    /// Preambles compare whole, requirements by id, and sections by kind —
    /// never by position.
    #[must_use]
    pub fn between(from: &str, outgoing: &Revision, incoming: &Revision) -> Self {
        Self {
            from: from.to_string(),
            spec: SpecDiff::between(&outgoing.spec, &incoming.spec),
            design: DesignDiff::between(&outgoing.design, &incoming.design),
        }
    }
}

/// The requirements that differ between two revisions.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecDiff {
    /// Whether the preamble changed.
    pub preamble: bool,
    /// Requirements present only in the incoming revision.
    pub added: Vec<Entry>,
    /// Requirements present only in the outgoing revision.
    pub removed: Vec<Entry>,
    /// Requirements present in both whose content changed.
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

/// One requirement named by a diff.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Entry {
    /// The requirement id.
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

/// One requirement whose content changed, and the fields that differ.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Changed {
    /// The requirement, named as the diff names every other.
    #[serde(flatten)]
    pub requirement: Entry,
    /// The differing fields, in declaration order.
    pub fields: Vec<&'static str>,
}

/// The design sections that differ between two revisions, by kind.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DesignDiff {
    /// Whether the preamble changed.
    pub preamble: bool,
    /// Sections present only in the incoming revision.
    pub added: Vec<SectionKind>,
    /// Sections present only in the outgoing revision.
    pub removed: Vec<SectionKind>,
    /// Sections present in both whose blocks changed.
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

// Names the fields, other than `id`, on which `before` and `after` differ.
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
