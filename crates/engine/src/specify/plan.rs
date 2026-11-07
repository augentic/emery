//! Synthesises the build plan from requirement bases and design data types.
//!
//! Partitions requirements into dependency-ordered parts floored by stem
//! and assigns each [`Role::Data`](crate::revision::Role)
//! type key to one part. An amendment places what its code rules left
//! open over the current parts through [`PartAmend`].

mod amend;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::{Trigger, is_kebab};
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

pub use self::amend::{Open, PartAmend};
use crate::brief::{Brief, Review};
use crate::revision::{self, EMERY, Part, PartId, Plan, ReqId, Spec, ranks};
use crate::specify::basis::Basis;
use crate::specify::sections::BasesSection;

// A synthesis brief for the build plan.
//
// The brief contains the requirement bases, the floors they fall under,
// and the design's data type keys.
pub struct PartBrief<'a> {
    bases: &'a [Basis<'a>],
    types: Vec<String>,
    floors: Floors<'a>,
}

impl<'a> PartBrief<'a> {
    // Returns a slicing brief for `bases` and the design's data `types`, in key order.
    //
    // `types` are the keys parts own: the accepted design's
    // `Design::data_types`.
    #[must_use]
    pub fn new(bases: &'a [Basis<'a>], types: Vec<String>) -> Self {
        let floors = Floors::new(
            bases.iter().map(|basis| (basis.id, basis.subject, basis.trigger.as_ref())),
        );
        Self { bases, types, floors }
    }

    // Derives the build plan.
    //
    // A specification whose requirements span two or more stems asks the
    // model to slice it. Otherwise the one stem is the one part and no
    // call is spent.
    //
    // - Returns `Error::BadRequest` when the model cannot produce a valid
    //   plan within the available rounds.
    // - Returns `Error::ServerError` when required prose is missing or a
    //   plan cannot be reconciled with the specification.
    // - Returns `Error::BadGateway` when the model operation fails.
    pub async fn derive<M: Model>(self, model: &M) -> Result<Plan, Error> {
        if self.floors.len() < 2 {
            return Ok(self.whole());
        }
        self.judge(model).await
    }

    // The one stem as the one part, named for it and owning every type.
    fn whole(self) -> Plan {
        let Self { types, floors, .. } = self;
        let parts = floors
            .stems
            .into_iter()
            .map(|stem| Part {
                id: PartId::FIRST,
                name: stem.label.to_owned(),
                requirements: stem.requirements,
                types: types.clone(),
                depends_on: vec![],
                brief: vec![],
            })
            .collect();

        Plan {
            emery: EMERY,
            parent: None,
            preamble: vec![],
            parts,
        }
    }
}

// The facts the slicing gate holds a plan to: the requirements by stem, and
// the triggers that reach them. Built from a specification's requirements or
// a grouping's bases alike, so one gate reads a specify's answer and an
// amended plan.
pub struct Floors<'a> {
    stems: Vec<Stem<'a>>,
    triggers: Vec<(&'a Trigger, Vec<ReqId>)>,
}

impl<'a> Floors<'a> {
    // The floors of `requirements`, each an id, its subject, and what
    // reaches it, in the order given.
    fn new(requirements: impl IntoIterator<Item = (ReqId, &'a str, Option<&'a Trigger>)>) -> Self {
        let requirements: Vec<(ReqId, &'a str, Option<&'a Trigger>)> =
            requirements.into_iter().collect();
        let mut stems: Vec<Stem<'a>> = Vec::new();
        for (id, subject, _) in &requirements {
            Stem::file(&mut stems, revision::stem(subject), *id);
        }

        // the requirements each trigger reaches, in first-occurrence order; a
        // trigger that reaches more than one non-`start` stem is a relative
        // route registered per stem, so each stem holds its own handler
        let mut triggers: Vec<(&'a Trigger, Option<&'a str>, Vec<ReqId>)> = Vec::new();
        for &(id, subject, trigger) in &requirements {
            let Some(trigger) = trigger else { continue };
            let mut domains = requirements
                .iter()
                .filter(|(_, _, other)| *other == Some(trigger))
                .map(|(_, other, _)| revision::stem(other))
                .filter(|&stem| stem != "start");
            let single = domains.next().is_none_or(|first| domains.all(|stem| stem == first));
            let scope = (!single).then(|| revision::stem(subject));
            match triggers.iter_mut().find(|(known, held, _)| *known == trigger && *held == scope) {
                Some((_, _, ids)) => ids.push(id),
                None => triggers.push((trigger, scope, vec![id])),
            }
        }
        let triggers = triggers.into_iter().map(|(trigger, _, ids)| (trigger, ids)).collect();

        Self { stems, triggers }
    }

