//! Reconciles requirement claims into deterministic requirement bases.
//!
//! Groups claims by identifier and model-assisted semantic matching, then
//! applies source authority to derive each requirement's winning statement,
//! status, and coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::Trigger;
use emery_adapter::source::{ClaimKind, SourceKind};
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
pub struct GroupingBrief<'a> {
    contributors: Vec<Contributor<'a>>,
    criteria: Vec<&'a str>,
    contracts: Vec<Contract<'a>>,
    sources: usize,
    baseline: Grouping,
    // How many contributors the run's sources contributed; the rest are
    // the standing requirements.
    changes: usize,
}

impl<'a> GroupingBrief<'a> {
    // Returns a grouping brief for `extracts`.
    //
    // A `retire` claim is passed over: with no standing requirements there
    // is nothing for it to withdraw.
    #[must_use]
    pub fn new(extracts: &'a [Extract]) -> Self {
        Self::assemble(extracts, Vec::new(), Vec::new(), false)
    }

    // Returns a grouping brief for `extracts` matched against `standing`.
    //
    // `standing` is the current revision's requirements as contributors
    // from `BASELINE`, each indexed after the claims, and `criteria` the
    // ids of those the revision holds covered. The model places each claim
    // in the group of the standing requirement it restates or revises, or
    // in a group of its own, and never two standing requirements in one
    // group. A `retire` claim is a contributor too, marked as withdrawing
    // the behaviour its id names: one at a standing requirement's id is
    // pre-merged with it, and the model places any other with the
    // standing requirement it withdraws.
    #[must_use]
    pub fn with_standing(
        extracts: &'a [Extract], standing: Vec<Contributor<'a>>, criteria: Vec<&'a str>,
    ) -> Self {
        Self::assemble(extracts, standing, criteria, true)
    }

    fn assemble(
        extracts: &'a [Extract], standing: Vec<Contributor<'a>>, criteria: Vec<&'a str>,
        withdrawals: bool,
    ) -> Self {
        let mut contributors: Vec<Contributor<'a>> = Vec::new();
        let mut criteria = criteria;
        let mut contracts = Vec::new();

        for extract in extracts {
            for claim in &extract.evidence.claims {
                // a contract binds by its id or its anchor, so it needs no id
                if claim.kind == ClaimKind::Contract {
                    if let Some(Ok(trigger)) = claim.trigger() {
                        contracts.push(Contract {
                            cited: Cited {
                                source: extract.source.clone(),
                                claim: claim.id.clone().unwrap_or_default(),
                                path: claim.path.clone(),
                            },
                            id: claim.id.as_deref(),
                            trigger,
                        });
                    }
                    continue;
                }
                let Some(id) = claim.id.as_deref() else { continue };

                let withdraws = match claim.kind {
                    ClaimKind::Requirement => false,
                    ClaimKind::Retire if withdrawals => true,
                    ClaimKind::Criterion => {
                        criteria.push(id);
                        continue;
                    }
                    _ => continue,
                };
                contributors.push(Contributor {
                    source: &extract.source,
                    kind: extract.kind,
                    rank: extract.rank,
                    id,
                    path: claim.path.as_deref(),
                    statement: claim.statement(),
                    synopsis: claim.synopsis.as_deref(),
                    withdraws,
                    index: contributors.len(),
                });
            }
        }

        // the standing requirements after the claims, indexed in place
        let changes = contributors.len();
        contributors.extend(standing.into_iter().enumerate().map(|(offset, member)| Contributor {
            index: changes + offset,
            ..member
        }));

        // count the sources that contribute a requirement claim
        let sources = contributors.iter().map(|claim| claim.source).collect::<BTreeSet<_>>().len();
        let baseline = Grouping::baseline(&contributors);

        Self {
            contributors,
            criteria,
            contracts,
            sources,
            baseline,
            changes,
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
    pub async fn derive<M: Model>(self, model: &M) -> Result<Vec<Basis<'a>>, Error> {
        if self.claims().is_empty() {
            return Err(bad_request!("no source contributed a requirement claim"));
        }

        if self.sources < 2 && self.stems() < 2 {
            self.baseline.bases(self.contributors, &self.criteria, &self.contracts)
        } else {
            self.judge(model).await
        }
    }

    // The distinct stems among the contributors' ids: the nouns the seams led
    // with, which differ when one source describes one behaviour twice.
    fn stems(&self) -> usize {
        self.contributors.iter().map(Contributor::stem).collect::<BTreeSet<_>>().len()
    }

    fn claims(&self) -> &[Contributor<'a>] {
        &self.contributors[..self.changes]
    }

    fn standing(&self) -> &[Contributor<'a>] {
        &self.contributors[self.changes..]
    }
}

impl<'a> Brief for GroupingBrief<'a> {
    type Answer = Grouping;
    type Output = Vec<Basis<'a>>;

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
        Ok(())
    }

