//! Places what an amended plan's code rules leave open, in one slicing turn.
//!
//! An amendment keeps the current plan's slices and places by code what a
//! rule decides. What no rule decides — the requirements of a stem no slice
//! holds, a stem the amendment leaves past the cap, and a design type no
//! slice owns — is put to one turn over the current slices as fixed context,
//! and the answer is held to the plan's rules over the whole.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use omnia_sdk::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::brief::{Brief, Review};
use crate::revision::{EMERY, Plan, ReqId, Slice, SliceId, Spec};
use crate::specify::basis::Basis;
use crate::specify::plan::{
    Draft, Floors, SLICE_CAP, SliceBrief, Stem, drafts, listed, quoted, triggered,
};
use crate::specify::sections::BasesSection;

// What the slicing turn decides of an amended plan: what the code rules
// left unplaced.
#[derive(Debug, Default)]
pub struct Open {
    // The requirements no current slice holds: those under a stem no slice
    // holds, and every one of a stem cut again.
    pub ids: BTreeSet<ReqId>,
    // The stems the amendment leaves past the cap, every requirement of
    // which is open, in stem order.
    pub recut: Vec<String>,
    // The design's data type keys no current slice owns, in the design's
    // order.
    pub keys: Vec<String>,
}

impl Open {
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty() && self.keys.is_empty()
    }
}

// The brief of an amendment's slicing turn: the current slices as fixed
// context, and what the answer places.
pub struct SliceAmend<'a> {
    base: &'a Plan,
    parent: String,
    // The current slices as the code rules left them: the base's less what
    // the change retired and moved, with what code placed, in id order.
    slices: Vec<Slice>,
    open: Open,
    floors: Floors<'a>,
    // The design's data type keys, in the design's order.
    types: &'a [String],
    spec: &'a Spec,
    // The requirements the change adds, revises, or leaves in conflict.
    drafted: &'a [Basis<'a>],
}

