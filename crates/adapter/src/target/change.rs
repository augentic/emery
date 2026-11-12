//! Defines the change records a target adapter receives during a change build.
//!
//! [`Change`] describes the revision-wide change put to [`Task::Scope`](super::Task::Scope),
//! and [`SliceChange`] describes the slice-level change carried by [`Slice::change`](super::Slice::change).
//! [`Change::findings`] validates the part ids returned in
//! [`Outcome::Scoped`](super::Outcome::Scoped).

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use crate::target::{DesignType, Requirement};

/// What a requirement of a change slice is against the built revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Standing {
    /// A requirement the built revision does not hold.
    Added,
    /// A requirement of the built revision whose statement, scenarios, or
    /// trigger changed.
    Changed,
    /// A requirement of the built revision that stands as it was built.
    Unchanged,
}

/// A requirement the built revision's part held that this part does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Departure {
    /// The requirement as the built revision held it.
    pub requirement: Requirement,
    /// The part the requirement moved to (`PART-NNN`); `None` when retired.
    pub moved_to: Option<String>,
}

/// A design type whose signature changed between the built and current revisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeChange {
    /// The type's key, as the design names it.
    pub key: String,
    /// The built revision's signature; `None` when added.
    pub before: Option<String>,
    /// The current revision's signature; `None` when removed.
    pub after: Option<String>,
}

/// A design section that changed between the built and current revisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionChange {
    /// The section's key (`overview`, `domain-model`, `apis`,
    /// `technical-logic`, `ui-layout`, or `observability`).
    pub kind: String,
    /// The built revision's prose blocks, joined; `None` when added.
    pub before: Option<String>,
    /// The current revision's prose blocks, joined; `None` when removed.
    pub after: Option<String>,
}

/// What one slice of a change build changes against the built revision.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SliceChange {
    /// The revision whose verified build the tree holds.
    pub built: String,
    /// The commit the built revision's label stands at.
    pub commit: String,
    /// Each slice requirement's standing against the built revision, in the
    /// order [`Slice::requirements`](super::Slice::requirements) lists them.
    pub standing: Vec<Standing>,
    /// The built form of each [`Standing::Changed`] requirement of the slice,
    /// in id order.
    pub prior: Vec<Requirement>,
    /// The requirements the built revision's part held that this part no
    /// longer holds, in id order.
    pub departed: Vec<Departure>,
    /// The requirement ids of this part that earlier slices of this build
    /// already rebuilt, in id order.
    pub built_before: Vec<String>,
    /// The stems that requirements of the built revision led with and no
    /// requirement of the current revision leads with, sorted.
    pub vanished: Vec<String>,
    /// The design types this part owns or uses whose signatures changed, in
    /// key order.
    pub types: Vec<TypeChange>,
    /// The design sections that changed between the built revision and the
    /// current revision, in document order.
    pub sections: Vec<SectionChange>,
}

impl TypeChange {
    /// Returns the stem the type's [`key`](Self::key) leads with, when dotted.
    #[must_use]
    pub fn stem(&self) -> Option<&str> {
        type_stem(&self.key)
    }
}

impl SliceChange {
    /// Returns the built form of the [`Standing::Changed`] requirement `id`.
    #[must_use]
    pub fn prior(&self, id: &str) -> Option<&Requirement> {
        self.prior.iter().find(|requirement| requirement.id == id)
    }

    /// Returns the requirements the built revision's part held that this part does not.
    #[must_use]
    pub fn departed(&self) -> &[Departure] {
        &self.departed
    }

    /// Returns the [`Standing`] of requirement `id` in `requirements`.
    #[must_use]
    pub fn standing(&self, requirements: &[Requirement], id: &str) -> Option<Standing> {
        let index = requirements.iter().position(|requirement| requirement.id == id)?;
        self.standing.get(index).copied()
    }

    /// Returns whether `requirements` and this change describe a retiring slice.
    ///
    /// A retiring slice has no current requirements and lists every
    /// requirement of the removed part in [`departed`](Self::departed).
    #[must_use]
    pub const fn is_retiring(&self, requirements: &[Requirement]) -> bool {
        requirements.is_empty() && !self.departed.is_empty()
    }

