//! Defines the typed amendment and its Markdown rendering.
//!
//! An [`Amendment`] is a change against the current revision, staged for
//! review: the base it was prepared against, one [`Change`] per
//! requirement, classed by what the change did to it, the amended
//! specification, design, and plan, and what the last two change against
//! the base's. A seal writes those three documents as a revision over the
//! base, never the amendment itself.

use std::fmt::{self, Display, Formatter};

use omnia_sdk::{Error, server_error};
use serde::{Deserialize, Serialize};
use strum::VariantArray;

use crate::revision::{
    self, Cited, Design, DesignDiff, HEADING, ID, NOTE, Plan, PlanDiff, ReqId, SOURCES, SliceId,
    Spec,
};

const SLICE: &str = "### Slice:";

/// A change against the current revision in its staged form.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Amendment {
    /// The grammar the document was written under.
    pub emery: u32,
    /// The revision the amendment was prepared against.
    pub base: AmendmentBase,
    /// One entry per requirement of the amended specification, and per
    /// requirement retired from it, in id order.
    pub changes: Vec<Change>,
    /// The amended specification.
    ///
    /// Holds the base's preamble and unchanged requirements as they stand,
    /// with each added, changed, or conflicting requirement drafted whole
    /// under its id and each retired one left out. Its `parent` is the base
    /// revision.
    pub spec: Spec,
    /// The amended design.
    ///
    /// The base's, less the type blocks owned by the slices the change
    /// emptied, a section left with no block going with them. Its `parent`
    /// is the base revision.
    pub design: Design,
    /// The amended plan.
    ///
    /// The base's slices under their ids and names, each retired
    /// requirement taken out, each requirement restated under another stem
    /// moved to the slice of its new stem, each added one in the slice of
    /// its stem, a stem no slice held in a new slice named for it, a slice
    /// left with none removed and every slice that depended on it depending
    /// on what it depended on instead, and the design's data types owned as
    /// the change leaves them. Its `parent` is the base revision.
    pub plan: Plan,
    /// What the amended design changes against the base's, by section.
    pub sections: DesignDiff,
    /// What the amended plan changes against the base's: one entry per
    /// slice added, changed, or removed, in id order.
    pub slices: Vec<SliceChange>,
}

impl Amendment {
    /// Reads and validates the amendment stored under `id`.
    ///
    /// The content digest is checked before the document is deserialised.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] with code `spec-outdated` when the
    ///   document uses a different grammar.
    /// - Returns [`Error::ServerError`] when the content does not match
    ///   `id`, is not valid JSON, or does not match the document shape.
    pub fn read(id: &str, body: &[u8]) -> Result<Self, Error> {
        if Self::identify(body) != id {
            return Err(server_error!("amendment `{id}` does not match its content"));
        }
        revision::Document::from_json(body)
    }

    /// The content identifier of the document as stored.
    #[must_use]
    pub fn identify(body: &[u8]) -> String {
        revision::digest(&[body])
    }

    /// Returns how many changes are of `class`.
    #[must_use]
    pub fn count(&self, class: ChangeClass) -> usize {
        self.changes.iter().filter(|change| change.class == class).count()
    }

    /// Returns whether the amendment changes anything of the base.
    ///
    /// An amendment every entry of which is [`ChangeClass::Unchanged`] that
    /// changes no design section and no slice does not, and is nothing to
    /// stage.
    #[must_use]
    pub fn is_change(&self) -> bool {
        self.changes.iter().any(|change| change.class != ChangeClass::Unchanged)
            || !self.sections.is_empty()
            || !self.slices.is_empty()
    }

    /// Returns the stems of the requirements the amendment touches, once each, sorted.
    ///
    /// The stem is the first segment of a requirement's dotted subject, the
    /// floor a slice is cut at, so these are the slices the change reaches.
    /// An unchanged requirement counts for nothing.
    #[must_use]
    pub fn stems(&self) -> Vec<&str> {
        let mut stems: Vec<&str> = self
            .changes
            .iter()
            .filter(|change| change.class != ChangeClass::Unchanged)
            .map(Change::stem)
            .collect();
        stems.sort_unstable();
        stems.dedup();
        stems
    }

    /// Returns the subjects of the requirements the change sources disagree over, in id order.
    pub fn conflicts(&self) -> impl Iterator<Item = &str> {
        self.changes
            .iter()
            .filter(|change| change.class == ChangeClass::Conflict)
            .map(|change| change.subject.as_str())
    }
}

