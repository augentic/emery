//! Reconciles requirement claims into deterministic requirement bases.
//!
//! Groups claims by identifier and model-assisted semantic matching, then
//! applies source authority to derive each requirement's winning statement,
//! status, and coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::Trigger;
use emery_adapter::source::{Claim, ClaimKind, SourceKind};
use omnia_sdk::{Error, Model, bad_request, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::Rank;
use crate::brief::{Brief, Review};
use crate::revision::{self, Cited, Loser, ReqId, Requirement, Scenario, Spec, Status};
use crate::specify::{Extract, shape};

// The reserved source name the current revision's requirements contribute under.
//
// An amend run reconciles its change sources against the requirements the
// current revision holds, each a `Contributor` from this source, so no
// operator source may take the name.
pub const BASELINE: &str = "baseline";

// A synthesis brief for grouping requirement claims.
//
// The brief contains requirement claims in source order and every acceptance
// criterion identifier. Source authority is withheld from the model. An
// amend run's brief carries the current revision's requirements after the
// claims, as the standing requirements the claims are matched against.
pub struct GroupingBrief {
    contributors: Vec<Contributor>,
    criteria: Vec<String>,
    contracts: Contracts,
    sources: usize,
    baseline: Grouping,
    // How many contributors the run's sources contributed; the rest are
    // the standing requirements.
    changes: usize,
    // Anchor and live-surface hints matching claims to standing requirements.
    hints: Vec<Hint>,
    // Whole-scope behaviour evidence reconciled against standing requirements.
    whole: WholeScope,
}

impl GroupingBrief {
    // A `retire` claim is passed over: with no standing requirements there
    // is nothing for it to withdraw.
    #[must_use]
    pub fn new(extracts: &[Extract]) -> Self {
        Self::assemble(extracts, Vec::new(), Vec::new(), WholeScope::default(), false)
    }

    // `standing` is the current revision's requirements as contributors
    // from `BASELINE`, each indexed after the claims, and `criteria` the
    // ids of those the revision holds covered. A source that withdraws and
    // restates one behaviour under one id has restated it: its `retire`
    // is passed over and its `requirement` stands. A claim anchored where
    // a standing requirement's citation was read, from the same source,
    // is hinted to revise it under `## Baseline` and pre-merged with
    // nothing.
    #[must_use]
    pub fn with_standing(
        extracts: &[Extract], standing: Vec<Contributor>, criteria: Vec<String>, whole: WholeScope,
    ) -> Self {
        let whole = if standing.is_empty() { WholeScope::default() } else { whole };
        Self::assemble(extracts, standing, criteria, whole, true)
    }

    fn assemble(
        extracts: &[Extract], standing: Vec<Contributor>, mut criteria: Vec<String>,
        whole: WholeScope, withdrawals: bool,
    ) -> Self {
        let mut contributors: Vec<Contributor> = Vec::new();
        let mut contracts = Contracts::default();
        let has_standing = !standing.is_empty();

        for extract in extracts {
            ExtractNotes::new(extract, has_standing).collect(
                extract,
                withdrawals,
                &mut contributors,
                &mut criteria,
                &mut contracts,
            );
        }

        // the standing requirements after the claims, indexed in place
        let changes = contributors.len();
        contributors.extend(standing.into_iter().enumerate().map(|(offset, member)| Contributor {
            index: changes + offset,
            ..member
        }));

        // count the sources that contribute a requirement claim
        let sources =
            contributors.iter().map(|claim| claim.source.as_str()).collect::<BTreeSet<_>>().len();
        let baseline = Grouping::baseline(&contributors);
        let hints =
            Hint::all(&contributors[..changes], &contributors[changes..], &contracts, &whole);

        Self {
            contributors,
            criteria,
            contracts,
            sources,
            baseline,
            changes,
            hints,
            whole,
        }
    }

    // Derives every requirement basis.
    //
    // A run whose requirement claims come from two or more sources asks the
    // model to group them, as does one matching claims against standing
    // requirements. So does a run over one contributing source whose
    // requirement ids span two or more stems, since its seams may describe
    // one behaviour under different nouns. Otherwise the baseline stands alone
    // and no call is spent. Whatever the model answers, claims sharing an id
    // stay one group, and one source's distinct ids under one stem stay
    // distinct groups — a group that merges them is split, with no round
    // spent.
    //
    // - Returns `Error::BadRequest` when no source contributed a requirement
    //   claim, or when the model cannot produce a valid grouping within the
    //   available rounds.
    // - Returns `Error::ServerError` when required prose is missing or a
    //   grouping cannot be reconciled with the claims.
    // - Returns `Error::BadGateway` when the model operation fails.
    pub async fn derive<M: Model>(self, model: &M) -> Result<Vec<Basis>, Error> {
        if self.claims().is_empty() {
            return Err(bad_request!("no source contributed a requirement claim"));
        }

        if self.sources < 2 && self.stems() < 2 {
            self.baseline.bases(&self.contributors, &self.criteria, &self.contracts)
        } else {
            self.judge(model).await
        }
    }

    // The distinct stems among the contributors' ids: the nouns the seams led
    // with, which differ when one source describes one behaviour twice.
    fn stems(&self) -> usize {
        self.contributors.iter().map(Contributor::stem).collect::<BTreeSet<_>>().len()
    }

    #[must_use]
    pub const fn claim_count(&self) -> usize {
        self.changes
    }

    #[must_use]
    pub const fn standing_count(&self) -> usize {
        self.contributors.len().saturating_sub(self.changes)
    }

    fn claims(&self) -> &[Contributor] {
        &self.contributors[..self.changes]
    }

    fn standing(&self) -> &[Contributor] {
        &self.contributors[self.changes..]
    }

    const fn is_change_claim(&self, index: usize) -> bool {
        index < self.changes
    }

    const fn is_standing_index(&self, index: usize) -> bool {
        index >= self.changes && index < self.contributors.len()
    }

    fn verify_placements(&self, placements: &Placements, review: &mut Review) {
        for index in placements.duplicate_group_members() {
            review.note(format_args!("claim {index} appears in more than one group"));
        }

        for index in 0..self.contributors.len() {
            if !placements.is_accounted(index) {
                review.note(format_args!("claim {index} is in no group"));
            }
        }

        // byte-equal ids may not be split across groups
        for group in &self.baseline.groups {
            if group.claims.len() <= 1
                || !group.claims.iter().all(|&index| placements.is_accounted(index))
            {
                continue;
            }
            if !placements.all_share_group(&group.claims) {
                let id = &self.contributors[group.claims[0]].id;
                review.note(format_args!("claims sharing the id `{id}` are split across groups"));
            }
        }
    }

    fn verify_restated(&self, answer: &Grouping, placements: &mut Placements, review: &mut Review) {
        if self.standing().is_empty() {
            if !answer.restated.is_empty() {
                review.note(
                    "`restated` is only used when `## Standing requirements` is listed; leave it \
                     empty",
                );
            }
            return;
        }

        for entry in &answer.restated {
            self.verify_restated_entry(entry, placements, review);
        }
    }

    fn verify_restated_entry(
        &self, entry: &Restated, placements: &mut Placements, review: &mut Review,
    ) {
        let valid_claim = self.verify_restated_claim(entry.claim, placements, review);
        if entry.standing.is_empty() {
            review.note(format_args!(
                "`restated` claim {} lists no standing requirements",
                entry.claim
            ));
            return;
        }

        let mut seen_standing = BTreeSet::new();
        for &standing_index in &entry.standing {
            if !self.is_standing_index(standing_index) {
                review.note(format_args!(
                    "`restated` standing index {standing_index} is not a standing requirement"
                ));
                continue;
            }
            if !seen_standing.insert(standing_index) {
                review.note(format_args!(
                    "`restated` claim {} lists standing requirement {standing_index} more than \
                     once",
                    entry.claim
                ));
                continue;
            }
            placements.record_restated_standing(standing_index);
            if valid_claim && placements.shares_group_with(entry.claim, standing_index) {
                review.note(format_args!(
                    "`restated` claim {} lists standing requirement {standing_index}, whose group \
                     already contains claim {}",
                    entry.claim, entry.claim
                ));
            }
            let standing_member = &self.contributors[standing_index];
            if let Some(vanished) = self
                .whole
                .vanished_trigger(&standing_member.citations, standing_member.trigger.as_ref())
            {
                review.note(format_args!(
                    "`restated` claim {} lists standing requirement {standing_index} (`{}`), \
                     which carries {} — absent from the extracted contracts; omit it from \
                     `restated` so whole-source scope retires it",
                    entry.claim,
                    standing_member.id,
                    trigger_note(&vanished),
                ));
            }
        }
    }

    fn verify_restated_claim(
        &self, claim: usize, placements: &mut Placements, review: &mut Review,
    ) -> bool {
        if !self.is_change_claim(claim) {
            review.note(format_args!("`restated` claim {claim} is not a change claim"));
            return false;
        }
        if !placements.record_restated_claim(claim) {
            review.note(format_args!("claim {claim} appears in `restated` more than once"));
        }
        if self.contributors[claim].withdraws {
            review.note(format_args!(
                "`restated` claim {claim} (`{}`) is a `(retire)` claim; place it in the group of \
                 the standing requirement it withdraws",
                self.contributors[claim].id
            ));
            return false;
        }
        true
    }

    fn verify_evidenced(&self, answer: &Grouping, placements: &Placements, review: &mut Review) {
        if !self.whole.has_details() {
            if !answer.evidenced.is_empty() {
                review.note(
                    "`evidenced` is only used when `## Whole-source detail` is listed; leave it \
                     empty",
                );
            }
            return;
        }
        let mut seen_evidenced = BTreeSet::new();
        for &index in &answer.evidenced {
            if !self.is_standing_index(index) {
                review
                    .note(format_args!("`evidenced` index {index} is not a standing requirement"));
            } else if !seen_evidenced.insert(index) {
                review.note(format_args!(
                    "standing requirement {index} appears in `evidenced` more than once"
                ));
            } else if placements.is_matched_standing(index, &answer.groups) {
                review.note(format_args!(
                    "standing requirement {index} is already matched to a claim; list in \
                     `evidenced` only standing requirements that stand alone"
                ));
            } else {
                let member = &self.contributors[index];
                match self.whole.evidenced_gap(
                    member.stem(),
                    member.trigger.as_ref(),
                    &member.citations,
                ) {
                    Some(EvidencedGap::MissingContract(trigger)) => review.note(format_args!(
                        "standing requirement {index} (`{}`) carries {}, which no contract of its \
                         cited whole-scope source declares; omit it from `evidenced`",
                        member.id,
                        trigger_note(&trigger),
                    )),
                    Some(EvidencedGap::MissingDetail) => review.note(format_args!(
                        "standing requirement {index} (`{}`) has no `## Whole-source detail` in \
                         any file its citations name; omit it from `evidenced`",
                        member.id,
                    )),
                    None => {}
                }
            }
        }
    }

    fn verify_vanished_trigger(
        &self, position: usize, standing_index: usize, members: &BTreeSet<usize>,
        review: &mut Review,
    ) {
        let Some(standing_member) = self.contributors.get(standing_index) else {
            return;
        };
        let Some(vanished) = self
            .whole
            .vanished_trigger(&standing_member.citations, standing_member.trigger.as_ref())
        else {
            return;
        };

        let mut claims_by_source: BTreeMap<String, Vec<Contributor>> = BTreeMap::new();
        for claim in members
            .iter()
            .copied()
            .filter(|&index| self.is_change_claim(index))
            .filter_map(|index| self.contributors.get(index))
            .filter(|claim| !claim.withdraws && self.whole.contains_source(&claim.source))
        {
            claims_by_source.entry(claim.source.clone()).or_default().push(claim.clone());
        }

        for (source, claims) in &claims_by_source {
            let has_replacement =
                self.contracts.trigger_in_source_of(source, claims).is_some_and(|trigger| {
                    !self.whole.has_other_standing_trigger(&standing_member.id, &trigger)
                });

            for claim in claims.iter().filter(|claim| claim.id != standing_member.id) {
                let existing_trigger =
                    self.contracts.trigger_in_source(&Cited::from(claim)).filter(|trigger| {
                        self.whole.has_other_standing_trigger(&standing_member.id, trigger)
                    });
                if let Some(existing) = existing_trigger {
                    review.note(format_args!(
                        "group {position}: standing requirement {standing_index} (`{}`) carries \
                         {}, which is absent from the extracted contracts of `{source}`, while \
                         claim {} (`{}`) belongs to existing {}; leave standing requirement \
                         {standing_index} alone in its own group so whole-source scope retires it",
                        standing_member.id,
                        trigger_note(&vanished),
                        claim.index,
                        claim.id,
                        trigger_note(&existing),
                    ));
                } else if !has_replacement {
                    review.note(format_args!(
                        "group {position}: standing requirement {standing_index} (`{}`) carries \
                         {}, which is absent from the extracted contracts of `{source}`, so \
                         claim {} (`{}`) cannot keep it standing without a replacement contract; \
                         leave standing requirement {standing_index} alone in its own group so \
                         whole-source scope retires it, and place claim {} with the live \
                         requirement it covers (or in its own group if new)",
                        standing_member.id,
                        trigger_note(&vanished),
                        claim.index,
                        claim.id,
                        claim.index,
                    ));
                }
            }
        }
    }

    fn write_instructions(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.standing().is_empty() {
            return f.write_str(
                "\nAnswer with every index in one group, and every group's claims in one \
                 agreeing class. Two claims of one source that describe one requirement under \
                 different stems — the first segment of their ids — are one group, whatever \
                 nouns their seams gave them. Two claims of one source with different ids under \
                 one stem are distinct requirements: the call that minted them under one noun \
                 told them apart, and a group that merges them is split, each id its own \
                 requirement.\n",
            );
        }

        f.write_str(
            "\nAnswer with every index in at most one group: every standing requirement in one \
             group, every change claim in one group or in `restated`, and every group's claims \
             in one agreeing class. Two claims of one source that describe one requirement \
             under different stems — the first segment of their ids — are one group, whatever \
             nouns their seams gave them.\n\nEach standing requirement is one requirement of \
             the current specification, shown with its scenarios; claims carry any criteria \
             extracted under them. A claim that restates, revises, or narrows a standing \
             requirement joins its group, in the standing requirement's class when the \
             behaviour, values, thresholds, and outcomes match (even if reworded or given \
             another sub-stem or leaf id) and in a class of its own when a value, condition, or \
             outcome changed. Multiple claims of one source under one stem that state facets of \
             one standing requirement join that standing requirement's group in one class — the \
             standing requirement's class when unchanged, or one class of their own when \
             revised — rather than adding separate groups for already-stated facets. When a \
             standing requirement combines facets across files or steps and a listed claim \
             states only one facet whose behaviour, values, and outcomes match what that \
             standing requirement already says for that facet, place the claim in the standing \
             requirement's agreeing class (when no other claim revises it) or list the standing \
             requirement in `restated`, rather than placing the partial facet in a separate \
             class that replaces the standing statement. Two \
             standing requirements never share a group. When one change claim states nothing \
             beyond what one or more standing requirements already state, list `{\"claim\": \
             <claim>, \"standing\": [<standing>, ...]}` in `restated` (omitting the claim from \
             `groups`, or placing it in one revised standing requirement's group and listing \
             only the other unchanged standing requirements in `restated`). A claim stating a \
             behaviour no standing requirement or scenario covers is a group of its own. A \
             standing requirement no claim touches is a group of its own too.\n\nA claim marked \
             `(retire)` withdraws the behaviour its id names rather than stating one: it joins \
             the group of the standing requirement that states the behaviour, in a class of its \
             own, and is never a group of its own, of other claims alone, or in `restated`, \
             since a withdrawal of a behaviour the specification does not hold is refused after \
             your answer.\n",
        )?;
        self.write_whole_scope_instructions(f)
    }

    fn write_whole_scope_instructions(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if !self.whole.is_empty() {
            if let Some(sources) = self.whole.narrowed_sources() {
                writeln!(
                    f,
                    "\nWhole-scope changed-file extraction ({sources}): claims from unchanged \
                     files were omitted before grouping, while same-stem standing requirements \
                     read at unchanged files remain listed in play. Leave any standing \
                     requirement whose behaviour no listed claim touches alone in its own group \
                     — standing requirements read at unchanged files with live contracts stay \
                     unchanged automatically, so never force claims from a changed file onto \
                     unrelated standing requirements on the stem or into `restated`.",
                )?;
            }
            if let Some(sources) = self.whole.full_sources() {
                writeln!(
                    f,
                    "\nWhole-scope re-extraction ({sources}): every file of the source was \
                     re-extracted, including files the change did not touch. When a \
                     re-extracted claim only repeats preconditions, facets, or startup setup \
                     already stated by standing requirements or scenarios, place it in the \
                     covered standing requirement's class or list the covered standing \
                     requirements in `restated` rather than adding a duplicate requirement; \
                     leave a claim in a singleton group only when it states behaviour absent \
                     from every standing requirement and scenario.",
                )?;
            }
            f.write_str(
                "\nNever match an untriggered claim or another surface's claim to, or list in \
                 `restated` or `evidenced`, a standing requirement whose whole-scope route or \
                 message trigger vanished (`absent from extracted contracts`); leave a standing \
                 requirement whose surface vanished alone in its own group so whole-source \
                 scope retires it, unless the group binds a replacement contract for that \
                 surface.\n",
            )?;
        }
        if self.whole.has_details() {
            f.write_str(
                "\nWhen a standing requirement stands alone in its group only because this \
                 extraction recorded its behaviour in `## Whole-source detail` as an `excerpt` \
                 or `call` rather than a `requirement` claim, and that detail in its cited file \
                 (or under its stem when its whole-scope route or message contract still \
                 stands) shows the behaviour is still implemented unchanged, list that standing \
                 requirement's index in `evidenced` so whole-source scope does not retire it. \
                 Never list in `evidenced` a standing requirement matched to a claim or listed \
                 in `restated`, one whose route or message trigger vanished from the extracted \
                 contracts, or one whose cited file holds no matching `## Whole-source \
                 detail`.\n",
            )?;
        }
        Ok(())
    }
}

impl Brief for GroupingBrief {
    type Answer = Grouping;
    type Output = Vec<Basis>;

    const NAME: &'static str = "grouping";
    const PROSE: &'static [&'static str] = &["grouping.md"];

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        let last = self.contributors.len().saturating_sub(1);
        for pointer in
            ["/$defs/Group/properties/claims/items", "/$defs/Group/properties/classes/items/items"]
        {
            Self::at(schema, pointer)?.insert("maximum".to_owned(), json!(last));
        }
        Self::at(schema, "/properties/groups")?.insert("minItems".to_owned(), json!(1));
        if self.standing().is_empty() {
            Self::at(schema, "/properties")?.remove("restated");
            Self::at(schema, "/$defs")?.remove("Restated");
        } else {
            let last_claim = self.changes.saturating_sub(1);
            Self::at(schema, "/$defs/Restated/properties/claim")?
                .insert("maximum".to_owned(), json!(last_claim));
            Self::at(schema, "/$defs/Restated/properties/standing")?
                .insert("minItems".to_owned(), json!(1));
            let items = Self::at(schema, "/$defs/Restated/properties/standing/items")?;
            items.insert("minimum".to_owned(), json!(self.changes));
            items.insert("maximum".to_owned(), json!(last));
        }
        if self.whole.has_details() {
            let items = Self::at(schema, "/properties/evidenced/items")?;
            items.insert("minimum".to_owned(), json!(self.changes));
            items.insert("maximum".to_owned(), json!(last));
        } else {
            Self::at(schema, "/properties")?.remove("evidenced");
        }
        Ok(())
    }

    fn verify(&self, answer: &Grouping, review: &mut Review) {
        let count = self.contributors.len();
        let mut placements = Placements::default();

        for (position, group) in answer.groups.iter().enumerate() {
            if group.claims.is_empty() {
                review.note(format_args!("group {position} has no claims"));
            }

            let mut members = BTreeSet::new();
            for &index in &group.claims {
                if index >= count {
                    review.note(format_args!("group {position}: claim {index} does not exist"));
                } else if !members.insert(index) {
                    review.note(format_args!(
                        "group {position}: claim {index} appears more than once in the group"
                    ));
                } else {
                    placements.record_group(index, position);
                }
            }

            let mut classed = BTreeSet::new();
            for class in &group.classes {
                if class.is_empty() {
                    review.note(format_args!("group {position} has an empty class"));
                }

                for &index in class {
                    if !members.contains(&index) {
                        review.note(format_args!(
                            "group {position}: class member {index} is not one of its claims"
                        ));
                    } else if !classed.insert(index) {
                        review.note(format_args!(
                            "group {position}: claim {index} appears in more than one class"
                        ));
                    }
                }

                // a withdrawal never says the same thing as a statement
                let withdraws = class
                    .first()
                    .and_then(|&index| self.contributors.get(index))
                    .is_some_and(|member| member.withdraws);
                if class
                    .iter()
                    .filter_map(|&index| self.contributors.get(index))
                    .any(|member| member.withdraws != withdraws)
                {
                    review.note(format_args!(
                        "group {position}: a `(retire)` claim withdraws a behaviour and cannot \
                         share a class with a statement"
                    ));
                }
            }

            for index in members.difference(&classed) {
                review.note(format_args!("group {position}: claim {index} is in no class"));
            }

            // two standing requirements are two requirements already
            let standing_indices: Vec<usize> =
                members.iter().copied().filter(|&index| self.is_standing_index(index)).collect();
            if standing_indices.len() > 1 {
                let standing: BTreeSet<String> = standing_indices
                    .iter()
                    .filter_map(|&index| self.contributors.get(index))
                    .map(|member| format!("`{}`", member.id))
                    .collect();
                let ids = standing.into_iter().collect::<Vec<_>>().join(", ");
                review.note(format_args!(
                    "group {position} merges the standing requirements {ids}, which the current \
                     specification holds as distinct requirements"
                ));
            } else if let [standing_index] = standing_indices.as_slice() {
                self.verify_vanished_trigger(position, *standing_index, &members, review);
            }
        }

        self.verify_restated(answer, &mut placements, review);
        self.verify_placements(&placements, review);
        self.verify_evidenced(answer, &placements, review);
    }

    fn into_output(self, answer: Grouping) -> Result<Vec<Basis>, Error> {
        answer.split(&self.contributors, self.changes).bases(
            &self.contributors,
            &self.criteria,
            &self.contracts,
        )
    }
}

