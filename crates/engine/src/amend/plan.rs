//! Derives the amended plan from the base's by code, and opens one slicing
//! turn for what code cannot decide.
//!
//! An amendment keeps the base plan's parts under their ids and names and
//! changes their membership only as the change set says. [`Pruned::new`]
//! takes the retired requirements out of their parts before the design
//! turn, so the types a part they emptied owned leave the design with it,
//! and takes out the requirements the change moved under another stem;
//! [`Pruned::assemble`] places what the amendment adds and moves after the
//! turn where a rule decides, owns the design's data types as the amendment
//! leaves them, and answers the plan settled, or the slicing turn's brief
//! over the current parts for what no rule placed: a stem no part holds,
//! a type key no part owns.

use std::collections::{BTreeMap, BTreeSet};

use omnia_sdk::{Error, server_error};

use crate::brief::Review;
use crate::revision::{Change, ChangeClass, EMERY, Part, PartId, Plan, ReqId, Spec};
use crate::specify::{Basis, Floors, Open, PartAmend, Types, drafts};

// The base plan with the requirements that leave their parts taken out and
// any part they emptied removed, ready to place what the amendment adds
// and moves once the specification and the design are settled.
pub struct Pruned<'a> {
    base: &'a Plan,
    parent: String,
    parts: Vec<Part>,
    // The design type keys the parts emptied by retirement owned.
    pub orphaned: BTreeSet<String>,
}

impl<'a> Pruned<'a> {
    // Removes from `base` the requirements `changes` retires and the ones it
    // moves under another stem from `spec`, under `parent` as the revision
    // amended.
    //
    // A part every requirement of which leaves is removed, and every part
    // that depended on it depends on what it depended on instead,
    // transitively, each list deduplicated and sorted. The types a part
    // emptied by retirement owned are `orphaned`, so they leave the design
    // before the design turn; the types of a part emptied by a move stay in
    // the design and are owned again by `assemble`.
    pub fn new(base: &'a Plan, spec: &Spec, changes: &[Change], parent: &str) -> Self {
        let retired: BTreeSet<ReqId> = changes
            .iter()
            .filter(|change| change.class == ChangeClass::Retired)
            .map(|change| change.id)
            .collect();
        let (parts, removed) = without(&base.parts, &retired);
        let orphaned = removed.into_iter().flat_map(|part| part.types.iter().cloned()).collect();

        let moved: BTreeSet<ReqId> = changes
            .iter()
            .filter(|change| change.class == ChangeClass::Changed)
            .filter(|change| {
                spec.requirement(change.id)
                    .is_some_and(|requirement| requirement.stem() != change.stem())
            })
            .map(|change| change.id)
            .collect();
        let (parts, _) = without(&parts, &moved);

        Self {
            base,
            parent: parent.to_owned(),
            parts,
            orphaned,
        }
    }

