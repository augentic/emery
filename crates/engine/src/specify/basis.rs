//! Reconciles requirement claims into deterministic requirement bases.
//!
//! Claims sharing an identifier are grouped before any model request. When
//! requirement claims come from several sources, or from one source whose
//! requirement ids span several stems, the model may group the remaining
//! claims by meaning and agreement, except that one source's distinct ids
//! under one stem stay distinct requirements. The engine validates that
//! partition, applies source authority, and derives status, coverage,
//! winners, and losing statements. A run in which no source contributes a
//! requirement claim is refused before any request.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{ClaimKind, SourceKind};
use omnia_sdk::{Error, Model, bad_request, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::revision::{Cited, Loser, ReqId, Requirement, Scenario, Status};
use crate::specify::brief::{Brief, Review};
use crate::specify::{Extract, shape};

/// A synthesis brief for grouping requirement claims.
///
/// The brief contains requirement claims in source order and every acceptance
/// criterion identifier. Source authority is withheld from the model.
pub struct GroupingBrief<'a> {
    contributors: Vec<Contributor<'a>>,
    criteria: Vec<&'a str>,
    sources: usize,
    baseline: Grouping,
}

impl<'a> GroupingBrief<'a> {
    /// Returns a grouping brief for `extracts`.
    #[must_use]
    pub fn new(extracts: &'a [Extract]) -> Self {
        let mut contributors: Vec<Contributor<'a>> = Vec::new();
        let mut criteria = Vec::new();

        for extract in extracts {
            for claim in &extract.evidence.claims {
                let Some(id) = claim.id.as_deref() else { continue };

                match claim.kind {
                    ClaimKind::Requirement => contributors.push(Contributor {
                        source: &extract.source,
                        kind: extract.kind,
                        id,
                        statement: claim.statement(),
                        synopsis: claim.synopsis.as_deref(),
                        index: contributors.len(),
                    }),
                    ClaimKind::Criterion => criteria.push(id),
                    _ => {}
                }
            }
        }

        // count the sources that contribute a requirement claim
        let sources = contributors.iter().map(|claim| claim.source).collect::<BTreeSet<_>>().len();
        let baseline = baseline(&contributors);

        Self {
            contributors,
            criteria,
            sources,
            baseline,
        }
    }

    /// Derives every requirement basis.
    ///
    /// A run whose requirement claims come from two or more sources asks the
    /// model to group them. So does a run over one contributing source whose
    /// requirement ids span two or more stems, since its seams may describe
    /// one behaviour under different nouns. Otherwise the baseline stands alone
    /// and no call is spent. Whatever the model answers, claims sharing an id
    /// stay one group, and one source's distinct ids under one stem stay
    /// distinct groups — a group that merges them is split, with no round
    /// spent.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when no source contributed a requirement
    ///   claim, or when the model cannot produce a valid grouping within the
    ///   available rounds.
    /// - Returns [`Error::ServerError`] when required prose is missing or a
    ///   grouping cannot be reconciled with the claims.
    /// - Returns [`Error::BadGateway`] when the model operation fails.
    pub async fn derive<M: Model>(self, model: &M) -> Result<Vec<Basis<'a>>, Error> {
        if self.contributors.is_empty() {
            return Err(bad_request!("no source contributed a requirement claim"));
        }

        if self.sources < 2 && self.stems() < 2 {
            bases(self.contributors, &self.criteria, &self.baseline)
        } else {
            self.judge(model).await
        }
    }

    // The distinct stems among the contributors' ids: the nouns the seams led
    // with, which differ when one source describes one behaviour twice.
    fn stems(&self) -> usize {
        self.contributors.iter().map(|claim| shape::stem(claim.id)).collect::<BTreeSet<_>>().len()
    }
}

// Byte-equal ids are one group, in first-appearance order, and whitespace-equal
// statements one class; every answer must contain this grouping.
fn baseline(contributors: &[Contributor<'_>]) -> Grouping {
    let mut groups: Vec<Group> = Vec::new();
    let mut by_id: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, claim) in contributors.iter().enumerate() {
        let position = *by_id.entry(claim.id).or_insert_with(|| {
            groups.push(Group::default());
            groups.len() - 1
        });
        let group = &mut groups[position];
        group.claims.push(index);
        let class = group
            .classes
            .iter_mut()
            .find(|class| contributors[class[0]].statement == claim.statement);

        match class {
            Some(class) => class.push(index),
            None => group.classes.push(vec![index]),
        }
    }

    Grouping { groups }
}

// Ordered by each group's earliest claim and numbered from `REQ-001` in that
// order. The grouping was verified to place every contributor exactly once, so
// an index it misses or repeats is the engine's own defect.
fn bases<'a>(
    contributors: Vec<Contributor<'a>>, criteria: &[&str], grouping: &Grouping,
) -> Result<Vec<Basis<'a>>, Error> {
    let mut contributors: Vec<Option<Contributor<'a>>> =
        contributors.into_iter().map(Some).collect();
    let mut groups: Vec<(usize, Vec<Vec<Contributor<'a>>>)> =
        Vec::with_capacity(grouping.groups.len());
    for group in &grouping.groups {
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
        .zip(1..)
        .map(|((_, classes), number)| Basis::of(ReqId::new(number), classes, criteria))
        .collect()
}

impl<'a> Brief for GroupingBrief<'a> {
    type Answer = Grouping;
    type Output = Vec<Basis<'a>>;

    const NAME: &'static str = "grouping";
    const PROSE: &'static [&'static str] = &["grouping.md"];

    fn tighten(&self, schema: &mut Value) {
        let last = self.contributors.len().saturating_sub(1);
        for pointer in ["/properties/claims/items", "/properties/classes/items/items"] {
            if let Some(index) = schema.pointer_mut(&format!("/$defs/Group{pointer}")) {
                index["maximum"] = json!(last);
            }
        }
        schema["properties"]["groups"]["minItems"] = json!(1);
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
            }

            for index in members.difference(&classed) {
                review.note(format_args!("group {position}: claim {index} is in no class"));
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
        let answer = split(&self.contributors, answer);
        bases(self.contributors, &self.criteria, &answer)
    }
}

