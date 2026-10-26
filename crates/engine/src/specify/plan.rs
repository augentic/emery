//! Synthesises the build plan from requirement bases and design data types.
//!
//! Partitions requirements into dependency-ordered slices floored by stem (or
//! sub-stem past [`SLICE_CAP`]) and assigns each [`Role::Data`](crate::revision::Role)
//! type key to one slice. An amendment places what its code rules left
//! open over the current slices through [`SliceAmend`].

mod amend;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::{Trigger, is_kebab};
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

pub use self::amend::{Open, SliceAmend};
use crate::brief::{Brief, Review};
use crate::revision::{self, EMERY, Plan, ReqId, Slice, SliceId, Spec, ranks};
use crate::specify::basis::Basis;
use crate::specify::sections::BasesSection;

/// The most requirements a stem holds before its sub-stems are the slice floor.
///
/// A stem within the cap is one slice at the least. Past it, the floor is the
/// sub-stem — the first two segments of the subject's dotted id — so the model
/// may split the stem along its sub-stems and never through one. The cap is
/// sized to what one build turn implements and verifies within its budget.
pub const SLICE_CAP: usize = 12;

// A synthesis brief for the build plan.
//
// The brief contains the requirement bases, the floors they fall under,
// and the design's data type keys.
pub struct SliceBrief<'a> {
    bases: &'a [Basis<'a>],
    types: Vec<String>,
    floors: Floors<'a>,
}

impl<'a> SliceBrief<'a> {
    // Returns a slicing brief for `bases` and the design's data `types`, in key order.
    //
    // `types` are the keys slices own: the accepted design's
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
    // A specification whose requirements span two or more floors — stems, or
    // the sub-stems of a stem past `SLICE_CAP` — asks the model to slice
    // it. Otherwise the one stem is the one slice and no call is spent.
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

    // The one stem as the one slice, named for it and owning every type.
    fn whole(self) -> Plan {
        let Self { types, floors, .. } = self;
        let slices = floors
            .stems
            .into_iter()
            .map(|stem| Slice {
                id: SliceId::FIRST,
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
            slices,
        }
    }
}

// The facts the slicing gate holds a plan to: the requirements by stem, with
// the sub-stems of a stem past `SLICE_CAP`, and the triggers that reach
// them. Built from a specification's requirements or a grouping's bases
// alike, so one gate reads a specify's answer and an amended plan.
pub struct Floors<'a> {
    stems: Vec<Stem<'a>>,
    triggers: Vec<(&'a Trigger, Vec<ReqId>)>,
}

impl<'a> Floors<'a> {
    // The floors of `requirements`, each an id, its subject, and what
    // reaches it, in the order given.
    pub fn new(
        requirements: impl IntoIterator<Item = (ReqId, &'a str, Option<&'a Trigger>)>,
    ) -> Self {
        let requirements: Vec<(ReqId, &'a str, Option<&'a Trigger>)> =
            requirements.into_iter().collect();
        let mut stems: Vec<Stem<'a>> = Vec::new();
        for (id, subject, _) in &requirements {
            Stem::file(&mut stems, revision::stem(subject), *id);
        }

        // a stem past the cap is floored at its sub-stems
        for stem in stems.iter_mut().filter(|stem| stem.requirements.len() > SLICE_CAP) {
            for (id, subject, _) in &requirements {
                if revision::stem(subject) == stem.label {
                    Stem::file(&mut stem.substems, revision::substem(subject), *id);
                }
            }
        }

        // the requirements each trigger reaches, in first-occurrence order
        let mut triggers: Vec<(&'a Trigger, Vec<ReqId>)> = Vec::new();
        for (id, _, trigger) in &requirements {
            let Some(trigger) = trigger else { continue };
            match triggers.iter_mut().find(|(known, _)| known == trigger) {
                Some((_, ids)) => ids.push(*id),
                None => triggers.push((trigger, vec![*id])),
            }
        }

        Self { stems, triggers }
    }

    // The floors of `spec`'s requirements.
    pub fn from_spec(spec: &'a Spec) -> Self {
        Self::new(spec.requirements.iter().map(|requirement| {
            (requirement.id, requirement.subject.as_str(), requirement.trigger.as_ref())
        }))
    }

    // How many floors an answer may cut the specification into at most.
    fn len(&self) -> usize {
        self.iter().count()
    }

    // The floors an answer may not split, in stem order.
    fn iter(&self) -> impl Iterator<Item = &Stem<'a>> {
        self.stems.iter().flat_map(Stem::floors)
    }

