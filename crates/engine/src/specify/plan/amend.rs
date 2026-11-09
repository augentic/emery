//! Places what an amended plan's code rules leave open, in one slicing turn.
//!
//! An amendment keeps the current plan's parts and places by code what a
//! rule decides. What no rule decides — the requirements of a stem no part
//! holds, and a design type no part owns — is put to one turn over the
//! current parts as fixed context, and the answer is held to the plan's
//! rules over the whole. The turn also redrafts the brief of each current
//! part the amendment touched, and of each one its answer joins into, so
//! what a builder reads says what the part now delivers.

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use omnia_sdk::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::brief::{Brief, Review};
use crate::revision::{EMERY, Part, PartId, Plan, ReqId};
use crate::specify::basis::Basis;
use crate::specify::plan::{Draft, Floors, PartBrief, Stem, Types, listed, quoted};
use crate::specify::sections::BasesSection;

// What the slicing turn decides of an amended plan: what the code rules
// left unplaced.
#[derive(Debug, Default)]
pub struct Open {
    // The requirements no current part holds: those under a stem no part
    // holds.
    pub ids: BTreeSet<ReqId>,
    // The design's data type keys no current part owns, in the design's
    // order.
    pub keys: Vec<String>,
}

impl Open {
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty() && self.keys.is_empty()
    }
}

// The brief of an amendment's slicing turn: the current parts as fixed
// context, and what the answer places.
pub struct PartAmend<'a> {
    base: &'a Plan,
    parent: String,
    // The current parts as the code rules left them: the base's less what
    // the change retired and moved, with what code placed, in id order.
    parts: Vec<Part>,
    open: Open,
    floors: Floors<'a>,
    // The design's data types.
    types: &'a Types,
    // The requirements the change adds, revises, or leaves in conflict.
    drafted: &'a [Basis<'a>],
}

impl<'a> PartAmend<'a> {
    // Returns the brief over `parts`, the current plan as the code rules
    // left it, with `open` what they left for the turn.
    //
    // `base` is the plan amended under `parent`, `floors` the amended
    // specification's floors, `types` the amended design's data types,
    // and `drafted` the requirements the change drafts anew.
    #[must_use]
    pub const fn new(
        base: &'a Plan, parent: String, parts: Vec<Part>, open: Open, floors: Floors<'a>,
        types: &'a Types, drafted: &'a [Basis<'a>],
    ) -> Self {
        Self {
            base,
            parent,
            parts,
            open,
            floors,
            types,
            drafted,
        }
    }

    fn part(&self, id: PartId) -> Option<&Part> {
        self.parts.iter().find(|part| part.id == id)
    }

    // Whether this amendment changed `part` against the base: its
    // requirements differ, or one of them is drafted anew.
    fn touched(&self, part: &Part) -> bool {
        self.base.part(part.id).is_none_or(|was| was.requirements != part.requirements)
            || part.requirements.iter().any(|id| self.drafted.iter().any(|basis| basis.id == *id))
    }

    // The open requirements of `stem`, listed with the triggers that reach
    // them.
    fn opened(&self, stem: &Stem<'a>) -> String {
        let ids: Vec<ReqId> =
            stem.requirements.iter().copied().filter(|id| self.open.ids.contains(id)).collect();
        format!("{}{}", listed(&ids), self.floors.triggered(&ids))
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

    // The whole plan as the gate reads it: the current parts as `answer`
    // joins them, then its new parts.
    fn assembled(&self, answer: &PartAmendAnswer) -> Vec<Draft> {
        let mut drafts = Draft::all(&self.parts);
        for join in &answer.joins {
            let Some(position) = self.parts.iter().position(|part| part.id == join.part) else {
                continue;
            };
            drafts[position].requirements.extend(join.requirements.iter().copied());
            drafts[position].types.extend(join.types.iter().cloned());
            drafts[position].uses.retain(|key| !join.types.contains(key));
            drafts[position].uses.extend(join.uses.iter().cloned());
        }
        drafts.extend(answer.parts.iter().cloned());
        drafts
    }

    // `## Current plan`: each current part on one line, as the code rules
    // left it.
    fn write_current(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name =
            |id: &PartId| self.part(*id).map_or_else(|| id.to_string(), |part| part.name.clone());
        f.write_str("## Current plan\n\n")?;
        for part in &self.parts {
            write!(f, "- `{}` `{}` — {}", part.id, part.name, listed(&part.requirements))?;
            if !part.types.is_empty() {
                write!(f, " — types: {}", quoted(&part.types))?;
            }
            if !part.uses.is_empty() {
                write!(f, " — uses: {}", quoted(&part.uses))?;
            }
            if !part.depends_on.is_empty() {
                write!(f, " — depends on: {}", quoted(part.depends_on.iter().map(name)))?;
            }
            if self.touched(part) {
                f.write_str(" — touched by this amendment")?;
            }
            f.write_str("\n")?;
        }
        f.write_str(
            "\nA part marked touched gained, lost, or had redrafted a requirement, so its brief \
             no longer says what it delivers: redraft it under `briefs`, one entry per touched \
             part and per part a join names, each a brief whole in the part's new terms. Every \
             other part keeps its brief.\n",
        )
    }

    // `## Open`: the new stems, the unowned keys, then the standing rules.
    fn write_open(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\n## Open\n\n")?;
        let new: Vec<&Stem<'a>> = self
            .floors
            .stems
            .iter()
            .filter(|stem| stem.requirements.iter().any(|id| self.open.ids.contains(id)))
            .collect();
        if !new.is_empty() {
            f.write_str(
                "Requirements under a stem no current part holds, each stem one part at the \
                 least: draft a new part for it, named for what it builds, or join it into a \
                 current part only where neither can be built and verified apart from the \
                 other.\n\n",
            )?;
            for stem in &new {
                writeln!(f, "- `{}` — {}", stem.label, self.opened(stem))?;
            }
            f.write_str("\n")?;
        }
        if !self.open.keys.is_empty() {
            f.write_str(
                "Type keys the design holds that no current part owns, each to be owned by \
                 exactly one part — a new part's `types`, or a join's — the one that builds \
                 the requirements reading or producing it, whatever module declared it in the \
                 source:\n\n",
            )?;
            for key in &self.open.keys {
                self.types.write_key(f, key)?;
            }
            f.write_str("\n")?;
        }
        self.floors.write_rules(f)
    }

    // `## Requirements`: the drafted bases.
    fn write_requirements(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\n## Requirements\n\n")?;
        if self.drafted.is_empty() {
            f.write_str(
                "The change adds or revises no requirement; the specification's stand as they \
                 are.\n",
            )
        } else {
            write!(
                f,
                "The requirements the change adds or revises, drafted beside this \
                 plan:\n\n{bases}",
                bases = BasesSection(self.drafted)
            )
        }
    }
}

impl Brief for PartAmend<'_> {
    type Answer = PartAmendAnswer;
    type Output = Plan;

    const NAME: &'static str = PartBrief::NAME;
    const PROSE: &'static [&'static str] = PartBrief::PROSE;

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        let ids = json!(self.open.ids.iter().collect::<Vec<_>>());
        let parts = json!(self.parts.iter().map(|part| part.id).collect::<Vec<_>>());