impl Display for GroupingBrief {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let standing = self.standing();
        if standing.is_empty() {
            f.write_str("Group the requirement claims.\n\n")?;
        } else {
            f.write_str("Group the requirement claims against the standing requirements.\n\n")?;
        }
        f.write_str("## Requirement claims (index, source, id, statement, synopsis)\n\n")?;

        for claim in self.claims() {
            let synopsis = claim.synopsis.as_deref().unwrap_or("-");
            let kind = if claim.withdraws { " (retire)" } else { "" };
            writeln!(
                f,
                "- {index} `{source}` `{id}`{kind} — {statement} — {synopsis}",
                index = claim.index,
                source = claim.source,
                id = claim.id,
                statement = claim.statement,
            )?;
            for note in &claim.notes {
                writeln!(f, "  - {note}")?;
            }
        }

        if !standing.is_empty() {
            f.write_str("\n## Standing requirements (index, id, statement)\n\n")?;
            for claim in standing {
                writeln!(
                    f,
                    "- {index} `{id}` — {statement}",
                    index = claim.index,
                    id = claim.id,
                    statement = claim.statement,
                )?;
                for note in &claim.notes {
                    writeln!(f, "  - {note}")?;
                }
            }
        }

        if self.whole.has_details() {
            f.write_str(
                "\n## Whole-source detail (source, path, detail)\n\nNon-requirement detail \
                 (`excerpt` and `call` claims) extracted from whole-scope sources:\n\n",
            )?;
            for detail in self.whole.details() {
                writeln!(f, "{detail}")?;
            }
        }