    fn verify(&self, answer: &Grouping, review: &mut Review) {
        let count = self.contributors.len();
        let mut placed: BTreeMap<usize, usize> = BTreeMap::new();

        for (position, group) in answer.groups.iter().enumerate() {
            if group.claims.is_empty() {
                review.note(format_args!("group {position} has no claims"));
            }

            for &index in &group.claims {
                if index >= count {
                    review.note(format_args!("group {position}: claim {index} does not exist"));
                } else if placed.insert(index, position).is_some() {
                    review.note(format_args!("claim {index} appears in more than one group"));
                }
            }

            let members: BTreeSet<usize> = group.claims.iter().copied().collect();
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
            let standing: BTreeSet<&str> = members
                .iter()
                .filter(|&&index| index >= self.changes)
                .filter_map(|&index| self.contributors.get(index).map(|member| member.id))
                .collect();
            if standing.len() > 1 {
                let ids =
                    standing.iter().map(|id| format!("`{id}`")).collect::<Vec<_>>().join(", ");
                review.note(format_args!(
                    "group {position} merges the standing requirements {ids}, which the current \
                     specification holds as distinct requirements"
                ));
            }
        }

        for index in (0..count).filter(|index| !placed.contains_key(index)) {
            review.note(format_args!("claim {index} is in no group"));
        }

        // byte-equal ids may not be split across groups
        for group in &self.baseline.groups {
            let positions: BTreeSet<&usize> =
                group.claims.iter().filter_map(|index| placed.get(index)).collect();
            if positions.len() > 1 {
                let id = self.contributors[group.claims[0]].id;
                review.note(format_args!("claims sharing the id `{id}` are split across groups"));
            }
        }
    }

    fn into_output(self, answer: Grouping) -> Result<Vec<Basis<'a>>, Error> {
        answer.split(&self.contributors).bases(self.contributors, &self.criteria, &self.contracts)
    }
}

