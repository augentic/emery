//! Defines the typed data and Markdown rendering for `spec.md`.
//!
//! A [`Spec`] contains introductory paragraphs and ordered [`Requirement`]
//! records. Each requirement combines reconciled source facts with drafted
//! acceptance scenarios.

use std::fmt::{self, Display, Formatter};

use emery_adapter::Trigger;
use emery_adapter::source::{Anchor, SourceKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Rank;
use crate::revision::{self, ReqId};

pub const ID: &str = "ID:";
pub const SOURCES: &str = "Sources:";
pub const STATUS: &str = "Status:";
pub const ROUTE: &str = "Route:";
pub const MESSAGE: &str = "Message:";
pub const NOTE: &str = "Note:";
pub const HEADING: &str = "### Requirement:";

const SCENARIO: &str = "#### Scenario:";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub emery: u32,
    // The revision this one amends, or `None` when drafted from sources alone.
    pub parent: Option<String>,
    pub preamble: Vec<String>,
    pub requirements: Vec<Requirement>,
}

impl Spec {
    #[must_use]
    pub fn requirement(&self, id: ReqId) -> Option<&Requirement> {
        self.requirements.iter().find(|requirement| requirement.id == id)
    }

    // The preamble, then the requirements `ids` name in the specification's
    // order, rendered as `Display` renders the whole.
    #[must_use]
    pub fn cut(&self, ids: &[ReqId]) -> String {
        let requirements: Vec<&Requirement> =
            self.requirements.iter().filter(|requirement| ids.contains(&requirement.id)).collect();
        fmt::from_fn(|f| revision::write(f, "Specification", &self.preamble, &requirements))
            .to_string()
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: ReqId,
    // The heading derived from the highest-authority contributing claim.
    pub subject: String,
    pub status: Status,
    pub covered: bool,
    // What reaches the requirement, where a source's `contract` claim binds
    // it; rendered as its `Route:` or `Message:` line.
    pub trigger: Option<Trigger>,
    // Contributing claims in descending authority order.
    pub sources: Vec<Cited>,
    // Empty when the requirement is in conflict.
    pub body: Vec<String>,
    // Losing classes for a divergence, and every class for a conflict.
    pub losers: Vec<Loser>,
    pub scenarios: Vec<Scenario>,
}

impl Requirement {
    // Returns the stem of the subject: the first segment of its dotted id,
    // the part floor every requirement under it shares.
    #[must_use]
    pub fn stem(&self) -> &str {
        revision::stem(&self.subject)
    }

    // Returns the requirement's statement: its `body` paragraphs
    // joined, or `None` when it is in conflict.
    #[must_use]
    pub fn statement(&self) -> Option<String> {
        match self.status {
            Status::Conflict => None,
            Status::Agreed | Status::Unknown | Status::Divergence => {
                Some(self.body.join("\n\n")).filter(|body| !body.is_empty())
            }
        }
    }

    // Returns what the requirement states as a change reads it: its
    // statement, or, in conflict, the competing statements joined as
    // `[conflict] <a> | <b>`, so a conflicted requirement is one statement to
    // match a claim against and one `Before:` to show.
    #[must_use]
    pub fn stated(&self) -> Option<String> {
        match self.status {
            Status::Conflict => (!self.losers.is_empty()).then(|| {
                let competing: Vec<&str> =
                    self.losers.iter().map(|loser| loser.statement.as_str()).collect();
                format!("[{}] {}", Status::Conflict, competing.join(" | "))
            }),
            Status::Agreed | Status::Unknown | Status::Divergence => self.statement(),
        }
    }

    // Writes the requirement's `Loser` notes and, when in conflict, the
    // reconciliation note. Shared with `Amendment`'s rendering of a
    // conflicting entry, which notes the competing statements without the
    // coverage note.
    pub(crate) fn write_conflict_notes(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut separator = "\n\n";
        for loser in &self.losers {
            write!(f, "{separator}{loser}")?;
            separator = "\n";
        }
        if self.status == Status::Conflict {
            write!(f, "{separator}{NOTE} Operator reconciliation required.")?;
        }
        Ok(())
    }

