//! # The specification
//!
//! The typed master of `spec.md`: a preamble and one requirement per subject,
//! each carrying the facts the engine derived — id, status, coverage, cited
//! claims, the statements that lost — and the drafted body and scenarios.
//! `Display` renders the Markdown projection an operator reads.

use std::fmt::{self, Display, Formatter, Write as _};

use emery_source::types::Authority;
use serde::{Deserialize, Serialize};

use crate::artifact::Markdown;

/// The requirement heading marker.
pub const HEADING: &str = "### Requirement:";
/// The scenario heading marker.
pub const SCENARIO: &str = "#### Scenario:";
/// The `ID:` provenance key; the three keys follow the heading in this order.
pub const ID: &str = "ID:";
/// The `Sources:` provenance key.
pub const SOURCES: &str = "Sources:";
/// The `Status:` provenance key.
pub const STATUS: &str = "Status:";
/// The `Note:` key: the engine's own lines below the provenance.
pub const NOTE: &str = "Note:";

/// The specification master.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// The master grammar the document was written under.
    pub emery: u32,
    /// The next requirement id to allocate; ids are never reused.
    pub next_id: u32,
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
        let mut document = Markdown::new("Specification");
        document.extend(&self.preamble);
        for requirement in &self.requirements {
            requirement.render(&mut document);
        }
        f.write_str(&document.finish())
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
        let mut fields = Vec::new();
        for (name, differs) in [
            ("subject", self.subject != other.subject),
            ("status", self.status != other.status),
            ("covered", self.covered != other.covered),
            ("sources", self.sources != other.sources),
            ("body", self.body != other.body),
            ("losers", self.losers != other.losers),
            ("scenarios", self.scenarios != other.scenarios),
        ] {
            if differs {
                fields.push(name);
            }
        }
        fields
    }

    // Renders the block: the tagged heading, the provenance lines, the body,
    // the notes, then each scenario.
    fn render(&self, document: &mut Markdown) {
        let tag = self.status.tag().map(|tag| format!(" [{tag}]")).unwrap_or_default();
        document.push(format!("{HEADING} {}{tag}", self.subject));

        let sources: Vec<String> = self.sources.iter().map(ToString::to_string).collect();
        document.push(format!(
            "{ID} {id}\n{SOURCES} [{sources}]\n{STATUS} {status}",
            id = self.id,
            sources = sources.join(", "),
            status = self.status,
        ));

        document.extend(&self.body);
        if let Some(notes) = self.notes() {
            document.push(notes);
        }

        for scenario in &self.scenarios {
            document.push(format!("{SCENARIO} {}", scenario.name.trim()));
            document.push(scenario.bullets());
        }
    }

    // Builds the `Note:` lines: one per loser, the reconciliation line for a
    // conflict, then one when the acceptance criteria are not evidenced.
    fn notes(&self) -> Option<String> {
        let mut lines: Vec<String> = self.losers.iter().map(ToString::to_string).collect();
        if self.status == Status::Conflict {
            lines.push(format!("{NOTE} Operator reconciliation required."));
        }
        if !self.covered {
            lines.push(format!("{NOTE} acceptance criteria not evidenced."));
        }

        (!lines.is_empty()).then(|| lines.join("\n"))
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

/// One acceptance scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The scenario heading name.
    pub name: String,
    /// GIVEN context, one line each.
    pub given: Vec<String>,
    /// The WHEN trigger, one line.
    pub when: String,
    /// The THEN outcome, one line.
    pub then: String,
    /// Further AND outcomes, one line each.
    pub and: Vec<String>,
}

impl Scenario {
    // Renders the bullet list under the scenario heading.
    fn bullets(&self) -> String {
        let mut bullets = String::new();
        for given in &self.given {
            let _ = writeln!(bullets, "- **GIVEN** {}", given.trim());
        }
        let _ = writeln!(bullets, "- **WHEN** {}", self.when.trim());
        let _ = write!(bullets, "- **THEN** {}", self.then.trim());
        for and in &self.and {
            let _ = write!(bullets, "\n- **AND** {}", and.trim());
        }
        bullets
    }
}

/// A requirement id, `REQ-NNN`: a positive number, zero-padded to at least
/// three digits, allocated once and never reused.
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

    /// The id's number.
    #[must_use]
    pub const fn number(self) -> u32 {
        self.0
    }
}

impl TryFrom<String> for ReqId {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        let digits = text.strip_prefix(Self::PREFIX).unwrap_or_default();
        let number = digits.parse::<u32>().ok();
        match number {
            Some(number) if digits.len() >= 3 && number > 0 && text == Self(number).to_string() => {
                Ok(Self(number))
            }
            _ => Err(format!("malformed id `{text}`")),
        }
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

impl Status {
    /// The heading tag this status pairs with; `agreed` carries none.
    #[must_use]
    pub fn tag(self) -> Option<Self> {
        (self != Self::Agreed).then_some(self)
    }
}
