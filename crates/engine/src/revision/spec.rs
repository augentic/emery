//! Defines the typed data and Markdown rendering for `spec.md`.
//!
//! A [`Spec`] contains introductory paragraphs and ordered [`Requirement`]
//! records. Each requirement combines reconciled source facts with drafted
//! acceptance scenarios.

use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{Anchor, SourceKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::revision::{self, ReqId};

/// The `ID:` provenance key written below a requirement or slice heading.
pub const ID: &str = "ID:";
/// The `Sources:` provenance key.
pub const SOURCES: &str = "Sources:";
/// The `Status:` provenance key.
pub const STATUS: &str = "Status:";
/// The `Note:` key used for generated requirement notes.
pub const NOTE: &str = "Note:";

const HEADING: &str = "### Requirement:";
const SCENARIO: &str = "#### Scenario:";

/// A behavioural specification in its stored form.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// The grammar the document was written under.
    pub emery: u32,
    /// Markdown paragraphs preceding the first requirement.
    pub preamble: Vec<String>,
    /// The requirements, in id order.
    pub requirements: Vec<Requirement>,
}

impl Spec {
    /// Returns the requirement `id` names, if the specification has one.
    #[must_use]
    pub fn requirement(&self, id: ReqId) -> Option<&Requirement> {
        self.requirements.iter().find(|requirement| requirement.id == id)
    }
}

impl revision::Document for Spec {
    const NAME: &'static str = "spec";
}

impl Display for Spec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        revision::write(f, "Specification", &self.preamble, &self.requirements)
    }
}

/// A reconciled requirement and its drafted acceptance scenarios.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// The stable requirement identifier.
    pub id: ReqId,
    /// The heading derived from the highest-authority contributing claim.
    pub subject: String,
    /// The reconciliation outcome for contributing claims.
    pub status: Status,
    /// Whether an acceptance criterion covers the requirement.
    pub covered: bool,
    /// Contributing claims in descending authority order.
    pub sources: Vec<Cited>,
    /// Markdown body paragraphs, empty when the requirement is in conflict.
    pub body: Vec<String>,
    /// Contributor classes rendered as notes.
    ///
    /// This contains losing classes for a divergence and every class for a
    /// conflict.
    pub losers: Vec<Loser>,
    /// The acceptance scenarios.
    pub scenarios: Vec<Scenario>,
}

impl Requirement {
    /// Returns the stem of the subject: the first segment of its dotted id,
    /// the slice floor every requirement under it shares.
    #[must_use]
    pub fn stem(&self) -> &str {
        revision::stem(&self.subject)
    }

    /// Returns how many anchors this requirement and `other` cite in common:
    /// the pairs of their citations that [overlap](Cited::overlaps).
    #[must_use]
    pub fn shared_anchors(&self, other: &Self) -> usize {
        self.sources
            .iter()
            .flat_map(|ours| other.sources.iter().filter(move |theirs| ours.overlaps(theirs)))
            .count()
    }
}

impl Display for Requirement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{HEADING} {}", self.subject)?;
        if self.status != Status::Agreed {
            write!(f, " [{}]", self.status)?;
        }

        write!(f, "\n\n{ID} {}\n{SOURCES} [", self.id)?;
        for (position, source) in self.sources.iter().enumerate() {
            if position > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{source}")?;
        }
        write!(f, "]\n{STATUS} {}", self.status)?;

        // body
        for paragraph in &self.body {
            write!(f, "\n\n{paragraph}")?;
        }

        // notes
        let mut separator = "\n\n";
        for loser in &self.losers {
            write!(f, "{separator}{loser}")?;
            separator = "\n";
        }
        if self.status == Status::Conflict {
            write!(f, "{separator}{NOTE} Operator reconciliation required.")?;
            separator = "\n";
        }
        if !self.covered {
            write!(f, "{separator}{NOTE} acceptance criteria not evidenced.")?;
        }

        // scenarios
        for scenario in &self.scenarios {
            write!(f, "\n\n{scenario}")?;
        }
        Ok(())
    }
}

/// A claim cited by a requirement.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cited {
    /// The name of the source the claim was extracted from.
    pub source: String,
    /// The claim id within that source.
    pub claim: String,
    /// Where in the source the claim anchors, in the claim `path` grammar
    /// (`src/orders.ts#L12-L34`), when the claim carries one.
    pub path: Option<String>,
}

