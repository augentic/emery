//! Lends the current revision's requirements to an amend run's grouping.
//!
//! A [`Baseline`] reads the base specification as contributors from the
//! reserved source [`BASELINE`], one per requirement, so the grouping
//! matches each change claim against the requirement it restates or
//! revises. The standing rank is one step below the lowest change source's,
//! so a change always wins over what it revises and the base decides
//! nothing but which requirement a claim lands on.

use std::collections::BTreeSet;

use emery_adapter::source::{ClaimKind, SourceKind};

use crate::Rank;
use crate::revision::{self, Cited, ReqId, Requirement, Scenario, Spec, Status};
use crate::specify::{BASELINE, Basis, Contributor, Detail, Extract, trigger_note};

const START_STEM: &str = "start";

// The base specification as the grouping and the change set read it.
pub struct Baseline {
    spec: Spec,
    rank: Rank,
    whole_sources: BTreeSet<String>,
}

impl Baseline {
    // The base `spec` ranked beneath every source of `extracts`.
    pub fn new(spec: &Spec, extracts: &[Extract]) -> Self {
        let lowest = extracts
            .iter()
            .map(|extract| extract.rank)
            .max()
            .unwrap_or_else(|| Rank::from(SourceKind::Behaviour));
        let whole_sources = extracts
            .iter()
            .filter(|extract| extract.is_whole_behaviour())
            .map(|extract| extract.source.clone())
            .collect();
        Self {
            spec: spec.clone(),
            rank: lowest.below(),
            whole_sources,
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
        let include_whole_context = !self.whole_sources.is_empty();
        self.spec
            .requirements
            .iter()
            .filter_map(|requirement| Some((requirement, requirement.stated()?)))
            .enumerate()
            .map(|(index, (requirement, statement))| Contributor {
                source: BASELINE.to_owned(),
                kind: SourceKind::Documentation,
                rank: self.rank,
                id: requirement.subject.clone(),
                path: None,
                statement,
                synopsis: None,
                notes: standing_notes(requirement, include_whole_context),
                withdraws: false,
                trigger: requirement.trigger.clone(),
                citations: requirement.sources.clone(),
                index,
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
            .find(|member| member.is_baseline())
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

    // Whether `id` may be kept unchanged by `evidenced` alone: a standing
    // requirement cited only to whole-scope behaviour sources must keep any
    // route or message contract it carries and hold a detail claim in at least
    // one file its citations name.
    pub fn admits_evidenced(&self, id: ReqId, extracts: &[Extract]) -> bool {
        let Some(requirement) = self.spec.requirement(id) else {
            return false;
        };
        let cites_only_whole = !requirement.sources.is_empty()
            && requirement.sources.iter().all(|cited| self.whole_sources.contains(&cited.source));
        if !cites_only_whole {
            return true;
        }
        if let Some(trigger) = &requirement.trigger {
            let has_contract = extracts.iter().any(|extract| {
                self.whole_sources.contains(&extract.source)
                    && requirement.cites(&extract.source)
                    && extract.evidence.claims.iter().any(|claim| {
                        claim.kind == ClaimKind::Contract
                            && claim.trigger().is_some_and(|parsed| parsed.as_ref() == Ok(trigger))
                    })
            });
            if !has_contract {
                return false;
            }
        }
        extracts.iter().any(|extract| {
            requirement.cites(&extract.source)
                && extract
                    .evidence
                    .claims
                    .iter()
                    .filter_map(|claim| Detail::from_claim(&extract.source, claim))
                    .any(|detail| requirement.cites_same_file(&detail.cited))
        })
    }

    // Whether `basis` is whole-scope re-extraction noise on an untouched
    // `start` bootstrap file: an untriggered singleton `start.*` claim from an
    // already-cited whole-scope source whose cited file already holds standing
    // `start.*` requirements, none of which changed or retired and with no new
    // `start` contract.
    pub fn is_untouched_bootstrap(
        &self, basis: &Basis, unchanged: &BTreeSet<ReqId>, extracts: &[Extract],
    ) -> bool {
        if basis.stem() != START_STEM || basis.trigger.is_some() || basis.statement().is_none() {
            return false;
        }
        let Some(member) = basis.sole_change_contributor() else {
            return false;
        };
        if member.path.is_none()
            || !self.whole_sources.contains(&member.source)
            || !self.spec.requirements.iter().any(|requirement| requirement.cites(&member.source))
        {
            return false;
        }
        let cited = Cited::from(member);
        let has_new_start_contract = extracts
            .iter()
            .filter(|extract| extract.source == member.source)
            .flat_map(|extract| &extract.evidence.claims)
            .filter(|claim| claim.kind == ClaimKind::Contract)
            .any(|claim| {
                let in_start =
                    claim.id.as_deref().is_some_and(|id| revision::stem(id) == START_STEM)
                        || cited.same_file(&Cited::from_claim(&member.source, claim));
                in_start
                    && claim
                        .trigger()
                        .and_then(Result::ok)
                        .is_some_and(|trigger| !self.spec.has_trigger(&trigger))
            });
        if has_new_start_contract {
            return false;
        }
        let mut standing = self
            .spec
            .requirements
            .iter()
            .filter(|requirement| {
                requirement.stem() == START_STEM && requirement.cites_same_file(&cited)
            })
            .map(|requirement| requirement.id)
            .peekable();
        standing.peek().is_some() && standing.all(|id| unchanged.contains(&id))
    }

    // Whether `basis` agrees with `requirement` under its standing stem.
    //
    // Co-grouped claims from two seams of a source (`start` and `health`)
    // preserve the standing stem whenever the winning class holds a claim
    // under it, or when every non-`BASELINE` winner comes from an
    // already-cited whole-scope source in a file where `requirement` was
    // already read.
    pub fn agrees(&self, requirement: &Requirement, basis: &Basis) -> bool {
        if requirement.status == Status::Conflict || !basis.agrees_with_baseline() {
            return false;
        }
        if basis.winning_class_has_stem(requirement.stem()) {
            return true;
        }
        basis.winning_changes().all(|member| {
            self.whole_sources.contains(&member.source)
                && requirement.cites_same_file(&Cited::from(member))
        })
    }

    // Whether `basis` leaves `requirement`'s trigger unchanged. A change
    // silent on the trigger keeps the base's, as does an untriggered
    // standing requirement whose already-cited sources agree with the base
    // and only pick up an existing trigger or a secondary co-grouped stem's
    // contract on whole-scope re-extraction.
    pub fn same_trigger(&self, requirement: &Requirement, basis: &Basis) -> bool {
        let Some(trigger) = &basis.trigger else {
            return true;
        };
        if requirement.trigger.as_ref() == Some(trigger) {
            return true;
        }
        if requirement.trigger.is_some()
            || !basis.agrees_with_baseline()
            || !basis.change_contributors().all(|member| requirement.cites(&member.source))
        {
            return false;
        }
        if self.spec.has_trigger_in_stem(requirement.stem(), trigger) {
            return true;
        }
        let all_whole =
            basis.change_contributors().all(|member| self.whole_sources.contains(&member.source));
        all_whole
            && (!basis.has_contract_under(requirement.stem()) || self.spec.has_trigger(trigger))
    }

    // The ids an added requirement takes, from the first above every id the
    // base holds: an id the base once held is never reused, a retired one
    // included.
    pub fn free_ids(&self) -> impl Iterator<Item = ReqId> {
        ReqId::free_after(self.spec.requirements.iter().map(|requirement| requirement.id))
    }
}

fn standing_notes(requirement: &Requirement, include_whole_context: bool) -> Vec<String> {
    if requirement.status == Status::Conflict {
        return Vec::new();
    }
    let mut notes: Vec<String> = requirement.scenarios.iter().map(scenario_note).collect();
    if include_whole_context {
        if let Some(trigger) = &requirement.trigger {
            notes.push(trigger_note(trigger));
        }
        for path in requirement.sources.iter().filter_map(|cited| cited.path.as_deref()) {
            notes.push(format!("read at `{path}`"));
        }
    }
    notes
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