        f.write_str("\n## Baseline\n\n")?;
        let merged: Vec<&Group> =
            self.baseline.groups.iter().filter(|group| group.claims.len() > 1).collect();
        if merged.is_empty() {
            f.write_str("No two claims share an id; every grouping is your judgement.\n")?;
        }

        for group in merged {
            let id = &self.contributors[group.claims[0]].id;
            let indices = join_indices(&group.claims);
            writeln!(
                f,
                "- claims {indices} share the id `{id}` and are pre-merged; an answer that splits \
                 them across groups is refused."
            )?;
        }
        for hint in &self.hints {
            writeln!(f, "{hint}")?;
        }
        self.write_instructions(f)
    }
}

struct CriterionNote {
    id: String,
    condition: String,
}

impl CriterionNote {
    fn from_claim(claim: &Claim) -> Option<Self> {
        if claim.kind != ClaimKind::Criterion {
            return None;
        }
        let id = claim.id.clone()?;
        let condition = claim.criterion()?.split_whitespace().collect::<Vec<_>>().join(" ");
        (!condition.is_empty()).then_some(Self { id, condition })
    }
}

struct ExtractNotes {
    source: String,
    is_whole_behaviour: bool,
    criteria: Vec<CriterionNote>,
    contracts: Contracts,
}