        // at most one new part per open requirement, each over the open
        // requirements alone
        Self::at(schema, "/properties/parts")?
            .insert("maxItems".to_owned(), json!(self.open.ids.len()));
        Self::at(schema, "/properties/joins")?
            .insert("maxItems".to_owned(), json!(self.parts.len()));
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

        // the open type keys alone, and a join names a current part
        for pointer in ["/$defs/Draft/properties/types", "/$defs/Join/properties/types"] {
            if self.open.keys.is_empty() {
                Self::at(schema, pointer)?.insert("maxItems".to_owned(), json!(0));
            } else {
                Self::at(schema, &format!("{pointer}/items"))?
                    .insert("enum".to_owned(), json!(self.open.keys));
            }
        }
        for pointer in ["/$defs/Draft/properties/uses", "/$defs/Join/properties/uses"] {
            if self.types.is_empty() {
                Self::at(schema, pointer)?.insert("maxItems".to_owned(), json!(0));
            } else {
                Self::at(schema, &format!("{pointer}/items"))?
                    .insert("enum".to_owned(), json!(self.types.keys));
            }
        }
        Self::at(schema, "/$defs/Join/properties/part")?.insert("enum".to_owned(), parts.clone());

        // one brief per touched part at the least, one per current part at
        // the most, since a join's part is redrafted too
        let touched = self.parts.iter().filter(|part| self.touched(part)).count();
        let briefs = Self::at(schema, "/properties/briefs")?;
        briefs.insert("minItems".to_owned(), json!(touched));
        briefs.insert("maxItems".to_owned(), json!(self.parts.len()));
        Self::at(schema, "/$defs/Rebrief/properties/part")?.insert("enum".to_owned(), parts);
        Self::at(schema, "/$defs/Rebrief/properties/brief")?
            .insert("minItems".to_owned(), json!(1));
        Ok(())
    }

    fn verify(&self, answer: &PartAmendAnswer, review: &mut Review) {
        // the new parts over the open set alone, named apart from the
        // current ones
        let names: BTreeSet<&str> = self.parts.iter().map(|part| part.name.as_str()).collect();
        for draft in &answer.parts {
            let label = format!("part `{}`", draft.name);
            if names.contains(draft.name.as_str()) {
                review.note(format_args!(
                    "`{}` names a current part; list what joins it under `joins`, or name the \
                     new part otherwise",
                    draft.name
                ));
            }
            self.moved(review, &label, &draft.requirements, &draft.types);
        }

        // each join a current part, once, over the open set alone
        let mut joined = BTreeSet::new();
        for join in &answer.joins {
            let label = format!("`joins` `{}`", join.part);
            if self.part(join.part).is_none() {
                review.note(format_args!("{label} names no part of the current plan"));
            } else if !joined.insert(join.part) {
                review.note(format_args!(
                    "{label} is listed more than once; list what joins it once"
                ));
            }
            self.moved(review, &label, &join.requirements, &join.types);
        }

        // one brief per part the amendment touched or a join named, each
        // whole, and none for a part that stands as it was
        let mut rebriefed = BTreeSet::new();
        for rebrief in &answer.briefs {
            let label = format!("`briefs` `{}`", rebrief.part);
            match self.part(rebrief.part) {
                None => review.note(format_args!("{label} names no part of the current plan")),
                Some(part) if !self.touched(part) && !joined.contains(&part.id) => {
                    review.note(format_args!(
                        "{label}: `{}` stands as it was; it keeps its brief, so list no entry \
                         for it",
                        part.name
                    ));
                }
                Some(_) if !rebriefed.insert(rebrief.part) => {
                    review.note(format_args!("{label} is listed more than once; redraft it once"));
                }
                Some(part) if rebrief.brief.is_empty() => review.note(format_args!(
                    "{label}: the brief is empty; redraft `{}`'s brief whole",
                    part.name
                )),
                Some(_) => {}
            }
            review.paragraphs(&rebrief.brief, format_args!("{label} brief"));
        }
        for part in &self.parts {
            if (self.touched(part) || joined.contains(&part.id)) && !rebriefed.contains(&part.id) {
                review.note(format_args!(
                    "`{}` ({}) is touched by this amendment, or joined by the answer, and its \
                     brief is not redrafted; list it under `briefs`",
                    part.name, part.id
                ));
            }
        }

        // the whole held to the plan's rules
        let drafts = self.assembled(answer);
        self.floors.check(self.types, &drafts, review);
    }

    // New parts are numbered after the current plan's highest id by their
    // lowest requirement; every list is written in id order, every type
    // list in the design's; a redrafted brief stands in its part's place.
    fn into_output(self, answer: PartAmendAnswer) -> Result<Plan, Error> {
        let mut parts = self.parts;
        for join in answer.joins {
            let part = parts.iter_mut().find(|part| part.id == join.part).ok_or_else(|| {
                server_error!("`{}` was accepted joining no part of the plan", join.part)
            })?;
            part.requirements.extend(join.requirements);
            part.requirements.sort_unstable();
            part.requirements.dedup();
            part.types.extend(join.types);
            part.uses.extend(join.uses);
            self.types.settle(&mut part.types, &mut part.uses);
        }
        for rebrief in answer.briefs {
            let part = parts.iter_mut().find(|part| part.id == rebrief.part).ok_or_else(|| {
                server_error!("`{}` was accepted rebriefing no part of the plan", rebrief.part)
            })?;
            part.brief = rebrief.brief;
        }

        let mut drafts = answer.parts;
        drafts.sort_by_key(|draft| draft.requirements.iter().min().copied());
        let mut free_ids = self.base.free_ids();
        for draft in drafts {
            let id = free_ids.next().ok_or_else(|| server_error!("the part ids are exhausted"))?;
            parts.push(draft.into_part(id, self.types));
        }
        self.types.depend(&mut parts);

        Ok(Plan {
            emery: EMERY,
            parent: Some(self.parent),
            preamble: self.base.preamble.clone(),
            parts,
        })
    }
}

