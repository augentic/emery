//! Derives what a change did to each requirement of the base.
//!
//! A [`ChangeSet`] reads the grouping's bases against the standing
//! requirements: a group holding a standing requirement keeps its id and is
//! changed, retired, unchanged, or in conflict by what the change sources
//! say beside it, and a group of change claims alone is added under the next
//! free id, or in conflict. Nothing here is the model's: the classes are
//! code's over the statements and triggers the grouping settled, and a
//! `retire` the grouping matched to no standing requirement is refused. The
//! bases the amended specification drafts from are kept beside the changes,
//! each under the id its requirement takes. A change that states no
//! requirement is grouped against nothing and leaves every standing
//! requirement as it is. A behaviour source read whole then retires what it
//! falls silent on under the stems it covers.

use std::collections::BTreeSet;

use emery_adapter::source::{ClaimKind, SourceKind};
use omnia_sdk::{Error, server_error};

use crate::amend::standing::Standing;
use crate::revision::{self, Change, ChangeClass, Cited};
use crate::specify::{BASELINE, Basis, Contributor, Extract, Scope};

pub struct ChangeSet<'a> {
    // One per requirement of the amended specification and per requirement
    // retired from it, in id order.
    pub changes: Vec<Change>,
    // The bases of the requirements drafted anew, each under the id and
    // subject its change carries, in id order.
    pub drafted: Vec<Basis<'a>>,
}

impl<'a> ChangeSet<'a> {
    // Whether any claim of `extracts` states or withdraws a requirement, so
    // the change is matched against the standing requirements. A change of
    // other claims alone, a `decision` or a `type`, informs the design and
    // nothing of the specification.
    pub fn states(extracts: &[Extract]) -> bool {
        extracts
            .iter()
            .flat_map(|extract| &extract.evidence.claims)
            .any(|claim| matches!(claim.kind, ClaimKind::Requirement | ClaimKind::Retire))
    }

    // The change set of a change that states no requirement: every standing
    // requirement as it is, with no source behind it, and nothing drafted.
    pub fn unchanged(standing: &Standing<'a>) -> Self {
        let changes = standing
            .requirements()
            .map(|requirement| {
                let before = requirement.statement();
                Change {
                    id: requirement.id,
                    subject: requirement.subject.clone(),
                    class: ChangeClass::Unchanged,
                    before: before.clone(),
                    after: before,
                    sources: Vec::new(),
                    scope: None,
                }
            })
            .collect();
        Self {
            changes,
            drafted: Vec::new(),
        }
    }

    // Retires every standing requirement a behaviour source read whole falls
    // silent on: under a stem the source's requirement claims cover, a
    // requirement no claim of the change touched is withdrawn by the
    // source's silence and names the source. A prose source's scope is
    // passed over, since a brief states a change and never its subject
    // whole, and the first source in declaration order to cover a stem is
    // the one named.
    pub fn retire_by_scope(&mut self, extracts: &[Extract]) {
        for extract in extracts.iter().filter(|extract| extract.scope == Scope::Whole) {
            if extract.kind != SourceKind::Behaviour {
                tracing::info!(
                    source = %extract.source,
                    kind = %extract.kind,
                    "the source is read whole, which only a behaviour source's scope says; \
                     its silence retires nothing"
                );
                continue;
            }
            let covered: BTreeSet<&str> = extract
                .evidence
                .claims
                .iter()
                .filter(|claim| claim.kind == ClaimKind::Requirement)
                .filter_map(|claim| claim.id.as_deref())
                .map(revision::stem)
                .collect();
            for change in &mut self.changes {
                let silent = change.class == ChangeClass::Unchanged && change.sources.is_empty();
                if !silent || !covered.contains(change.stem()) {
                    continue;
                }
                tracing::info!(
                    source = %extract.source,
                    requirement = %change.id,
                    subject = %change.subject,
                    "retired by scope: the source read whole states nothing of it"
                );
                change.class = ChangeClass::Retired;
                change.after = None;
                change.scope = Some(extract.source.clone());
            }
        }
    }