// One source's distinct ids under one stem are distinct requirements — the
// call that minted them under one noun told them apart — so a group that
// merges them is split without a round: the first such id keeps the group and
// every other member, each further id takes its own claims into a new group,
// and each group's classes are the answer's cut to its members.
fn split(contributors: &[Contributor<'_>], answer: Grouping) -> Grouping {
    let mut groups = Vec::with_capacity(answer.groups.len());
    for group in answer.groups {
        // (source, stem) → the distinct ids the group holds under it, in order
        let mut by_stem: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
        for claim in group.claims.iter().filter_map(|&index| contributors.get(index)) {
            let ids = by_stem.entry((claim.source, shape::stem(claim.id))).or_default();
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
    Grouping { groups }
}

impl Display for GroupingBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            "Group the requirement claims.\n\n\
             ## Requirement claims (index, source, id, statement, synopsis)\n\n",
        )?;

        for (index, claim) in self.contributors.iter().enumerate() {
            let synopsis = claim.synopsis.unwrap_or("-");
            writeln!(
                f,
                "- {index} `{source}` `{id}` — {statement} — {synopsis}",
                source = claim.source,
                id = claim.id,
                statement = claim.statement,
            )?;
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
        )
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

/// The claims assigned to one requirement and their agreement classes.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Indices of every claim describing this requirement.
    pub claims: Vec<usize>,
    /// Partitions of [`Group::claims`] that express equivalent statements.
    pub classes: Vec<Vec<usize>>,
}

/// Reconciled facts used to construct one requirement.
///
/// Contributors are grouped by agreement, with the winning class first.
#[derive(Debug)]
pub struct Basis<'a> {
    /// The requirement identifier, assigned from `REQ-001`.
    pub id: ReqId,
    /// The heading derived from the highest-authority claim identifier.
    pub subject: &'a str,
    /// The reconciliation outcome.
    pub status: Status,
    /// Whether any criterion claim covers the requirement.
    pub covered: bool,
    /// Agreement classes, with the winning class first.
    pub classes: Vec<Vec<Contributor<'a>>>,
}

impl<'a> Basis<'a> {
    fn of(
        id: ReqId, mut classes: Vec<Vec<Contributor<'a>>>, criteria: &[&str],
    ) -> Result<Self, Error> {
        // refuse an empty class, so every class below has a lead
        if classes.is_empty() || classes.iter().any(Vec::is_empty) {
            return Err(server_error!("requirement {id} was grouped with a class of no claims"));
        }
        for class in &mut classes {
            class.sort_by_key(|member| (member.kind, member.index));
        }
        classes.sort_by_key(|class| (class[0].kind, class[0].index));

        // covered by a criterion at the claim id or a dotted child of it
        let covered = classes.iter().flatten().any(|member| {
            criteria.iter().any(|id| {
                id.strip_prefix(member.id)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
            })
        });

        let top = classes[0][0].kind;
        let status = match classes.len() {
            1 if covered => Status::Agreed,
            1 => Status::Unknown,
            _ if classes.iter().skip(1).all(|class| class[0].kind != top) => Status::Divergence,
            _ => Status::Conflict,
        };

        Ok(Self {
            id,
            subject: classes[0][0].id,
            status,
            covered,
            classes,
        })
    }

    /// Returns contributors by descending authority and then source order.
    pub fn contributors(&self) -> impl Iterator<Item = &Contributor<'a>> {
        let mut members: Vec<&Contributor<'a>> = self.classes.iter().flatten().collect();
        members.sort_by_key(|member| (member.kind, member.index));
        members.into_iter()
    }

    /// Returns the requirement produced from this basis and `scenarios`.
    ///
    /// The body is the winning statement, or none for a conflict. The losing
    /// classes become notes.
    #[must_use]
    pub fn requirement(&self, scenarios: Vec<Scenario>) -> Requirement {
        let winner = &self.classes[0][0].statement;
        let (body, noted): (Vec<String>, &[Vec<Contributor<'a>>]) = match self.status {
            Status::Agreed | Status::Unknown => (vec![winner.clone()], &[]),
            Status::Divergence => (vec![winner.clone()], &self.classes[1..]),
            Status::Conflict => (Vec::new(), &self.classes),
        };
        let losers = noted
            .iter()
            .map(|class| {
                let lead = &class[0];
                Loser {
                    sources: class.iter().map(|member| member.source.to_string()).collect(),
                    kind: lead.kind,
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
            sources: self.contributors().map(Cited::from).collect(),
            body,
            losers,
            scenarios,
        }
    }
}

/// A source claim contributing to a requirement.
#[derive(Debug)]
pub struct Contributor<'a> {
    /// The name of the source the claim was extracted from.
    pub source: &'a str,
    /// The source kind used to rank this contributor.
    pub kind: SourceKind,
    /// The claim identifier, which may differ from the requirement subject.
    pub id: &'a str,
    /// The claim statement with whitespace normalised.
    pub statement: String,
    /// An optional synopsis provided to the grouping model.
    pub synopsis: Option<&'a str>,
    /// Position in source order, used to break authority ties.
    pub index: usize,
}

impl From<&Contributor<'_>> for Cited {
    fn from(member: &Contributor<'_>) -> Self {
        Self {
            source: member.source.to_string(),
            claim: member.id.to_string(),
        }
    }
}
