//! # The specification
//!
//! The typed form of `spec.md`: a preamble and one requirement per subject,
//! each carrying the facts the engine derived — id, status, coverage, cited
//! claims, the statements that lost — and the drafted body and scenarios.
//! `Display` renders the Markdown projection an operator reads.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use emery_adapter::source::Authority;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::revision;

/// The `ID:` provenance key; the three keys follow the heading in this order.
pub const ID: &str = "ID:";
/// The `Sources:` provenance key.
pub const SOURCES: &str = "Sources:";
/// The `Status:` provenance key.
pub const STATUS: &str = "Status:";
/// The `Note:` key: the engine's own lines below the provenance.
pub const NOTE: &str = "Note:";

const HEADING: &str = "### Requirement:";
const SCENARIO: &str = "#### Scenario:";

/// The specification.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// The grammar the document was written under.
    pub emery: u32,
    /// Markdown paragraphs before the first requirement.
    pub preamble: Vec<String>,
    /// The requirements, in id order.
    pub requirements: Vec<Requirement>,
}

impl Spec {
    /// Finds the requirement `id` names.
    #[must_use]
    pub fn requirement(&self, id: ReqId) -> Option<&Requirement> {
        self.requirements.iter().find(|requirement| requirement.id == id)
    }
}

impl revision::Document for Spec {
    const NAME: &'static str = "spec";
}

// Renders `spec.md`: the preamble, then every requirement block.
impl Display for Spec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        revision::write(f, "Specification", &self.preamble, &self.requirements)
    }
}

/// One requirement: the engine's facts and the drafted prose.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// The stable id.
    pub id: ReqId,
    /// The heading name: the top contributor's claim id.
    pub subject: String,
    /// How the contributors agree.
    pub status: Status,
    /// Whether an acceptance criterion covers the requirement.
    pub covered: bool,
    /// Every cited claim, highest authority first.
    pub sources: Vec<Cited>,
    /// Markdown paragraphs; empty for a requirement in conflict.
    pub body: Vec<String>,
    /// The classes whose statements are notes: the losing classes of a
    /// divergence, every class of a conflict.
    pub losers: Vec<Loser>,
    /// The acceptance scenarios.
    pub scenarios: Vec<Scenario>,
}

// Renders the tagged heading, provenance, body, notes, and scenarios.
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

/// One cited claim: the source key and the claim id it contributed.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cited {
    /// The source key.
    pub source: String,
    /// The claim id within that source.
    pub claim: String,
}

// Writes the citation as the `Sources:` line spells it, `<source>:<claim>`.
impl Display for Cited {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.claim)
    }
}

/// One class whose statement the requirement records as a note.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loser {
    /// Every member's source key, in authority order.
    pub sources: Vec<String>,
    /// The lead member's authority.
    pub authority: Authority,
    /// The lead member's claim id.
    pub claim: String,
    /// The lead member's statement, whitespace-normalised.
    pub statement: String,
}

// Writes `Note: <sources> (<authority>, <claim>): <statement>`.
impl Display for Loser {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{NOTE} {sources} ({authority}, {claim}): {statement}",
            sources = self.sources.join(", "),
            authority = self.authority,
            claim = self.claim,
            statement = self.statement,
        )
    }
}

/// One acceptance scenario: the shape the specification stores and the shape a
/// draft answers in — the same fields, so the draft is placed as it stands.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The scenario heading name.
    pub name: String,
    /// Optional GIVEN context, one line each.
    #[serde(default)]
    pub given: Vec<String>,
    /// The WHEN trigger, one line.
    pub when: String,
    /// The THEN outcome, one line.
    pub then: String,
    /// Optional further AND outcomes, one line each.
    #[serde(default)]
    pub and: Vec<String>,
}

impl Scenario {
    /// Yields the scenario's lines in document order — every `given`, the
    /// `when`, the `then`, every `and` — each with its field name.
    pub fn lines(&self) -> impl Iterator<Item = (&'static str, &str)> {
        self.given
            .iter()
            .map(|text| ("given", text.as_str()))
            .chain([("when", self.when.as_str()), ("then", self.then.as_str())])
            .chain(self.and.iter().map(|text| ("and", text.as_str())))
    }
}

// Writes `#### Scenario: <name>`, then one `- **GIVEN**` / `**WHEN**` /
// `**THEN**` / `**AND**` bullet per line.
impl Display for Scenario {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{SCENARIO} {}", self.name.trim())?;
        for (field, text) in self.lines() {
            write!(f, "\n- **{}** {}", field.to_ascii_uppercase(), text.trim())?;
        }
        Ok(())
    }
}

/// A requirement id, `REQ-NNN`: a positive number, zero-padded to at least
/// three digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ReqId(u32);

impl ReqId {
    const PREFIX: &str = "REQ-";

    /// The id numbered `number`.
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

/// The closed `Status:` vocabulary; every status but `agreed` doubles as
/// the heading `[tag]`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Status {
    /// One class of contributors, covered by a criterion.
    Agreed,
    /// One class of contributors, no acceptance criterion in evidence.
    Unknown,
    /// Tied top-authority disagreement; the operator must reconcile.
    Conflict,
    /// Authority-resolved disagreement; the losers are notes.
    Divergence,
}
