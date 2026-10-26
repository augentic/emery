//! Derives the amended plan from the base's by code.
//!
//! An amendment keeps the base plan's slices under their ids and names and
//! changes their membership only as the change set says. [`Pruned::new`]
//! takes the retired requirements out of their slices before the design
//! turn, so the types a slice they emptied owned leave the design with it,
//! and takes out the requirements the change moved under another stem;
//! [`Pruned::assemble`] places what the amendment adds and moves after the
//! turn, each into the slice of its stem or a new slice named for it, and
//! owns the design's data types as the amendment leaves them. No turn is
//! put: what code cannot decide is refused as the engine's, until the
//! slicing turn amends a plan.

use std::collections::{BTreeMap, BTreeSet};

use omnia_sdk::{Error, server_error};

use crate::brief::Review;
use crate::revision::{
    self, Change, ChangeClass, EMERY, Plan, ReqId, Requirement, Slice, SliceId, Spec,
};
use crate::specify::{Draft, Floors, SLICE_CAP};

// The base plan with the requirements that leave their slices taken out and
// any slice they emptied removed, ready to place what the amendment adds
// and moves once the specification and the design are settled.
pub struct Pruned<'a> {
    base: &'a Plan,
    parent: String,
    slices: Vec<Slice>,
    // The design type keys the slices emptied by retirement owned.
    pub orphaned: BTreeSet<String>,
}

impl<'a> Pruned<'a> {
    // Removes from `base` the requirements `changes` retires and the ones it
    // moves under another stem from `spec`, under `parent` as the revision
    // amended.
    //
    // A slice every requirement of which leaves is removed, and every slice
    // that depended on it depends on what it depended on instead,
    // transitively, each list deduplicated and sorted. The types a slice
    // emptied by retirement owned are `orphaned`, so they leave the design
    // before the design turn; the types of a slice emptied by a move stay in
    // the design and are owned again by `assemble`.
    pub fn new(base: &'a Plan, spec: &Spec, changes: &[Change], parent: &str) -> Self {
        let retired: BTreeSet<ReqId> = changes
            .iter()
            .filter(|change| change.class == ChangeClass::Retired)
            .map(|change| change.id)
            .collect();
        let (slices, removed) = without(&base.slices, &retired);
        let orphaned = removed.into_iter().flat_map(|slice| slice.types.iter().cloned()).collect();

        let moved: BTreeSet<ReqId> = changes
            .iter()
            .filter(|change| change.class == ChangeClass::Changed)
            .filter(|change| {
                spec.requirement(change.id).is_some_and(|requirement| {
                    requirement.stem() != revision::stem(&change.subject)
                })
            })
            .map(|change| change.id)
            .collect();
        let (slices, _) = without(&slices, &moved);

        Self {
            base,
            parent: parent.to_owned(),
            slices,
            orphaned,
        }
    }

    // Places into the pruned slices what the amendment adds and moves, and
    // owns the design's data `types`, under the rules of the plan.
    //
    // Each requirement of `spec` no slice holds, in id order, joins the
    // slice holding its stem, or a new slice named for the stem, numbered
    // after the highest id `base` holds and depending on nothing. A type key
    // the design no longer holds leaves its slice, and one no slice owns is
    // owned by the one slice whose requirements this amendment changed
    // against `base`. The assembled plan is held to the plan's rules.
    //
    // Returns `Error::ServerError` for what the slicing turn alone decides
    // and no amendment puts yet: a stem holding more than `SLICE_CAP`
    // requirements once one joins it, a type no slice owns where no slice or
    // more than one changed, and a plan the rules refuse.
    pub fn assemble(mut self, spec: &Spec, types: &[String]) -> Result<Plan, Error> {
        self.place(spec)?;
        self.own(types)?;
        held(&self.slices, spec, types)?;
        Ok(Plan {
            emery: EMERY,
            parent: Some(self.parent),
            preamble: self.base.preamble.clone(),
            slices: self.slices,
        })
    }

    // Places every requirement of `spec` no slice holds, in id order: each
    // joins the slice holding its stem, else a new slice named for the stem,
    // numbered after the highest id `base` holds. Every slice's requirements
    // are sorted.
    fn place(&mut self, spec: &Spec) -> Result<(), Error> {
        let held: BTreeSet<ReqId> =
            self.slices.iter().flat_map(|slice| slice.requirements.iter().copied()).collect();
        let placing: Vec<(ReqId, &str)> = spec
            .requirements
            .iter()
            .filter(|requirement| !held.contains(&requirement.id))
            .map(|requirement| (requirement.id, requirement.stem()))
            .collect();

        // a stem past the cap once a requirement joins it is cut along its
        // sub-stems by the turn
        let past: Vec<&str> = placing
            .iter()
            .map(|(_, stem)| *stem)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|stem| {
                spec.requirements.iter().filter(|requirement| requirement.stem() == *stem).count()
                    > SLICE_CAP
            })
            .collect();
        if !past.is_empty() {
            return Err(server_error!(
                "the amendment leaves {} with more than {SLICE_CAP} requirements; cutting a stem \
                 along its sub-stems is the slicing turn's, which no amendment puts yet",
                stems(&past)
            ));
        }