impl ExtractNotes {
    fn new(extract: &Extract, has_standing: bool) -> Self {
        let criteria = if has_standing {
            extract.evidence.claims.iter().filter_map(CriterionNote::from_claim).collect()
        } else {
            Vec::new()
        };
        Self {
            source: extract.source.clone(),
            is_whole_behaviour: has_standing && extract.is_whole_behaviour(),
            criteria,
            contracts: Contracts::from_extract(extract),
        }
    }

    fn claim_notes(&self, id: &str, path: Option<&str>) -> Vec<String> {
        let mut notes: Vec<String> = self
            .criteria
            .iter()
            .filter(|criterion| revision::leads(id, &criterion.id))
            .map(|criterion| format!("criterion `{}`: {}", criterion.id, criterion.condition))
            .collect();
        if self.is_whole_behaviour {
            let cited = Cited {
                source: self.source.clone(),
                claim: id.to_owned(),
                path: path.map(str::to_owned),
            };
            if let Some(trigger) = self.contracts.trigger_for(&cited) {
                notes.push(trigger_note(&trigger));
            }
            if let Some(path) = path {
                notes.push(format!("at `{path}`"));
            }
        }
        notes
    }

    fn collect(
        self, extract: &Extract, withdrawals: bool, contributors: &mut Vec<Contributor>,
        criteria: &mut Vec<String>, contracts: &mut Contracts,
    ) {
        // a source cannot conflict with itself: a withdrawal beside its own
        // restatement is passed over so the restatement stands
        let stated_ids: BTreeSet<String> = if withdrawals {
            extract
                .evidence
                .claims
                .iter()
                .filter(|claim| claim.kind == ClaimKind::Requirement)
                .filter_map(|claim| claim.id.clone())
                .collect()
        } else {
            BTreeSet::new()
        };

        for claim in &extract.evidence.claims {
            // a contract binds by its id or its anchor, so it needs no id
            if claim.kind == ClaimKind::Contract {
                continue;
            }
            let Some(id) = claim.id.as_deref() else { continue };

            let withdraws = match claim.kind {
                ClaimKind::Requirement => false,
                ClaimKind::Retire if withdrawals => {
                    if stated_ids.contains(id) {
                        tracing::debug!(
                            source = %extract.source,
                            id,
                            "withdrawal beside its own restatement; the restatement stands"
                        );
                        continue;
                    }
                    true
                }
                ClaimKind::Criterion => {
                    criteria.push(id.to_owned());
                    continue;
                }
                _ => continue,
            };
            let notes =
                if withdraws { Vec::new() } else { self.claim_notes(id, claim.path.as_deref()) };
            contributors.push(Contributor {
                source: extract.source.clone(),
                kind: extract.kind,
                rank: extract.rank,
                id: id.to_owned(),
                path: claim.path.clone(),
                statement: claim.statement(),
                synopsis: claim.synopsis.clone(),
                notes,
                withdraws,
                trigger: None,
                citations: Vec::new(),
                index: contributors.len(),
            });
        }
        contracts.extend(self.contracts);
    }
}

fn trigger_note(trigger: &Trigger) -> String {
    match trigger {
        Trigger::Http(route) => format!("route `{route}`"),
        Trigger::Message(topic) => format!("message `{topic}`"),
    }
}

fn join_indices(indices: &[usize]) -> String {
    indices.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
}

#[derive(Default)]
struct Placements {
    groups_by_member: BTreeMap<usize, BTreeSet<usize>>,
    restated_claims: BTreeSet<usize>,
    restated_standing: BTreeSet<usize>,
}

impl Placements {
    fn record_group(&mut self, member: usize, group: usize) {
        self.groups_by_member.entry(member).or_default().insert(group);
    }

    fn record_restated_claim(&mut self, claim: usize) -> bool {
        self.restated_claims.insert(claim)
    }

    fn record_restated_standing(&mut self, standing: usize) {
        self.restated_standing.insert(standing);
    }

    fn duplicate_group_members(&self) -> impl Iterator<Item = usize> + '_ {
        self.groups_by_member.iter().filter(|(_, groups)| groups.len() > 1).map(|(&index, _)| index)
    }

    fn is_accounted(&self, member: usize) -> bool {
        self.groups_by_member.contains_key(&member) || self.restated_claims.contains(&member)
    }

    fn in_group(&self, member: usize, group: usize) -> bool {
        self.groups_by_member.get(&member).is_some_and(|groups| groups.contains(&group))
    }

    fn shares_group_with(&self, claim: usize, standing: usize) -> bool {
        match (self.groups_by_member.get(&claim), self.groups_by_member.get(&standing)) {
            (Some(claim_groups), Some(standing_groups)) => {
                !claim_groups.is_disjoint(standing_groups)
            }
            _ => false,
        }
    }

    fn all_share_group(&self, members: &[usize]) -> bool {
        let Some((&first, rest)) = members.split_first() else {
            return true;
        };
        self.groups_by_member.get(&first).is_some_and(|groups| {
            groups.iter().any(|&group| rest.iter().all(|&member| self.in_group(member, group)))
        })
    }

    fn is_matched_standing(&self, standing: usize, groups: &[Group]) -> bool {
        self.restated_standing.contains(&standing)
            || self
                .groups_by_member
                .get(&standing)
                .into_iter()
                .flatten()
                .filter_map(|&pos| groups.get(pos))
                .any(|group| group.claims.len() > 1)
    }
}

enum EvidencedGap {
    MissingContract(Trigger),
    MissingDetail,
}

// Whole-scope behaviour evidence reconciled against standing requirements:
// each source's extracted triggers and stem files, plus non-requirement
// detail (`excerpt` and `call` claims).
#[derive(Debug, Default, Clone)]
pub struct WholeScope {
    sources: BTreeMap<String, WholeSource>,
    details: Vec<Detail>,
    standing_triggers: BTreeMap<Trigger, BTreeSet<String>>,
}

#[derive(Debug, Default, Clone)]
struct WholeSource {
    narrowed: bool,
    triggers: BTreeSet<Trigger>,
    stem_files: BTreeMap<String, BTreeSet<String>>,
}

impl WholeSource {
    fn record_stem_file(&mut self, stem: &str, path: &str) {
        self.stem_files.entry(stem.to_owned()).or_default().insert(shape::file(path).to_owned());
    }