    // Places into the pruned parts what the amendment adds and moves, and
    // owns the design's data `types`, under the rules of the plan.
    //
    // Each requirement of `spec` no part holds is placed by code where a
    // rule decides: into the part holding another requirement its trigger
    // reaches, else the part holding its stem. A type key the design no
    // longer holds leaves its part, and one no part owns is owned by the
    // one part whose requirements this amendment changed against `base`,
    // where nothing else is open. What no rule decides is open: the
    // requirements of a stem no part holds, and a type key where no part
    // or more than one changed.
    //
    // Returns the plan `Settled` where nothing is open, held to the plan's
    // rules, and the slicing turn's brief `Open` over the current parts
    // otherwise, `drafted` the requirements the change drafts anew.
    //
    // Returns `Error::ServerError` for a settled plan the rules refuse, since
    // nothing of it is the model's.
    pub fn assemble(
        mut self, spec: &'a Spec, types: &'a Types, drafted: &'a [Basis<'a>],
    ) -> Result<Assembled<'a>, Error> {
        let floors = Floors::from_spec(spec);
        let mut open = self.place(spec, &floors);
        self.own(types, &mut open);
        if !open.is_empty() {
            return Ok(Assembled::Open(PartAmend::new(
                self.base,
                self.parent,
                self.parts,
                open,
                floors,
                types,
                drafted,
            )));
        }
        held(&self.parts, &floors, types)?;
        Ok(Assembled::Settled(Plan {
            emery: EMERY,
            parent: Some(self.parent),
            preamble: self.base.preamble.clone(),
            parts: self.parts,
        }))
    }

    // Places every requirement of `spec` no part holds, in id order, and
    // returns what no rule placed. Every part's requirements are sorted.
    fn place(&mut self, spec: &Spec, floors: &Floors<'_>) -> Open {
        let mut open = Open::default();
        let held: BTreeSet<ReqId> =
            self.parts.iter().flat_map(|part| part.requirements.iter().copied()).collect();
        let placing =
            spec.requirements.iter().filter(|requirement| !held.contains(&requirement.id));

        // by trigger, then by stem
        for requirement in placing {
            let placed = self
                .parts
                .iter()
                .position(|part| {
                    part.holds(spec, |held| floors.shares_trigger(requirement.id, held.id))
                })
                .or_else(|| {
                    self.parts
                        .iter()
                        .position(|part| part.holds(spec, |held| held.stem() == requirement.stem()))
                });
            match placed {
                Some(position) => self.parts[position].requirements.push(requirement.id),
                None => {
                    open.ids.insert(requirement.id);
                }
            }
        }
        for part in &mut self.parts {
            part.requirements.sort_unstable();
        }
        open
    }

    // Owns the design's data `types` across the parts: a key the design
    // dropped leaves its part, and one no part owns goes to the one part
    // whose requirements differ from `base`'s, in the design's order, where
    // no requirement is open and exactly one part changed; otherwise the
    // unowned keys are `open`.
    fn own(&mut self, types: &Types, open: &mut Open) {
        for part in &mut self.parts {
            part.types.retain(|key| types.keys.contains(key));
        }
        let owned: BTreeSet<&str> =
            self.parts.iter().flat_map(|part| part.types.iter().map(String::as_str)).collect();
        let unowned: Vec<String> =
            types.keys.iter().filter(|key| !owned.contains(key.as_str())).cloned().collect();
        if unowned.is_empty() {
            return;
        }

        if open.ids.is_empty() {
            let mut changed: Vec<&mut Part> = self
                .parts
                .iter_mut()
                .filter(|part| {
                    self.base.part(part.id).is_none_or(|was| was.requirements != part.requirements)
                })
                .collect();
            if let [part] = changed.as_mut_slice() {
                part.types.extend(unowned);
                part.types = types.ordered(&part.types);
                return;
            }
        }
        open.keys = unowned;
    }
}

// What `Pruned::assemble` answers: the amended plan, or the turn that settles it.
pub enum Assembled<'a> {
    Settled(Plan),
    Open(PartAmend<'a>),
}

// Holds `parts` to the plan's rules over `floors` and the design's `types`;
// a break is the engine's, since nothing of the plan is the model's.
fn held(parts: &[Part], floors: &Floors<'_>, types: &Types) -> Result<(), Error> {
    let mut review = Review::default();
    floors.check(types, &drafts(parts), &mut review);
    review.verdict().map_err(|findings| {
        server_error!("the amended plan breaks the plan's rules:\n{}", findings.join("\n"))
    })
}

// `parts` less `leaving`: a part every requirement of which leaves is
// removed, and every part that depended on it depends on what it depended
// on instead, transitively, each list deduplicated and sorted. The kept
// parts are answered as adjusted, the removed as they were.
fn without<'a>(parts: &'a [Part], leaving: &BTreeSet<ReqId>) -> (Vec<Part>, Vec<&'a Part>) {
    let (removed, kept): (Vec<&Part>, Vec<&Part>) = parts.iter().partition(|part| {
        !part.requirements.is_empty() && part.requirements.iter().all(|id| leaving.contains(id))
    });
    let replaced: BTreeMap<PartId, &[PartId]> =
        removed.iter().map(|part| (part.id, part.depends_on.as_slice())).collect();

    let kept = kept
        .into_iter()
        .map(|part| Part {
            id: part.id,
            name: part.name.clone(),
            requirements: part
                .requirements
                .iter()
                .copied()
                .filter(|id| !leaving.contains(id))
                .collect(),
            types: part.types.clone(),
            uses: part.uses.clone(),
            depends_on: spliced(part, &replaced),
            brief: part.brief.clone(),
        })
        .collect();
    (kept, removed)
}

// The dependencies of `part` with each removed part replaced by its own,
// until none remain, deduplicated and sorted.
fn spliced(part: &Part, replaced: &BTreeMap<PartId, &[PartId]>) -> Vec<PartId> {
    let mut pending = part.depends_on.clone();
    let mut visited = BTreeSet::new();
    let mut dependencies = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        match replaced.get(&id) {
            Some(own) => pending.extend(own.iter().copied()),
            None => {
                dependencies.insert(id);
            }
        }
    }
    dependencies.remove(&part.id);
    dependencies.into_iter().collect()
}
