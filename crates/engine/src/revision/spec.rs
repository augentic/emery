//! Defines the typed data and Markdown rendering for `spec.md`.
//!
//! A [`Spec`] contains introductory paragraphs and ordered [`Requirement`]
//! records. Each requirement combines reconciled source facts with drafted
//! acceptance scenarios.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use emery_adapter::source::SourceKind;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::revision;

/// The `ID:` provenance key written below a requirement heading.
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
    /// The source key.
    pub source: String,
    /// The claim id within that source.
    pub claim: String,
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
    /// Every member's source key, in authority order.
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
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The scenario heading name.
    pub name: String,
    /// Optional `GIVEN` conditions, one line each.
    #[serde(default)]
    pub given: Vec<String>,
    /// The `WHEN` trigger.
    pub when: String,
    /// The primary `THEN` outcome.
    pub then: String,
    /// Additional `AND` outcomes, one line each.
    #[serde(default)]
    pub and: Vec<String>,
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
}

impl Display for Scenario {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{SCENARIO} {}", self.name.trim())?;
        for (field, text) in self.lines() {
            write!(f, "\n- **{}** {}", field.to_ascii_uppercase(), text.trim())?;
        }
        Ok(())
    }
}

/// A requirement identifier rendered as `REQ-NNN`.
///
/// Parsing accepts positive numbers in canonical form, padded to at least
/// three digits.
///
/// # Examples
///
/// ```
/// use emery_engine::specify::ReqId;
///
/// assert_eq!(ReqId::new(7).to_string(), "REQ-007");
/// assert_eq!("REQ-007".parse::<ReqId>()?, ReqId::new(7));
/// assert!("REQ-7".parse::<ReqId>().is_err());
/// # Ok::<(), String>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ReqId(u32);

impl ReqId {
    const PREFIX: &str = "REQ-";

    /// Returns the id numbered `number`.
    ///
    /// Requirement numbers produced by the engine begin at one.
    #[must_use]
    pub const fn new(number: u32) -> Self {
        Self(number)
    }
}

impl FromStr for ReqId {
    type Err = String;

    // An id is well formed exactly when it renders back to itself.
    fn from_str(text: &str) -> Result<Self, String> {
        text.strip_prefix(Self::PREFIX)
            .and_then(|digits| digits.parse().ok())
            .map(Self)
            .filter(|id| id.0 > 0 && id.to_string() == text)
            .ok_or_else(|| format!("malformed id `{text}`"))
    }
}

impl TryFrom<String> for ReqId {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        text.parse()
    }
}

impl From<ReqId> for String {
    fn from(id: ReqId) -> Self {
        id.to_string()
    }
}

impl Display for ReqId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:03}", Self::PREFIX, self.0)
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