    fn has_stem_file(&self, stem: &str, file: &str) -> bool {
        self.stem_files.get(stem).is_some_and(|files| files.contains(file))
    }
}

impl WholeScope {
    #[must_use]
    pub fn from_spec(extracts: &[Extract], spec: &Spec) -> Self {
        let mut sources = BTreeMap::new();
        let mut details = Vec::new();
        let mut standing_triggers: BTreeMap<Trigger, BTreeSet<String>> = BTreeMap::new();

        for extract in extracts.iter().filter(|extract| extract.is_whole_behaviour()) {
            let entry = sources.entry(extract.source.clone()).or_insert_with(WholeSource::default);
            entry.narrowed = extract.is_narrowed();
            for claim in &extract.evidence.claims {
                if claim.kind == ClaimKind::Contract
                    && let Some(Ok(trigger)) = claim.trigger()
                {
                    entry.triggers.insert(trigger);
                }
                if let Some(detail) = Detail::from_claim(&extract.source, claim) {
                    details.push(detail);
                }
                if let (Some(id), Some(path)) = (claim.id.as_deref(), claim.path.as_deref()) {
                    entry.record_stem_file(revision::stem(id), path);
                }
            }
        }

        for requirement in &spec.requirements {
            let stem = requirement.stem();
            if let Some(trigger) = &requirement.trigger {
                standing_triggers
                    .entry(trigger.clone())
                    .or_default()
                    .insert(requirement.subject.clone());
            }
            for cited in &requirement.sources {
                if let Some(path) = cited.path.as_deref()
                    && let Some(whole) = sources.get_mut(&cited.source)
                {
                    whole.record_stem_file(stem, path);
                }
            }
        }

        Self {
            sources,
            details,
            standing_triggers,
        }
    }

    #[must_use]
    pub fn has_other_standing_trigger(&self, subject: &str, trigger: &Trigger) -> bool {
        self.standing_triggers
            .get(trigger)
            .is_some_and(|subjects| subjects.iter().any(|other| other != subject))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    #[must_use]
    pub fn contains_source(&self, source: &str) -> bool {
        self.sources.contains_key(source)
    }

    #[must_use]
    pub fn cites_only_whole(&self, citations: &[Cited]) -> bool {
        !citations.is_empty() && citations.iter().all(|cited| self.contains_source(&cited.source))
    }

    #[must_use]
    pub fn has_contract(&self, source: &str, trigger: &Trigger) -> bool {
        self.sources.get(source).is_some_and(|whole| whole.triggers.contains(trigger))
    }

    #[must_use]
    pub fn has_live_contract(&self, citations: &[Cited], trigger: &Trigger) -> bool {
        citations.iter().any(|cited| self.has_contract(&cited.source, trigger))
    }

    #[must_use]
    pub fn vanished_trigger(
        &self, citations: &[Cited], trigger: Option<&Trigger>,
    ) -> Option<Trigger> {
        if !self.cites_only_whole(citations) {
            return None;
        }
        let trigger = trigger?;
        (!self.has_live_contract(citations, trigger)).then(|| trigger.clone())
    }

    #[must_use]
    pub fn standing_trigger_note(&self, citations: &[Cited], trigger: &Trigger) -> String {
        let note = trigger_note(trigger);
        if self.vanished_trigger(citations, Some(trigger)).is_some() {
            format!("{note} — absent from extracted contracts")
        } else {
            note
        }
    }

    #[must_use]
    pub fn admits_evidenced(
        &self, stem: &str, trigger: Option<&Trigger>, citations: &[Cited],
    ) -> bool {
        self.evidenced_gap(stem, trigger, citations).is_none()
    }

    const fn has_details(&self) -> bool {
        !self.details.is_empty()
    }

    fn details(&self) -> &[Detail] {
        &self.details
    }

    fn narrowed_sources(&self) -> Option<String> {
        self.quoted_sources(true)
    }

    fn full_sources(&self) -> Option<String> {
        self.quoted_sources(false)
    }

    fn quoted_sources(&self, narrowed: bool) -> Option<String> {
        let names: Vec<String> = self
            .sources
            .iter()
            .filter(|(_, whole)| whole.narrowed == narrowed)
            .map(|(name, _)| format!("`{name}`"))
            .collect();
        (!names.is_empty()).then(|| names.join(", "))
    }

    fn evidenced_gap(
        &self, stem: &str, trigger: Option<&Trigger>, citations: &[Cited],
    ) -> Option<EvidencedGap> {
        if !self.cites_only_whole(citations) {
            return None;
        }
        if let Some(vanished) = self.vanished_trigger(citations, trigger) {
            return Some(EvidencedGap::MissingContract(vanished));
        }
        let has_detail = self.details.iter().any(|detail| {
            citations.iter().any(|cited| cited.same_file(&detail.cited))
                || trigger.is_some_and(|live| {
                    self.has_contract(&detail.cited.source, live)
                        && citations.iter().any(|cited| cited.source == detail.cited.source)
                        && self.matches_stem(&detail.cited, stem)
                })
        });
        (!has_detail).then_some(EvidencedGap::MissingDetail)
    }

    fn matches_stem(&self, cited: &Cited, stem: &str) -> bool {
        let Some(path) = cited.path.as_deref() else {
            return false;
        };
        let file = shape::file(path);
        file.split('/').any(|part| part.split('.').next() == Some(stem))
            || self.sources.get(&cited.source).is_some_and(|whole| whole.has_stem_file(stem, file))
    }
}

// Non-requirement detail (`excerpt` or `call`) extracted from a whole-scope
// behaviour source, shown so a standing requirement still implemented there is
// not retired by scope when an extraction pass emitted no `requirement` claim
// for it.
#[derive(Debug, Clone)]
struct Detail {
    cited: Cited,
    text: String,
}

impl Detail {
    fn from_claim(source: &str, claim: &Claim) -> Option<Self> {
        let collapse = |raw: &str| raw.split_whitespace().collect::<Vec<_>>().join(" ");
        let text = match claim.kind {
            ClaimKind::Excerpt => collapse(claim.extra("excerpt")?),
            ClaimKind::Call => {
                let callee = claim.callee()?;
                match claim.synopsis.as_deref().map(collapse) {
                    Some(synopsis) if !synopsis.is_empty() => format!("{callee} — {synopsis}"),
                    _ => callee.to_owned(),
                }
            }
            _ => return None,
        };
        if text.is_empty() {
            return None;
        }
        Some(Self {
            cited: Cited::from_claim(source, claim),
            text,
        })
    }
}

impl Display for Detail {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.cited.path {
            Some(path) => write!(f, "- `{}` `{path}` — {}", self.cited.source, self.text),
            None => write!(f, "- `{}` — {}", self.cited.source, self.text),
        }
    }
}

#[derive(Debug, Default)]
struct Contracts(Vec<Contract>);

impl Contracts {
    fn from_extract(extract: &Extract) -> Self {
        let items = extract
            .evidence
            .claims
            .iter()
            .filter(|claim| claim.kind == ClaimKind::Contract)
            .filter_map(|claim| Contract::from_claim(&extract.source, claim))
            .collect();
        Self(items)
    }

    const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    fn by_id(&self, id: &str) -> Option<Trigger> {
        self.0
            .iter()
            .filter(|contract| contract.leads(id))
            .max_by_key(|contract| contract.id.as_deref().map_or(0, str::len))
            .map(|contract| contract.trigger.clone())
    }

    fn by_source_id(&self, source: &str, id: &str) -> Option<Trigger> {
        self.0
            .iter()
            .filter(|contract| contract.cited.source == source && contract.leads(id))
            .max_by_key(|contract| contract.id.as_deref().map_or(0, str::len))
            .map(|contract| contract.trigger.clone())
    }

    fn by_anchor(&self, cited: &Cited) -> Option<Trigger> {
        self.0
            .iter()
            .find(|contract| cited.overlaps(&contract.cited))
            .map(|contract| contract.trigger.clone())
    }