    // The floors of `spec`'s requirements.
    pub fn from_spec(spec: &'a Spec) -> Self {
        Self::new(spec.requirements.iter().map(|requirement| {
            (requirement.id, requirement.subject.as_str(), requirement.trigger.as_ref())
        }))
    }

    // Whether `a` and `b` are reached through one trigger handler.
    pub fn shares_trigger(&self, a: ReqId, b: ReqId) -> bool {
        self.triggers.iter().any(|(_, reached)| reached.contains(&a) && reached.contains(&b))
    }

    // How many floors an answer may cut the specification into at most.
    const fn len(&self) -> usize {
        self.stems.len()
    }

    // The triggers that reach `ids`, each once in `ids`'s order, as a
    // baseline line lists them after its requirements; empty when none does.
    fn triggered(&self, ids: &[ReqId]) -> String {
        let mut triggers: Vec<&'a Trigger> = Vec::new();
        for id in ids {
            if let Some((trigger, _)) =
                self.triggers.iter().find(|(_, reached)| reached.contains(id))
                && !triggers.contains(trigger)
            {
                triggers.push(trigger);
            }
        }
        if triggers.is_empty() {
            return String::new();
        }
        format!(" — triggers: {}", quoted(triggers))
    }

    // The standing rules an answer is held to, as the brief states them after
    // its floors: the floor rule, and the trigger rule where any trigger
    // reaches a requirement.
    fn write_rules(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            "Requirements sharing a stem are one part at the least; an answer that splits them \
             across parts is refused. Merge two stems into one part only when neither can be \
             built and verified apart from the other.\n",
        )?;
        if !self.triggers.is_empty() {
            f.write_str(
                "The triggers that reach a floor — a route, or a message — are listed after its \
                 requirements. Requirements reached through one trigger are one handler, built \
                 by one part: an answer that splits a trigger's requirements across parts is \
                 refused.\n",
            )?;
        }
        Ok(())
    }

    // Holds `drafts` to the plan's rules over these floors and the design's
    // data `types`, each break a finding on `review`: every requirement in
    // exactly one part, no stem and no trigger split across parts, every
    // type key owned by exactly one part, and the dependencies naming
    // parts with no cycle among them.
    pub fn check(&self, types: &[String], drafts: &[Draft], review: &mut Review) {
        // each part on its own
        let known: BTreeSet<ReqId> =
            self.stems.iter().flat_map(|stem| stem.requirements.iter().copied()).collect();
        let mut names = BTreeSet::new();
        let mut placed: BTreeMap<ReqId, &str> = BTreeMap::new();
        let mut owners: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for draft in drafts {
            let name = draft.name.as_str();
            let label = format!("part `{name}`");
            if !is_kebab(name) {
                review.note(format_args!("part name `{name}` is not kebab-case"));
            }
            if !names.insert(name) {
                review.note(format_args!("{label} is drafted more than once"));
            }
            if draft.requirements.is_empty() {
                review.note(format_args!("{label} has no requirement"));
            }
            for &id in &draft.requirements {
                if !known.contains(&id) {
                    review.note(format_args!("{label}: `{id}` is not a requirement"));
                } else if placed.insert(id, name).is_some() {
                    review.note(format_args!("`{id}` appears in more than one part"));
                }
            }
            for key in &draft.types {
                owners.entry(key.as_str()).or_default().insert(name);
            }
            review.paragraphs(&draft.brief, format_args!("{label} brief"));
        }

        // requirements no part places
        for id in known.iter().filter(|id| !placed.contains_key(id)) {
            review.note(format_args!("`{id}` is in no part"));
        }

        // stems split across parts
        for stem in &self.stems {
            let parts: BTreeSet<&str> =
                stem.requirements.iter().filter_map(|id| placed.get(id).copied()).collect();
            if parts.len() > 1 {
                review.note(format_args!(
                    "requirements sharing the stem `{}` are split across {}",
                    stem.label,
                    quoted(parts)
                ));
            }
        }

        // triggers split across parts
        for (trigger, ids) in &self.triggers {
            let parts: BTreeSet<&str> =
                ids.iter().filter_map(|id| placed.get(id).copied()).collect();
            if parts.len() > 1 {
                review.note(format_args!(
                    "requirements reached through `{trigger}` ({}) are split across {}: one \
                     trigger is one handler, built by one part",
                    listed(ids),
                    quoted(parts)
                ));
            }
        }

        // type ownership
        let keys: BTreeSet<&str> = types.iter().map(String::as_str).collect();
        for key in &keys {
            match owners.get(key) {
                Some(parts) if parts.len() == 1 => {}
                None => review.note(format_args!("type `{key}` is owned by no part")),
                Some(parts) => review.note(format_args!(
                    "type `{key}` is owned by {} parts: {}",
                    parts.len(),
                    quoted(parts)
                )),
            }
        }
        for key in owners.keys().filter(|key| !keys.contains(*key)) {
            review.note(format_args!("`{key}` is not a type in the design"));
        }

        // dependencies
        for draft in drafts {
            let name = draft.name.as_str();
            for dependency in &draft.depends_on {
                if dependency == name {
                    review.note(format_args!("part `{name}` depends on itself"));
                } else if !names.contains(dependency.as_str()) {
                    review.note(format_args!(
                        "part `{name}` depends on `{dependency}`, which is not a part"
                    ));
                }
            }
        }
        let cyclic = unorderable(drafts);
        if !cyclic.is_empty() {
            review.note(format_args!(
                "parts {} cannot be ordered: their `depends-on` edges contain a cycle",
                quoted(cyclic)
            ));
        }
    }
}