        let stem_of = |id: ReqId| spec.requirement(id).map(Requirement::stem);
        let mut next = self
            .base
            .slices
            .iter()
            .map(|slice| slice.id)
            .max()
            .map_or(Some(SliceId::FIRST), SliceId::next);
        for (id, stem) in placing {
            if let Some(slice) = self
                .slices
                .iter_mut()
                .find(|slice| slice.requirements.iter().any(|&held| stem_of(held) == Some(stem)))
            {
                slice.requirements.push(id);
                continue;
            }
            let slice = next.ok_or_else(|| server_error!("the slice ids are exhausted"))?;
            next = slice.next();
            self.slices.push(Slice {
                id: slice,
                name: stem.to_owned(),
                requirements: vec![id],
                types: Vec::new(),
                depends_on: Vec::new(),
                brief: Vec::new(),
            });
        }
        for slice in &mut self.slices {
            slice.requirements.sort_unstable();
        }
        Ok(())
    }

    // Owns the design's data `types` across the slices: a key the design
    // dropped leaves its slice, and one no slice owns goes to the one slice
    // whose requirements differ from `base`'s, in the design's order.
    fn own(&mut self, types: &[String]) -> Result<(), Error> {
        for slice in &mut self.slices {
            slice.types.retain(|key| types.contains(key));
        }
        let owned: BTreeSet<&str> =
            self.slices.iter().flat_map(|slice| slice.types.iter().map(String::as_str)).collect();
        let unowned: Vec<&str> =
            types.iter().map(String::as_str).filter(|key| !owned.contains(key)).collect();
        if unowned.is_empty() {
            return Ok(());
        }

        let mut changed: Vec<&mut Slice> = self
            .slices
            .iter_mut()
            .filter(|slice| {
                self.base.slice(slice.id).is_none_or(|was| was.requirements != slice.requirements)
            })
            .collect();
        let [slice] = changed.as_mut_slice() else {
            return Err(server_error!(
                "the design's {} owned by no slice while {} slices changed in this amendment; \
                 owning a type is the slicing turn's, which no amendment puts yet",
                keys(&unowned),
                changed.len()
            ));
        };
        slice.types = types
            .iter()
            .filter(|key| slice.types.contains(key) || unowned.contains(&key.as_str()))
            .cloned()
            .collect();
        Ok(())
    }
}

// Holds `slices` to the plan's rules over `spec` and the design's `types`;
// a break is the engine's, since nothing of the plan is the model's.
fn held(slices: &[Slice], spec: &Spec, types: &[String]) -> Result<(), Error> {
    let floors = Floors::new(spec.requirements.iter().map(|requirement| {
        (requirement.id, requirement.subject.as_str(), requirement.trigger.as_ref())
    }));
    let mut review = Review::default();
    floors.check(types, &drafts(slices), &mut review);
    review.verdict().map_err(|findings| {
        server_error!("the amended plan breaks the plan's rules:\n{}", findings.join("\n"))
    })
}

// `slices` less `leaving`: a slice every requirement of which leaves is
// removed, and every slice that depended on it depends on what it depended
// on instead, transitively, each list deduplicated and sorted. The kept
// slices are answered as adjusted, the removed as they were.
fn without<'a>(slices: &'a [Slice], leaving: &BTreeSet<ReqId>) -> (Vec<Slice>, Vec<&'a Slice>) {
    let (removed, kept): (Vec<&Slice>, Vec<&Slice>) = slices.iter().partition(|slice| {
        !slice.requirements.is_empty() && slice.requirements.iter().all(|id| leaving.contains(id))
    });
    let replaced: BTreeMap<SliceId, &[SliceId]> =
        removed.iter().map(|slice| (slice.id, slice.depends_on.as_slice())).collect();

    let kept = kept
        .into_iter()
        .map(|slice| Slice {
            id: slice.id,
            name: slice.name.clone(),
            requirements: slice
                .requirements
                .iter()
                .copied()
                .filter(|id| !leaving.contains(id))
                .collect(),
            types: slice.types.clone(),
            depends_on: spliced(slice, &replaced),
            brief: slice.brief.clone(),
        })
        .collect();
    (kept, removed)
}

// The dependencies of `slice` with each removed slice replaced by its own,
// until none remain, deduplicated and sorted.
fn spliced(slice: &Slice, replaced: &BTreeMap<SliceId, &[SliceId]>) -> Vec<SliceId> {
    let mut pending = slice.depends_on.clone();
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
    dependencies.remove(&slice.id);
    dependencies.into_iter().collect()
}

// `slices` as the gate reads them, each dependency by the name of the slice
// it names, or its id where no slice holds it, so the gate names the break.
fn drafts(slices: &[Slice]) -> Vec<Draft> {
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

// `the stem `a`` or `the stems `a`, `b``.
fn stems(names: &[&str]) -> String {
    let noun = if names.len() > 1 { "stems" } else { "stem" };
    format!("the {noun} {}", quoted(names))
}

// `type `a` is` or `types `a`, `b` are`.
fn keys(names: &[&str]) -> String {
    let (noun, verb) = if names.len() > 1 { ("types", "are") } else { ("type", "is") };
    format!("{noun} {} {verb}", quoted(names))
}

fn quoted(names: &[&str]) -> String {
    names.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
}
