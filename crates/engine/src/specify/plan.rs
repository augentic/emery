//! Synthesises the build plan over the requirement bases and the design's
//! type keys.
//!
//! Requirements sharing a stem — the first segment of their subject's dotted
//! id — are one slice at the least. A stem of more than [`SLICE_CAP`]
//! requirements is floored at its sub-stems instead — the first two segments
//! — so the model may split it along them and never through one. The model
//! may merge floors into one slice; it names each slice, assigns it the
//! design types it owns, orders slices by build dependency, and briefs each.
//! The engine validates that partition, numbers the slices by their lowest
//! requirement, and writes every list in canonical order. A specification
//! under one floor is one slice and no call is spent. The brief runs from the
//! bases and the keys the design will reference, so it is derived beside the
//! specification and design drafts rather than after them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::is_kebab;
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::revision::{EMERY, Plan, ReqId, Slice, SliceId, ranks};
use crate::specify::basis::Basis;
use crate::specify::brief::{BasesSection, Brief, Review};

/// The most requirements a stem holds before its sub-stems are the slice floor.
///
/// A stem within the cap is one slice at the least. Past it, the floor is the
/// sub-stem — the first two segments of the subject's dotted id — so the model
/// may split the stem along its sub-stems and never through one.
pub const SLICE_CAP: usize = 20;

/// A synthesis brief for the build plan.
///
/// The brief contains the requirement bases, the stems they fall under, and
/// the design's type keys.
pub struct SliceBrief<'a> {
    bases: &'a [Basis<'a>],
    types: Vec<String>,
    stems: Vec<Stem<'a>>,
}

impl<'a> SliceBrief<'a> {
    /// Returns a slicing brief for `bases` and the design's `types`, in key order.
    #[must_use]
    pub fn new(bases: &'a [Basis<'a>], types: Vec<String>) -> Self {
        let mut stems: Vec<Stem<'a>> = Vec::new();
        for basis in bases {
            Stem::file(&mut stems, basis.stem(), basis.id);
        }

        // a stem past the cap is floored at its sub-stems
        for stem in stems.iter_mut().filter(|stem| stem.requirements.len() > SLICE_CAP) {
            for basis in bases.iter().filter(|basis| basis.stem() == stem.label) {
                Stem::file(&mut stem.substems, basis.substem(), basis.id);
            }
        }

        Self { bases, types, stems }
    }

    /// Derives the build plan.
    ///
    /// A specification whose requirements span two or more floors — stems, or
    /// the sub-stems of a stem past [`SLICE_CAP`] — asks the model to slice
    /// it. Otherwise the one stem is the one slice and no call is spent.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the model cannot produce a valid
    ///   plan within the available rounds.
    /// - Returns [`Error::ServerError`] when required prose is missing or a
    ///   plan cannot be reconciled with the specification.
    /// - Returns [`Error::BadGateway`] when the model operation fails.
    pub async fn derive<M: Model>(self, model: &M) -> Result<Plan, Error> {
        if self.floors().count() < 2 {
            return Ok(self.whole());
        }
        self.judge(model).await
    }

    // The floors an answer may not split, in stem order.
    fn floors(&self) -> impl Iterator<Item = &Stem<'a>> {
        self.stems.iter().flat_map(Stem::floors)
    }

    // The one stem as the one slice, named for it and owning every type.
    fn whole(self) -> Plan {
        let Self { types, stems, .. } = self;
        let slices = stems
            .into_iter()
            .map(|stem| Slice {
                id: SliceId::new(1),
                name: stem.label.to_owned(),
                requirements: stem.requirements,
                types: types.clone(),
                depends_on: vec![],
                brief: vec![],
            })
            .collect();

        Plan {
            emery: EMERY,
            preamble: vec![],
            slices,
        }
    }
}

