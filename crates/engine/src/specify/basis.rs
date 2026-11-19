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
use crate::revision::{self, Cited, Loser, ReqId, Requirement, Scenario, Status};
use crate::specify::Extract;

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
    contracts: Vec<Contract>,
    sources: usize,
    baseline: Grouping,
    // How many contributors the run's sources contributed; the rest are
    // the standing requirements.
    changes: usize,
    // The claims anchored where a standing requirement was read.
    hints: Vec<Hint>,
    // Non-requirement detail (`excerpt` and `call` claims) from whole-scope
    // behaviour sources during an amend run.
    details: Vec<Detail>,
    // Names of whole-scope behaviour sources reconciled against standing
    // requirements.
    whole_sources: BTreeSet<String>,
}

impl GroupingBrief {
    // A `retire` claim is passed over: with no standing requirements there
    // is nothing for it to withdraw.
    #[must_use]
    pub fn new(extracts: &[Extract]) -> Self {
        Self::assemble(extracts, Vec::new(), Vec::new(), false)
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
        extracts: &[Extract], standing: Vec<Contributor>, criteria: Vec<String>,
    ) -> Self {
        Self::assemble(extracts, standing, criteria, true)
    }

    fn assemble(
        extracts: &[Extract], standing: Vec<Contributor>, mut criteria: Vec<String>,
        withdrawals: bool,
    ) -> Self {
        let mut contributors: Vec<Contributor> = Vec::new();
        let mut contracts = Vec::new();
        let mut details = Vec::new();
        let mut whole_sources = BTreeSet::new();
        let has_standing = !standing.is_empty();

        for extract in extracts {
            let extract_notes = ExtractNotes::new(extract, has_standing);
            if extract_notes.whole_behaviour {
                whole_sources.insert(extract.source.clone());
            }
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
                if extract_notes.whole_behaviour
                    && let Some(detail) = Detail::from_claim(&extract.source, claim)
                {
                    details.push(detail);
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
                let notes = if withdraws {
                    Vec::new()
                } else {
                    extract_notes.for_claim(id, claim.path.as_deref())
                };
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
            contracts.extend(extract_notes.contracts);
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
        let hints = Hint::all(&contributors[..changes], &contributors[changes..]);

        Self {
            contributors,
            criteria,
            contracts,
            sources,
            baseline,
            changes,
            hints,
            details,
            whole_sources,
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

    fn claims(&self) -> &[Contributor] {
        &self.contributors[..self.changes]
    }

    fn standing(&self) -> &[Contributor] {
        &self.contributors[self.changes..]
    }

    fn verify_placements(
        &self, placed: &BTreeMap<usize, Vec<usize>>, revising: &BTreeMap<usize, Vec<usize>>,
        group_has_standing: &[bool], review: &mut Review,
    ) {
        let count = self.contributors.len();
        let has_standing = !self.standing().is_empty();

        // a change claim may join multiple groups only when it covers distinct
        // standing requirements and revises at most one of them
        for (&index, groups) in placed {
            if groups.len() <= 1 {
                continue;
            }
            if !has_standing || index >= self.changes {
                review.note(format_args!("claim {index} appears in more than one group"));
                continue;
            }
            if groups.iter().any(|&pos| !group_has_standing[pos]) {
                review.note(format_args!(
                    "claim {index} appears in more than one group, including one without a \
                     standing requirement; a claim may join multiple groups only when each holds \
                     a distinct standing requirement it covers"
                ));
                continue;
            }
            if revising.get(&index).is_some_and(|revisions| revisions.len() > 1) {
                review.note(format_args!(
                    "claim {index} is placed outside the standing requirement's class in more \
                     than one group; share the standing requirement's class for each standing \
                     requirement whose behaviour stands unchanged"
                ));
            }
        }

        for index in (0..count).filter(|index| !placed.contains_key(index)) {
            review.note(format_args!("claim {index} is in no group"));
        }

        // byte-equal ids may not be split across groups
        for group in &self.baseline.groups {
            if group.claims.len() <= 1
                || !group.claims.iter().all(|index| placed.contains_key(index))
            {
                continue;
            }
            let mut shared: Option<BTreeSet<usize>> = None;
            for index in &group.claims {
                let positions: BTreeSet<usize> =
                    placed.get(index).into_iter().flatten().copied().collect();
                shared = Some(match shared {
                    None => positions,
                    Some(prev) => prev.intersection(&positions).copied().collect(),
                });
            }
            if shared.is_some_and(|common| common.is_empty()) {
                let id = &self.contributors[group.claims[0]].id;
                review.note(format_args!("claims sharing the id `{id}` are split across groups"));
            }
        }
    }

    fn verify_evidenced(
        &self, answer: &Grouping, placed: &BTreeMap<usize, Vec<usize>>, review: &mut Review,
    ) {
        if self.details.is_empty() {
            if !answer.evidenced.is_empty() {
                review.note(
                    "`evidenced` is only used when `## Whole-source detail` is listed; leave it \
                     empty",
                );
            }
            return;
        }
        let count = self.contributors.len();
        let mut seen_evidenced = BTreeSet::new();
        for &index in &answer.evidenced {
            if index < self.changes || index >= count {
                review
                    .note(format_args!("`evidenced` index {index} is not a standing requirement"));
            } else if !seen_evidenced.insert(index) {
                review.note(format_args!(
                    "standing requirement {index} appears in `evidenced` more than once"
                ));
            } else if placed
                .get(&index)
                .and_then(|positions| positions.first())
                .and_then(|&pos| answer.groups.get(pos))
                .is_some_and(|group| group.claims.len() > 1)
            {
                review.note(format_args!(
                    "standing requirement {index} is already matched to a claim; list in \
                     `evidenced` only standing requirements that stand alone"
                ));
            } else {
                match self.evidenced_gap(&self.contributors[index]) {
                    Some(EvidencedGap::MissingContract(trigger)) => review.note(format_args!(
                        "standing requirement {index} (`{}`) carries {}, which no contract of its \
                         cited whole-scope source declares; omit it from `evidenced`",
                        self.contributors[index].id,
                        trigger_note(&trigger),
                    )),
                    Some(EvidencedGap::MissingDetail) => review.note(format_args!(
                        "standing requirement {index} (`{}`) has no `## Whole-source detail` in \
                         any file its citations name; omit it from `evidenced`",
                        self.contributors[index].id,
                    )),
                    None => {}
                }
            }
        }
    }

    fn evidenced_gap(&self, member: &Contributor) -> Option<EvidencedGap> {
        let cites_only_whole = !member.citations.is_empty()
            && member.citations.iter().all(|cited| self.whole_sources.contains(&cited.source));
        if !cites_only_whole {
            return None;
        }
        if let Some(trigger) = &member.trigger {
            let has_contract = self.contracts.iter().any(|contract| {
                contract.trigger == *trigger
                    && member.citations.iter().any(|cited| cited.source == contract.cited.source)
            });
            if !has_contract {
                return Some(EvidencedGap::MissingContract(trigger.clone()));
            }
        }
        let has_detail = self
            .details
            .iter()
            .any(|detail| member.citations.iter().any(|cited| cited.same_file(&detail.cited)));
        (!has_detail).then_some(EvidencedGap::MissingDetail)
    }

    fn write_instructions(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            "\nAnswer with every index in a group, and every group's claims in exactly one \
             agreeing class. Two claims of one source that describe one requirement under \
             different stems — the first segment of their ids — are one group, whatever nouns \
             their seams gave them. Without a standing requirement, two claims of one source \
             with different ids under one stem are distinct requirements: the call that minted \
             them under one noun told them apart, and a group that merges them is split, each id \
             its own requirement.\n",
        )?;
        if !self.standing().is_empty() {
            f.write_str(
                "\nEach standing requirement is one requirement of the current specification, \
                 shown with its scenarios; claims carry any criteria extracted under them. A \
                 claim that restates, revises, or narrows the behaviour a standing requirement \
                 states joins its group, in the standing requirement's class when the behaviour, \
                 values, thresholds, and outcomes match (even if reworded or given another \
                 sub-stem or leaf id) and in a class of its own when a value, a condition, or an \
                 outcome changed. Multiple claims of one source under one stem that together \
                 state facets of one standing requirement join that standing requirement's group \
                 in one class — the standing requirement's class when nothing changed, or one \
                 class of their own when revised — rather than adding separate groups for \
                 already-stated facets. When a single claim consolidates behaviours the current \
                 specification holds as distinct standing requirements, place that claim in each \
                 covered standing requirement's group (in the standing requirement's class for \
                 each whose behaviour is unchanged, and outside it in at most one group). A \
                 claim stating a behaviour no standing requirement covers is a group of its own. \
                 A standing requirement no claim touches is a group of its own too. Two standing \
                 requirements never share a group: an answer that merges them is refused.\n\nA \
                 claim marked `(retire)` withdraws the behaviour its id names rather than \
                 stating one: it joins the group of the standing requirement that states the \
                 behaviour, in a class of its own, and is never a group of its own or of other \
                 claims alone, since a withdrawal of a behaviour the specification does not hold \
                 is refused after your answer.\n",
            )?;
            if !self.whole_sources.is_empty() {
                let sources = self
                    .whole_sources
                    .iter()
                    .map(|name| format!("`{name}`"))
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(
                    f,
                    "\nWhole-scope re-extraction ({sources}): every file of the source was \
                     re-extracted, including files the change did not touch, so untouched \
                     routes, modules, and `start` bootstrap code appear again alongside any \
                     edit. On an existing route, message surface, or `start` bootstrap, a claim \
                     spelling out an inherited precondition or shared-helper outcome already \
                     covered by standing requirements (such as per-action missing-entity lookup \
                     when entity lookup and not-found shaping already stand), a validation or \
                     normalisation facet already covered by a standing requirement or scenario \
                     (including a claim spanning validation rules and default persistence across \
                     two standing requirements, placed in both groups), or process startup, \
                     framework header or middleware setup (`x-powered-by`), and listen mechanics \
                     (`listen` or `serve` after startup checks) joins the \
                     covered standing requirement's group in the standing requirement's class \
                     rather than adding a standalone requirement. Leave a whole-scope claim in a \
                     singleton group only when it states behaviour absent from every standing \
                     requirement and scenario, such as a newly added route or message surface, a \
                     new stem, or a newly added domain rule."
                )?;
            }
            if !self.details.is_empty() {
                f.write_str(
                    "\nWhen a standing requirement stands alone in its group only because this \
                     extraction recorded its behaviour in `## Whole-source detail` as an \
                     `excerpt` or `call` rather than a `requirement` claim, and that detail in \
                     its cited file shows the behaviour (and any route or message trigger it \
                     carries) is still implemented unchanged, list that standing requirement's \
                     index in `evidenced` so whole-source scope does not retire it. Never list \
                     in `evidenced` a standing requirement matched to a claim, one whose route \
                     or message trigger vanished from the extracted contracts, or one whose \
                     cited file holds no matching `## Whole-source detail`.\n",
                )?;
            }
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
        if self.details.is_empty() {
            Self::at(schema, "/properties")?.remove("evidenced");
        } else {
            let items = Self::at(schema, "/properties/evidenced/items")?;
            items.insert("minimum".to_owned(), json!(self.changes));
            items.insert("maximum".to_owned(), json!(last));
        }
        Ok(())
    }

    fn verify(&self, answer: &Grouping, review: &mut Review) {
        let count = self.contributors.len();
        let mut placed: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut revising: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut group_has_standing = Vec::with_capacity(answer.groups.len());

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
                    placed.entry(index).or_default().push(position);
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
                members.iter().copied().filter(|&index| index >= self.changes).collect();
            group_has_standing.push(!standing_indices.is_empty());
            if standing_indices.len() > 1 {
                let standing: BTreeSet<&str> = standing_indices
                    .iter()
                    .filter_map(|&index| {
                        self.contributors.get(index).map(|member| member.id.as_str())
                    })
                    .collect();
                let ids =
                    standing.iter().map(|id| format!("`{id}`")).collect::<Vec<_>>().join(", ");
                review.note(format_args!(
                    "group {position} merges the standing requirements {ids}, which the current \
                     specification holds as distinct requirements"
                ));
            } else if let [standing_index] = standing_indices.as_slice() {
                let standing_class =
                    group.classes.iter().find(|class| class.contains(standing_index));
                for &index in members.iter().filter(|&&index| index < self.changes) {
                    if self.contributors.get(index).is_some_and(|member| !member.withdraws)
                        && !standing_class.is_some_and(|class| class.contains(&index))
                    {
                        revising.entry(index).or_default().push(position);
                    }
                }
            }
        }

        self.verify_placements(&placed, &revising, &group_has_standing, review);
        self.verify_evidenced(answer, &placed, review);
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

        if !self.details.is_empty() {
            f.write_str(
                "\n## Whole-source detail (source, path, detail)\n\nNon-requirement detail \
                 (`excerpt` and `call` claims) extracted from whole-scope sources:\n\n",
            )?;
            for detail in &self.details {
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
            let indices =
                group.claims.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
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
    whole_behaviour: bool,
    criteria: Vec<CriterionNote>,
    contracts: Vec<Contract>,
}

impl ExtractNotes {
    fn new(extract: &Extract, has_standing: bool) -> Self {
        let criteria = if has_standing {
            extract.evidence.claims.iter().filter_map(CriterionNote::from_claim).collect()
        } else {
            Vec::new()
        };
        let contracts = extract
            .evidence
            .claims
            .iter()
            .filter(|claim| claim.kind == ClaimKind::Contract)
            .filter_map(|claim| Contract::from_claim(&extract.source, claim))
            .collect();
        Self {
            source: extract.source.clone(),
            whole_behaviour: has_standing && extract.is_whole_behaviour(),
            criteria,
            contracts,
        }
    }

    fn for_claim(&self, id: &str, path: Option<&str>) -> Vec<String> {
        let mut notes: Vec<String> = self
            .criteria
            .iter()
            .filter(|criterion| revision::leads(id, &criterion.id))
            .map(|criterion| format!("criterion `{}`: {}", criterion.id, criterion.condition))
            .collect();
        if self.whole_behaviour {
            let cited = Cited {
                source: self.source.clone(),
                claim: id.to_owned(),
                path: path.map(str::to_owned),
            };
            if let Some(trigger) = Contract::trigger_for(&self.contracts, &cited) {
                notes.push(trigger_note(&trigger));
            }
            if let Some(path) = path {
                notes.push(format!("at `{path}`"));
            }
        }
        notes
    }
}

pub fn trigger_note(trigger: &Trigger) -> String {
    match trigger {
        Trigger::Http(route) => format!("route `{route}`"),
        Trigger::Message(topic) => format!("message `{topic}`"),
    }
}

enum EvidencedGap {
    MissingContract(Trigger),
    MissingDetail,
}

// Non-requirement detail (`excerpt` or `call`) extracted from a whole-scope
// behaviour source, shown so a standing requirement still implemented there is
// not retired by scope when an extraction pass emitted no `requirement` claim
// for it.
#[derive(Debug)]
pub struct Detail {
    pub cited: Cited,
    text: String,
}

impl Detail {
    #[must_use]
    pub fn from_claim(source: &str, claim: &Claim) -> Option<Self> {
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

    fn trigger_by_id(contracts: &[Self], id: &str) -> Option<Trigger> {
        contracts
            .iter()
            .filter(|contract| contract.leads(id))
            .max_by_key(|contract| contract.id.as_deref().map_or(0, str::len))
            .map(|contract| contract.trigger.clone())
    }

    fn trigger_by_anchor(contracts: &[Self], cited: &Cited) -> Option<Trigger> {
        contracts
            .iter()
            .find(|contract| cited.overlaps(&contract.cited))
            .map(|contract| contract.trigger.clone())
    }

    fn trigger_for(contracts: &[Self], cited: &Cited) -> Option<Trigger> {
        Self::trigger_by_id(contracts, &cited.claim)
            .or_else(|| Self::trigger_by_anchor(contracts, cited))
    }
}

// A change claim anchored where a standing requirement was read: the same
// source and file, lines that meet. A hint to the grouping, never a
// pre-merge, since line numbers drift as code changes.
#[derive(Debug)]
struct Hint {
    claim: usize,
    standing: usize,
    path: String,
}

impl Hint {
    // One hint per claim and standing requirement it anchors within. A claim
    // at a standing requirement's id is pre-merged already, so none is
    // hinted for it.
    fn all(claims: &[Contributor], standing: &[Contributor]) -> Vec<Self> {
        if standing.is_empty() {
            return Vec::new();
        }
        let mut hints = Vec::new();
        for claim in claims {
            let Some(path) = claim.path.as_deref() else { continue };
            if standing.iter().any(|member| member.id == claim.id) {
                continue;
            }
            let cited = Cited::from(claim);
            for member in standing {
                if member.citations.iter().any(|read| cited.overlaps(read)) {
                    hints.push(Self {
                        claim: claim.index,
                        standing: member.index,
                        path: path.to_owned(),
                    });
                }
            }
        }
        hints
    }
}

impl Display for Hint {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "- claim {} anchors at `{}`, where standing requirement {} was read; they are one \
             requirement unless the code moved.",
            self.claim, self.path, self.standing
        )
    }
}

/// A partition of requirement claims into requirements and agreement classes.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery grouping answer")]
pub struct Grouping {
    /// One group per requirement.
    pub groups: Vec<Group>,
    /// Standing requirement indices that stand alone in `groups` yet are still
    /// evidenced unchanged in `## Whole-source detail`.
    #[serde(default)]
    pub evidenced: Vec<usize>,
}

impl Grouping {
    // Byte-equal ids are one group, in first-appearance order, and
    // whitespace-equal statements one class; every answer must contain this
    // grouping.
    fn baseline(contributors: &[Contributor]) -> Self {
        let mut groups: Vec<Group> = Vec::new();
        let mut by_id: BTreeMap<&str, usize> = BTreeMap::new();
        for (index, claim) in contributors.iter().enumerate() {
            let position = *by_id.entry(claim.id.as_str()).or_insert_with(|| {
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
            evidenced: self.evidenced,
        }
    }

    // Ordered by each group's earliest claim and numbered from `REQ-001` in
    // that order.
    fn bases(
        &self, contributors: &[Contributor], criteria: &[String], contracts: &[Contract],
    ) -> Result<Vec<Basis>, Error> {
        let evidenced: BTreeSet<usize> = self.evidenced.iter().copied().collect();
        let mut groups: Vec<OrderedGroup> = Vec::with_capacity(self.groups.len());
        for group in &self.groups {
            let first = group.claims.iter().copied().min().unwrap_or_default();
            let last = group.claims.iter().copied().max().unwrap_or_default();
            let is_evidenced = group.claims.len() == 1 && evidenced.contains(&group.claims[0]);
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
                evidenced: is_evidenced,
            });
        }
        groups.sort_by_key(|group| group.order);
        groups
            .into_iter()
            .zip(ReqId::sequence())
            .map(|(group, id)| Basis::of(id, group.classes, criteria, contracts, group.evidenced))
            .collect()
    }
}

struct OrderedGroup {
    order: (usize, usize),
    classes: Vec<Vec<Contributor>>,
    evidenced: bool,
}

/// The claims assigned to one requirement and their agreement classes.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Indices of every claim describing this requirement.
    pub claims: Vec<usize>,
    /// Partitions of [`Group::claims`] that express equivalent statements.
    pub classes: Vec<Vec<usize>>,
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
    // Whether a standing requirement with no matching `requirement` claim is
    // still evidenced in `## Whole-source detail`.
    pub evidenced: bool,
    // Stems of contracts bound by non-`BASELINE` contributors in `classes`.
    contract_stems: BTreeSet<String>,
}

impl Basis {
    fn of(
        id: ReqId, mut classes: Vec<Vec<Contributor>>, criteria: &[String], contracts: &[Contract],
        evidenced: bool,
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
        let trigger = Self::trigger_of(&classes, contracts);
        let contract_stems = Self::contract_stems(&classes, contracts);

        Ok(Self {
            id,
            subject: classes[0][0].id.clone(),
            status,
            covered,
            trigger,
            classes,
            evidenced,
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
            evidenced: false,
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

    // The trigger a contract binds to the requirement: by id first, the
    // contract whose id the member's leads with, the longest such id for
    // the member highest in authority; else by anchor, the contract whose
    // citation the member's meets, in the same order.
    fn trigger_of(classes: &[Vec<Contributor>], contracts: &[Contract]) -> Option<Trigger> {
        let mut members: Vec<&Contributor> = classes.iter().flatten().collect();
        members.sort_by_key(|member| (member.rank, member.index));
        members.iter().find_map(|member| Contract::trigger_by_id(contracts, &member.id)).or_else(
            || {
                members.iter().find_map(|member| {
                    Contract::trigger_by_anchor(contracts, &Cited::from(*member))
                })
            },
        )
    }

    fn contract_stems(classes: &[Vec<Contributor>], contracts: &[Contract]) -> BTreeSet<String> {
        let mut stems = BTreeSet::new();
        for member in classes.iter().flatten().filter(|member| !member.is_baseline()) {
            let cited = Cited::from(member);
            for contract in contracts.iter().filter(|contract| contract.matches(&cited)) {
                let stem = contract.id.as_deref().map_or_else(|| member.stem(), revision::stem);
                stems.insert(stem.to_owned());
            }
        }
        stems
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