impl revision::Document for Amendment {
    const NAME: &'static str = "amendment";
}

impl Display for Amendment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "# Amendment\n\nBase: {} at {}\n\n", self.base.revision, self.base.commit)?;
        for (position, class) in ChangeClass::VARIANTS.iter().enumerate() {
            if position > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{} {class}", self.count(*class))?;
        }
        f.write_str(".")?;

        // one section per class with entries, each in id order, a drafted
        // entry followed by the scenarios the amended specification drafted,
        // a conflict's competing statements noted first as the
        // specification notes them
        for class in ChangeClass::VARIANTS {
            let mut changes =
                self.changes.iter().filter(|change| change.class == *class).peekable();
            if changes.peek().is_none() {
                continue;
            }
            write!(f, "\n\n## {}", class.heading())?;
            for change in changes {
                write!(f, "\n\n{change}")?;
                if !class.is_drafted() {
                    continue;
                }
                if let Some(requirement) = self.spec.requirement(change.id) {
                    if *class == ChangeClass::Conflict {
                        requirement.write_conflict_notes(f)?;
                    }
                    for scenario in &requirement.scenarios {
                        write!(f, "\n\n{scenario}")?;
                    }
                }
            }
        }

        // the design sections changed, the new text of each added or changed
        // one whole, so the reviewer reads what the amendment holds
        if !self.sections.is_empty() {
            write!(f, "\n\n## Design\n\n{}", sections_line(&self.sections))?;
            for section in &self.design.sections {
                let kind = section.kind;
                if !self.sections.added.contains(&kind) && !self.sections.changed.contains(&kind) {
                    continue;
                }
                write!(f, "\n\n### {kind}")?;
                for block in &section.blocks {
                    write!(f, "\n\n{block}")?;
                }
            }
        }

        // the slices changed, an added or changed one as `plan.md` renders it
        if !self.slices.is_empty() {
            f.write_str("\n\n## Plan")?;
            for change in &self.slices {
                write!(f, "\n\n{SLICE} {} [{}]\n\n{ID} {}", change.name, change.class, change.id)?;
                if change.class != SliceClass::Removed
                    && let Some(slice) = self.plan.slice(change.id)
                {
                    f.write_str("\n")?;
                    slice.write_lists(f)?;
                }
            }
        }
        f.write_str("\n")
    }
}

// `Changed: the preamble, Domain model; added: APIs and integrations;
// removed: Observability.`: each part present when it names anything, the
// sections by heading.
fn sections_line(diff: &DesignDiff) -> String {
    let headings = |kinds: &[revision::SectionKind]| -> Vec<String> {
        kinds.iter().map(ToString::to_string).collect()
    };
    let changed: Vec<String> = diff
        .preamble
        .then(|| "the preamble".to_owned())
        .into_iter()
        .chain(diff.changed.iter().map(ToString::to_string))
        .collect();
    let clauses = [
        ("changed", changed),
        ("added", headings(&diff.added)),
        ("removed", headings(&diff.removed)),
    ];
    let mut line = clauses
        .into_iter()
        .filter(|(_, names)| !names.is_empty())
        .map(|(verb, names)| format!("{verb}: {}", names.join(", ")))
        .collect::<Vec<_>>()
        .join("; ");
    if let Some(first) = line.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    line.push('.');
    line
}

/// The revision an [`Amendment`] was prepared against, at the commit it was read at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmendmentBase {
    /// The base revision's id.
    pub revision: String,
    /// The commit the base revision was read at.
    pub commit: String,
}

/// What an [`Amendment`] does to one requirement.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    /// The requirement's identifier: the base's for one it holds, the next
    /// free one for an added requirement.
    pub id: ReqId,
    /// The requirement's subject.
    pub subject: String,
    /// What the change did to the requirement.
    pub class: ChangeClass,
    /// The statement the base revision holds; `None` for a requirement it
    /// does not. For one it holds in conflict, the competing statements
    /// joined as `[conflict] <a> | <b>`, which a change matched to it
    /// resolves.
    pub before: Option<String>,
    /// The statement the change states; `None` for a retired requirement,
    /// and for a conflict among the change sources, whose competing claims
    /// [`sources`](Self::sources) lists. For an unchanged requirement, the
    /// base's as [`before`](Self::before) holds it.
    pub after: Option<String>,
    /// The claims behind the entry, in descending authority order.
    pub sources: Vec<Cited>,
    /// The behaviour source read whole whose silence retired the
    /// requirement; `None` for every entry a claim decided.
    ///
    /// Set on a [`ChangeClass::Retired`] entry alone, whose
    /// [`sources`](Self::sources) are then empty: the source covers the
    /// requirement's stem with its requirement claims and states nothing of
    /// the requirement.
    pub scope: Option<String>,
}

