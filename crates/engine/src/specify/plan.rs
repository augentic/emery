//! Synthesises the build plan over the requirement bases and the design's
//! type keys.
//!
//! Requirements sharing a stem — the first segment of their subject's dotted
//! id — are one slice at the least. The model may merge stems into one slice
//! and never splits one; it names each slice, assigns it the design types it
//! owns, orders slices by build dependency, and briefs each. The engine
//! validates that partition, numbers the slices by their lowest requirement,
//! and writes every list in canonical order. A specification under one stem
//! is one slice and no call is spent. The brief runs from the bases and the
//! keys the design will reference, so it is derived beside the specification
//! and design drafts rather than after them.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::is_kebab;
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::revision::{EMERY, Plan, ReqId, Slice, SliceId, toposort};
use crate::specify::basis::Basis;
use crate::specify::brief::{BasesSection, Brief, Review};

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
            let stem = basis.stem();
            match stems.iter_mut().find(|entry| entry.stem == stem) {
                Some(entry) => entry.requirements.push(basis.id),
                None => stems.push(Stem {
                    stem,
                    requirements: vec![basis.id],
                }),
            }
        }

        Self { bases, types, stems }
    }

    /// Derives the build plan.
    ///
    /// A specification whose requirements span two or more stems asks the
    /// model to slice it. Otherwise the one stem is the one slice and no call
    /// is spent.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the model cannot produce a valid
    ///   plan within the available rounds.
    /// - Returns [`Error::ServerError`] when required prose is missing or a
    ///   plan cannot be reconciled with the specification.
    /// - Returns [`Error::BadGateway`] when the model operation fails.
    pub async fn derive<M: Model>(self, model: &M) -> Result<Plan, Error> {
        if self.stems.len() < 2 {
            return Ok(self.whole());
        }
        self.judge(model).await
    }

    // The one stem as the one slice, named for it and owning every type.
    fn whole(self) -> Plan {
        let Self { types, stems, .. } = self;
        let slices = stems
            .into_iter()
            .map(|stem| Slice {
                id: SliceId::new(1),
                name: stem.stem.to_owned(),
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
        schema["properties"]["slices"]["maxItems"] = json!(self.stems.len());

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

        // stems split across slices
        for stem in &self.stems {
            let slices: BTreeSet<&str> =
                stem.requirements.iter().filter_map(|id| placed.get(id).copied()).collect();
            if slices.len() > 1 {
                review.note(format_args!(
                    "requirements sharing the stem `{}` are split across {}",
                    stem.stem,
                    quoted(slices)
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

impl Display for SliceBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Slice the specification into a build plan.\n\n## Baseline\n\n")?;
        for stem in &self.stems {
            let ids = stem.requirements.iter().map(ToString::to_string).collect::<Vec<_>>();
            writeln!(f, "- `{}` — {}", stem.stem, ids.join(", "))?;
        }
        f.write_str(
            "\nRequirements sharing a stem are one slice at the least; an answer that splits \
             them across slices is refused. Merge two stems into one slice only when neither \
             can be built and verified apart from the other.\n",
        )?;

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

        let (_, cyclic) = toposort(pending);
        cyclic
    }
}

/// The drafted content of one slice, before the engine numbers and orders it.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Draft {
    /// A kebab-case name unique within the plan.
    pub name: String,
    /// The requirement ids this slice builds; every stem whole.
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

struct Stem<'a> {
    stem: &'a str,
    requirements: Vec<ReqId>,
}
