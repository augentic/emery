//! The `spec.md` brief
//!
//! Asks the model for the content of `spec.md`: the preamble and, for every
//! requirement, a body and its acceptance scenarios. The requirements, their
//! headings, and their provenance lines are the engine's: the schema names
//! the subjects, every candidate draft is verified to carry exactly one entry
//! per requirement in the shape its status allows, and the engine renders the
//! accepted draft into the canonical document.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter, Write as _};

use emery_source::claims::DOTTED_KEBAB_PATTERN;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::artifact::{HEADING, ID, NOTE, ReqId, SCENARIO, SOURCES, STATUS, Status};
use crate::specify::Extract;
use crate::specify::basis::{Basis, Contributor, normalise};
use crate::specify::brief::{Brief, Review};
use crate::specify::dossier::{ClaimsSection, Markdown};

/// What the engine needs to ask the model for `spec.md` and to verify its
/// draft: the extracts and the requirement bases derived from them.
pub struct SpecBrief<'a> {
    extracts: &'a [Extract],
    bases: &'a [Basis],
}

impl<'a> SpecBrief<'a> {
    /// Creates the brief for `spec.md` from the `extracts` and the
    /// requirement `bases` derived from them.
    #[must_use]
    pub const fn new(extracts: &'a [Extract], bases: &'a [Basis]) -> Self {
        Self { extracts, bases }
    }
}

impl Brief for SpecBrief<'_> {
    type Answer = SpecAnswer;
    type Output = String;

    const NAME: &'static str = "spec-draft";
    // Prompt order is significant.
    const PROSE: &'static [&'static str] = &[
        "synthesis/synthesise.md",
        "synthesis/authority.md",
        "synthesis/claim-landing.md",
        "synthesis/requirement-block.md",
        "synthesis/spec-format.md",
        "synthesis/tags.md",
    ];

    // Tightens the derived schema to this run: exactly one entry per
    // requirement, each `subject` drawn from the requirement subjects, and at
    // least one scenario per entry.
    fn tighten(&self, schema: &mut Value) {
        let count = self.bases.len();
        schema["properties"]["requirements"]["minItems"] = json!(count);
        schema["properties"]["requirements"]["maxItems"] = json!(count);
        schema["$defs"]["Entry"]["properties"]["subject"]["enum"] =
            json!(self.bases.iter().map(Basis::subject).collect::<Vec<_>>());
        schema["$defs"]["Entry"]["properties"]["scenarios"]["minItems"] = json!(1);
    }

    // Verifies a candidate draft against the requirements: every requirement
    // drafted exactly once and nothing else, at least one scenario per entry
    // with one-line fields, a body unless in conflict, no reserved opener.
    fn verify(&self, answer: &SpecAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");

        let by_subject: BTreeMap<&str, &Basis> =
            self.bases.iter().map(|basis| (basis.subject(), basis)).collect();
        let mut seen = BTreeSet::new();
        for entry in &answer.requirements {
            let subject = entry.subject.as_str();
            if !seen.insert(subject) {
                review.note(format_args!("`{subject}` is drafted more than once"));
                continue;
            }

            let Some(basis) = by_subject.get(subject) else {
                review.note(format_args!("`{subject}` is not a requirement"));
                continue;
            };

            let label = format!("`{subject}`");
            review.paragraphs(&entry.body, &label);

            // A conflict's statements are the renderer's notes, so a body
            // would assert what the operator has yet to reconcile.
            let conflict = basis.status() == Status::Conflict;
            if conflict && !entry.body.is_empty() {
                review.note(format_args!("{label} is in conflict and carries a body"));
            } else if !conflict && entry.body.is_empty() {
                review.note(format_args!("{label} has no body paragraph"));
            }

            if entry.scenarios.is_empty() {
                review.note(format_args!("{label} has no scenario"));
            }

            for scenario in &entry.scenarios {
                for (field, text) in
                    [("name", &scenario.name), ("when", &scenario.when), ("then", &scenario.then)]
                {
                    review.line(text, format_args!("{label} scenario `{field}`"));
                }
                for given in &scenario.given {
                    review.line(given, format_args!("{label} scenario `given`"));
                }
            }
        }

        for subject in by_subject.keys().filter(|subject| !seen.contains(*subject)) {
            review.note(format_args!("requirement `{subject}` is not drafted"));
        }
    }

    // Renders `spec.md`: the requirements in order, each with its drafted
    // content.
    fn into_output(self, answer: SpecAnswer) -> Self::Output {
        let entries: BTreeMap<&str, &Entry> =
            answer.requirements.iter().map(|entry| (entry.subject.as_str(), entry)).collect();

        let mut document = Markdown::new("Specification");
        document.extend(&answer.preamble);

        for (index, basis) in self.bases.iter().enumerate() {
            let status = basis.status();
            let tag = status.tag().map(|tag| format!(" [{tag}]")).unwrap_or_default();
            document.push(format!("{HEADING} {}{tag}", basis.subject()));
            document.push(format!(
                "{ID} {id}\n{SOURCES} [{sources}]\n{STATUS} {status}",
                id = ReqId::nth(index),
                sources = basis.sources().collect::<Vec<_>>().join(", "),
            ));

            let draft =
                entries.get(basis.subject()).expect("verify held the draft to the requirements");
            if status != Status::Conflict {
                document.extend(&draft.body);
            }
            if let Some(notes) = notes(basis) {
                document.push(notes);
            }

            for scenario in &draft.scenarios {
                document.push(format!("{SCENARIO} {}", scenario.name.trim()));
                let mut bullets = String::new();
                for given in &scenario.given {
                    let _ = writeln!(bullets, "- **GIVEN** {}", given.trim());
                }
                let _ = writeln!(bullets, "- **WHEN** {}", scenario.when.trim());
                let _ = write!(bullets, "- **THEN** {}", scenario.then.trim());
                document.push(bullets);
            }
        }

        document.finish()
    }
}

