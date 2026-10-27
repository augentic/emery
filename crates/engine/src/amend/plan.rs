//! Derives the amended plan from the base's by code, and opens one slicing
//! turn for what code cannot decide.
//!
//! An amendment keeps the base plan's slices under their ids and names and
//! changes their membership only as the change set says. [`Pruned::new`]
//! takes the retired requirements out of their slices before the design
//! turn, so the types a slice they emptied owned leave the design with it,
//! and takes out the requirements the change moved under another stem;
//! [`Pruned::assemble`] places what the amendment adds and moves after the
//! turn where a rule decides, owns the design's data types as the amendment
//! leaves them, and answers the plan settled, or the slicing turn's brief
//! over the current slices for what no rule placed: a stem no slice holds,
//! a stem left past the cap, a type key no slice owns.

use std::collections::{BTreeMap, BTreeSet};

use omnia_sdk::{Error, server_error};

use crate::brief::Review;
use crate::revision::{Change, ChangeClass, EMERY, Plan, ReqId, Requirement, Slice, SliceId, Spec};
use crate::specify::{Basis, Floors, Open, SLICE_CAP, SliceAmend, drafts};

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
                spec.requirement(change.id)
                    .is_some_and(|requirement| requirement.stem() != change.stem())
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
    // Each requirement of `spec` no slice holds is placed by code where a
    // rule decides: into the slice holding another requirement its trigger
    // reaches, else the slice holding its floor, the stem, or the sub-stem
    // of a stem past `SLICE_CAP` whose additions all fall under held
    // sub-stems. A type key the design no longer holds leaves its slice,
    // and one no slice owns is owned by the one slice whose requirements
    // this amendment changed against `base`, where nothing else is open.
    // What no rule decides is open: the requirements of a stem no slice
    // holds, every requirement of a stem past the cap an addition reaches
    // under a sub-stem no slice holds, with the types of a slice that
    // empties, and a type key where no slice or more than one changed.
    //
    // Returns the plan `Settled` where nothing is open, held to the plan's
    // rules, and the slicing turn's brief `Open` over the current slices
    // otherwise, `drafted` the requirements the change drafts anew.
    //
    // Returns `Error::ServerError` for a settled plan the rules refuse, since
    // nothing of it is the model's.
    pub fn assemble(
        mut self, spec: &'a Spec, types: &'a [String], drafted: &'a [Basis<'a>],
    ) -> Result<Assembled<'a>, Error> {
        let mut open = self.place(spec);
        self.own(types, &mut open);
        if !open.is_empty() {
            return Ok(Assembled::Open(SliceAmend::new(
                self.base,
                self.parent,
                self.slices,
                open,
                spec,
                types,
                drafted,
            )));
        }
        held(&self.slices, spec, types)?;
        Ok(Assembled::Settled(Plan {
            emery: EMERY,
            parent: Some(self.parent),
            preamble: self.base.preamble.clone(),
            slices: self.slices,
        }))
    }

    // Places every requirement of `spec` no slice holds, in id order, and
    // returns what no rule placed. Every slice's requirements are sorted.
    fn place(&mut self, spec: &Spec) -> Open {
        let mut open = Open::default();
        let held: BTreeSet<ReqId> =
            self.slices.iter().flat_map(|slice| slice.requirements.iter().copied()).collect();
        let placing: Vec<&Requirement> = spec
            .requirements
            .iter()
            .filter(|requirement| !held.contains(&requirement.id))
            .collect();

        // a stem past the cap is cut again where an addition falls under a
        // sub-stem no slice holds: every requirement of it is open, and a
        // slice it empties gives up its types
        let past: BTreeSet<&str> = placing
            .iter()
            .map(|requirement| requirement.stem())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|&stem| {
                spec.requirements.iter().filter(|held| held.stem() == stem).count() > SLICE_CAP
            })
            .collect();
        for &stem in &past {
            let reopened = placing.iter().filter(|requirement| requirement.stem() == stem).any(
                |requirement| {
                    !self.slices.iter().any(|slice| {
                        slice.holds(spec, |held| held.substem() == requirement.substem())
                    })
                },
            );
            if !reopened {
                continue;
            }
            let ids: BTreeSet<ReqId> = spec
                .requirements
                .iter()
                .filter(|requirement| requirement.stem() == stem)
                .map(|requirement| requirement.id)
                .collect();
            for slice in &mut self.slices {
                slice.requirements.retain(|id| !ids.contains(id));
                if slice.requirements.is_empty() {
                    slice.types.clear();
                }
            }
            open.ids.extend(ids);
            open.recut.insert(stem.to_owned());
        }

        // the rest by trigger, then by floor
        for requirement in placing {
            if open.ids.contains(&requirement.id) {
                continue;
            }
            let trigger = requirement.trigger.as_ref();
            let floor = |held: &Requirement| {
                if past.contains(&requirement.stem()) {
                    held.substem() == requirement.substem()
                } else {
                    held.stem() == requirement.stem()
                }
            };
            let placed = self
                .slices
                .iter()
                .position(|slice| {
                    slice.holds(spec, |held| trigger.is_some() && held.trigger.as_ref() == trigger)
                })
                .or_else(|| self.slices.iter().position(|slice| slice.holds(spec, floor)));
            match placed {
                Some(position) => self.slices[position].requirements.push(requirement.id),
                None => {
                    open.ids.insert(requirement.id);
                }
            }
        }
        for slice in &mut self.slices {
            slice.requirements.sort_unstable();
        }
        open
    }

    // Owns the design's data `types` across the slices: a key the design
    // dropped leaves its slice, and one no slice owns goes to the one slice
    // whose requirements differ from `base`'s, in the design's order, where
    // no requirement is open and exactly one slice changed; otherwise the
    // unowned keys are `open`.
    fn own(&mut self, types: &[String], open: &mut Open) {
        for slice in &mut self.slices {
            slice.types.retain(|key| types.contains(key));
        }
        let owned: BTreeSet<&str> =
            self.slices.iter().flat_map(|slice| slice.types.iter().map(String::as_str)).collect();
        let unowned: Vec<String> =
            types.iter().filter(|key| !owned.contains(key.as_str())).cloned().collect();
        if unowned.is_empty() {
            return;
        }

        if open.ids.is_empty() {
            let mut changed: Vec<&mut Slice> = self
                .slices
                .iter_mut()
                .filter(|slice| {
                    self.base
                        .slice(slice.id)
                        .is_none_or(|was| was.requirements != slice.requirements)
                })
                .collect();
            if let [slice] = changed.as_mut_slice() {
                slice.types.extend(unowned);
                slice.types =
                    types.iter().filter(|key| slice.types.contains(key)).cloned().collect();
                return;
            }
        }
        open.keys = unowned;
    }
}

// What `Pruned::assemble` answers: the amended plan, or the turn that settles it.
pub enum Assembled<'a> {
    Settled(Plan),
    Open(SliceAmend<'a>),
}

// Holds `slices` to the plan's rules over `spec` and the design's `types`;
// a break is the engine's, since nothing of the plan is the model's.
fn held(slices: &[Slice], spec: &Spec, types: &[String]) -> Result<(), Error> {
    let mut review = Review::default();
    Floors::from_spec(spec).check(types, &drafts(slices), &mut review);
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
