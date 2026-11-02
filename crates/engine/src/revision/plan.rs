//! Defines the typed data and Markdown rendering for `plan.md`.
//!
//! A [`Plan`] contains introductory paragraphs and ordered [`Slice`] records.
//! Each slice is a subset of the specification that can be built on its own,
//! with the design types it owns and the slices built before it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::revision::{self, ID, ReqId, Requirement, SliceId, Spec};

pub const REQUIREMENTS: &str = "Requirements:";
pub const TYPES: &str = "Types:";
pub const DEPENDS_ON: &str = "Depends on:";

const HEADING: &str = "## Slice:";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub emery: u32,
    // The revision this one amends, or `None` when drafted from sources alone.
    pub parent: Option<String>,
    pub preamble: Vec<String>,
    pub slices: Vec<Slice>,
}

impl Plan {
    #[must_use]
    pub fn slice(&self, id: SliceId) -> Option<&Slice> {
        self.slices.iter().find(|slice| slice.id == id)
    }

    // The ids a new slice takes in an amendment of this plan, from the first
    // above every id the plan holds: an id a slice once held is never reused,
    // a removed one included.
    pub fn free_ids(&self) -> impl Iterator<Item = SliceId> {
        SliceId::free_after(self.slices.iter().map(|slice| slice.id))
    }

    // Returns the unmerged slices whose in-plan dependencies are all in
    // `merged`, in id order. An empty result with slices left unmerged
    // means the rest wait on one another.
    #[must_use]
    pub fn ready(&self, merged: &BTreeSet<SliceId>) -> Vec<&Slice> {
        self.slices
            .iter()
            .filter(|slice| !merged.contains(&slice.id))
            .filter(|slice| self.dependencies(slice).all(|id| merged.contains(&id)))
            .collect()
    }

    // The first wave holds every slice that depends on nothing; each wave
    // after it holds every slice whose dependencies are all in the waves
    // before, in id order. Were a stored plan to hold a cycle, its slices
    // are one last wave, in id order.
    #[must_use]
    pub fn waves(&self) -> Waves {
        let pending = self
            .slices
            .iter()
            .map(|slice| (slice.id, self.dependencies(slice).collect()))
            .collect();

        let (mut waves, cyclic) = ranks(pending);
        if !cyclic.is_empty() {
            waves.push(cyclic);
        }
        Waves(waves)
    }

    // The in-plan slices `slice` waits on: a dependency naming `slice`
    // itself or no slice of the plan holds nothing back.
    fn dependencies<'a>(&'a self, slice: &'a Slice) -> impl Iterator<Item = SliceId> + 'a {
        slice
            .depends_on
            .iter()
            .copied()
            .filter(move |&dependency| dependency != slice.id && self.slice(dependency).is_some())
    }
}

// `pending` maps each node to the nodes it waits on. Kahn's algorithm by
// rank: each rank is every node waiting on nothing once the ranks before it
// are placed, in its own order. Returns the ranks, then the nodes a cycle
// leaves unplaceable, in their own order.
pub fn ranks<T: Ord + Copy>(mut pending: BTreeMap<T, BTreeSet<T>>) -> (Vec<Vec<T>>, Vec<T>) {
    let mut ranks = Vec::new();
    loop {
        let ready: Vec<T> = pending
            .iter()
            .filter(|(_, dependencies)| dependencies.is_empty())
            .map(|(node, _)| *node)
            .collect();
        if ready.is_empty() {
            break;
        }
        for node in &ready {
            pending.remove(node);
        }
        for dependencies in pending.values_mut() {
            for node in &ready {
                dependencies.remove(node);
            }
        }
        ranks.push(ready);
    }
    (ranks, pending.into_keys().collect())
}

/// The slices of a plan grouped into the sets ready to build at once.
///
/// Each wave lists its slice ids in id order, and the waves come in build
/// order. The value serialises as a list of lists of ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Waves(Vec<Vec<SliceId>>);

impl Waves {
    /// Returns how many waves the plan builds in: its longest dependency chain.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether the plan has no slices.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns how many slices the waves hold between them.
    #[must_use]
    pub fn slices(&self) -> usize {
        self.0.iter().map(Vec::len).sum()
    }

    /// Returns the width of the widest wave: the most slices ready at once.
    #[must_use]
    pub fn widest(&self) -> usize {
        self.0.iter().map(Vec::len).max().unwrap_or(0)
    }

    /// Returns the waves in build order, each its slice ids in id order.
    pub fn iter(&self) -> impl Iterator<Item = &[SliceId]> {
        self.0.iter().map(Vec::as_slice)
    }
}

impl revision::Document for Plan {
    const NAME: &'static str = "plan";
}

impl Display for Plan {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        revision::write(f, "Plan", &self.preamble, &self.slices)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Slice {
    pub id: SliceId,
    // Unique within the plan.
    pub name: String,
    pub requirements: Vec<ReqId>,
    // In the design's order.
    pub types: Vec<String>,
    // Acyclic, in id order.
    pub depends_on: Vec<SliceId>,
    pub brief: Vec<String>,
}

impl Slice {
    // Whether `self` holds a requirement of `spec` that `matches`.
    #[must_use]
    pub fn holds(&self, spec: &Spec, matches: impl Fn(&Requirement) -> bool) -> bool {
        self.requirements.iter().any(|id| spec.requirement(*id).is_some_and(&matches))
    }

    // Writes the lists and the brief: everything after the `ID:` line, with
    // no trailing newline, so a projection under another heading renders
    // the slice as `plan.md` does.
    pub fn write_lists(&self, f: &mut Formatter<'_>) -> fmt::Result {
        revision::write_list(f, REQUIREMENTS, &self.requirements)?;

        // ownership and order, where there is any
        if !self.types.is_empty() {
            f.write_str("\n")?;
            revision::write_list(f, TYPES, &self.types)?;
        }
        if !self.depends_on.is_empty() {
            f.write_str("\n")?;
            revision::write_list(f, DEPENDS_ON, &self.depends_on)?;
        }

        // brief
        for paragraph in &self.brief {
            write!(f, "\n\n{paragraph}")?;
        }
        Ok(())
    }
}

impl Display for Slice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{HEADING} {}\n\n{ID} {}\n", self.name, self.id)?;
        self.write_lists(f)
    }
}