impl Display for GroupingBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let standing = self.standing();
        if standing.is_empty() {
            f.write_str("Group the requirement claims.\n\n")?;
        } else {
            f.write_str("Group the requirement claims against the standing requirements.\n\n")?;
        }
        f.write_str("## Requirement claims (index, source, id, statement, synopsis)\n\n")?;

        for claim in self.claims() {
            let synopsis = claim.synopsis.unwrap_or("-");
            let kind = if claim.withdraws { " (retire)" } else { "" };
            writeln!(
                f,
                "- {index} `{source}` `{id}`{kind} — {statement} — {synopsis}",
                index = claim.index,
                source = claim.source,
                id = claim.id,
                statement = claim.statement,
            )?;
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
            }
        }

        f.write_str("\n## Baseline\n\n")?;
        let merged: Vec<&Group> =
            self.baseline.groups.iter().filter(|group| group.claims.len() > 1).collect();
        if merged.is_empty() {
            f.write_str("No two claims share an id; every grouping is your judgement.\n")?;
        }

        for group in merged {
            let id = self.contributors[group.claims[0]].id;
            let indices =
                group.claims.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
            writeln!(
                f,
                "- claims {indices} share the id `{id}` and are pre-merged; an answer that splits \
                 them across groups is refused."
            )?;
        }

        f.write_str(
            "\nAnswer with every index in exactly one group, and every group's claims in exactly \
             one agreeing class. Two claims of one source that describe one requirement under \
             different stems — the first segment of their ids — are one group, whatever nouns \
             their seams gave them. Two claims of one source with different ids under one stem \
             are distinct requirements: the call that minted them under one noun told them \
             apart, and a group that merges them is split, each id its own requirement.\n",
        )?;
        if !standing.is_empty() {
            f.write_str(
                "\nEach standing requirement is one requirement of the current specification. A \
                 claim that restates, revises, or narrows the behaviour a standing requirement \
                 states joins its group, in the standing requirement's class when it says the \
                 same thing and in a class of its own when it changes a value, a condition, or \
                 an outcome. A claim stating a behaviour no standing requirement covers is a \
                 group of its own. A standing requirement no claim touches is a group of its own \
                 too. Two standing requirements never share a group: an answer that merges them \
                 is refused.\n\nA claim marked `(retire)` withdraws the behaviour its id names \
                 rather than stating one: it joins the group of the standing requirement that \
                 states the behaviour, in a class of its own, and is never a group of its own or \
                 of other claims alone, since a withdrawal of a behaviour the specification does \
                 not hold is refused after your answer.\n",
            )?;
        }
        Ok(())
    }
}

/// A partition of requirement claims into requirements and agreement classes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery grouping answer")]
pub struct Grouping {
    /// One group per requirement.
    pub groups: Vec<Group>,
}