impl Display for PartAmend<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            "Amend the build plan: the current plan's parts stand under their ids and names, and \
             the answer places what `## Open` lists, and nothing else — each open requirement \
             and type key into a new part under `parts`, or into a current part under \
             `joins` — and redrafts under `briefs` the brief of each part this amendment \
             touched or a join names.\n\n",
        )?;
        self.write_current(f)?;
        self.write_open(f)?;
        self.write_requirements(f)
    }
}

/// Model-authored content for an amended build plan: what the current plan leaves open, placed.
///
/// The engine keeps the current plan's parts, numbers the new ones, and
/// orders every list.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery slicing amendment")]
pub struct PartAmendAnswer {
    /// The new parts, each over open requirements and type keys alone, in
    /// any order.
    pub parts: Vec<Draft>,
    /// The open requirements and type keys that join a current part, one
    /// entry per part joined.
    pub joins: Vec<Join>,
    /// The briefs redrafted, one entry per current part the amendment
    /// touched and per part a join names, in any order.
    pub briefs: Vec<Rebrief>,
}

/// The brief of one part of the current plan, redrafted whole.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rebrief {
    /// The id of the current part whose brief this is.
    #[schemars(with = "String")]
    pub part: PartId,
    /// Markdown paragraphs a builder reads before taking the part up, in
    /// place of the ones it held.
    pub brief: Vec<String>,
}

/// Open requirements and type keys joining one part of the current plan.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Join {
    /// The id of the current part they join.
    #[schemars(with = "String")]
    pub part: PartId,
    /// The open requirement ids that join it.
    #[serde(default)]
    #[schemars(with = "Vec<String>")]
    pub requirements: Vec<ReqId>,
    /// The open design type keys it owns.
    #[serde(default)]
    pub types: Vec<String>,
    /// The design type keys it uses without owning.
    #[serde(default)]
    pub uses: Vec<String>,
}