    // The triggers that reach `ids`, each once, in `ids`'s order.
    fn triggers_of(&self, ids: &[ReqId]) -> Vec<&'a Trigger> {
        let mut triggers: Vec<&'a Trigger> = Vec::new();
        for id in ids {
            if let Some((trigger, _)) =
                self.triggers.iter().find(|(_, reached)| reached.contains(id))
                && !triggers.contains(trigger)
            {
                triggers.push(trigger);
            }
        }
        triggers
    }

    // The standing rules an answer is held to, as the brief states them after
    // its floors: the floor rule, the trigger rule where any trigger reaches
    // a requirement, and the cap rule where a stem is `cut` along its
    // sub-stems.
    fn write_rules(&self, f: &mut Formatter<'_>, cut: bool) -> fmt::Result {
        f.write_str(
            "Requirements sharing a stem are one slice at the least; an answer that splits them \
             across slices is refused. Merge two stems into one slice only when neither can be \
             built and verified apart from the other.\n",
        )?;
        if !self.triggers.is_empty() {
            f.write_str(
                "The triggers that reach a floor — a route, or a message — are listed after its \
                 requirements. Requirements reached through one trigger are one handler, built \
                 by one slice: an answer that splits a trigger's requirements across slices is \
                 refused.\n",
            )?;
        }
        if cut {
            writeln!(
                f,
                "A stem past the cap is listed by its sub-stems, the first two segments of its \
                 ids, and each sub-stem is the floor in its place: an answer that splits a \
                 sub-stem across slices is refused, and the sub-stems of one stem merge into \
                 slices of up to {SLICE_CAP} requirements by what builds and verifies together. \
                 Cut the stem at as few sub-stem boundaries as the cap allows, never one slice \
                 per sub-stem: an answer leaving two of a stem's slices that would fit one \
                 slice within the cap is refused, unless a third slice stands between them in \
                 `depends-on`, as the last stage stands between an entry and the first, since \
                 merging those would close a cycle; such a pair stays apart."
            )?;
        }
        Ok(())
    }

    // Holds `drafts` to the plan's rules over these floors and the design's
    // data `types`, each break a finding on `review`: every requirement in
    // exactly one slice, no floor and no trigger split across slices, a
    // stem past the cap cut no finer than the cap asks, every type key
    // owned by exactly one slice, and the dependencies naming slices with no
    // cycle among them.
    pub fn check(&self, types: &[String], drafts: &[Draft], review: &mut Review) {
        // each slice on its own
        let known: BTreeSet<ReqId> =
            self.stems.iter().flat_map(|stem| stem.requirements.iter().copied()).collect();
        let mut names = BTreeSet::new();
        let mut placed: BTreeMap<ReqId, &str> = BTreeMap::new();
        let mut owners: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for draft in drafts {
            let name = draft.name.as_str();
            let label = format!("slice `{name}`");
            if !is_kebab(name) {
                review.note(format_args!("slice name `{name}` is not kebab-case"));
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
                    review.note(format_args!("`{id}` appears in more than one slice"));
                }
            }
            for key in &draft.types {
                owners.entry(key.as_str()).or_default().insert(name);
            }
            review.paragraphs(&draft.brief, format_args!("{label} brief"));
        }

        // requirements no slice places
        for id in known.iter().filter(|id| !placed.contains_key(id)) {
            review.note(format_args!("`{id}` is in no slice"));
        }