impl Cited {
    /// Returns whether this citation and `other` anchor at one place: the
    /// same source and the same file, with line ranges that meet, or neither
    /// naming lines. A citation without a `path` anchors nowhere.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        if self.source != other.source {
            return false;
        }
        let (Some(ours), Some(theirs)) = (self.path.as_deref(), other.path.as_deref()) else {
            return false;
        };
        let (Ok(ours), Ok(theirs)) = (Anchor::parse(ours), Anchor::parse(theirs)) else {
            return false;
        };
        ours.path == theirs.path
            && match (ours.lines, theirs.lines) {
                (Some((start, end)), Some((other_start, other_end))) => {
                    start <= other_end && other_start <= end
                }
                (None, None) => true,
                _ => false,
            }
    }
}

impl Display for Cited {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.claim)
    }
}

/// A contributor class rendered as a requirement note.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loser {
    /// Every member's source name, in authority order.
    pub sources: Vec<String>,
    /// The authority class of the leading contributor.
    pub kind: SourceKind,
    /// The leading contributor's claim identifier.
    pub claim: String,
    /// The leading contributor's normalised statement.
    pub statement: String,
}

impl Display for Loser {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{NOTE} {sources} ({kind}, {claim}): {statement}",
            sources = self.sources.join(", "),
            kind = self.kind,
            claim = self.claim,
            statement = self.statement,
        )
    }
}

/// An acceptance scenario attached to a requirement.
///
/// `given` and `and` are lines; a draft that writes one line as a bare string
/// where the schema asks for a sequence is read as the one-line sequence, so
/// the slip costs no correction round. The stored form is always the sequence.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The scenario heading name.
    pub name: String,
    /// Optional `GIVEN` conditions, one line each.
    #[serde(default, deserialize_with = "lines")]
    pub given: Vec<String>,
    /// The `WHEN` trigger.
    pub when: String,
    /// The primary `THEN` outcome.
    pub then: String,
    /// Additional `AND` outcomes, one line each.
    #[serde(default, deserialize_with = "lines")]
    pub and: Vec<String>,
}

// A sequence of lines, or one line as a bare string.
fn lines<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Lines {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Lines::deserialize(deserializer)? {
        Lines::One(line) => {
            tracing::debug!("a scenario's lines arrived as one bare string; read as one line");
            vec![line]
        }
        Lines::Many(lines) => lines,
    })
}

impl Scenario {
    /// Returns the scenario's lines in document order, each with its field name.
    ///
    /// Every `given`, then the `when`, the `then`, and every `and`.
    pub fn lines(&self) -> impl Iterator<Item = (&'static str, &str)> {
        self.given
            .iter()
            .map(|text| ("given", text.as_str()))
            .chain([("when", self.when.as_str()), ("then", self.then.as_str())])
            .chain(self.and.iter().map(|text| ("and", text.as_str())))
    }

    /// Returns the scenario with the whitespace around each line dropped.
    ///
    /// A revision stores lines in this form, so two drafts that differ only
    /// in padding commit as one revision.
    #[must_use]
    pub fn trimmed(self) -> Self {
        let line = |text: String| text.trim().to_owned();
        let lines = |texts: Vec<String>| texts.into_iter().map(line).collect();
        Self {
            name: line(self.name),
            given: lines(self.given),
            when: line(self.when),
            then: line(self.then),
            and: lines(self.and),
        }
    }
}

impl Display for Scenario {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{SCENARIO} {}", self.name)?;
        for (field, text) in self.lines() {
            write!(f, "\n- **{}** {text}", field.to_ascii_uppercase())?;
        }
        Ok(())
    }
}

/// The reconciliation status of a requirement.
///
/// Every status except [`Status::Agreed`] is also rendered as a tag on the
/// requirement heading.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Status {
    /// All contributors agree and acceptance criteria provide coverage.
    Agreed,
    /// All contributors agree but no acceptance criterion provides coverage.
    Unknown,
    /// Highest-authority contributors disagree and require reconciliation.
    Conflict,
    /// A higher-authority contributor resolves the disagreement.
    Divergence,
}