impl Grouping {
    // Byte-equal ids are one group, in first-appearance order, and
    // whitespace-equal statements one class; every answer must contain this
    // grouping.
    fn baseline(contributors: &[Contributor<'_>]) -> Self {
        let mut groups: Vec<Group> = Vec::new();
        let mut by_id: BTreeMap<&str, usize> = BTreeMap::new();
        for (index, claim) in contributors.iter().enumerate() {
            let position = *by_id.entry(claim.id).or_insert_with(|| {
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

        Self { groups }
    }

    // One source's distinct ids under one stem are distinct requirements — the
    // call that minted them under one noun told them apart — so a group that
    // merges them is split without a round: the first such id keeps the group
    // and every other member, each further id takes its own claims into a new
    // group, and each group's classes are the answer's cut to its members.
    fn split(self, contributors: &[Contributor<'_>]) -> Self {
        let mut groups = Vec::with_capacity(self.groups.len());
        for group in self.groups {
            // (source, stem) → the distinct ids the group holds under it, in order
            let mut by_stem: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
            for claim in group.claims.iter().filter_map(|&index| contributors.get(index)) {
                let ids = by_stem.entry((claim.source, claim.stem())).or_default();
                if !ids.contains(&claim.id) {
                    ids.push(claim.id);
                }
            }
            let spun: BTreeSet<&str> = by_stem
                .iter()
                .filter(|(_, ids)| ids.len() > 1)
                .inspect(|((source, stem), ids)| {
                    tracing::debug!(
                        source,
                        stem,
                        ?ids,
                        "the grouping merged one source's distinct ids under one stem; split"
                    );
                })
                .flat_map(|(_, ids)| ids.iter().skip(1).copied())
                .collect();
            if spun.is_empty() {
                groups.push(group);
                continue;
            }
            let id_of = |index: usize| contributors.get(index).map(|claim| claim.id);
            let cut = |keep: &dyn Fn(usize) -> bool| Group {
                claims: group.claims.iter().copied().filter(|&index| keep(index)).collect(),
                classes: group
                    .classes
                    .iter()
                    .map(|class| class.iter().copied().filter(|&index| keep(index)).collect())
                    .filter(|class: &Vec<usize>| !class.is_empty())
                    .collect(),
            };
            groups.push(cut(&|index| !id_of(index).is_some_and(|id| spun.contains(id))));
            for id in spun {
                groups.push(cut(&|index| id_of(index) == Some(id)));
            }
        }
        Self { groups }
    }

    // Ordered by each group's earliest claim and numbered from `REQ-001` in
    // that order. The grouping was verified to place every contributor exactly
    // once, so an index it misses or repeats is the engine's own defect.
    fn bases<'a>(
        &self, contributors: Vec<Contributor<'a>>, criteria: &[&str], contracts: &[Contract<'a>],
    ) -> Result<Vec<Basis<'a>>, Error> {
        let mut contributors: Vec<Option<Contributor<'a>>> =
            contributors.into_iter().map(Some).collect();
        let mut groups: Vec<(usize, Vec<Vec<Contributor<'a>>>)> =
            Vec::with_capacity(self.groups.len());
        for group in &self.groups {
            let first = group.claims.iter().copied().min().unwrap_or_default();
            let mut classes = Vec::with_capacity(group.classes.len());
            for class in &group.classes {
                let members = class
                    .iter()
                    .map(|&index| {
                        contributors.get_mut(index).and_then(Option::take).ok_or_else(|| {
                            server_error!(
                                "the grouping names claim {index}, which does not exist or is \
                                 already placed"
                            )
                        })
                    })
                    .collect::<Result<_, _>>()?;
                classes.push(members);
            }
            groups.push((first, classes));
        }
        groups.sort_by_key(|(first, _)| *first);
        groups
            .into_iter()
            .zip(ReqId::sequence())
            .map(|((_, classes), id)| Basis::of(id, classes, criteria, contracts))
            .collect()
    }
}

// A `contract` claim carrying a trigger: what binds it to a requirement is
// its id, which the requirement's subject leads with, or its anchor, which a
// contributor's citation meets.
#[derive(Debug)]
struct Contract<'a> {
    cited: Cited,
    id: Option<&'a str>,
    trigger: Trigger,
}

impl Contract<'_> {
    // Whether `id` is this contract's id or a dotted child of it.
    fn leads(&self, id: &str) -> bool {
        self.id.is_some_and(|own| {
            id.strip_prefix(own).is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
        })
    }
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

// Reconciled facts used to construct one requirement.
//
// Contributors are grouped by agreement, with the winning class first.
#[derive(Debug)]
pub struct Basis<'a> {
    // The requirement identifier, assigned from `REQ-001`.
    pub id: ReqId,
    // The heading derived from the highest-authority claim identifier.
    pub subject: &'a str,
    // The reconciliation outcome.
    pub status: Status,
    // Whether any criterion claim covers the requirement.
    pub covered: bool,
    // What reaches the requirement, where a `contract` claim binds it.
    pub trigger: Option<Trigger>,
    // Agreement classes, with the winning class first.
    pub classes: Vec<Vec<Contributor<'a>>>,
}

impl<'a> Basis<'a> {
    fn of(
        id: ReqId, mut classes: Vec<Vec<Contributor<'a>>>, criteria: &[&str],
        contracts: &[Contract<'a>],
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
        let covered = classes.iter().flatten().any(|member| {
            criteria.iter().any(|id| {
                id.strip_prefix(member.id)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
            })
        });

        let status = Self::settle(&classes, covered);
        let trigger = Self::trigger_of(&classes, contracts);

        Ok(Self {
            id,
            subject: classes[0][0].id,
            status,
            covered,
            trigger,
            classes,
        })
    }

    // The status sorted, non-empty `classes` settle to: one class agreed or
    // unknown by `covered`, a class alone at the top rank a divergence, two
    // tied there a conflict.
    fn settle(classes: &[Vec<Contributor<'a>>], covered: bool) -> Status {
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
            class.retain(|member| member.source != BASELINE);
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
        })
    }