    /// Returns the stems this change reaches across `requirements` and `owned` types, sorted.
    ///
    /// Includes the stems of [`Standing::Added`] and [`Standing::Changed`]
    /// requirements, [`departed`](Self::departed) requirements, and `owned`
    /// or removed types in [`types`](Self::types).
    #[must_use]
    pub fn reached_stems(&self, requirements: &[Requirement], owned: &[DesignType]) -> Vec<String> {
        let mut stems = BTreeSet::new();
        for (requirement, standing) in requirements.iter().zip(&self.standing) {
            if matches!(standing, Standing::Added | Standing::Changed) {
                stems.insert(requirement.stem().to_owned());
            }
        }
        for departure in &self.departed {
            stems.insert(departure.requirement.stem().to_owned());
        }
        let fallback = || requirements.first().map(Requirement::stem);
        for design_type in owned {
            if self.types.iter().any(|changed| changed.key == design_type.key)
                && let Some(stem) = design_type.stem().or_else(fallback)
            {
                stems.insert(stem.to_owned());
            }
        }
        for removed in self.types.iter().filter(|changed| changed.after.is_none()) {
            if let Some(stem) = removed.stem().or_else(fallback) {
                stems.insert(stem.to_owned());
            }
        }
        stems.into_iter().collect()
    }
}

pub fn type_stem(key: &str) -> Option<&str> {
    let dotted = key.split(" (").next().unwrap_or(key).trim();
    dotted.split_once('.').map(|(stem, _)| stem)
}

/// One part of the current plan and the typed reasons the change reaches it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartReach {
    /// The stable part identifier, `PART-001`.
    pub id: String,
    /// The part's drafted kebab-case name.
    pub name: String,
    /// The requirement ids the part places, `REQ-001`, in id order.
    pub requirements: Vec<String>,
    /// The typed reasons the engine's minimum scope reaches the part
    /// (`requirements`, `departed`, `types`, `plan`); empty when the minimum
    /// leaves the part carried.
    pub reasons: Vec<String>,
}

/// The change one build makes against the built revision, put to [`Task::Scope`](super::Task::Scope).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Change {
    /// The revision whose verified build the tree holds.
    pub built: String,
    /// The commit the built revision's label stands at.
    pub commit: String,
    /// Every part of the current plan, in id order, with its requirements and
    /// the typed reasons the engine's minimum scope reaches it.
    pub parts: Vec<PartReach>,
    /// The requirements retired across the revision, in id order.
    pub retired: Vec<Departure>,
    /// The design types whose signatures changed, in key order.
    pub types: Vec<TypeChange>,
    /// The design sections that changed, in document order.
    pub sections: Vec<SectionChange>,
}

impl Change {
    /// Returns every rule `scoped` breaks against this change, one line each.
    ///
    /// An empty result means `scoped` passes the gate: every id is a
    /// canonical `PART-NNN` identifier naming one of [`parts`](Self::parts)
    /// at most once.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_adapter::target::{Change, PartReach};
    ///
    /// let change = Change {
    ///     parts: vec![PartReach {
    ///         id: "PART-001".into(),
    ///         name: "orders".into(),
    ///         requirements: vec!["REQ-001".into()],
    ///         reasons: Vec::new(),
    ///     }],
    ///     ..Change::default()
    /// };
    ///
    /// assert!(change.findings(&["PART-001".into()]).is_empty());
    /// assert_eq!(
    ///     change
    ///         .findings(&["orders".into(), "PART-002".into(), "PART-001".into(), "PART-001".into()])
    ///         .len(),
    ///     3
    /// );
    /// ```
    #[must_use]
    pub fn findings(&self, scoped: &[String]) -> Vec<String> {
        let mut findings = Vec::new();
        let ids: Vec<&str> = self.parts.iter().map(|part| part.id.as_str()).collect();
        let mut seen = BTreeSet::new();
        for id in scoped {
            if !is_part_id(id) {
                findings.push(format!("- scoped `{id}` is not a part id (`PART-NNN`)"));
            } else if ids.is_empty() {
                findings.push(format!(
                    "- scoped `{id}` is not a part of the plan; the plan has no parts"
                ));
            } else if !ids.contains(&id.as_str()) {
                findings.push(format!(
                    "- scoped `{id}` is not a part of the plan; its parts are {}",
                    ids.join(", ")
                ));
            } else if !seen.insert(id.as_str()) {
                findings.push(format!("- scoped `{id}` is listed twice"));
            }
        }
        findings
    }
}

fn is_part_id(id: &str) -> bool {
    id.strip_prefix("PART-")
        .and_then(|digits| digits.parse::<NonZeroU32>().ok())
        .is_some_and(|number| format!("PART-{number:03}") == id)
}
