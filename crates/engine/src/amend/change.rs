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
//! each under the id its requirement takes.

use omnia_sdk::{Error, server_error};

use crate::amend::standing::Standing;
use crate::revision::{Change, ChangeClass, Cited};
use crate::specify::{BASELINE, Basis, Contributor};

pub struct ChangeSet<'a> {
    // One per requirement of the amended specification and per requirement
    // retired from it, in id order.
    pub changes: Vec<Change>,
    // The bases of the requirements drafted anew, each under the id and
    // subject its change carries, in id order.
    pub drafted: Vec<Basis<'a>>,
}

impl<'a> ChangeSet<'a> {
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
                // the base's standing
                let (class, after) = if sources.is_empty() {
                    (ChangeClass::Unchanged, before.clone())
                } else {
                    let after = basis.statement().map(str::to_owned);
                    let same_trigger =
                        basis.trigger.is_none() || basis.trigger == requirement.trigger;
                    let class = if basis.withdraws() {
                        ChangeClass::Retired
                    } else if after.is_none() {
                        ChangeClass::Conflict
                    } else if after == before && same_trigger {
                        ChangeClass::Unchanged
                    } else {
                        ChangeClass::Changed
                    };
                    (class, after)
                };
                Change {
                    id: requirement.id,
                    subject: requirement.subject.clone(),
                    class,
                    before,
                    after,
                    sources,
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
                }
            };

            if change.class.is_drafted() {
                let subject =
                    matched.map_or(basis.subject, |requirement| requirement.subject.as_str());
                let trigger = basis
                    .trigger
                    .take()
                    .or_else(|| matched.and_then(|requirement| requirement.trigger.clone()));
                let amended = basis.amended(change.id, subject, trigger).ok_or_else(|| {
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