// Renders the user turn of the prompt: every claim in every extract, then
// every requirement with its id, status, sources, and coverage, each
// contributing claim labelled winner / loser / contributor.
impl Display for SpecBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Draft `spec.md`.\n\n{claims}", claims = ClaimsSection(self.extracts))?;

        f.write_str("\n## Requirements (draft one entry per subject)\n\n")?;
        for (index, basis) in self.bases.iter().enumerate() {
            let sources = basis.sources().collect::<Vec<_>>().join(", ");
            let coverage = if basis.covered() { "evidenced" } else { "not evidenced" };
            writeln!(
                f,
                "- {id} `{subject}` — Status: {status} — Sources: [{sources}] — acceptance criteria {coverage}",
                id = ReqId::nth(index),
                subject = basis.subject(),
                status = basis.status(),
            )?;

            for (position, class) in basis.classes().iter().enumerate() {
                let role = match (basis.status(), position) {
                    (Status::Divergence, 0) => "winner",
                    (Status::Divergence, _) => "loser",
                    _ => "contributor",
                };

                for member in class {
                    writeln!(
                        f,
                        "  - {role}: {source} ({authority}, `{claim}`): {statement}",
                        source = member.source,
                        authority = member.authority,
                        claim = member.id,
                        statement = member.statement,
                    )?;
                }
            }
        }

        Ok(())
    }
}

/// The `spec.md` draft: preamble paragraphs and one entry per requirement.
/// Only what needs synthesis is asked for; every heading, provenance line, and
/// note is the renderer's.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery spec draft")]
pub struct SpecAnswer {
    /// Markdown paragraphs before the first requirement.
    pub preamble: Vec<String>,
    /// One entry per requirement, keyed by subject; any order.
    pub requirements: Vec<Entry>,
}

/// The drafted content of one requirement.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// The requirement's subject, exactly as listed.
    #[schemars(regex(pattern = DOTTED_KEBAB_PATTERN))]
    pub subject: String,
    /// Markdown paragraphs; empty for a requirement in conflict.
    pub body: Vec<String>,
    /// At least one scenario.
    pub scenarios: Vec<Scenario>,
}

/// One acceptance scenario.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
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
}

// Builds the `Note:` lines appended to a requirement: one per losing class of
// a divergence, one per class plus the reconciliation line for a conflict,
// then one when the acceptance criteria are not evidenced. `None` if none.
fn notes(basis: &Basis) -> Option<String> {
    let mut lines = Vec::new();

    let classes = basis.classes();
    match basis.status() {
        Status::Divergence => lines.extend(classes.iter().skip(1).map(|class| note(class))),
        Status::Conflict => {
            lines.extend(classes.iter().map(|class| note(class)));
            lines.push(format!("{NOTE} Operator reconciliation required."));
        }
        Status::Agreed | Status::Unknown => {}
    }

    if !basis.covered() {
        lines.push(format!("{NOTE} acceptance criteria not evidenced."));
    }

    (!lines.is_empty()).then(|| lines.join("\n"))
}

// Formats one class as `Note: <sources> (<authority>, <id>): <statement>`,
// listing every member's source but taking the rest from the lead member.
fn note(class: &[Contributor]) -> String {
    let sources = class.iter().map(|member| member.source.as_str()).collect::<Vec<_>>().join(", ");
    let lead = &class[0];

    format!(
        "{NOTE} {sources} ({authority}, {id}): {statement}",
        authority = lead.authority,
        id = lead.id,
        statement = normalise(&lead.statement),
    )
}