impl Brief for SliceBrief<'_> {
    type Answer = SliceAnswer;
    type Output = Plan;

    const NAME: &'static str = "slicing";
    const PROSE: &'static [&'static str] = &["slicing.md"];

    fn tighten(&self, schema: &mut Value) {
        schema["properties"]["slices"]["minItems"] = json!(1);
        schema["properties"]["slices"]["maxItems"] = json!(self.floors().count());

        // restrict each list to this run's requirement ids and type keys
        let draft = &mut schema["$defs"]["Draft"]["properties"];
        draft["requirements"]["minItems"] = json!(1);
        draft["requirements"]["items"]["enum"] =
            json!(self.bases.iter().map(|basis| basis.id).collect::<Vec<_>>());
        if self.types.is_empty() {
            draft["types"]["maxItems"] = json!(0);
        } else {
            draft["types"]["items"]["enum"] = json!(self.types);
        }
    }

    fn verify(&self, answer: &SliceAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");

        // each slice on its own
        let known: BTreeSet<ReqId> = self.bases.iter().map(|basis| basis.id).collect();
        let mut names = BTreeSet::new();
        let mut placed: BTreeMap<ReqId, &str> = BTreeMap::new();
        let mut owners: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for draft in &answer.slices {
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
        for floor in self.floors() {
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

        // a stem past the cap cut finer than the cap asks
        let sizes: BTreeMap<&str, usize> = answer
            .slices
            .iter()
            .map(|draft| (draft.name.as_str(), draft.requirements.len()))
            .collect();
        for stem in self.stems.iter().filter(|stem| !stem.substems.is_empty()) {
            let names: BTreeSet<&str> =
                stem.requirements.iter().filter_map(|id| placed.get(id).copied()).collect();
            let mut slices: Vec<(usize, &str)> = names
                .into_iter()
                .map(|name| (sizes.get(name).copied().unwrap_or(0), name))
                .collect();
            slices.sort_unstable();
            if let [(first, a), (second, b), ..] = slices[..]
                && first + second <= SLICE_CAP
            {
                review.note(format_args!(
                    "the stem `{}` is cut into {} slices, yet `{a}` ({first} requirements) and \
                     `{b}` ({second}) fit one slice of {} within the cap of {SLICE_CAP}: merge \
                     the stem's slices until no two fit together",
                    stem.label,
                    slices.len(),
                    first + second
                ));
            }
        }

        // type ownership
        let keys: BTreeSet<&str> = self.types.iter().map(String::as_str).collect();
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
        for draft in &answer.slices {
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
        let cyclic = answer.unorderable();
        if !cyclic.is_empty() {
            review.note(format_args!(
                "slices {} cannot be ordered: their `depends-on` edges contain a cycle",
                quoted(cyclic)
            ));
        }
    }

    // Slices are numbered by their lowest requirement, so re-runs over one
    // specification number alike; every list is written in id order.
    fn into_output(self, answer: SliceAnswer) -> Result<Plan, Error> {
        let mut drafts = answer.slices;
        drafts.sort_by_key(|draft| draft.requirements.iter().min().copied());
        let ids: BTreeMap<String, SliceId> = drafts
            .iter()
            .zip(1..)
            .map(|(draft, number)| (draft.name.clone(), SliceId::new(number)))
            .collect();

        let mut slices = Vec::with_capacity(drafts.len());
        for (mut draft, number) in drafts.into_iter().zip(1..) {
            draft.requirements.sort_unstable();
            let types =
                self.types.iter().filter(|key| draft.types.contains(key)).cloned().collect();
            let mut depends_on = draft
                .depends_on
                .iter()
                .map(|name| {
                    ids.get(name).copied().ok_or_else(|| {
                        server_error!(
                            "slice `{}` depends on `{name}`, which was accepted without a slice",
                            draft.name
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            depends_on.sort_unstable();
            depends_on.dedup();

            slices.push(Slice {
                id: SliceId::new(number),
                name: draft.name,
                requirements: draft.requirements,
                types,
                depends_on,
                brief: draft.brief,
            });
        }

        Ok(Plan {
            emery: EMERY,
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

impl Display for SliceBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Slice the specification into a build plan.\n\n## Baseline\n\n")?;
        for stem in &self.stems {
            if stem.substems.is_empty() {
                writeln!(f, "- `{}` — {}", stem.label, listed(&stem.requirements))?;
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
                writeln!(f, "  - `{}` — {}", substem.label, listed(&substem.requirements))?;
            }
        }
        f.write_str(
            "\nRequirements sharing a stem are one slice at the least; an answer that splits \
             them across slices is refused. Merge two stems into one slice only when neither \
             can be built and verified apart from the other.\n",
        )?;
        if self.stems.iter().any(|stem| !stem.substems.is_empty()) {
            writeln!(
                f,
                "A stem past the cap is listed by its sub-stems, the first two segments of its \
                 ids, and each sub-stem is the floor in its place: an answer that splits a \
                 sub-stem across slices is refused, and the sub-stems of one stem merge into \
                 slices of up to {SLICE_CAP} requirements by what builds and verifies together. \
                 Cut the stem at as few sub-stem boundaries as the cap allows, never one slice \
                 per sub-stem: an answer leaving two of a stem's slices that would fit one \
                 slice within the cap is refused."
            )?;
        }

        if self.types.is_empty() {
            f.write_str(
                "\n## Type keys\n\nThe design has no type; answer every `types` list empty.\n",
            )?;
        } else {
            f.write_str(
                "\n## Type keys\n\nEach key is owned by exactly one slice, the one that builds \
                 the requirements defining it.\n\n",
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

impl SliceAnswer {
    // The slice names a cycle leaves unorderable. A dependency on a name that
    // is no slice, or on the slice itself, is found elsewhere and does not
    // count.
    fn unorderable(&self) -> Vec<&str> {
        let names: BTreeSet<&str> = self.slices.iter().map(|draft| draft.name.as_str()).collect();
        let pending = self
            .slices
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
            .collect();

        let (_, cyclic) = ranks(pending);
        cyclic
    }
}

/// The drafted content of one slice, before the engine numbers and orders it.
#[derive(Debug, Deserialize, JsonSchema)]
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
