//! Reconciles requirement claims into deterministic requirement bases.
//!
//! Claims sharing an identifier are grouped before any model request. For
//! multiple sources, the model may group remaining claims by meaning and
//! agreement. The engine validates that partition, applies source authority,
//! and derives status, coverage, winners, and losing statements.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{ClaimKind, SourceKind};
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::revision::{Cited, Loser, ReqId, Requirement, Scenario, Status};
use crate::specify::Extract;
use crate::specify::brief::{Brief, Review};

/// A synthesis brief for grouping requirement claims.
///
/// The brief contains requirement claims in source order and every acceptance
/// criterion identifier. Source authority is withheld from the model.
pub struct GroupingBrief<'a> {
    contributors: Vec<Contributor>,
    criteria: Vec<&'a str>,
    sources: usize,
}

impl<'a> GroupingBrief<'a> {
    /// Returns a grouping brief for `extracts`.
    #[must_use]
    pub fn new(extracts: &'a [Extract]) -> Self {
        let mut contributors: Vec<Contributor> = Vec::new();
        let mut criteria = Vec::new();
        for extract in extracts {
            for claim in &extract.evidence.claims {
                let Some(id) = claim.id.as_deref() else { continue };
                match claim.kind {
                    ClaimKind::Requirement => contributors.push(Contributor {
                        source: extract.source.clone(),
                        kind: extract.kind,
                        id: id.to_string(),
                        statement: claim.statement(),
                        synopsis: claim.synopsis.clone(),
                        index: contributors.len(),
                    }),
                    ClaimKind::Criterion => criteria.push(id),
                    _ => {}
                }
            }
        }

        Self {
            contributors,
            criteria,
            sources: extracts.len(),
        }
    }

    /// Derives every requirement basis.
    ///
    /// A run over two or more sources asks the model to group the claims; a
    /// run over one takes the baseline alone and spends no call.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the model cannot produce a valid
    ///   grouping within the available rounds.
    /// - Returns [`Error::ServerError`] when required prose is missing or a
    ///   grouping cannot be reconciled with the claims.
    /// - Returns [`Error::BadGateway`] when the model operation fails.
    pub async fn derive<M: Model>(self, model: &M) -> Result<Vec<Basis>, Error> {
        if self.sources < 2 { self.bases(&self.baseline()) } else { self.judge(model).await }
    }

    // Byte-equal ids are one group and whitespace-equal statements one class;
    // every answer must contain this grouping.
    fn baseline(&self) -> Grouping {
        let mut groups: Vec<(&str, Group)> = Vec::new();
        for (index, claim) in self.contributors.iter().enumerate() {
            let position = groups.iter().position(|(id, _)| *id == claim.id).unwrap_or_else(|| {
                groups.push((claim.id.as_str(), Group::default()));
                groups.len() - 1
            });
            let group = &mut groups[position].1;
            group.claims.push(index);
            let class = group
                .classes
                .iter_mut()
                .find(|class| self.contributors[class[0]].statement == claim.statement);

            match class {
                Some(class) => class.push(index),
                None => group.classes.push(vec![index]),
            }
        }

        Grouping {
            groups: groups.into_iter().map(|(_, group)| group).collect(),
        }
    }

    // Ordered by each group's earliest claim and numbered from `REQ-001` in
    // that order.
    fn bases(&self, grouping: &Grouping) -> Result<Vec<Basis>, Error> {
        let mut groups: Vec<(usize, Vec<Vec<Contributor>>)> =
            Vec::with_capacity(grouping.groups.len());
        for group in &grouping.groups {
            let first = group.claims.iter().copied().min().unwrap_or_default();
            let mut classes = Vec::with_capacity(group.classes.len());
            for class in &group.classes {
                classes.push(
                    class.iter().map(|&index| self.contributor(index)).collect::<Result<_, _>>()?,
                );
            }
            groups.push((first, classes));
        }
        groups.sort_by_key(|(first, _)| *first);
        groups
            .into_iter()
            .zip(1..)
            .map(|((_, classes), number)| Basis::of(ReqId::new(number), classes, &self.criteria))
            .collect()
    }

    // The grouping was verified against these contributors, so a miss is the
    // engine's own defect.
    fn contributor(&self, index: usize) -> Result<Contributor, Error> {
        self.contributors
            .get(index)
            .cloned()
            .ok_or_else(|| server_error!("the grouping names claim {index}, which does not exist"))
    }
}