    fn trigger_for(&self, cited: &Cited) -> Option<Trigger> {
        self.by_id(&cited.claim).or_else(|| self.by_anchor(cited))
    }

    fn trigger_in_source(&self, cited: &Cited) -> Option<Trigger> {
        self.by_source_id(&cited.source, &cited.claim).or_else(|| self.by_anchor(cited))
    }

    // The trigger a contract binds to `members`: by id first (the longest
    // matching contract id for the member highest in authority), else by
    // overlapping anchor in the same authority order.
    fn trigger_of<'a>(
        &self, members: impl IntoIterator<Item = &'a Contributor>,
    ) -> Option<Trigger> {
        let mut ordered: Vec<&Contributor> = members.into_iter().collect();
        ordered.sort_by_key(|member| (member.rank, member.index));
        ordered
            .iter()
            .find_map(|member| self.by_id(&member.id))
            .or_else(|| ordered.iter().find_map(|member| self.by_anchor(&Cited::from(*member))))
    }

    fn trigger_in_source_of<'a>(
        &self, source: &str, members: impl IntoIterator<Item = &'a Contributor>,
    ) -> Option<Trigger> {
        let mut ordered: Vec<&Contributor> =
            members.into_iter().filter(|member| member.source == source).collect();
        ordered.sort_by_key(|member| (member.rank, member.index));
        ordered
            .iter()
            .find_map(|member| self.by_source_id(source, &member.id))
            .or_else(|| ordered.iter().find_map(|member| self.by_anchor(&Cited::from(*member))))
    }

    fn stems_for<'a>(
        &self, members: impl IntoIterator<Item = &'a Contributor>,
    ) -> BTreeSet<String> {
        let mut stems = BTreeSet::new();
        for member in members {
            let cited = Cited::from(member);
            for contract in self.0.iter().filter(|contract| contract.matches(&cited)) {
                let stem = contract.id.as_deref().map_or_else(|| member.stem(), revision::stem);
                stems.insert(stem.to_owned());
            }
        }
        stems
    }
}

// A `contract` claim carrying a trigger: what binds it to a requirement is
// its id, which the requirement's subject leads with, or its anchor, which a
// contributor's citation meets.
#[derive(Debug)]
struct Contract {
    cited: Cited,
    id: Option<String>,
    trigger: Trigger,
}

impl Contract {
    fn from_claim(source: &str, claim: &Claim) -> Option<Self> {
        let trigger = claim.trigger()?.ok()?;
        Some(Self {
            cited: Cited::from_claim(source, claim),
            id: claim.id.clone(),
            trigger,
        })
    }

    // Whether `id` is this contract's id or a dotted child of it.
    fn leads(&self, id: &str) -> bool {
        self.id.as_deref().is_some_and(|own| revision::leads(own, id))
    }

    fn matches(&self, cited: &Cited) -> bool {
        self.leads(&cited.claim) || cited.overlaps(&self.cited)
    }
}

// A hint to the grouping, never a pre-merge: either a change claim anchored
// where a standing requirement was read, or claims bound to a live whole-scope
// surface whose standing requirement was read in another file (such as a thin
// route wrapper over a service module).
#[derive(Debug)]
enum Hint {
    Anchor { claim: usize, standing: usize, path: String },
    Surface { trigger: Trigger, claims: Vec<usize>, standing: Vec<usize> },
}

impl Hint {
    // One anchor hint per claim and standing requirement it anchors within (a
    // claim at a standing requirement's id is pre-merged already, so none is
    // hinted for it), followed by one live-surface hint per whole-scope
    // surface that still has an unhinted standing requirement.
    fn all(
        claims: &[Contributor], standing: &[Contributor], contracts: &Contracts, whole: &WholeScope,
    ) -> Vec<Self> {
        if standing.is_empty() {
            return Vec::new();
        }
        let (mut hints, anchor_hinted) = Self::anchor_hints(claims, standing);
        hints.extend(Self::surface_hints(claims, standing, contracts, whole, &anchor_hinted));
        hints
    }

    fn anchor_hints(
        claims: &[Contributor], standing: &[Contributor],
    ) -> (Vec<Self>, BTreeSet<usize>) {
        let mut hints = Vec::new();
        let mut hinted_standing = BTreeSet::new();
        for claim in claims {
            let Some(path) = claim.path.as_deref() else { continue };
            if standing.iter().any(|member| member.id == claim.id) {
                continue;
            }
            let cited = Cited::from(claim);
            for member in standing {
                if member.citations.iter().any(|read| cited.overlaps(read)) {
                    hinted_standing.insert(member.index);
                    hints.push(Self::Anchor {
                        claim: claim.index,
                        standing: member.index,
                        path: path.to_owned(),
                    });
                }
            }
        }
        (hints, hinted_standing)
    }

    fn surface_hints(
        claims: &[Contributor], standing: &[Contributor], contracts: &Contracts,
        whole: &WholeScope, anchor_hinted: &BTreeSet<usize>,
    ) -> Vec<Self> {
        if whole.is_empty() || contracts.is_empty() {
            return Vec::new();
        }
        let mut hints = Vec::new();
        let mut seen_triggers = BTreeSet::new();
        for member in standing {
            let Some(trigger) = &member.trigger else { continue };
            let pre_merged = claims.iter().any(|claim| !claim.withdraws && claim.id == member.id);
            if anchor_hinted.contains(&member.index)
                || pre_merged
                || !whole.has_live_contract(&member.citations, trigger)
                || !seen_triggers.insert(trigger.clone())
            {
                continue;
            }
            let bound_claims: Vec<usize> = claims
                .iter()
                .filter(|claim| !claim.withdraws && whole.contains_source(&claim.source))
                .filter(|claim| {
                    contracts.trigger_in_source(&Cited::from(*claim)).as_ref() == Some(trigger)
                })
                .map(|claim| claim.index)
                .collect();
            if bound_claims.is_empty() {
                continue;
            }
            let standing_on_surface: Vec<usize> = standing
                .iter()
                .filter(|other| other.trigger.as_ref() == Some(trigger))
                .map(|other| other.index)
                .collect();
            hints.push(Self::Surface {
                trigger: trigger.clone(),
                claims: bound_claims,
                standing: standing_on_surface,
            });
        }
        hints
    }
}

impl Display for Hint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Anchor {
                claim,
                standing,
                path,
            } => write!(
                f,
                "- claim {claim} anchors at `{path}`, where standing requirement {standing} was \
                 read; they are one requirement unless the code moved."
            ),
            Self::Surface {
                trigger,
                claims,
                standing,
            } => {
                let lead = match claims.as_slice() {
                    [sole] => format!("claim {sole} shares"),
                    many => format!("claims {} share", join_indices(many)),
                };
                let surface = trigger_note(trigger);
                if let [sole] = standing.as_slice() {
                    write!(
                        f,
                        "- {lead} {surface} with standing requirement {sole} across files; match \
                         a claim that states or revises its behaviour to standing requirement \
                         {sole} (in its class when unchanged)."
                    )
                } else {
                    let standing_list = join_indices(standing);
                    write!(
                        f,
                        "- {lead} {surface} with standing requirements {standing_list} across \
                         files; match each claim to the standing requirement whose behaviour it \
                         states or revises, and list any other standing requirement on the \
                         surface whose behaviour a claim repeats unchanged in `restated`."
                    )
                }
            }
        }
    }
}

/// A partition of requirement claims into requirements and agreement classes.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery grouping answer")]
pub struct Grouping {
    /// One group per requirement.
    groups: Vec<Group>,
    /// Change claims that state nothing beyond what the listed standing
    /// requirements already state.
    #[serde(default)]
    restated: Vec<Restated>,
    /// Standing requirement indices that stand alone in `groups` yet are still
    /// evidenced unchanged in `## Whole-source detail`.
    #[serde(default)]
    evidenced: Vec<usize>,
}

/// A change claim that only restates one or more standing requirements.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Restated {
    /// Index of the change claim.
    claim: usize,
    /// Standing requirement indices whose statements and scenarios already
    /// cover `claim`.
    standing: Vec<usize>,
}

