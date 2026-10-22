//! Lends the current revision's requirements to an amend run's grouping.
//!
//! A [`Standing`] reads the base specification as contributors from the
//! reserved source [`BASELINE`], one per statement a requirement holds, so
//! the grouping matches each change claim against the requirement it
//! restates or revises. The standing rank is one step below the lowest
//! change source's, so a change always wins over what it revises and the
//! base decides nothing but which requirement a claim lands on.

use emery_adapter::source::SourceKind;

use crate::Rank;
use crate::revision::{ReqId, Requirement, Spec, Status};
use crate::specify::{BASELINE, Basis, Contributor, Extract};

// The base specification as the grouping and the change set read it.
pub struct Standing<'a> {
    spec: &'a Spec,
    rank: Rank,
}

impl<'a> Standing<'a> {
    // The base `spec` ranked beneath every source of `extracts`.
    pub fn new(spec: &'a Spec, extracts: &[Extract]) -> Self {
        let lowest = extracts
            .iter()
            .map(|extract| extract.rank)
            .max()
            .unwrap_or_else(|| Rank::from(SourceKind::Behaviour));
        Self {
            spec,
            rank: lowest.below(),
        }
    }

    // One contributor per statement a requirement holds: its body for a
    // requirement with one, each competing statement for a conflict, so the
    // model sees every statement the base reads as the requirement's. The
    // kind is nominal, since no adapter extracted the base; `index` is the
    // position among the standing alone, which the brief re-indexes.
    pub fn contributors(&self) -> Vec<Contributor<'a>> {
        let mut contributors = Vec::with_capacity(self.spec.requirements.len());
        for requirement in &self.spec.requirements {
            for statement in statements(requirement) {
                contributors.push(Contributor {
                    source: BASELINE,
                    kind: SourceKind::Documentation,
                    rank: self.rank,
                    id: &requirement.subject,
                    path: None,
                    statement,
                    synopsis: None,
                    index: contributors.len(),
                });
            }
        }
        contributors
    }

    // The subjects of the requirements the base holds covered, as criterion
    // ids, so a requirement the change leaves standing keeps its coverage.
    pub fn criteria(&self) -> Vec<&'a str> {
        self.spec
            .requirements
            .iter()
            .filter(|requirement| requirement.covered)
            .map(|requirement| requirement.subject.as_str())
            .collect()
    }

    // The base requirement `basis` matched against, when a standing
    // contributor joined its group.
    pub fn matched(&self, basis: &Basis<'_>) -> Option<&'a Requirement> {
        basis
            .classes
            .iter()
            .flatten()
            .find(|member| member.source == BASELINE)
            .and_then(|member| self.requirement(member.id))
    }

    // The base requirement a standing contributor stands for, by the subject
    // it carries as its claim id.
    pub fn requirement(&self, subject: &str) -> Option<&'a Requirement> {
        self.spec.requirements.iter().find(|requirement| requirement.subject == subject)
    }

    // The ids an added requirement takes, from the first above every id the
    // base holds: an id the base once held is never reused.
    pub fn free_ids(&self) -> impl Iterator<Item = ReqId> {
        let first = self
            .spec
            .requirements
            .iter()
            .map(|requirement| requirement.id)
            .max()
            .map_or(Some(ReqId::FIRST), ReqId::next);
        std::iter::successors(first, |&id| id.next())
    }
}

fn statements(requirement: &Requirement) -> Vec<String> {
    match requirement.status {
        Status::Conflict => {
            requirement.losers.iter().map(|loser| loser.statement.clone()).collect()
        }
        Status::Agreed | Status::Unknown | Status::Divergence => {
            requirement.statement().into_iter().collect()
        }
    }
}
