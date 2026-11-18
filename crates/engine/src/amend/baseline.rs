//! Lends the current revision's requirements to an amend run's grouping.
//!
//! A [`Baseline`] reads the base specification as contributors from the
//! reserved source [`BASELINE`], one per requirement, so the grouping
//! matches each change claim against the requirement it restates or
//! revises. The standing rank is one step below the lowest change source's,
//! so a change always wins over what it revises and the base decides
//! nothing but which requirement a claim lands on.

use emery_adapter::source::SourceKind;

use crate::Rank;
use crate::revision::{ReqId, Requirement, Scenario, Spec, Status};
use crate::specify::{BASELINE, Basis, Contributor, Extract};

// The base specification as the grouping and the change set read it.
pub struct Baseline {
    spec: Spec,
    rank: Rank,
}

impl Baseline {
    // The base `spec` ranked beneath every source of `extracts`.
    pub fn new(spec: &Spec, extracts: &[Extract]) -> Self {
        let lowest = extracts
            .iter()
            .map(|extract| extract.rank)
            .max()
            .unwrap_or_else(|| Rank::from(SourceKind::Behaviour));
        Self {
            spec: spec.clone(),
            rank: lowest.below(),
        }
    }

    // One contributor per requirement, its statement what the requirement
    // states as it stands (`Requirement::stated`): its body, or the
    // competing statements of a conflict, so the model sees one standing
    // requirement however many statements the base reads as its, with its
    // scenarios summarised underneath so evidenced thresholds and facets are
    // visible to the grouping. The kind is nominal, since no adapter
    // extracted the base; `index` is the position among the standing alone,
    // which the brief re-indexes; the citations are where the requirement
    // was read, for the brief's anchor hints.
    pub fn contributors(&self) -> Vec<Contributor> {
        self.spec
            .requirements
            .iter()
            .filter_map(|requirement| Some((requirement, requirement.stated()?)))
            .enumerate()
            .map(|(index, (requirement, statement))| {
                let notes = if requirement.status == Status::Conflict {
                    Vec::new()
                } else {
                    requirement.scenarios.iter().map(scenario_note).collect()
                };
                Contributor {
                    source: BASELINE.to_owned(),
                    kind: SourceKind::Documentation,
                    rank: self.rank,
                    id: requirement.subject.clone(),
                    path: None,
                    statement,
                    synopsis: None,
                    notes,
                    withdraws: false,
                    citations: requirement.sources.clone(),
                    index,
                }
            })
            .collect()
    }

    // The base's requirements, in id order.
    pub fn requirements(&self) -> &[Requirement] {
        &self.spec.requirements
    }

    // The subjects of the requirements the base holds covered, as criterion
    // ids, so a requirement the change leaves standing keeps its coverage.
    pub fn criteria(&self) -> Vec<String> {
        self.spec
            .requirements
            .iter()
            .filter(|requirement| requirement.covered)
            .map(|requirement| requirement.subject.clone())
            .collect()
    }

    // The base requirement `basis` matched against, when a standing
    // contributor joined its group.
    pub fn matched(&self, basis: &Basis) -> Option<&Requirement> {
        basis
            .classes
            .iter()
            .flatten()
            .find(|member| member.source == BASELINE)
            .and_then(|member| self.requirement(&member.id))
    }

    // The base requirement a standing contributor stands for, by the subject
    // it carries as its claim id.
    pub fn requirement(&self, subject: &str) -> Option<&Requirement> {
        self.spec.requirements.iter().find(|requirement| requirement.subject == subject)
    }

    // Whether the base requirement `id` cites `source` and no other source.
    pub fn cites_only(&self, id: ReqId, source: &str) -> bool {
        self.spec.requirement(id).is_some_and(|requirement| requirement.cites_only(source))
    }

    // Whether `basis` leaves `requirement`'s trigger unchanged. A change
    // silent on the trigger keeps the base's, and a trigger bound from an
    // already-cited source that the base already holds under the same stem is
    // sub-stem slug drift on a shared helper rather than a new surface.
    pub fn same_trigger(&self, requirement: &Requirement, basis: &Basis) -> bool {
        let Some(trigger) = &basis.trigger else {
            return true;
        };
        if requirement.trigger.as_ref() == Some(trigger) {
            return true;
        }
        requirement.trigger.is_none()
            && basis.agrees_with_baseline()
            && basis
                .contributors()
                .filter(|member| member.source != BASELINE)
                .all(|member| requirement.sources.iter().any(|cited| cited.source == member.source))
            && self.spec.requirements.iter().any(|other| {
                other.stem() == requirement.stem() && other.trigger.as_ref() == Some(trigger)
            })
    }

    // The ids an added requirement takes, from the first above every id the
    // base holds: an id the base once held is never reused, a retired one
    // included.
    pub fn free_ids(&self) -> impl Iterator<Item = ReqId> {
        ReqId::free_after(self.spec.requirements.iter().map(|requirement| requirement.id))
    }
}

fn scenario_note(scenario: &Scenario) -> String {
    let mut clauses = Vec::new();
    if !scenario.given.is_empty() {
        clauses.push(format!("given {}", scenario.given.join("; ")));
    }
    clauses.push(format!("when {} -> {}", scenario.when, scenario.then));
    if !scenario.and.is_empty() {
        clauses.push(format!("and {}", scenario.and.join("; ")));
    }
    format!("scenario `{}`: {}", scenario.name, clauses.join("; "))
}