impl<'a> SliceAmend<'a> {
    // Returns the brief over `slices`, the current plan as the code rules
    // left it, with `open` what they left for the turn.
    //
    // `base` is the plan amended under `parent`, `spec` the amended
    // specification, `types` the amended design's data type keys, and
    // `drafted` the requirements the change drafts anew.
    #[must_use]
    pub fn new(
        base: &'a Plan, parent: String, slices: Vec<Slice>, open: Open, spec: &'a Spec,
        types: &'a [String], drafted: &'a [Basis<'a>],
    ) -> Self {
        Self {
            base,
            parent,
            slices,
            open,
            floors: Floors::from_spec(spec),
            types,
            spec,
            drafted,
        }
    }

    // Whether this amendment changed `slice` against the base: its
    // requirements differ, or one of them is drafted anew.
    fn touched(&self, slice: &Slice) -> bool {
        self.base.slice(slice.id).is_none_or(|was| was.requirements != slice.requirements)
            || slice.requirements.iter().any(|id| self.drafted.iter().any(|basis| basis.id == *id))
    }

    // The current slices that held `stem` in the base, by id and name.
    fn holders(&self, stem: &Stem<'a>) -> Vec<&Slice> {
        self.slices
            .iter()
            .filter(|slice| {
                self.base.slice(slice.id).is_some_and(|was| {
                    was.requirements.iter().any(|id| stem.requirements.contains(id))
                })
            })
            .collect()
    }

    // The open requirements of `stem`, listed with the triggers that reach
    // them.
    fn opened(&self, stem: &Stem<'a>) -> String {
        let ids: Vec<ReqId> =
            stem.requirements.iter().copied().filter(|id| self.open.ids.contains(id)).collect();
        format!("{}{}", listed(&ids), triggered(&self.floors.triggers_of(&ids)))
    }

    // Notes each of `ids` and `keys` the current plan places, which the
    // answer may not move.
    fn moved(&self, review: &mut Review, label: &str, ids: &[ReqId], keys: &[String]) {
        for id in ids.iter().filter(|id| !self.open.ids.contains(id)) {
            review.note(format_args!(
                "{label}: `{id}` is not open to this amendment; the current plan places it, and \
                 it stays"
            ));
        }
        for key in keys.iter().filter(|key| !self.open.keys.contains(key)) {
            review.note(format_args!(
                "{label}: type `{key}` is not open to this amendment; the current plan owns it, \
                 and it stays"
            ));
        }
    }

    // The whole plan as the gate reads it: the current slices as `answer`
    // joins them, then its new slices, every dependency by name.
    fn assembled(&self, answer: &SliceAmendAnswer) -> Vec<Draft> {
        let mut drafts = drafts(&self.slices);
        for join in &answer.joins {
            let Some(position) = self.slices.iter().position(|slice| slice.id == join.slice) else {
                continue;
            };
            drafts[position].requirements.extend(join.requirements.iter().copied());
            drafts[position].types.extend(join.types.iter().cloned());
        }
        drafts.extend(answer.slices.iter().cloned());
        drafts
    }

    // `## Current plan`: each current slice on one line, as the code rules
    // left it.
    fn write_current(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = |id: &SliceId| {
            self.slices
                .iter()
                .find(|slice| slice.id == *id)
                .map_or_else(|| id.to_string(), |slice| slice.name.clone())
        };
        f.write_str("## Current plan\n\n")?;
        for slice in &self.slices {
            write!(f, "- `{}` `{}` — ", slice.id, slice.name)?;
            if slice.requirements.is_empty() {
                f.write_str("no requirement; every one it held is open")?;
            } else {
                f.write_str(&listed(&slice.requirements))?;
            }
            if !slice.types.is_empty() {
                write!(f, " — types: {}", quoted(slice.types.iter().map(String::as_str)))?;
            }
            if !slice.depends_on.is_empty() {
                let names: Vec<String> = slice.depends_on.iter().map(name).collect();
                write!(f, " — depends on: {}", quoted(names.iter().map(String::as_str)))?;
            }
            if self.touched(slice) {
                f.write_str(" — touched by this amendment")?;
            }
            f.write_str("\n")?;
        }
        Ok(())
    }

    // `## Open`: the new stems, the recut stems by their sub-stems, the
    // unowned keys, then the standing rules.
    fn write_open(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\n## Open\n\n")?;
        let (recut, new): (Vec<&Stem<'a>>, Vec<&Stem<'a>>) = self
            .floors
            .stems
            .iter()
            .filter(|stem| stem.requirements.iter().any(|id| self.open.ids.contains(id)))
            .partition(|stem| self.open.recut.iter().any(|label| label == stem.label));
        if !new.is_empty() {
            f.write_str(
                "Requirements under a stem no current slice holds, each stem one slice at the \
                 least: draft a new slice for it, named for what it builds, or join it into a \
                 current slice only where neither can be built and verified apart from the \
                 other.\n\n",
            )?;
            for stem in &new {
                writeln!(f, "- `{}` — {}", stem.label, self.opened(stem))?;
            }
            f.write_str("\n")?;
        }
        if !recut.is_empty() {
            writeln!(
                f,
                "Stems the amendment leaves past the cap of {SLICE_CAP}, every requirement of \
                 which is open, so the stem is cut again along its sub-stems: join the sub-stems \
                 that stay into the current slice that held them, which keeps its id and keeps \
                 at least one, and draft a new slice for each other set of sub-stems, cut at as \
                 few sub-stem boundaries as the cap allows; the types that slice owned are open \
                 with them.\n"
            )?;
            for stem in &recut {
                self.write_recut(f, stem)?;
            }
            f.write_str("\n")?;
        }
        if !self.open.keys.is_empty() {
            f.write_str(
                "Type keys the design holds that no current slice owns, each to be owned by \
                 exactly one slice — a new slice's `types`, or a join's — the one that builds \
                 the requirements reading or producing it, whatever module declared it in the \
                 source:\n\n",
            )?;
            for key in &self.open.keys {
                writeln!(f, "- `{key}`")?;
            }
            f.write_str("\n")?;
        }
        self.floors.write_rules(f, !recut.is_empty())
    }

    // One recut stem: its size, the slices that held it, and its sub-stems
    // with what reaches them.
    fn write_recut(&self, f: &mut Formatter<'_>, stem: &Stem<'a>) -> fmt::Result {
        let holders: Vec<String> = self
            .holders(stem)
            .into_iter()
            .map(|slice| format!("`{}` `{}`", slice.id, slice.name))
            .collect();
        writeln!(
            f,
            "- `{}` — {} requirements, past the cap of {SLICE_CAP}, so its sub-stems are the \
             floor; held by {}:",
            stem.label,
            stem.requirements.len(),
            if holders.is_empty() { "no current slice".to_owned() } else { holders.join(", ") }
        )?;
        for substem in &stem.substems {
            writeln!(f, "  - `{}` — {}", substem.label, self.opened(substem))?;
        }
        Ok(())
    }

    // `## Requirements`: the drafted bases, then the standing open
    // requirements with their statements.
    fn write_requirements(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\n## Requirements\n\n")?;
        if self.drafted.is_empty() {
            f.write_str(
                "The change adds or revises no requirement; the specification's stand as they \
                 are.\n",
            )?;
        } else {
            write!(
                f,
                "The requirements the change adds or revises, drafted beside this \
                 plan:\n\n{bases}",
                bases = BasesSection(self.drafted)
            )?;
        }
        let standing: Vec<_> = self
            .open
            .ids
            .iter()
            .filter(|id| self.drafted.iter().all(|basis| basis.id != **id))
            .filter_map(|id| self.spec.requirement(*id))
            .collect();
        if standing.is_empty() {
            return Ok(());
        }
        f.write_str(
            "\nThe standing requirements the answer places, as the specification holds \
             them:\n\n",
        )?;
        for requirement in standing {
            writeln!(
                f,
                "- {} `{}` — {}",
                requirement.id,
                requirement.subject,
                requirement.statement().unwrap_or_else(|| "[conflict]".to_owned())
            )?;
        }
        Ok(())
    }
}

impl Brief for SliceAmend<'_> {
    type Answer = SliceAmendAnswer;
    type Output = Plan;

