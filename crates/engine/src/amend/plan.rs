//! Derives the amended plan from the base's by code.
//!
//! An amendment keeps the base plan's slices under their ids and names and
//! changes their membership only as the change set says. [`prune`] takes
//! the retired requirements out of their slices, removes a slice left with
//! none, and splices each slice that depended on a removed one onto what it
//! depended on, so the order the removed slice held between its neighbours
//! survives it.

use std::collections::{BTreeMap, BTreeSet};

use crate::revision::{EMERY, Plan, ReqId, Slice, SliceId};

pub struct Pruned {
    // The base plan less the retired requirements and the slices they
    // emptied, its `parent` the base revision.
    pub plan: Plan,
    // The design type keys the removed slices owned.
    pub orphaned: BTreeSet<String>,
}

// Removes `retired` from the slices of `base`, under `parent` as the revision amended.
//
// A slice every requirement of which is retired is removed, and every slice
// that depended on it depends on what it depended on instead, transitively,
// each list deduplicated and sorted. The preamble, the slice ids, the names,
// the types a kept slice owns, and the briefs are the base's.
pub fn prune(base: &Plan, retired: &BTreeSet<ReqId>, parent: &str) -> Pruned {
    let (removed, kept): (Vec<&Slice>, Vec<&Slice>) = base.slices.iter().partition(|slice| {
        !slice.requirements.is_empty() && slice.requirements.iter().all(|id| retired.contains(id))
    });
    let replaced: BTreeMap<SliceId, &[SliceId]> =
        removed.iter().map(|slice| (slice.id, slice.depends_on.as_slice())).collect();

    let slices = kept
        .into_iter()
        .map(|slice| Slice {
            id: slice.id,
            name: slice.name.clone(),
            requirements: slice
                .requirements
                .iter()
                .copied()
                .filter(|id| !retired.contains(id))
                .collect(),
            types: slice.types.clone(),
            depends_on: spliced(slice, &replaced),
            brief: slice.brief.clone(),
        })
        .collect();
    Pruned {
        plan: Plan {
            emery: EMERY,
            parent: Some(parent.to_owned()),
            preamble: base.preamble.clone(),
            slices,
        },
        orphaned: removed.iter().flat_map(|slice| slice.types.iter().cloned()).collect(),
    }
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
