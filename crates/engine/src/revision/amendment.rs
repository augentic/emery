//! Defines the typed amendment and its Markdown rendering.
//!
//! An [`Amendment`] is a change against the current revision, staged for
//! review: the base it was prepared against, one [`Change`] per
//! requirement, classed by what the change did to it, and the amended
//! specification. It is never sealed as it stands.

use std::fmt::{self, Display, Formatter};

use omnia_sdk::{Error, server_error};
use serde::{Deserialize, Serialize};
use strum::VariantArray;

use crate::revision::{self, Cited, HEADING, ID, NOTE, ReqId, SOURCES, Spec};

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
    /// under its id and each retired one left out. Its
    /// [`parent`](Spec::parent) is the base revision.
    pub spec: Spec,
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
    /// An amendment every entry of which is [`ChangeClass::Unchanged`] does
    /// not, and is nothing to stage.
    #[must_use]
    pub fn is_change(&self) -> bool {
        self.changes.iter().any(|change| change.class != ChangeClass::Unchanged)
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
            .map(|change| revision::stem(&change.subject))
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
        // entry followed by the scenarios the amended specification drafted
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
                    for scenario in &requirement.scenarios {
                        write!(f, "\n\n{scenario}")?;
                    }
                }
            }
        }
        f.write_str("\n")
    }
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
    /// does not, and for one it holds in conflict, which states none.
    pub before: Option<String>,
    /// The statement the change states; `None` for a retired requirement,
    /// and for a conflict, whose competing claims [`sources`](Self::sources)
    /// lists.
    pub after: Option<String>,
    /// The claims behind the entry, in descending authority order.
    pub sources: Vec<Cited>,
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

        // the statement the class carries
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
            (ChangeClass::Retired, Some(before), _) => {
                write!(f, "\n\n{before}")?;
            }
            (ChangeClass::Conflict, _, _) => {
                write!(f, "\n\n{NOTE} Operator reconciliation required.")?;
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