    const NAME: &'static str = SliceBrief::NAME;
    const PROSE: &'static [&'static str] = SliceBrief::PROSE;

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        let ids = json!(self.open.ids.iter().collect::<Vec<_>>());
        let slices = json!(self.slices.iter().map(|slice| slice.id).collect::<Vec<_>>());

        // at most one new slice per open requirement, each over the open
        // requirements alone
        Self::at(schema, "/properties/slices")?
            .insert("maxItems".to_owned(), json!(self.open.ids.len()));
        Self::at(schema, "/properties/joins")?
            .insert("maxItems".to_owned(), json!(self.slices.len()));
        for pointer in
            ["/$defs/Draft/properties/requirements", "/$defs/Join/properties/requirements"]
        {
            if self.open.ids.is_empty() {
                Self::at(schema, pointer)?.insert("maxItems".to_owned(), json!(0));
            } else {
                Self::at(schema, &format!("{pointer}/items"))?
                    .insert("enum".to_owned(), ids.clone());
            }
        }
        if !self.open.ids.is_empty() {
            Self::at(schema, "/$defs/Draft/properties/requirements")?
                .insert("minItems".to_owned(), json!(1));
        }

        // the open type keys alone, and a join names a current slice
        for pointer in ["/$defs/Draft/properties/types", "/$defs/Join/properties/types"] {
            if self.open.keys.is_empty() {
                Self::at(schema, pointer)?.insert("maxItems".to_owned(), json!(0));
            } else {
                Self::at(schema, &format!("{pointer}/items"))?
                    .insert("enum".to_owned(), json!(self.open.keys));
            }
        }
        Self::at(schema, "/$defs/Join/properties/slice")?.insert("enum".to_owned(), slices);
        Ok(())
    }

    fn verify(&self, answer: &SliceAmendAnswer, review: &mut Review) {
        // the new slices over the open set alone, named apart from the
        // current ones
        let names: BTreeSet<&str> = self.slices.iter().map(|slice| slice.name.as_str()).collect();
        for draft in &answer.slices {
            let label = format!("slice `{}`", draft.name);
            if names.contains(draft.name.as_str()) {
                review.note(format_args!(
                    "`{}` names a current slice; list what joins it under `joins`, or name the \
                     new slice otherwise",
                    draft.name
                ));
            }
            self.moved(review, &label, &draft.requirements, &draft.types);
        }

        // each join a current slice, once, over the open set alone
        let mut joined = BTreeSet::new();
        for join in &answer.joins {
            let label = format!("`joins` `{}`", join.slice);
            if self.slices.iter().all(|slice| slice.id != join.slice) {
                review.note(format_args!("{label} names no slice of the current plan"));
            } else if !joined.insert(join.slice) {
                review.note(format_args!(
                    "{label} is listed more than once; list what joins it once"
                ));
            }
            self.moved(review, &label, &join.requirements, &join.types);
        }

        // a current slice keeps its id, so one whose requirements are all
        // open takes at least one back
        let drafts = self.assembled(answer);
        for (slice, draft) in self.slices.iter().zip(&drafts) {
            if draft.requirements.is_empty() {
                review.note(format_args!(
                    "`{}` ({}) is left with no requirement; a current slice keeps its id, so \
                     join at least one of its open requirements back into it",
                    slice.name, slice.id
                ));
            }
        }

        // the whole held to the plan's rules
        self.floors.check(self.types, &drafts, review);
    }

    // New slices are numbered after the current plan's highest id by their
    // lowest requirement; every list is written in id order, every type
    // list in the design's.
    fn into_output(self, answer: SliceAmendAnswer) -> Result<Plan, Error> {
        let mut slices = self.slices;
        for join in answer.joins {
            let slice =
                slices.iter_mut().find(|slice| slice.id == join.slice).ok_or_else(|| {
                    server_error!("`{}` was accepted joining no slice of the plan", join.slice)
                })?;
            slice.requirements.extend(join.requirements);
            slice.requirements.sort_unstable();
            slice.requirements.dedup();
            slice.types.extend(join.types);
            slice.types =
                self.types.iter().filter(|key| slice.types.contains(key)).cloned().collect();
        }

        let mut drafts = answer.slices;
        drafts.sort_by_key(|draft| draft.requirements.iter().min().copied());
        let mut free_ids = self.base.free_ids();
        let mut ids: BTreeMap<String, SliceId> =
            slices.iter().map(|slice| (slice.name.clone(), slice.id)).collect();
        let mut numbered = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let id = free_ids.next().ok_or_else(|| server_error!("the slice ids are exhausted"))?;
            ids.insert(draft.name.clone(), id);
            numbered.push((draft, id));
        }
        for (draft, id) in numbered {
            slices.push(draft.into_slice(id, &ids, self.types)?);
        }

        Ok(Plan {
            emery: EMERY,
            parent: Some(self.parent),
            preamble: self.base.preamble.clone(),
            slices,
        })
    }
}

