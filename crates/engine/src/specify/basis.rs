//! Requirement bases
//!
//! Derives the basis each requirement in `spec.md` is built on from the
//! requirement claims in the extracts. Which claims across sources describe
//! one requirement, and which of them agree, is a judgement: the model answers
//! it as one partition — claims into requirements, each requirement's claims
//! into agreeing classes — over a baseline that pre-merges byte-equal ids. The
//! engine validates the partition, then derives everything else from it and the
//! closed authority ranking: the subject, the status, the winner and losers,
//! and whether any acceptance criterion covers the requirement.
//!
//! Authority is withheld from the request, so the answer cannot be steered
//! toward a winner; a run over one source never asks at all.
//!
//! Each basis then takes its id from the master the run continues: the
//! requirement it shares a cited `(source, claim)` with, else the one on the
//! same subject, and a fresh id from the master's `next_id` otherwise. A basis
//! whose facts still match the requirement it continues names it as its
//! incumbent, and the incumbent's drafted content is kept rather than asked
//! for again.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_source::types::{Authority, ClaimKind};
use omnia_guest::{Error, Model};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::artifact::{Cited, Loser, ReqId, Requirement, Scenario, Spec, Status};
use crate::specify::Extract;
use crate::specify::brief::{Brief, Review};

/// Derives every requirement basis in `extracts`, asking the model to group
/// the claims on any run over two or more sources, then numbers the bases
/// from the `master` they continue.
///
/// # Errors
///
/// A model failure is `bad_gateway`; an answer outside the schema, or a
/// grouping the backend could not repair within its rounds, is `bad_request`.
pub async fn derive<M: Model>(
    model: &M, extracts: &[Extract], master: Option<&Spec>,
) -> Result<Vec<Basis>, Error> {
    let brief = GroupingBrief::collect(extracts);
    let mut bases = if extracts.len() < 2 {
        brief.bases(&brief.baseline())
    } else {
        brief.judge(model).await?
    };

    inherit(&mut bases, master);
    Ok(bases)
}

/// The id the next new requirement takes after `bases`: past every id in use
/// and never below the `master`'s own allocation, so an id is never reused.
#[must_use]
pub fn next_id(bases: &[Basis], master: Option<&Spec>) -> u32 {
    bases
        .iter()
        .map(|basis| basis.id.number() + 1)
        .chain(master.map(|master| master.next_id))
        .max()
        .unwrap_or(1)
}

// Numbers the bases from the master: each master requirement, lowest id
// first, lends its id to one unnumbered basis continuing it — the one holding
// its subject claim when several do — so a merge keeps the lowest id and a
// split keeps the id with the subject; the rest allocate from `next_id`.
// A basis whose facts still match the requirement it continues keeps it as
// the incumbent. Without a master, the bases number from 1 in order.
fn inherit(bases: &mut [Basis], master: Option<&Spec>) {
    let Some(master) = master else {
        for (basis, id) in bases.iter_mut().zip(1..) {
            basis.id = ReqId::new(id);
        }
        return;
    };

    let mut ordered: Vec<&Requirement> = master.requirements.iter().collect();
    ordered.sort_by_key(|requirement| requirement.id);
    let continued: Vec<Vec<usize>> = bases.iter().map(|basis| basis.continued(&ordered)).collect();

    let mut numbered = vec![false; bases.len()];
    for (position, requirement) in ordered.iter().enumerate() {
        let claimants: Vec<usize> = (0..bases.len())
            .filter(|&index| !numbered[index] && continued[index].contains(&position))
            .collect();
        let owner = claimants
            .iter()
            .copied()
            .find(|&index| bases[index].holds(&requirement.subject))
            .or_else(|| claimants.first().copied());

        if let Some(index) = owner {
            let heir = &mut bases[index];
            heir.id = requirement.id;
            if heir.requirement(requirement.scenarios.clone()) == **requirement {
                heir.incumbent = Some((*requirement).clone());
            }
            numbered[index] = true;
        }
    }

    let fresh = bases.iter_mut().zip(numbered).filter(|(_, numbered)| !numbered);
    for (next, (heir, _)) in (master.next_id..).zip(fresh) {
        heir.id = ReqId::new(next);
    }
}

/// A partition of every requirement claim into requirements, each carrying a
/// partition of its claims into agreeing classes.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery grouping answer")]
pub struct Grouping {
    /// One entry per requirement.
    pub groups: Vec<Group>,
}

/// The claims of one requirement and how they agree.
#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Indices of every claim describing this requirement.
    pub claims: Vec<usize>,
    /// A partition of `claims`: each class holds claims that say the same
    /// thing.
    pub classes: Vec<Vec<usize>>,
}