impl Brief for PartBrief<'_> {
    type Answer = PartAnswer;
    type Output = Plan;

    const NAME: &'static str = "slicing";
    const PROSE: &'static [&'static str] = &["slicing.md"];

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        let parts = Self::at(schema, "/properties/parts")?;
        parts.insert("minItems".to_owned(), json!(1));
        parts.insert("maxItems".to_owned(), json!(self.floors.len()));

        // restrict each list to this run's requirement ids and type keys
        let ids = json!(self.bases.iter().map(|basis| basis.id).collect::<Vec<_>>());
        Self::at(schema, "/$defs/Draft/properties/requirements")?
            .insert("minItems".to_owned(), json!(1));
        Self::at(schema, "/$defs/Draft/properties/requirements/items")?
            .insert("enum".to_owned(), ids);
        if self.types.is_empty() {
            Self::at(schema, "/$defs/Draft/properties/types")?
                .insert("maxItems".to_owned(), json!(0));
        } else {
            Self::at(schema, "/$defs/Draft/properties/types/items")?
                .insert("enum".to_owned(), json!(self.types));
        }
        Ok(())
    }

    fn verify(&self, answer: &PartAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");
        self.floors.check(&self.types, &answer.parts, review);
    }

    // Parts are numbered by their lowest requirement, so re-runs over one
    // specification number alike; every list is written in id order.
    fn into_output(self, answer: PartAnswer) -> Result<Plan, Error> {
        let mut drafts = answer.parts;
        drafts.sort_by_key(|draft| draft.requirements.iter().min().copied());
        let ids: BTreeMap<String, PartId> = drafts
            .iter()
            .zip(PartId::sequence())
            .map(|(draft, id)| (draft.name.clone(), id))
            .collect();

        let mut parts = Vec::with_capacity(drafts.len());
        for (draft, id) in drafts.into_iter().zip(PartId::sequence()) {
            parts.push(draft.into_part(id, &ids, &self.types)?);
        }

        Ok(Plan {
            emery: EMERY,
            parent: None,
            preamble: answer.preamble,
            parts,
        })
    }
}

fn quoted(names: impl IntoIterator<Item: Display>) -> String {
    names.into_iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
}

fn listed(ids: &[ReqId]) -> String {
    ids.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
}

// `parts` as the gate reads them, each dependency by the name of the part
// it names, or its id where no part holds it, so the gate names the break.
pub fn drafts(parts: &[Part]) -> Vec<Draft> {
    let name = |id: PartId| {
        parts
            .iter()
            .find(|part| part.id == id)
            .map_or_else(|| id.to_string(), |part| part.name.clone())
    };
    parts
        .iter()
        .map(|part| Draft {
            name: part.name.clone(),
            requirements: part.requirements.clone(),
            types: part.types.clone(),
            depends_on: part.depends_on.iter().map(|id| name(*id)).collect(),
            brief: part.brief.clone(),
        })
        .collect()
}