impl Brief for GroupingBrief<'_> {
    type Answer = Grouping;
    type Output = Vec<Basis>;

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
        let mut by_id: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
        for (index, claim) in self.contributors.iter().enumerate() {
            if let Some(position) = placed.get(&index) {
                by_id.entry(claim.id.as_str()).or_default().insert(*position);
            }
        }

        for (id, positions) in by_id {
            if positions.len() > 1 {
                review.note(format_args!("claims sharing the id `{id}` are split across groups"));
            }
        }
    }

    fn into_output(self, answer: Grouping) -> Result<Vec<Basis>, Error> {
        self.bases(&answer)
    }
}

impl Display for GroupingBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            "Group the requirement claims.\n\n\
             ## Requirement claims (index, source, id, statement, synopsis)\n\n",
        )?;

        for (index, claim) in self.contributors.iter().enumerate() {
            let synopsis = claim.synopsis.as_deref().unwrap_or("-");
            writeln!(
                f,
                "- {index} `{source}` `{id}` — {statement} — {synopsis}",
                source = claim.source,
                id = claim.id,
                statement = claim.statement,
            )?;
        }

        f.write_str("\n## Baseline\n\n")?;
        let baseline = self.baseline();
        let merged: Vec<&Group> =
            baseline.groups.iter().filter(|group| group.claims.len() > 1).collect();
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

        f.write_str(
            "\nAnswer with every index in exactly one group, and every group's claims in exactly \
             one agreeing class.\n",
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
pub struct Basis {
    /// The requirement identifier, assigned from `REQ-001`.
    pub id: ReqId,
    /// The heading derived from the highest-authority claim identifier.
    pub subject: String,
    /// The reconciliation outcome.
    pub status: Status,
    /// Whether any criterion claim covers the requirement.
    pub covered: bool,
    /// Agreement classes, with the winning class first.
    pub classes: Vec<Vec<Contributor>>,
}

impl Basis {
    fn of(id: ReqId, mut classes: Vec<Vec<Contributor>>, criteria: &[&str]) -> Result<Self, Error> {
        // an empty class is the engine's own defect; from here every class has a lead
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
                id.strip_prefix(member.id.as_str())
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
            subject: classes[0][0].id.clone(),
            status,
            covered,
            classes,
        })
    }

    /// Returns contributors by descending authority and then source order.
    pub fn contributors(&self) -> impl Iterator<Item = &Contributor> {
        let mut members: Vec<&Contributor> = self.classes.iter().flatten().collect();
        members.sort_by_key(|member| (member.kind, member.index));
        members.into_iter()
    }

    /// Returns the requirement produced from this basis and `scenarios`.
    ///
    /// The body is the winning statement — none for a conflict — and the
    /// losing classes become notes.
    #[must_use]
    pub fn requirement(&self, scenarios: Vec<Scenario>) -> Requirement {
        let winner = &self.classes[0][0].statement;
        let (body, noted): (Vec<String>, &[Vec<Contributor>]) = match self.status {
            Status::Agreed | Status::Unknown => (vec![winner.clone()], &[]),
            Status::Divergence => (vec![winner.clone()], &self.classes[1..]),
            Status::Conflict => (Vec::new(), &self.classes),
        };
        let losers = noted
            .iter()
            .map(|class| {
                let lead = &class[0];
                Loser {
                    sources: class.iter().map(|member| member.source.clone()).collect(),
                    kind: lead.kind,
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
            sources: self.contributors().map(Cited::from).collect(),
            body,
            losers,
            scenarios,
        }
    }
}

/// A source claim contributing to a requirement.
#[derive(Debug, Clone)]
pub struct Contributor {
    /// The source key.
    pub source: String,
    /// The source kind used to rank this contributor.
    pub kind: SourceKind,
    /// The claim identifier, which may differ from the requirement subject.
    pub id: String,
    /// The claim statement with whitespace normalised.
    pub statement: String,
    /// An optional synopsis provided to the grouping model.
    pub synopsis: Option<String>,
    /// Position in source order, used to break authority ties.
    pub index: usize,
}

impl From<&Contributor> for Cited {
    fn from(member: &Contributor) -> Self {
        Self {
            source: member.source.clone(),
            claim: member.id.clone(),
        }
    }
}
