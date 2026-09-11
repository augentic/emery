//! # The specification
//!
//! The typed form of `spec.md`: a preamble and one requirement per subject,
//! each carrying the facts the engine derived — id, status, coverage, cited
//! claims, the statements that lost — and the drafted body and scenarios.
//! `Display` renders the Markdown projection an operator reads.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use emery_source::types::Authority;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

// Renders `spec.md`: the preamble, then every requirement block.
impl Display for Spec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "# Specification")?;

        let blocks = self.requirements.iter().map(ToString::to_string);
        for block in self.preamble.iter().cloned().chain(blocks) {
            for (position, line) in block.lines().enumerate() {
                f.write_str(if position == 0 { "\n\n" } else { "\n" })?;
                f.write_str(line.trim_end())?;
            }
        }
        f.write_str("\n")
    }
}

/// One requirement: the engine's facts and the drafted prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl Requirement {
    /// Names the fields, other than `id`, on which `self` and `other` differ.
    #[must_use]
    pub fn differences(&self, other: &Self) -> Vec<&'static str> {
        [
            ("subject", self.subject != other.subject),
            ("status", self.status != other.status),
            ("covered", self.covered != other.covered),
            ("sources", self.sources != other.sources),
            ("body", self.body != other.body),
            ("losers", self.losers != other.losers),
            ("scenarios", self.scenarios != other.scenarios),
        ]
        .into_iter()
        .filter_map(|(name, differs)| differs.then_some(name))
        .collect()
    }
}

// Renders the tagged heading, provenance, body, notes, and scenarios.
impl Display for Requirement {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{HEADING} {}", self.subject)?;
        if self.status != Status::Agreed {
            write!(f, " [{}]", self.status)?;
        }

        let sources: Vec<String> = self.sources.iter().map(ToString::to_string).collect();
        write!(
            f,
            "\n\n{ID} {id}\n{SOURCES} [{sources}]\n{STATUS} {status}",
            id = self.id,
            sources = sources.join(", "),
            status = self.status,
        )?;

        // body
        for paragraph in &self.body {
            write!(f, "\n\n{paragraph}")?;
        }

        // notes
        let mut notes: Vec<String> = self.losers.iter().map(ToString::to_string).collect();
        if self.status == Status::Conflict {
            notes.push(format!("{NOTE} Operator reconciliation required."));
        }
        if !self.covered {
            notes.push(format!("{NOTE} acceptance criteria not evidenced."));
        }
        if !notes.is_empty() {
            write!(f, "\n\n{}", notes.join("\n"))?;
        }

        // scenarios
        for scenario in &self.scenarios {
            write!(f, "\n\n{scenario}")?;
        }
        Ok(())
    }
}

/// One cited claim: the source key and the claim id it contributed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