impl Grouping {
    // Byte-equal ids are one group, in first-appearance order, and
    // whitespace-equal statements one class; every answer must contain this
    // grouping.
    fn baseline(contributors: &[Contributor]) -> Self {
        let mut groups: Vec<Group> = Vec::new();
        let mut by_id: BTreeMap<String, usize> = BTreeMap::new();
        for (index, claim) in contributors.iter().enumerate() {
            let position = *by_id.entry(claim.id.clone()).or_insert_with(|| {
                groups.push(Group::default());
                groups.len() - 1
            });
            let group = &mut groups[position];
            group.claims.push(index);
            let class = group.classes.iter_mut().find(|class| {
                let lead = &contributors[class[0]];
                lead.withdraws == claim.withdraws && lead.statement == claim.statement
            });

            match class {
                Some(class) => class.push(index),
                None => group.classes.push(vec![index]),
            }
        }

        Self {
            groups,
            restated: Vec::new(),
            evidenced: Vec::new(),
        }
    }

    // Without a standing requirement, one source's distinct ids under one stem
    // are distinct requirements — the call that minted them under one noun told
    // them apart — so a group that merges them is split without a round. In a
    // group matched to a standing requirement, distinct ids of one `(source,
    // stem)` placed in one agreeing class stay together as facets of that
    // requirement, while any placed in another class spin out so one source
    // never disagrees with itself.
    fn split(self, contributors: &[Contributor], changes: usize) -> Self {
        let mut groups = Vec::with_capacity(self.groups.len());
        for group in self.groups {
            let spun = group.spun_ids(contributors, changes);
            if spun.is_empty() {
                groups.push(group);
                continue;
            }
            let id_of = |index: usize| contributors.get(index).map(|claim| claim.id.as_str());
            groups.push(group.filtered(|index| !id_of(index).is_some_and(|id| spun.contains(id))));
            for id in &spun {
                groups.push(group.filtered(|index| id_of(index) == Some(id.as_str())));
            }
        }
        Self {
            groups,
            restated: self.restated,
            evidenced: self.evidenced,
        }
    }

    fn restated_standing(&self) -> BTreeSet<usize> {
        self.restated.iter().flat_map(|entry| entry.standing.iter().copied()).collect()
    }

    // Ordered by each group's earliest claim and numbered from `REQ-001` in
    // that order.
    fn bases(
        &self, contributors: &[Contributor], criteria: &[String], contracts: &Contracts,
    ) -> Result<Vec<Basis>, Error> {
        let restated = self.restated_standing();
        let evidenced: BTreeSet<usize> = self.evidenced.iter().copied().collect();
        let mut groups: Vec<OrderedGroup> = Vec::with_capacity(self.groups.len());
        for group in &self.groups {
            let first = group.claims.iter().copied().min().unwrap_or_default();
            let last = group.claims.iter().copied().max().unwrap_or_default();
            let retained = if group.claims.iter().any(|index| restated.contains(index)) {
                Some(Retained::Restated)
            } else if let [sole] = group.claims.as_slice()
                && evidenced.contains(sole)
            {
                Some(Retained::Evidenced)
            } else {
                None
            };
            let mut classes = Vec::with_capacity(group.classes.len());
            for class in &group.classes {
                let members = class
                    .iter()
                    .map(|&index| {
                        contributors.get(index).cloned().ok_or_else(|| {
                            server_error!("the grouping names claim {index}, which does not exist")
                        })
                    })
                    .collect::<Result<_, _>>()?;
                classes.push(members);
            }
            groups.push(OrderedGroup {
                order: (first, last),
                classes,
                retained,
            });
        }
        groups.sort_by_key(|group| group.order);
        groups
            .into_iter()
            .zip(ReqId::sequence())
            .map(|(group, id)| Basis::of(id, group.classes, criteria, contracts, group.retained))
            .collect()
    }
}

struct OrderedGroup {
    order: (usize, usize),
    classes: Vec<Vec<Contributor>>,
    retained: Option<Retained>,
}

/// The claims assigned to one requirement and their agreement classes.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Group {
    /// Indices of every claim describing this requirement.
    claims: Vec<usize>,
    /// Partitions of `claims` that express equivalent statements.
    classes: Vec<Vec<usize>>,
}

impl Group {
    fn spun_ids(&self, contributors: &[Contributor], changes: usize) -> BTreeSet<String> {
        let has_standing = self.claims.iter().any(|&index| index >= changes);
        if has_standing {
            let mut lead_class: BTreeMap<(String, String), (String, Option<usize>)> =
                BTreeMap::new();
            let mut split_ids = BTreeSet::new();
            for &index in self.claims.iter().filter(|&&index| index < changes) {
                let Some(claim) = contributors.get(index) else { continue };
                let class_pos = self.classes.iter().position(|class| class.contains(&index));
                let (lead_id, first_class) = lead_class
                    .entry((claim.source.clone(), claim.stem().to_owned()))
                    .or_insert_with(|| (claim.id.clone(), class_pos));
                if claim.id != *lead_id && class_pos != *first_class {
                    split_ids.insert(claim.id.clone());
                }
            }
            return split_ids;
        }

        let mut by_stem: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
        for claim in self.claims.iter().filter_map(|&index| contributors.get(index)) {
            let ids = by_stem.entry((claim.source.clone(), claim.stem().to_owned())).or_default();
            if !ids.contains(&claim.id) {
                ids.push(claim.id.clone());
            }
        }
        by_stem
            .into_iter()
            .filter(|(_, ids)| ids.len() > 1)
            .inspect(|((source, stem), ids)| {
                tracing::debug!(
                    source,
                    stem,
                    ?ids,
                    "the grouping merged one source's distinct ids under one stem; split"
                );
            })
            .flat_map(|(_, ids)| ids.into_iter().skip(1))
            .collect()
    }

    fn filtered(&self, keep: impl Fn(usize) -> bool) -> Self {
        Self {
            claims: self.claims.iter().copied().filter(|&index| keep(index)).collect(),
            classes: self
                .classes
                .iter()
                .map(|class| class.iter().copied().filter(|&index| keep(index)).collect())
                .filter(|class: &Vec<usize>| !class.is_empty())
                .collect(),
        }
    }
}

// How an untouched standing requirement was retained against whole-source
// scope retirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retained {
    // Covered unchanged by a change claim in `Grouping::restated`.
    Restated,
    // Backed unchanged by `## Whole-source detail` in `Grouping::evidenced`.
    Evidenced,
}

// Reconciled facts used to construct one requirement.
//
// Contributors are grouped by agreement, with the winning class first.
#[derive(Debug, Clone)]
pub struct Basis {
    // The requirement identifier, assigned from `REQ-001`.
    pub id: ReqId,
    // The heading derived from the highest-authority claim identifier.
    pub subject: String,
    // The reconciliation outcome.
    pub status: Status,
    // Whether any criterion claim covers the requirement.
    pub covered: bool,
    // What reaches the requirement, where a `contract` claim binds it.
    pub trigger: Option<Trigger>,
    // Agreement classes, with the winning class first.
    pub classes: Vec<Vec<Contributor>>,
    // How a standing requirement was retained against whole-source scope
    // retirement when no change claim joined its group.
    pub retained: Option<Retained>,
    // Stems of contracts bound by non-`BASELINE` contributors in `classes`.
    contract_stems: BTreeSet<String>,
}

impl Basis {
    fn of(
        id: ReqId, mut classes: Vec<Vec<Contributor>>, criteria: &[String], contracts: &Contracts,
        retained: Option<Retained>,
    ) -> Result<Self, Error> {
        // refuse an empty class, so every class below has a lead
        if classes.is_empty() || classes.iter().any(Vec::is_empty) {
            return Err(server_error!("requirement {id} was grouped with a class of no claims"));
        }
        for class in &mut classes {
            class.sort_by_key(|member| (member.rank, member.index));
        }
        classes.sort_by_key(|class| (class[0].rank, class[0].index));

        // covered by a criterion at the claim id or a dotted child of it
        let covered = classes
            .iter()
            .flatten()
            .any(|member| criteria.iter().any(|id| revision::leads(&member.id, id)));

        let status = Self::settle(&classes, covered);
        let trigger = contracts.trigger_of(classes.iter().flatten());
        let contract_stems =
            contracts.stems_for(classes.iter().flatten().filter(|member| !member.is_baseline()));

        Ok(Self {
            id,
            subject: classes[0][0].id.clone(),
            status,
            covered,
            trigger,
            classes,
            retained,
            contract_stems,
        })
    }