    // Returns how many anchors this requirement and `other` cite in common:
    // the pairs of their citations that overlap.
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

        write!(f, "\n\n{ID} {}\n", self.id)?;
        revision::write_list(f, SOURCES, &self.sources)?;
        write!(f, "\n{STATUS} {}", self.status)?;
        match &self.trigger {
            Some(Trigger::Http(route)) => write!(f, "\n{ROUTE} {route}")?,
            Some(Trigger::Message(name)) => write!(f, "\n{MESSAGE} {name}")?,
            None => {}
        }

        // body
        for paragraph in &self.body {
            write!(f, "\n\n{paragraph}")?;
        }

        // notes
        self.write_conflict_notes(f)?;
        if !self.covered {
            let separator = if self.losers.is_empty() && self.status != Status::Conflict {
                "\n\n"
            } else {
                "\n"
            };
            write!(f, "{separator}{NOTE} acceptance criteria not evidenced.")?;
        }

        // scenarios
        for scenario in &self.scenarios {
            write!(f, "\n\n{scenario}")?;
        }
        Ok(())
    }
}

impl From<&Requirement> for emery_adapter::target::Requirement {
    fn from(requirement: &Requirement) -> Self {
        Self {
            id: requirement.id.to_string(),
            subject: requirement.subject.clone(),
            statement: requirement.body.clone(),
            scenarios: requirement.scenarios.iter().map(Into::into).collect(),
            trigger: requirement.trigger.clone(),
        }
    }
}

// A claim cited by a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cited {
    // The name of the source the claim was extracted from.
    pub source: String,
    // The claim id within that source.
    pub claim: String,
    // Where in the source the claim anchors, in the claim `path` grammar
    // (`src/orders.ts#L12-L34`), when the claim carries one.
    pub path: Option<String>,
}

impl Cited {
    // Returns whether this citation and `other` anchor at one place: the
    // same source and the same file, with line ranges that meet, or neither
    // naming lines. A citation without a `path` anchors nowhere.
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

// A contributor class rendered as a requirement note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loser {
    // Every member's source name, in authority order.
    pub sources: Vec<String>,
    // The kind of source the leading contributor was extracted from.
    pub kind: SourceKind,
    // The authority rank the leading contributor lost under.
    pub rank: Rank,
    // The leading contributor's claim identifier.
    pub claim: String,
    // The leading contributor's normalised statement.
    pub statement: String,
}

impl Display for Loser {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{NOTE} {sources} ({kind}, rank {rank}, {claim}): {statement}",
            sources = self.sources.join(", "),
            kind = self.kind,
            rank = self.rank,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    // Returns the scenario's lines in document order, each with its field name.
    //
    // Every `given`, then the `when`, the `then`, and every `and`.
    pub fn lines(&self) -> impl Iterator<Item = (&'static str, &str)> {
        self.given
            .iter()
            .map(|text| ("given", text.as_str()))
            .chain([("when", self.when.as_str()), ("then", self.then.as_str())])
            .chain(self.and.iter().map(|text| ("and", text.as_str())))
    }

    // Returns the scenario with the whitespace around each line dropped.
    //
    // A revision stores lines in this form, so two drafts that differ only
    // in padding commit as one revision.
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

impl From<&Scenario> for emery_adapter::target::Scenario {
    fn from(scenario: &Scenario) -> Self {
        Self {
            name: scenario.name.clone(),
            given: scenario.given.clone(),
            when: scenario.when.clone(),
            then: scenario.then.clone(),
            and: scenario.and.clone(),
        }
    }
}

// The reconciliation status of a requirement.
//
// Every status except `Status::Agreed` is also rendered as a tag on the
// requirement heading.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Status {
    // All contributors agree and acceptance criteria provide coverage.
    Agreed,
    // All contributors agree but no acceptance criterion provides coverage.
    Unknown,
    // Highest-authority contributors disagree and require reconciliation.
    Conflict,
    // A higher-authority contributor resolves the disagreement.
    Divergence,
}