    // The trigger a contract binds to the requirement: by id first, the
    // contract whose id the member's leads with, the longest such id for
    // the member highest in authority; else by anchor, the contract whose
    // citation the member's meets, in the same order.
    fn trigger_of(classes: &[Vec<Contributor<'a>>], contracts: &[Contract<'a>]) -> Option<Trigger> {
        let mut members: Vec<&Contributor<'a>> = classes.iter().flatten().collect();
        members.sort_by_key(|member| (member.rank, member.index));
        let by_id = members.iter().find_map(|member| {
            contracts
                .iter()
                .filter(|contract| contract.leads(member.id))
                .max_by_key(|contract| contract.id.map_or(0, str::len))
        });
        by_id
            .or_else(|| {
                members.iter().find_map(|member| {
                    let cited = Cited::from(*member);
                    contracts.iter().find(|contract| cited.overlaps(&contract.cited))
                })
            })
            .map(|contract| contract.trigger.clone())
    }

    // Returns the stem of the subject: the first segment of its dotted id.
    #[must_use]
    pub fn stem(&self) -> &'a str {
        revision::stem(self.subject)
    }

    // Returns contributors by descending authority and then source order.
    pub fn contributors(&self) -> impl Iterator<Item = &Contributor<'a>> {
        let mut members: Vec<&Contributor<'a>> = self.classes.iter().flatten().collect();
        members.sort_by_key(|member| (member.rank, member.index));
        members.into_iter()
    }

    // Returns the winning statement, or `None` when the contributors are in
    // conflict or the winning claim withdraws the requirement.
    #[must_use]
    pub fn statement(&self) -> Option<&str> {
        match self.status {
            Status::Conflict => None,
            Status::Agreed | Status::Unknown | Status::Divergence => {
                let lead = &self.classes[0][0];
                (!lead.withdraws).then_some(lead.statement.as_str())
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
    pub fn withdrawals(&self) -> impl Iterator<Item = &Contributor<'a>> {
        self.contributors().filter(|member| member.withdraws)
    }

    // Returns the requirement produced from this basis and `scenarios`.
    //
    // The body is the winning statement, or none for a conflict. The losing
    // classes become notes. The trigger is the basis's own, bound from the
    // sources' `contract` claims.
    #[must_use]
    pub fn requirement(&self, scenarios: Vec<Scenario>) -> Requirement {
        let body = self.statement().map(|statement| vec![statement.to_owned()]).unwrap_or_default();
        let noted: &[Vec<Contributor<'a>>] = match self.status {
            Status::Agreed | Status::Unknown => &[],
            Status::Divergence => &self.classes[1..],
            Status::Conflict => &self.classes,
        };
        let losers = noted
            .iter()
            .map(|class| {
                let lead = &class[0];
                Loser {
                    sources: class.iter().map(|member| member.source.to_string()).collect(),
                    kind: lead.kind,
                    rank: lead.rank,
                    claim: lead.id.to_string(),
                    statement: lead.statement.clone(),
                }
            })
            .collect();

        Requirement {
            id: self.id,
            subject: self.subject.to_string(),
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
#[derive(Debug)]
pub struct Contributor<'a> {
    // The name of the source the claim was extracted from.
    pub source: &'a str,
    // The kind of source the claim was extracted from.
    pub kind: SourceKind,
    // The authority rank the contributor is ordered by.
    pub rank: Rank,
    // The claim identifier, which may differ from the requirement subject.
    pub id: &'a str,
    // Where in the source the claim anchors, when it carries a `path`.
    pub path: Option<&'a str>,
    // The claim statement with whitespace normalised.
    pub statement: String,
    // An optional synopsis provided to the grouping model.
    pub synopsis: Option<&'a str>,
    // Whether the claim is a `retire`, withdrawing the behaviour its id
    // names rather than stating it.
    pub withdraws: bool,
    // Position in source order, used to break authority ties.
    pub index: usize,
}

impl<'a> Contributor<'a> {
    // Returns the stem of the claim id: the first segment of its dotted id.
    #[must_use]
    pub fn stem(&self) -> &'a str {
        revision::stem(self.id)
    }
}

impl From<&Contributor<'_>> for Cited {
    fn from(member: &Contributor<'_>) -> Self {
        Self {
            source: member.source.to_string(),
            claim: member.id.to_string(),
            path: member.path.map(str::to_owned),
        }
    }
}