        // floors split across slices
        for floor in self.iter() {
            let slices: BTreeSet<&str> =
                floor.requirements.iter().filter_map(|id| placed.get(id).copied()).collect();
            if slices.len() > 1 {
                review.note(format_args!(
                    "requirements sharing the stem `{}` are split across {}",
                    floor.label,
                    quoted(slices)
                ));
            }
        }

        // triggers split across slices
        for (trigger, ids) in &self.triggers {
            let slices: BTreeSet<&str> =
                ids.iter().filter_map(|id| placed.get(id).copied()).collect();
            if slices.len() > 1 {
                review.note(format_args!(
                    "requirements reached through `{trigger}` ({}) are split across {}: one \
                     trigger is one handler, built by one slice",
                    listed(ids),
                    quoted(slices)
                ));
            }
        }

        self.verify_cut(drafts, &placed, review);

        // type ownership
        let keys: BTreeSet<&str> = types.iter().map(String::as_str).collect();
        for key in &keys {
            match owners.get(key) {
                Some(slices) if slices.len() == 1 => {}
                None => review.note(format_args!("type `{key}` is owned by no slice")),
                Some(slices) => review.note(format_args!(
                    "type `{key}` is owned by {} slices: {}",
                    slices.len(),
                    quoted(slices.iter().copied())
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
                    review.note(format_args!("slice `{name}` depends on itself"));
                } else if !names.contains(dependency.as_str()) {
                    review.note(format_args!(
                        "slice `{name}` depends on `{dependency}`, which is not a slice"
                    ));
                }
            }
        }
        let cyclic = unorderable(drafts);
        if !cyclic.is_empty() {
            review.note(format_args!(
                "slices {} cannot be ordered: their `depends-on` edges contain a cycle",
                quoted(cyclic)
            ));
        }
    }

    // A stem past the cap cut finer than the cap asks: two of its slices
    // that would fit one within the cap are a finding, the smallest pair
    // named. A pair a third slice stands between in `depends-on` cannot
    // merge without closing a cycle, so it is no finding.
    fn verify_cut(&self, drafts: &[Draft], placed: &BTreeMap<ReqId, &str>, review: &mut Review) {
        let sizes: BTreeMap<&str, usize> =
            drafts.iter().map(|draft| (draft.name.as_str(), draft.requirements.len())).collect();
        let indirect = indirect(drafts);
        let apart = |a: &str, b: &str| {
            indirect.get(a).is_some_and(|reached| reached.contains(b))
                || indirect.get(b).is_some_and(|reached| reached.contains(a))
        };
        for stem in self.stems.iter().filter(|stem| !stem.substems.is_empty()) {
            let names: BTreeSet<&str> =
                stem.requirements.iter().filter_map(|id| placed.get(id).copied()).collect();
            let mut slices: Vec<(usize, &str)> = names
                .into_iter()
                .map(|name| (sizes.get(name).copied().unwrap_or(0), name))
                .collect();
            slices.sort_unstable();
            let pair = slices
                .iter()
                .enumerate()
                .flat_map(|(index, x)| slices[index + 1..].iter().map(move |y| (x, y)))
                .filter(|((first, a), (second, b))| first + second <= SLICE_CAP && !apart(a, b))
                .min_by_key(|((first, _), (second, _))| first + second);
            if let Some(((first, a), (second, b))) = pair {
                review.note(format_args!(
                    "the stem `{}` is cut into {} slices, yet `{a}` ({first} requirements) and \
                     `{b}` ({second}) fit one slice of {} within the cap of {SLICE_CAP}: merge \
                     the stem's slices until no two that could merge fit together; two a third \
                     slice stands between in `depends-on`, an entry and the first stage it \
                     runs, stay apart",
                    stem.label,
                    slices.len(),
                    first + second
                ));
            }
        }
    }
}

