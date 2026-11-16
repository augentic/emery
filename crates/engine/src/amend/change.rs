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
//! falls silent on under the stems it covers or where a base requirement
//! cites it alone.

use std::collections::BTreeSet;

use emery_adapter::source::{ClaimKind, SourceKind};
use omnia_sdk::{Error, Model, server_error};

use crate::amend::baseline::Baseline;
use crate::revision::{self, Change, ChangeClass, Cited, Spec};
use crate::specify::{BASELINE, Basis, Contributor, Extract, GroupingBrief, Scope};

pub struct ChangeSet {
    // One per requirement of the amended specification and per requirement
    // retired from it, in id order.
    pub changes: Vec<Change>,
    // The bases of the requirements drafted anew, each under the id and
    // subject its change carries, in id order.
    pub drafted: Vec<Basis>,
}

impl ChangeSet {
    // Matches `extracts` against `base`, settles each requirement's class
    // and drafted basis, and retires untouched requirements covered by a
    // whole-scope behaviour source. A change stating no requirement skips the
    // grouping turn.
    pub async fn derive<M: Model>(
        model: &M, base: &Spec, extracts: &[Extract],
    ) -> Result<Self, Error> {
        let baseline = Baseline::new(base, extracts);
        let mut set = if states_requirements(extracts) {
            let bases = GroupingBrief::with_standing(
                extracts,
                baseline.contributors(),
                baseline.criteria(),
            )
            .derive(model)
            .await?;
            Self::from_bases(&baseline, bases)?
        } else {
            tracing::info!("the change states no requirement; the specification stands");
            Self::unchanged(&baseline)
        };
        set.retire_by_scope(&baseline, extracts);
        Ok(set)
    }

    fn unchanged(baseline: &Baseline) -> Self {
        Self {
            changes: baseline.requirements().iter().map(Change::unchanged).collect(),
            drafted: Vec::new(),
        }
    }

    // Retires every standing requirement a behaviour source read whole falls
    // silent on: one no claim touched under a stem the source's requirement
    // claims cover, or one whose base citations name that source alone. A
    // prose source's scope retires nothing, and the first source in
    // declaration order to match is the one named.
    fn retire_by_scope(&mut self, baseline: &Baseline, extracts: &[Extract]) {
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
            let covered_stems: BTreeSet<String> = extract
                .evidence
                .claims
                .iter()
                .filter(|claim| claim.kind == ClaimKind::Requirement)
                .filter_map(|claim| claim.id.as_deref())
                .map(|id| revision::stem(id).to_owned())
                .collect();
            for change in &mut self.changes {
                let in_scope = covered_stems.contains(change.stem())
                    || baseline.cites_only(change.id, &extract.source);
                if !change.is_untouched() || !in_scope {
                    continue;
                }
                tracing::info!(
                    source = %extract.source,
                    requirement = %change.id,
                    subject = %change.subject,
                    "retired by scope: the source read whole states nothing of it"
                );
                change.retire_by_scope(&extract.source);
            }
        }
    }

    // A standing contributor in a basis names the base requirement the group
    // revises; its absence is a requirement the base lacks. A `retire` is
    // ranked as a statement is, so a withdrawal and a revision at one rank
    // are a conflict and the higher-ranked of the two decides.
    fn from_bases(baseline: &Baseline, bases: Vec<Basis>) -> Result<Self, Error> {
        let unmatched: Vec<&Contributor> = bases
            .iter()
            .filter(|basis| baseline.matched(basis).is_none())
            .flat_map(Basis::withdrawals)
            .collect();
        if !unmatched.is_empty() {
            return Err(retire_unmatched(&unmatched));
        }

        let mut free = baseline.free_ids();
        let mut changes = Vec::with_capacity(bases.len());
        let mut drafted = Vec::new();
        for mut basis in bases {
            let sources: Vec<Cited> = basis
                .contributors()
                .filter(|member| member.source != BASELINE)
                .map(Cited::from)
                .collect();
            let matched = baseline.matched(&basis);

            let (id, subject, class, before, after) = if let Some(requirement) = matched {
                // the base's statement, or the competing statements of a
                // conflict, which one claim matched to it resolves
                let before = requirement.stated();
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
                let subject = if class == ChangeClass::Retired {
                    requirement.subject.clone()
                } else {
                    basis.subject.clone()
                };
                (requirement.id, subject, class, before, after)
            } else {
                let id = free
                    .next()
                    .ok_or_else(|| server_error!("the requirement ids are exhausted"))?;
                let after = basis.statement().map(str::to_owned);
                let class =
                    if after.is_some() { ChangeClass::Added } else { ChangeClass::Conflict };
                (id, basis.subject.clone(), class, None, after)
            };
            let change = Change {
                id,
                subject,
                class,
                before,
                after,
                sources,
                scope: None,
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

// Whether any claim of `extracts` states or withdraws a requirement, so the
// change is matched against the standing requirements. A change of other
// claims alone, a `decision` or a `type`, informs the design and nothing of
// the specification.
fn states_requirements(extracts: &[Extract]) -> bool {
    extracts
        .iter()
        .flat_map(|extract| &extract.evidence.claims)
        .any(|claim| matches!(claim.kind, ClaimKind::Requirement | ClaimKind::Retire))
}

// The `retire-unmatched` refusal: `retires` withdraw behaviours no
// requirement of the current revision states, so there is nothing for the
// amendment to retire and a silent pass would lose the withdrawal.
fn retire_unmatched(retires: &[&Contributor]) -> Error {
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
