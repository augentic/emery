//! Defines the typed data and Markdown rendering for `plan.md`.
//!
//! A [`Plan`] contains introductory paragraphs and ordered [`Slice`] records.
//! Each slice is a subset of the specification that can be built on its own,
//! with the design types it owns and the slices built before it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::revision::{self, ID, ReqId, SliceId};

/// The `Requirements:` key listing a slice's requirement ids.
pub const REQUIREMENTS: &str = "Requirements:";
/// The `Types:` key listing the design type keys a slice owns.
pub const TYPES: &str = "Types:";
/// The `Depends on:` key listing the slices built before a slice.
pub const DEPENDS_ON: &str = "Depends on:";

const HEADING: &str = "## Slice:";

/// A build plan in its stored form.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// The grammar the document was written under.
    pub emery: u32,
    /// Markdown paragraphs preceding the first slice.
    pub preamble: Vec<String>,
    /// The slices, in id order.
    pub slices: Vec<Slice>,
}

impl Plan {
    /// Returns the slice `id` names, if the plan has one.
    #[must_use]
    pub fn slice(&self, id: SliceId) -> Option<&Slice> {
        self.slices.iter().find(|slice| slice.id == id)
    }

    /// Returns the slices in build order: the [`waves`](Plan::waves) flattened.
    ///
    /// Every slice comes after each slice it depends on; the slices of one
    /// wave come in id order.
    #[must_use]
    pub fn order(&self) -> Vec<&Slice> {
        self.waves().iter().flatten().filter_map(|id| self.slice(*id)).collect()
    }

    /// Returns the slices ready to build once `merged` are built: the unmerged slices whose dependencies are all merged, in id order.
    ///
    /// A dependency naming the slice itself or no slice of the plan holds
    /// nothing back, as [`waves`](Plan::waves) ignores it. An empty result
    /// with slices left unmerged means the rest wait on one another.
    #[must_use]
    pub fn ready(&self, merged: &BTreeSet<SliceId>) -> Vec<&Slice> {
        self.slices
            .iter()
            .filter(|slice| !merged.contains(&slice.id))
            .filter(|slice| {
                slice.depends_on.iter().all(|dependency| {
                    merged.contains(dependency)
                        || *dependency == slice.id
                        || self.slice(*dependency).is_none()
                })
            })
            .collect()
    }

    /// Returns the slices grouped into waves: the sets ready to build at once.
    ///
    /// The first wave holds every slice that depends on nothing; each wave
    /// after it holds every slice whose dependencies are all in the waves
    /// before, in id order. The engine writes no cycle; were a stored plan
    /// to hold one, its slices are one last wave, in id order.
    #[must_use]
    pub fn waves(&self) -> Waves {
        let pending = self
            .slices
            .iter()
            .map(|slice| {
                let dependencies = slice
                    .depends_on
                    .iter()
                    .copied()
                    .filter(|dependency| {
                        *dependency != slice.id && self.slice(*dependency).is_some()
                    })
                    .collect();
                (slice.id, dependencies)
            })
            .collect();

        let (mut waves, cyclic) = ranks(pending);
        if !cyclic.is_empty() {
            waves.push(cyclic);
        }
        Waves(waves)
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

/// A buildable subset of the specification.
///
/// Every requirement of the specification is in exactly one slice, and
/// every design type key is owned by exactly one.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Slice {
    /// The stable slice identifier.
    pub id: SliceId,
    /// The drafted kebab-case name, unique within the plan.
    pub name: String,
    /// The requirements this slice builds, in id order.
    pub requirements: Vec<ReqId>,
    /// The design type keys this slice owns, in the design's order.
    pub types: Vec<String>,
    /// The slices this one is built after, in id order; acyclic.
    pub depends_on: Vec<SliceId>,
    /// Drafted paragraphs a builder reads before taking the slice up.
    pub brief: Vec<String>,
}

impl Display for Slice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{HEADING} {}\n\n{ID} {}\n", self.name, self.id)?;
        list(f, REQUIREMENTS, &self.requirements)?;

        // ownership and order, where there is any
        if !self.types.is_empty() {
            f.write_str("\n")?;
            list(f, TYPES, &self.types)?;
        }
        if !self.depends_on.is_empty() {
            f.write_str("\n")?;
            list(f, DEPENDS_ON, &self.depends_on)?;
        }

        // brief
        for paragraph in &self.brief {
            write!(f, "\n\n{paragraph}")?;
        }
        Ok(())
    }
}

// A `Key: [a, b]` line with no trailing newline.
fn list<T: Display>(f: &mut Formatter<'_>, key: &str, items: &[T]) -> fmt::Result {
    write!(f, "{key} [")?;
    for (position, item) in items.iter().enumerate() {
        if position > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{item}")?;
    }
    f.write_str("]")
}