impl Brief for SliceBrief<'_> {
    type Answer = SliceAnswer;
    type Output = Plan;

    const NAME: &'static str = "slicing";
    const PROSE: &'static [&'static str] = &["slicing.md"];

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        let slices = Self::at(schema, "/properties/slices")?;
        slices.insert("minItems".to_owned(), json!(1));
        slices.insert("maxItems".to_owned(), json!(self.floors.len()));

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

    fn verify(&self, answer: &SliceAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");
        self.floors.check(&self.types, &answer.slices, review);
    }

    // Slices are numbered by their lowest requirement, so re-runs over one
    // specification number alike; every list is written in id order.
    fn into_output(self, answer: SliceAnswer) -> Result<Plan, Error> {
        let mut drafts = answer.slices;
        drafts.sort_by_key(|draft| draft.requirements.iter().min().copied());
        let ids: BTreeMap<String, SliceId> = drafts
            .iter()
            .zip(SliceId::sequence())
            .map(|(draft, id)| (draft.name.clone(), id))
            .collect();

        let mut slices = Vec::with_capacity(drafts.len());
        for (draft, id) in drafts.into_iter().zip(SliceId::sequence()) {
            slices.push(draft.into_slice(id, &ids, &self.types)?);
        }

        Ok(Plan {
            emery: EMERY,
            parent: None,
            preamble: answer.preamble,
            slices,
        })
    }
}

fn quoted<'a>(names: impl IntoIterator<Item = &'a str>) -> String {
    names.into_iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
}

fn listed(ids: &[ReqId]) -> String {
    ids.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
}

// `slices` as the gate reads them, each dependency by the name of the slice
// it names, or its id where no slice holds it, so the gate names the break.
pub fn drafts(slices: &[Slice]) -> Vec<Draft> {
    let name = |id: SliceId| {
        slices
            .iter()
            .find(|slice| slice.id == id)
            .map_or_else(|| id.to_string(), |slice| slice.name.clone())
    };
    slices
        .iter()
        .map(|slice| Draft {
            name: slice.name.clone(),
            requirements: slice.requirements.clone(),
            types: slice.types.clone(),
            depends_on: slice.depends_on.iter().map(|id| name(*id)).collect(),
            brief: slice.brief.clone(),
        })
        .collect()
}

// The triggers that reach a floor, as the baseline lists them after its
// requirements; nothing for a floor nothing reaches.
fn triggered(triggers: &[&Trigger]) -> String {
    if triggers.is_empty() {
        return String::new();
    }
    format!(
        " — triggers: {}",
        triggers.iter().map(|trigger| format!("`{trigger}`")).collect::<Vec<_>>().join(", ")
    )
}