impl Display for SliceAmend<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            "Amend the build plan: the current plan's slices stand under their ids, names, and \
             briefs, and the answer places what `## Open` lists, and nothing else — each open \
             requirement and type key into a new slice under `slices`, or into a current slice \
             under `joins`.\n\n",
        )?;
        self.write_current(f)?;
        self.write_open(f)?;
        self.write_requirements(f)
    }
}

/// Model-authored content for an amended build plan: what the current plan leaves open, placed.
///
/// The engine keeps the current plan's slices, numbers the new ones, and
/// orders every list.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery slicing amendment")]
pub struct SliceAmendAnswer {
    /// The new slices, each over open requirements and type keys alone, in
    /// any order.
    pub slices: Vec<Draft>,
    /// The open requirements and type keys that join a current slice, one
    /// entry per slice joined.
    pub joins: Vec<Join>,
}

/// Open requirements and type keys joining one slice of the current plan.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Join {
    /// The id of the current slice they join.
    #[schemars(with = "String")]
    pub slice: SliceId,
    /// The open requirement ids that join it.
    #[serde(default)]
    #[schemars(with = "Vec<String>")]
    pub requirements: Vec<ReqId>,
    /// The open design type keys it owns.
    #[serde(default)]
    pub types: Vec<String>,
}
