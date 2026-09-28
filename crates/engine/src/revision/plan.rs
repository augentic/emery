//! Defines the typed data and Markdown rendering for `plan.md`.
//!
//! A [`Plan`] contains introductory paragraphs and ordered [`Slice`] records.
//! Each slice is a subset of the specification that can be built on its own,
//! with the design types it owns and the slices built before it.

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