    // The status sorted, non-empty `classes` settle to: one class agreed or
    // unknown by `covered`, a class alone at the top rank a divergence, two
    // tied there a conflict.
    fn settle(classes: &[Vec<Contributor>], covered: bool) -> Status {
        let top = classes[0][0].rank;
        match classes.len() {
            1 if covered => Status::Agreed,
            1 => Status::Unknown,
            _ if classes.iter().skip(1).all(|class| class[0].rank != top) => Status::Divergence,
            _ => Status::Conflict,
        }
    }

    // Returns the basis as an amendment drafts its requirement from it.
    //
    // The requirement takes `id` and `trigger`, and keeps the subject, the
    // change's since the base is ranked below it. The `BASELINE`
    // contributors are left out, since the standing requirement is what the
    // change revises and not a source of it. Status is settled over the
    // change sources alone, while coverage stands as the grouping settled
    // it. Returns `None` when only the baseline contributed.
    #[must_use]
    pub fn amended(mut self, id: ReqId, trigger: Option<Trigger>) -> Option<Self> {
        for class in &mut self.classes {
            class.retain(|member| !member.is_baseline());
        }
        self.classes.retain(|class| !class.is_empty());
        if self.classes.is_empty() {
            return None;
        }
        Some(Self {
            id,
            subject: self.subject,
            status: Self::settle(&self.classes, self.covered),
            covered: self.covered,
            trigger,
            classes: self.classes,
            retained: None,
            contract_stems: self.contract_stems,
        })
    }

    // Returns whether the standing `BASELINE` contributor belongs to the
    // winning agreement class.
    #[must_use]
    pub fn agrees_with_baseline(&self) -> bool {
        self.classes.first().is_some_and(|class| class.iter().any(Contributor::is_baseline))
    }

    // Returns the non-`BASELINE` contributors in the winning agreement class.
    pub fn winning_changes(&self) -> impl Iterator<Item = &Contributor> {
        self.classes.first().into_iter().flatten().filter(|member| !member.is_baseline())
    }

    // Returns whether the winning agreement class holds a non-`BASELINE`
    // contributor under `stem`.
    #[must_use]
    pub fn winning_class_has_stem(&self, stem: &str) -> bool {
        self.winning_changes().any(|member| member.stem() == stem)
    }

    // Returns whether any contract bound to a change contributor of this basis
    // belongs to `stem`.
    #[must_use]
    pub fn has_contract_under(&self, stem: &str) -> bool {
        self.contract_stems.contains(stem)
    }

    // Returns the stem of the subject: the first segment of its dotted id.
    #[must_use]
    pub fn stem(&self) -> &str {
        revision::stem(&self.subject)
    }

    // Returns contributors by descending authority and then source order.
    pub fn contributors(&self) -> impl Iterator<Item = &Contributor> {
        let mut members: Vec<&Contributor> = self.classes.iter().flatten().collect();
        members.sort_by_key(|member| (member.rank, member.index));
        members.into_iter()
    }

    // Returns non-`BASELINE` contributors by descending authority and then
    // source order.
    pub fn change_contributors(&self) -> impl Iterator<Item = &Contributor> {
        self.contributors().filter(|member| !member.is_baseline())
    }

    // Returns the sole non-`BASELINE` contributor when exactly one change
    // claim contributed to this basis.
    #[must_use]
    pub fn sole_change_contributor(&self) -> Option<&Contributor> {
        let mut iter = self.classes.iter().flatten().filter(|member| !member.is_baseline());
        let sole = iter.next()?;
        iter.next().is_none().then_some(sole)
    }

    // Returns the winning statement, or `None` when the contributors are in
    // conflict or the winning claim withdraws the requirement. Distinct facet
    // ids from the winning `(source, stem)` merged onto one standing
    // requirement join in source order so no facet is dropped when drafted.
    #[must_use]
    pub fn statement(&self) -> Option<String> {
        match self.status {
            Status::Conflict => None,
            Status::Agreed | Status::Unknown | Status::Divergence => {
                let winning = &self.classes[0];
                let lead = &winning[0];
                if lead.withdraws {
                    return None;
                }
                let mut seen_ids = BTreeSet::from([lead.id.as_str()]);
                let mut statements = vec![lead.statement.as_str()];
                for member in winning.iter().skip(1) {
                    if !member.withdraws
                        && member.source == lead.source
                        && member.stem() == lead.stem()
                        && seen_ids.insert(member.id.as_str())
                        && !statements.contains(&member.statement.as_str())
                    {
                        statements.push(member.statement.as_str());
                    }
                }
                Some(statements.join(" "))
            }
        }
    }

    // Returns whether the winning claim withdraws the requirement.
    //
    // A `retire` claim leading the winning class withdraws the behaviour
    // the group's standing requirement states, so the requirement is
    // retired rather than restated. A conflict has no winner and answers
    // `false`.
    #[must_use]
    pub fn withdraws(&self) -> bool {
        self.status != Status::Conflict && self.classes[0][0].withdraws
    }

    // Returns the `retire` claims among the contributors, by descending authority.
    pub fn withdrawals(&self) -> impl Iterator<Item = &Contributor> {
        self.contributors().filter(|member| member.withdraws)
    }

    // Returns the requirement produced from this basis and `scenarios`.
    //
    // The body is the winning statement, or none for a conflict. The losing
    // classes become notes. The trigger is the basis's own, bound from the
    // sources' `contract` claims.
    #[must_use]
    pub fn requirement(&self, scenarios: Vec<Scenario>) -> Requirement {
        let body = self.statement().map_or_default(|statement| vec![statement]);
        let noted: &[Vec<Contributor>] = match self.status {
            Status::Agreed | Status::Unknown => &[],
            Status::Divergence => &self.classes[1..],
            Status::Conflict => &self.classes,
        };
        let losers = noted
            .iter()
            .map(|class| {
                let lead = &class[0];
                Loser {
                    sources: class.iter().map(|member| member.source.clone()).collect(),
                    kind: lead.kind,
                    rank: lead.rank,
                    claim: lead.id.clone(),
                    statement: lead.statement.clone(),
                }
            })
            .collect();

        Requirement {
            id: self.id,
            subject: self.subject.clone(),
            status: self.status,
            covered: self.covered,
            trigger: self.trigger.clone(),
            sources: self.contributors().map(Cited::from).collect(),
            body,
            losers,
            scenarios,
        }
    }
}

// A source claim contributing to a requirement.
#[derive(Debug, Clone)]
pub struct Contributor {
    pub source: String,
    pub kind: SourceKind,
    pub rank: Rank,
    pub id: String,
    pub path: Option<String>,
    pub statement: String,
    pub synopsis: Option<String>,
    pub notes: Vec<String>,
    pub withdraws: bool,
    // What reaches a standing requirement; `None` for a claim.
    pub trigger: Option<Trigger>,
    // Where a standing requirement was read, for the brief's anchor hints;
    // empty for a claim.
    pub citations: Vec<Cited>,
    // Position in source order, used to break authority ties.
    pub index: usize,
}

impl Contributor {
    #[must_use]
    pub fn stem(&self) -> &str {
        revision::stem(&self.id)
    }

    #[must_use]
    pub fn is_baseline(&self) -> bool {
        self.source == BASELINE
    }
}

impl From<&Contributor> for Cited {
    fn from(member: &Contributor) -> Self {
        Self {
            source: member.source.clone(),
            claim: member.id.clone(),
            path: member.path.clone(),
        }
    }
}
