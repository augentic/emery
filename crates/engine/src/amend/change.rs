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
//! requirement as it is. A behaviour source read whole retires untouched
//! requirements citing a changed or removed file or carrying a vanished
//! trigger when narrowed against a base source snapshot, and otherwise those
//! under the stems it covers or citing that source alone.

use std::collections::BTreeSet;

use emery_adapter::Trigger;
use emery_adapter::source::{ClaimKind, SourceKind};
use omnia_sdk::{Error, Model, server_error};

use crate::amend::baseline::{Baseline, Standing};
use crate::revision::{self, Change, ChangeClass, Cited, ReqId, Requirement, Spec};
use crate::specify::{Basis, Contributor, Extract, GroupingBrief, Scope};

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
        if extracts.iter().any(Extract::states_requirements) {
            let Standing { in_play, settled } = baseline.standing(extracts);
            let brief = GroupingBrief::with_standing(
                extracts,
                in_play,
                baseline.criteria(),
                baseline.whole_scope().clone(),
            );
            tracing::info!(
                claims = brief.claim_count(),
                standing = brief.standing_count(),
                settled = settled.len(),
                "grouping load"
            );
            let bases = brief.derive(model).await?;
            Self::from_bases(&baseline, extracts, bases, settled)
        } else {
            tracing::info!(
                claims = 0,
                standing = 0,
                settled = baseline.requirements().len(),
                "grouping load"
            );
            tracing::info!("the change states no requirement; the specification stands");
            let mut set = Self::unchanged(&baseline);
            set.retire_by_scope(&baseline, extracts, &BTreeSet::new());
            Ok(set)
        }
    }

    fn unchanged(baseline: &Baseline) -> Self {
        Self {
            changes: baseline.requirements().iter().map(Change::unchanged).collect(),
            drafted: Vec::new(),
        }
    }

    // Retires standing requirements a behaviour source read whole falls silent
    // on: for a snapshot-narrowed source, those citing a changed or removed
    // file of that source or carrying a vanished trigger; otherwise those under
    // a stem the source's requirement claims cover or citing that source alone.
    fn retire_by_scope(
        &mut self, baseline: &Baseline, extracts: &[Extract], evidenced: &BTreeSet<ReqId>,
    ) {
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
            let covered_stems: BTreeSet<String> = if extract.is_narrowed() {
                BTreeSet::new()
            } else {
                extract
                    .evidence
                    .claims
                    .iter()
                    .filter(|claim| claim.kind == ClaimKind::Requirement)
                    .filter_map(|claim| claim.id.as_deref())
                    .map(|id| revision::stem(id).to_owned())
                    .collect()
            };
            for change in &mut self.changes {
                let in_scope = match extract.delta() {
                    Some(delta) => baseline.in_narrowed_scope(change.id, &extract.source, delta),
                    None => {
                        covered_stems.contains(change.stem())
                            || baseline.cites_only(change.id, &extract.source)
                    }
                };
                let is_evidenced =
                    evidenced.contains(&change.id) && baseline.admits_evidenced(change.id);
                if !change.is_untouched() || is_evidenced || !in_scope {
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
    fn from_bases(
        baseline: &Baseline, extracts: &[Extract], bases: Vec<Basis>, settled: Vec<Change>,
    ) -> Result<Self, Error> {
        let unmatched: Vec<Contributor> = bases
            .iter()
            .filter(|basis| baseline.matched(basis).is_none())
            .flat_map(Basis::withdrawals)
            .cloned()
            .collect();
        if !unmatched.is_empty() {
            return Err(retire_unmatched(&unmatched));
        }

        // settle standing requirements
        let mut matched_bases = Vec::new();
        let mut added_bases = Vec::new();
        for basis in bases {
            if let Some(requirement) = baseline.matched(&basis) {
                matched_bases.push((requirement, basis));
            } else {
                added_bases.push(basis);
            }
        }

        let mut changes = settled;
        changes.reserve(matched_bases.len() + added_bases.len());
        let mut set = Self {
            changes,
            drafted: Vec::new(),
        };
        let mut evidenced = BTreeSet::new();
        for (requirement, basis) in matched_bases {
            let sources: Vec<Cited> = basis.change_contributors().map(Cited::from).collect();
            if basis.evidenced {
                evidenced.insert(requirement.id);
            }
            let change = matched_change(baseline, requirement, &basis, sources);
            set.record(change, basis, baseline.preserved_trigger(requirement))?;
        }
        set.changes.sort_by_key(|change| change.id);

        // retire silent whole-scope requirements
        set.retire_by_scope(baseline, extracts, &evidenced);
        let unchanged: BTreeSet<ReqId> = set
            .changes
            .iter()
            .filter(|change| change.class == ChangeClass::Unchanged)
            .map(|change| change.id)
            .collect();

        // admit added requirements
        let mut free = baseline.free_ids();
        for basis in added_bases {
            if baseline.is_untouched_bootstrap(&basis, &unchanged, extracts) {
                tracing::info!(
                    subject = %basis.subject,
                    "untouched bootstrap re-extraction noise; standing `start` requirements stand"
                );
                continue;
            }
            let sources: Vec<Cited> = basis.change_contributors().map(Cited::from).collect();
            let id =
                free.next().ok_or_else(|| server_error!("the requirement ids are exhausted"))?;
            let change = added_change(id, &basis, sources);
            set.record(change, basis, None)?;
        }
        set.changes.sort_by_key(|change| change.id);
        set.drafted.sort_by_key(|basis| basis.id);
        Ok(set)
    }

    fn record(
        &mut self, change: Change, mut basis: Basis, preserved_trigger: Option<Trigger>,
    ) -> Result<(), Error> {
        if change.class.is_drafted() {
            let trigger = basis.trigger.take().or(preserved_trigger);
            let amended = basis.amended(change.id, trigger).ok_or_else(|| {
                server_error!(
                    "requirement {} is drafted anew with no claim to draft from",
                    change.id
                )
            })?;
            self.drafted.push(amended);
        }
        self.changes.push(change);
        Ok(())
    }
}

fn matched_change(
    baseline: &Baseline, requirement: &Requirement, basis: &Basis, sources: Vec<Cited>,
) -> Change {
    // the base's statement, or the competing statements of a conflict, which
    // one claim matched to it resolves
    let before = requirement.stated();
    // a requirement no claim touched stands as it is, its own conflict
    // included; a change silent on the trigger leaves the base's standing; a
    // standing requirement in the winning agreement class under the same stem
    // and trigger keeps its base subject and statement
    let (class, after) = if sources.is_empty() {
        (ChangeClass::Unchanged, before.clone())
    } else {
        let after = basis.statement();
        let same_trigger = baseline.same_trigger(requirement, basis);
        let agrees = baseline.agrees(requirement, basis);
        let exact = after == before && basis.subject == requirement.subject;
        let class = if basis.withdraws() {
            ChangeClass::Retired
        } else if after.is_none() {
            ChangeClass::Conflict
        } else if same_trigger && (agrees || exact) {
            ChangeClass::Unchanged
        } else {
            ChangeClass::Changed
        };
        let after = if class == ChangeClass::Unchanged { before.clone() } else { after };
        (class, after)
    };
    let subject = if matches!(class, ChangeClass::Unchanged | ChangeClass::Retired) {
        requirement.subject.clone()
    } else {
        basis.subject.clone()
    };
    Change {
        id: requirement.id,
        subject,
        class,
        before,
        after,
        sources,
        scope: None,
    }
}

fn added_change(id: ReqId, basis: &Basis, sources: Vec<Cited>) -> Change {
    let after = basis.statement();
    let class = if after.is_some() { ChangeClass::Added } else { ChangeClass::Conflict };
    Change {
        id,
        subject: basis.subject.clone(),
        class,
        before: None,
        after,
        sources,
        scope: None,
    }
}

// The `retire-unmatched` refusal: `retires` withdraw behaviours no
// requirement of the current revision states, so there is nothing for the
// amendment to retire and a silent pass would lose the withdrawal.
fn retire_unmatched(retires: &[Contributor]) -> Error {
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
