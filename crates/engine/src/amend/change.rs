//! Derives what a change did to each requirement of the base.
//!
//! A [`ChangeSet`] reads the grouping's bases against the standing
//! requirements: a group holding a standing requirement keeps its id and is
//! changed, unchanged, or in conflict by what the change sources say beside
//! it, and a group of change claims alone is added under the next free id,
//! or in conflict. Nothing here is the model's: the classes are code's over
//! the statements and triggers the grouping settled.

use omnia_sdk::{Error, server_error};

use crate::amend::standing::Standing;
use crate::revision::{Amendment, AmendmentBase, Change, ChangeClass, Cited, EMERY};
use crate::specify::{BASELINE, Basis};

// One entry per requirement of the amended specification, in id order.
pub struct ChangeSet {
    pub changes: Vec<Change>,
}

impl ChangeSet {
    // What each basis did to the base: a standing requirement among its
    // contributors names the requirement the group revises, and its absence
    // a requirement the base lacks. A changed requirement's sources are the
    // change claims alone; so are a conflict's, the competing claims.
    pub fn derive(standing: &Standing<'_>, bases: &[Basis<'_>]) -> Result<Self, Error> {
        let mut free = standing.free_ids();
        let mut changes = Vec::with_capacity(bases.len());
        for basis in bases {
            let sources: Vec<Cited> = basis
                .contributors()
                .filter(|member| member.source != BASELINE)
                .map(Cited::from)
                .collect();
            let after = basis.statement().map(str::to_owned);

            let change = if let Some(requirement) = standing.matched(basis) {
                let before = requirement.statement();
                // a requirement no claim touched stands as it is, its own
                // conflict included; a change silent on the trigger leaves
                // the base's standing
                let (class, after) = if sources.is_empty() {
                    (ChangeClass::Unchanged, before.clone())
                } else {
                    let same_trigger =
                        basis.trigger.is_none() || basis.trigger == requirement.trigger;
                    let class = match &after {
                        None => ChangeClass::Conflict,
                        Some(_) if after == before && same_trigger => ChangeClass::Unchanged,
                        Some(_) => ChangeClass::Changed,
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
                Change {
                    id,
                    subject: basis.subject.to_owned(),
                    class: if after.is_some() { ChangeClass::Added } else { ChangeClass::Conflict },
                    before: None,
                    after,
                    sources,
                }
            };
            changes.push(change);
        }
        changes.sort_by_key(|change| change.id);
        Ok(Self { changes })
    }

    // The amendment prepared against `base`, under this engine's grammar.
    pub fn into_amendment(self, base: AmendmentBase) -> Amendment {
        Amendment {
            emery: EMERY,
            base,
            changes: self.changes,
        }
    }
}