/// The basis for one requirement before any prose: its id, subject, status,
/// acceptance-criterion coverage, and contributors in agreeing classes, the
/// winning class first — and the master requirement it continues unchanged,
/// when there is one.
#[derive(Debug, Clone)]
pub struct Basis {
    id: ReqId,
    subject: String,
    status: Status,
    covered: bool,
    classes: Vec<Vec<Contributor>>,
    incumbent: Option<Requirement>,
}

impl Basis {
    // Builds a requirement from its classes, sorted by authority then source
    // order. One class is agreed (unknown when no criterion covers it); several
    // are a divergence when one holds the top authority alone, else a conflict.
    // The id is `inherit`'s to set; until then it is the unallocated 0.
    fn of(mut classes: Vec<Vec<Contributor>>, criteria: &[&str]) -> Self {
        for class in &mut classes {
            class.sort_by_key(|member| (member.authority.rank(), member.index));
        }
        classes.sort_by_key(|class| (class[0].authority.rank(), class[0].index));

        let top = classes[0][0].authority.rank();
        let covered =
            classes.iter().flatten().any(|member| criteria.iter().any(|id| covers(id, &member.id)));
        let status = match classes.len() {
            1 if covered => Status::Agreed,
            1 => Status::Unknown,
            _ if classes.iter().skip(1).all(|class| class[0].authority.rank() != top) => {
                Status::Divergence
            }
            _ => Status::Conflict,
        };

        Self {
            id: ReqId::new(0),
            subject: classes[0][0].id.clone(),
            status,
            covered,
            classes,
            incumbent: None,
        }
    }

    /// The requirement id: inherited from the master, or newly allocated.
    #[must_use]
    pub const fn id(&self) -> ReqId {
        self.id
    }

    /// The heading name: the top contributor's claim id.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The master requirement this basis continues with every fact
    /// unchanged, whose drafted content is kept without a turn.
    #[must_use]
    pub const fn incumbent(&self) -> Option<&Requirement> {
        self.incumbent.as_ref()
    }

    /// The `Status:` value.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Whether any criterion claim covers the requirement.
    #[must_use]
    pub const fn covered(&self) -> bool {
        self.covered
    }

    /// The agreeing classes, the winning class first.
    #[must_use]
    pub fn classes(&self) -> &[Vec<Contributor>] {
        &self.classes
    }

    /// Lists every contributor, highest authority first and source order
    /// within an authority.
    pub fn contributors(&self) -> impl Iterator<Item = &Contributor> {
        let mut members: Vec<&Contributor> = self.classes.iter().flatten().collect();
        members.sort_by_key(|member| (member.authority.rank(), member.index));
        members.into_iter()
    }

    /// Places `scenarios` beside the engine's facts as the requirement this
    /// basis commits: the body is the winning statement, none for a conflict;
    /// the notes are the losing classes.
    #[must_use]
    pub fn requirement(&self, scenarios: Vec<Scenario>) -> Requirement {
        let body = match self.status {
            Status::Conflict => Vec::new(),
            _ => vec![normalise(&self.classes[0][0].statement)],
        };

        Requirement {
            id: self.id,
            subject: self.subject.clone(),
            status: self.status,
            covered: self.covered,
            sources: self.contributors().map(Cited::from).collect(),
            body,
            losers: self.losers(),
            scenarios,
        }
    }

    // Collects the classes the master records as notes: the losing classes of
    // a divergence, every class of a conflict, none otherwise.
    fn losers(&self) -> Vec<Loser> {
        let noted = match self.status {
            Status::Divergence => &self.classes[1..],
            Status::Conflict => &*self.classes,
            Status::Agreed | Status::Unknown => &[],
        };
        noted.iter().map(|class| loser(class)).collect()
    }

    // Finds the master requirements (positions in `ordered`) this basis
    // continues: any that cites one of its `(source, claim)` pairs, else the
    // one on its subject.
    fn continued(&self, ordered: &[&Requirement]) -> Vec<usize> {
        let cited: Vec<usize> = ordered
            .iter()
            .enumerate()
            .filter(|(_, requirement)| {
                requirement
                    .sources
                    .iter()
                    .any(|cited| self.classes.iter().flatten().any(|member| member.cites(cited)))
            })
            .map(|(position, _)| position)
            .collect();
        if !cited.is_empty() {
            return cited;
        }

        ordered
            .iter()
            .position(|requirement| requirement.subject == self.subject)
            .into_iter()
            .collect()
    }

    // Tells whether any contributor's claim is `subject`.
    fn holds(&self, subject: &str) -> bool {
        self.classes.iter().flatten().any(|member| member.id == subject)
    }
}

// Records one class: every member's source, the rest from the lead member.
fn loser(class: &[Contributor]) -> Loser {
    let lead = &class[0];
    Loser {
        sources: class.iter().map(|member| member.source.clone()).collect(),
        authority: lead.authority,
        claim: lead.id.clone(),
        statement: normalise(&lead.statement),
    }
}