    // A standing contributor in a basis names the base requirement the group
    // revises; its absence is a requirement the base lacks. A `retire` is
    // ranked as a statement is, so a withdrawal and a revision at one rank
    // are a conflict and the higher-ranked of the two decides.
    pub fn derive(standing: &Standing<'a>, bases: Vec<Basis<'a>>) -> Result<Self, Error> {
        let unmatched: Vec<&Contributor<'_>> = bases
            .iter()
            .filter(|basis| standing.matched(basis).is_none())
            .flat_map(Basis::withdrawals)
            .collect();
        if !unmatched.is_empty() {
            return Err(retire_unmatched(&unmatched));
        }

        let mut free = standing.free_ids();
        let mut changes = Vec::with_capacity(bases.len());
        let mut drafted = Vec::new();
        for mut basis in bases {
            let sources: Vec<Cited> = basis
                .contributors()
                .filter(|member| member.source != BASELINE)
                .map(Cited::from)
                .collect();
            let matched = standing.matched(&basis);

            let change = if let Some(requirement) = matched {
                let before = requirement.statement();
                // a requirement no claim touched stands as it is, its own
                // conflict included; a change silent on the trigger leaves
                // the base's standing; the subject is the change's, since
                // the base is ranked below it, so a requirement restated
                // under another stem moves with it
                let (class, after) = if sources.is_empty() {
                    (ChangeClass::Unchanged, before.clone())
                } else {
                    let after = basis.statement().map(str::to_owned);
                    let same_trigger =
                        basis.trigger.is_none() || basis.trigger == requirement.trigger;
                    let same_subject = basis.subject == requirement.subject;
                    let class = if basis.withdraws() {
                        ChangeClass::Retired
                    } else if after.is_none() {
                        ChangeClass::Conflict
                    } else if after == before && same_trigger && same_subject {
                        ChangeClass::Unchanged
                    } else {
                        ChangeClass::Changed
                    };
                    (class, after)
                };
                let subject = match class {
                    ChangeClass::Retired => requirement.subject.as_str(),
                    _ => basis.subject,
                };
                Change {
                    id: requirement.id,
                    subject: subject.to_owned(),
                    class,
                    before,
                    after,
                    sources,
                    scope: None,
                }
            } else {
                let id = free
                    .next()
                    .ok_or_else(|| server_error!("the requirement ids are exhausted"))?;
                let after = basis.statement().map(str::to_owned);
                Change {
                    id,
                    subject: basis.subject.to_owned(),
                    class: if after.is_some() { ChangeClass::Added } else { ChangeClass::Conflict },
                    before: None,
                    after,
                    sources,
                    scope: None,
                }
            };

            if change.class.is_drafted() {
                let trigger = basis
                    .trigger
                    .take()
                    .or_else(|| matched.and_then(|requirement| requirement.trigger.clone()));
                let amended = basis.amended(change.id, trigger).ok_or_else(|| {
                    server_error!(
                        "requirement {} is drafted anew with no claim to draft from",
                        change.id
                    )
                })?;
                drafted.push(amended);
            }
            changes.push(change);
        }
        changes.sort_by_key(|change| change.id);
        drafted.sort_by_key(|basis| basis.id);
        Ok(Self { changes, drafted })
    }
}

// The `retire-unmatched` refusal: `retires` withdraw behaviours no
// requirement of the current revision states, so there is nothing for the
// amendment to retire and a silent pass would lose the withdrawal.
fn retire_unmatched(retires: &[&Contributor<'_>]) -> Error {
    let named = retires
        .iter()
        .map(|retire| format!("`{}` from `{}`", retire.id, retire.source))
        .collect::<Vec<_>>()
        .join(", ");
    let what = if retires.len() > 1 { "withdraw behaviours" } else { "withdraws a behaviour" };
    Error::BadRequest {
        code: "retire-unmatched".into(),
        description: format!("retire {named} {what} no requirement of the current revision states"),
    }
}