impl Change {
    /// Returns the first segment of the dotted [`subject`](Self::subject).
    #[must_use]
    pub fn stem(&self) -> &str {
        revision::stem(&self.subject)
    }

    /// Returns whether the entry was retired by a source's scope rather than by a claim.
    #[must_use]
    pub fn by_scope(&self) -> bool {
        self.class == ChangeClass::Retired && self.scope.is_some()
    }
}

impl Display for Change {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{HEADING} {}", self.subject)?;
        if self.class == ChangeClass::Conflict {
            write!(f, " [{}]", self.class)?;
        }
        write!(f, "\n\n{ID} {}\n{SOURCES} [", self.id)?;
        for (position, source) in self.sources.iter().enumerate() {
            if position > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{source}")?;
        }
        f.write_str("]")?;

        // the statement the class carries; a conflict carries none, its
        // competing statements the amended specification's to note
        match (self.class, &self.before, &self.after) {
            (ChangeClass::Added | ChangeClass::Unchanged, _, Some(after)) => {
                write!(f, "\n\n{after}")?;
            }
            (ChangeClass::Changed, before, Some(after)) => {
                if let Some(before) = before {
                    write!(f, "\n\nBefore: {before}")?;
                }
                write!(f, "\n\nAfter: {after}")?;
            }
            (ChangeClass::Retired, before, _) => {
                if let Some(before) = before {
                    write!(f, "\n\n{before}")?;
                }
                if let Some(source) = &self.scope {
                    write!(
                        f,
                        "\n\n{NOTE} retired by scope: `{source}` was read whole and states \
                         nothing under `{}`.",
                        self.stem()
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// What an [`Amendment`] did to a requirement.
///
/// Variants are declared in the order the projection renders the classes.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, VariantArray, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ChangeClass {
    /// A requirement the base does not hold, stated by the change.
    Added,
    /// A requirement of the base restated by the change, its id kept.
    Changed,
    /// A requirement of the base the change withdraws.
    Retired,
    /// A requirement the change sources state and disagree over.
    Conflict,
    /// A requirement of the base the change leaves as it stands.
    Unchanged,
}

impl ChangeClass {
    /// Returns whether the amended specification drafts a requirement of this class anew.
    ///
    /// An added, changed, or conflicting requirement is drafted whole under
    /// its id. An unchanged one is the base's as it stands, and a retired one
    /// is left out.
    #[must_use]
    pub const fn is_drafted(self) -> bool {
        match self {
            Self::Added | Self::Changed | Self::Conflict => true,
            Self::Retired | Self::Unchanged => false,
        }
    }

    const fn heading(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Changed => "Changed",
            Self::Retired => "Retired",
            Self::Conflict => "Conflict",
            Self::Unchanged => "Unchanged",
        }
    }
}

/// What an [`Amendment`] does to one slice of the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliceChange {
    /// The slice's identifier.
    pub id: SliceId,
    /// The slice's name.
    pub name: String,
    /// What the change did to the slice.
    pub class: SliceClass,
}

impl SliceChange {
    /// Returns one entry per slice `diff` lists as added, changed, or removed, in id order.
    #[must_use]
    pub fn of(diff: &PlanDiff) -> Vec<Self> {
        let entry = |entry: &revision::SliceEntry, class| Self {
            id: entry.id,
            name: entry.name.clone(),
            class,
        };
        let mut changes: Vec<Self> = diff
            .added
            .iter()
            .map(|added| entry(added, SliceClass::Added))
            .chain(diff.changed.iter().map(|changed| entry(&changed.entry, SliceClass::Changed)))
            .chain(diff.removed.iter().map(|removed| entry(removed, SliceClass::Removed)))
            .collect();
        changes.sort_by_key(|change| change.id);
        changes
    }
}

/// What an [`Amendment`] did to a slice of the plan.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::Display, strum::EnumString,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum SliceClass {
    /// A slice the base plan does not hold.
    Added,
    /// A slice of the base plan whose requirements, types, dependencies, or
    /// brief the amendment changes.
    Changed,
    /// A slice of the base plan the amendment leaves no requirement in.
    Removed,
}