/// One source's claim in a requirement.
#[derive(Debug, Clone)]
pub struct Contributor {
    /// The source key.
    pub source: String,
    /// The source's authority class.
    pub authority: Authority,
    /// The claim id, which may differ from the requirement's subject.
    pub id: String,
    /// The claim's `statement` extra.
    pub statement: String,
    // The claim's synopsis, shown to the grouping judgment alone.
    synopsis: Option<String>,
    // Position in source order, the tie-break within an authority.
    index: usize,
}

impl Contributor {
    // Tells whether this contributor is the `(source, claim)` pair `cited`.
    fn cites(&self, cited: &Cited) -> bool {
        (&self.source, &self.id) == (&cited.source, &cited.claim)
    }
}

impl From<&Contributor> for Cited {
    fn from(member: &Contributor) -> Self {
        Self {
            source: member.source.clone(),
            claim: member.id.clone(),
        }
    }
}

// The grouping brief: every requirement claim in source order, and every
// criterion id.
struct GroupingBrief<'a> {
    contributors: Vec<Contributor>,
    criteria: Vec<&'a str>,
}

impl<'a> GroupingBrief<'a> {
    fn collect(extracts: &'a [Extract]) -> Self {
        let mut contributors: Vec<Contributor> = Vec::new();
        let mut criteria = Vec::new();
        for extract in extracts {
            for claim in &extract.evidence.claims {
                let Some(id) = claim.id.as_deref() else { continue };
                match claim.kind {
                    ClaimKind::Requirement => contributors.push(Contributor {
                        source: extract.key.clone(),
                        authority: extract.evidence.authority,
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
        }
    }

    // The grouping settled without a model, which every answer must contain:
    // byte-equal ids are one group, whitespace-equal statements one class.
    fn baseline(&self) -> Grouping {
        let mut groups: Vec<(&str, Group)> = Vec::new();
        for (index, claim) in self.contributors.iter().enumerate() {
            let position = groups.iter().position(|(id, _)| *id == claim.id).unwrap_or_else(|| {
                groups.push((claim.id.as_str(), Group::default()));
                groups.len() - 1
            });
            let group = &mut groups[position].1;
            group.claims.push(index);
            let statement = normalise(&claim.statement);
            let class = group
                .classes
                .iter_mut()
                .find(|class| normalise(&self.contributors[class[0]].statement) == statement);

            match class {
                Some(class) => class.push(index),
                None => group.classes.push(vec![index]),
            }
        }

        Grouping {
            groups: groups.into_iter().map(|(_, group)| group).collect(),
        }
    }

    // Turns a grouping into bases, ordered by each group's earliest claim.
    fn bases(&self, grouping: &Grouping) -> Vec<Basis> {
        let mut groups: Vec<(usize, Vec<Vec<Contributor>>)> = grouping
            .groups
            .iter()
            .map(|group| {
                let first = group.claims.iter().copied().min().unwrap_or_default();
                let classes = group
                    .classes
                    .iter()
                    .map(|class| {
                        class.iter().map(|&index| self.contributors[index].clone()).collect()
                    })
                    .collect();
                (first, classes)
            })
            .collect();
        groups.sort_by_key(|(first, _)| *first);
        groups.into_iter().map(|(_, classes)| Basis::of(classes, &self.criteria)).collect()
    }
}

impl Brief for GroupingBrief<'_> {
    type Answer = Grouping;
    type Output = Vec<Basis>;

    const NAME: &'static str = "grouping";
    const PROSE: &'static [&'static str] = &["synthesis/grouping.md"];

    // Tightens the derived schema to this run: every index at most the last
    // claim's, and at least one group.
    fn tighten(&self, schema: &mut Value) {
        let last = self.contributors.len().saturating_sub(1);
        for pointer in ["/properties/claims/items", "/properties/classes/items/items"] {
            if let Some(index) = schema.pointer_mut(&format!("/$defs/Group{pointer}")) {
                index["maximum"] = json!(last);
            }
        }
        schema["properties"]["groups"]["minItems"] = json!(1);
    }

    // Verifies a candidate grouping: every claim in exactly one group, every
    // group's claims in exactly one class, and no byte-equal ids split across
    // groups.
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

        // The baseline: byte-equal ids may not be split across groups.
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

    // Turns the accepted grouping into requirements.
    fn into_output(self, answer: Grouping) -> Self::Output {
        self.bases(&answer)
    }
}

// Renders the user turn of the prompt: every requirement claim with its index
// (authority withheld), then the baseline's pre-merged groups the answer may
// not split.
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

// Tells whether `criterion` covers `requirement`: it must be that claim id or
// a dotted child of it (`session.timeout.idle` covers `session.timeout`).
fn covers(criterion: &str, requirement: &str) -> bool {
    criterion.strip_prefix(requirement).is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

// Collapses every run of whitespace to one space, so a reflowed statement
// still matches.
pub fn normalise(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