impl Display for SliceBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Slice the specification into a build plan.\n\n## Baseline\n\n")?;
        for stem in &self.floors.stems {
            if stem.substems.is_empty() {
                writeln!(
                    f,
                    "- `{}` — {}{}",
                    stem.label,
                    listed(&stem.requirements),
                    triggered(&self.floors.triggers_of(&stem.requirements))
                )?;
                continue;
            }
            writeln!(
                f,
                "- `{}` — {} requirements, past the cap of {SLICE_CAP}, so its sub-stems are \
                 the floor:",
                stem.label,
                stem.requirements.len()
            )?;
            for substem in &stem.substems {
                writeln!(
                    f,
                    "  - `{}` — {}{}",
                    substem.label,
                    listed(&substem.requirements),
                    triggered(&self.floors.triggers_of(&substem.requirements))
                )?;
            }
        }
        f.write_str("\n")?;
        self.floors
            .write_rules(f, self.floors.stems.iter().any(|stem| !stem.substems.is_empty()))?;

        if self.types.is_empty() {
            f.write_str(
                "\n## Type keys\n\nThe design has no data type; answer every `types` list \
                 empty.\n",
            )?;
        } else {
            f.write_str(
                "\n## Type keys\n\nThe design's data types. Each key is owned by exactly one \
                 slice, the one that builds the requirements reading or producing it, whatever \
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
/// The engine supplies slice identifiers and the order of every list.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery slicing answer")]
pub struct SliceAnswer {
    /// Markdown paragraphs before the first slice.
    pub preamble: Vec<String>,
    /// One draft per slice, in any order.
    pub slices: Vec<Draft>,
}

// The dependencies of each draft on the other drafts by name: a dependency
// on a name that is no slice, or on the slice itself, is found elsewhere and
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

// The slice names a cycle leaves unorderable.
fn unorderable(drafts: &[Draft]) -> Vec<&str> {
    let pending = direct(drafts)
        .into_iter()
        .map(|(name, dependencies)| (name, dependencies.into_iter().collect()))
        .collect();
    let (_, cyclic) = ranks(pending);
    cyclic
}

// The slices each one depends on through another slice: the dependencies
// of its dependencies, and theirs. Merging a slice with one it reaches so
// would leave the slice between them depending on itself.
fn indirect(drafts: &[Draft]) -> BTreeMap<&str, BTreeSet<&str>> {
    let direct = direct(drafts);
    let step = |name: &str| direct.get(name).into_iter().flatten().copied();

    direct
        .keys()
        .map(|&name| {
            let mut reached = BTreeSet::new();
            let mut pending: Vec<&str> = step(name).flat_map(step).collect();
            while let Some(next) = pending.pop() {
                if reached.insert(next) {
                    pending.extend(step(next));
                }
            }
            (name, reached)
        })
        .collect()
}

/// The drafted content of one slice, before the engine numbers and orders it.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Draft {
    /// A kebab-case name unique within the plan.
    pub name: String,
    /// The requirement ids this slice builds; every stem whole, or every
    /// sub-stem of a stem past the cap.
    #[schemars(with = "Vec<String>")]
    pub requirements: Vec<ReqId>,
    /// The design type keys this slice owns.
    #[serde(default)]
    pub types: Vec<String>,
    /// The names of the slices this one is built after.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Markdown paragraphs a builder reads before taking the slice up.
    #[serde(default)]
    pub brief: Vec<String>,
}

impl Draft {
    // Lowers this draft to a numbered `Slice`, its `depends_on` mapped through
    // `ids` and every list sorted and deduplicated, `types` in the design's
    // order.
    fn into_slice(
        mut self, id: SliceId, ids: &BTreeMap<String, SliceId>, types: &[String],
    ) -> Result<Slice, Error> {
        self.requirements.sort_unstable();
        self.requirements.dedup();
        let types = types.iter().filter(|key| self.types.contains(key)).cloned().collect();
        let mut depends_on = self
            .depends_on
            .iter()
            .map(|name| {
                ids.get(name).copied().ok_or_else(|| {
                    server_error!(
                        "slice `{}` depends on `{name}`, which was accepted without a slice",
                        self.name
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        depends_on.sort_unstable();
        depends_on.dedup();

        Ok(Slice {
            id,
            name: self.name,
            requirements: self.requirements,
            types,
            depends_on,
            brief: self.brief,
        })
    }
}

// The requirements under one stem, or under one sub-stem of a stem past the
// cap, whose `substems` are then its floors in place of the stem itself.
struct Stem<'a> {
    label: &'a str,
    requirements: Vec<ReqId>,
    substems: Vec<Self>,
}

impl<'a> Stem<'a> {
    // Files `id` under `label`, in first-occurrence order.
    fn file(stems: &mut Vec<Self>, label: &'a str, id: ReqId) {
        match stems.iter_mut().find(|entry| entry.label == label) {
            Some(entry) => entry.requirements.push(id),
            None => stems.push(Self {
                label,
                requirements: vec![id],
                substems: Vec::new(),
            }),
        }
    }

    // The floors an answer may not split: the stem whole within the cap, each
    // of its sub-stems past it.
    fn floors(&self) -> impl Iterator<Item = &Self> {
        let whole = self.substems.is_empty().then_some(self);
        whole.into_iter().chain(&self.substems)
    }
}
