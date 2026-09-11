//! The `spec.md` brief
//!
//! Asks the model for the drafted content of `spec.md`: the preamble and, for
//! every requirement, its acceptance scenarios. The requirements, their ids,
//! their provenance, and their bodies are the engine's: the schema names the
//! subjects to draft, every candidate draft is verified to carry exactly one
//! entry per listed subject, and the engine places the accepted draft beside
//! its facts in the specification.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_source::claims::DOTTED_KEBAB_PATTERN;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::artifact::{EMERY, Scenario, Spec, Status};
use crate::specify::Extract;
use crate::specify::basis::Basis;
use crate::specify::brief::{Brief, Review};
use crate::specify::synthesis::ClaimsSection;

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
    type Output = Spec;

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
    // requirement, each `subject` drawn from their subjects, and at least one
    // scenario per entry.
    fn tighten(&self, schema: &mut Value) {
        let count = self.bases.len();
        schema["properties"]["requirements"]["minItems"] = json!(count);
        schema["properties"]["requirements"]["maxItems"] = json!(count);
        schema["$defs"]["Draft"]["properties"]["subject"]["enum"] =
            json!(self.bases.iter().map(|basis| &basis.subject).collect::<Vec<_>>());
        schema["$defs"]["Draft"]["properties"]["scenarios"]["minItems"] = json!(1);
    }

    // Verifies a candidate draft against the requirements: every requirement
    // exactly once and nothing else, at least one scenario per entry with
    // one-line fields, no reserved opener in the preamble.
    fn verify(&self, answer: &SpecAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");

        let subjects: BTreeSet<&str> =
            self.bases.iter().map(|basis| basis.subject.as_str()).collect();
        let mut seen = BTreeSet::new();
        for draft in &answer.requirements {
            let subject = draft.subject.as_str();
            if !seen.insert(subject) {
                review.note(format_args!("`{subject}` is drafted more than once"));
                continue;
            }
            if !subjects.contains(subject) {
                review.note(format_args!("`{subject}` is not a requirement"));
                continue;
            }

            let label = format!("`{subject}`");
            if draft.scenarios.is_empty() {
                review.note(format_args!("{label} has no scenario"));
            }

            for scenario in &draft.scenarios {
                review.line(&scenario.name, format_args!("{label} scenario `name`"));
                for (field, text) in scenario.lines() {
                    review.line(text, format_args!("{label} scenario `{field}`"));
                }
            }
        }

        for subject in subjects.difference(&seen) {
            review.note(format_args!("requirement `{subject}` is not drafted"));
        }
    }

    // Places the accepted draft in the specification: every requirement in
    // id order, each the engine's facts beside its drafted scenarios.
    fn into_output(self, answer: SpecAnswer) -> Self::Output {
        let mut drafts: BTreeMap<String, Vec<Scenario>> =
            answer.requirements.into_iter().map(|draft| (draft.subject, draft.scenarios)).collect();
        let requirements = self
            .bases
            .iter()
            .map(|basis| {
                let scenarios = drafts
                    .remove(basis.subject.as_str())
                    .expect("verify held the draft to the requirements");
                basis.requirement(scenarios)
            })
            .collect();

        Spec {
            emery: EMERY,
            preamble: answer.preamble,
            requirements,
        }
    }
}

// Renders the user turn of the prompt: every claim in every extract, then
// every requirement to draft with its id, status, sources, and coverage, each
// contributing claim labelled winner / loser / contributor.
impl Display for SpecBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Draft `spec.md`.\n\n{claims}", claims = ClaimsSection(self.extracts))?;

        f.write_str("\n## Requirements (draft one entry per subject)\n\n")?;
        for basis in self.bases {
            let coverage = if basis.covered { "evidenced" } else { "not evidenced" };
            write!(
                f,
                "- {id} `{subject}` — Status: {status} — Sources: [",
                id = basis.id,
                subject = basis.subject,
                status = basis.status,
            )?;
            for (position, member) in basis.contributors().enumerate() {
                if position > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{}:{}", member.source, member.id)?;
            }
            writeln!(f, "] — acceptance criteria {coverage}")?;

            for (position, class) in basis.classes.iter().enumerate() {
                let role = match (basis.status, position) {
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

/// The `spec.md` draft: preamble paragraphs and one entry per requirement to
/// draft. Only what needs synthesis is asked for; every heading, provenance
/// line, body, and note is the renderer's.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery spec draft")]
pub struct SpecAnswer {
    /// Markdown paragraphs before the first requirement.
    pub preamble: Vec<String>,
    /// One draft per listed requirement, keyed by subject; any order.
    pub requirements: Vec<Draft>,
}

/// The drafted content of one requirement.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    /// The requirement's subject, exactly as listed.
    #[schemars(regex(pattern = DOTTED_KEBAB_PATTERN))]
    pub subject: String,
    /// At least one scenario.
    pub scenarios: Vec<Scenario>,
}