impl Display for PartBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Slice the specification into a build plan.\n\n## Baseline\n\n")?;
        for stem in &self.floors.stems {
            writeln!(
                f,
                "- `{}` — {}{}",
                stem.label,
                listed(&stem.requirements),
                self.floors.triggered(&stem.requirements)
            )?;
        }
        f.write_str("\n")?;
        self.floors.write_rules(f)?;

        if self.types.is_empty() {
            f.write_str(
                "\n## Type keys\n\nThe design has no data type; answer every `types` list \
                 empty.\n",
            )?;
        } else {
            f.write_str(
                "\n## Type keys\n\nThe design's data types. Each key is owned by exactly one \
                 part, the one that builds the requirements reading or producing it, whatever \
                 module declared it in the source.\n\n",
            )?;
            for key in &self.types {
                writeln!(f, "- `{key}`")?;
            }
        }

        write!(f, "\n## Requirements\n\n{bases}", bases = BasesSection(self.bases))
    }
}

/// Model-authored content for a build plan.
///
/// The engine supplies part identifiers and the order of every list.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery slicing answer")]
pub struct PartAnswer {
    /// Markdown paragraphs before the first part.
    pub preamble: Vec<String>,
    /// One draft per part, in any order.
    pub parts: Vec<Draft>,
}

// The dependencies of each draft on the other drafts by name: a dependency
// on a name that is no part, or on the part itself, is found elsewhere and
// does not count.
fn direct(drafts: &[Draft]) -> BTreeMap<&str, Vec<&str>> {
    let names: BTreeSet<&str> = drafts.iter().map(|draft| draft.name.as_str()).collect();
    drafts
        .iter()
        .map(|draft| {
            let dependencies = draft
                .depends_on
                .iter()
                .map(String::as_str)
                .filter(|dependency| names.contains(dependency) && *dependency != draft.name)
                .collect();
            (draft.name.as_str(), dependencies)
        })
        .collect()
}

// The part names a cycle leaves unorderable.
fn unorderable(drafts: &[Draft]) -> Vec<&str> {
    let pending = direct(drafts)
        .into_iter()
        .map(|(name, dependencies)| (name, dependencies.into_iter().collect()))
        .collect();
    let (_, cyclic) = ranks(pending);
    cyclic
}

/// The drafted content of one part, before the engine numbers and orders it.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Draft {
    /// A kebab-case name unique within the plan.
    pub name: String,
    /// The requirement ids this part builds; every stem whole.
    #[schemars(with = "Vec<String>")]
    pub requirements: Vec<ReqId>,
    /// The design type keys this part owns.
    #[serde(default)]
    pub types: Vec<String>,
    /// The names of the parts this one is built after.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Markdown paragraphs a builder reads before taking the part up.
    #[serde(default)]
    pub brief: Vec<String>,
}

impl Draft {
    // Lowers this draft to a numbered `Part`, its `depends_on` mapped through
    // `ids` and every list sorted and deduplicated, `types` in the design's
    // order.
    fn into_part(
        mut self, id: PartId, ids: &BTreeMap<String, PartId>, types: &[String],
    ) -> Result<Part, Error> {
        self.requirements.sort_unstable();
        self.requirements.dedup();
        let types = types.iter().filter(|key| self.types.contains(key)).cloned().collect();
        let mut depends_on = self
            .depends_on
            .iter()
            .map(|name| {
                ids.get(name).copied().ok_or_else(|| {
                    server_error!(
                        "part `{}` depends on `{name}`, which was accepted without a part",
                        self.name
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        depends_on.sort_unstable();
        depends_on.dedup();

        Ok(Part {
            id,
            name: self.name,
            requirements: self.requirements,
            types,
            depends_on,
            brief: self.brief,
        })
    }
}

// The requirements under one stem.
struct Stem<'a> {
    label: &'a str,
    requirements: Vec<ReqId>,
}

impl<'a> Stem<'a> {
    // Files `id` under `label`, in first-occurrence order.
    fn file(stems: &mut Vec<Self>, label: &'a str, id: ReqId) {
        match stems.iter_mut().find(|entry| entry.label == label) {
            Some(entry) => entry.requirements.push(id),
            None => stems.push(Self {
                label,
                requirements: vec![id],
            }),
        }
    }
}
