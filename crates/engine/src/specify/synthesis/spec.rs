//! The `spec.md` brief
//!
//! Asks the model for the drafted content of `spec.md`: the preamble and, for
//! every requirement the evidence changed, its acceptance scenarios. The
//! requirements, their ids, their provenance, and their bodies are the
//! engine's: the schema names the subjects to draft, every candidate draft is
//! verified to carry exactly one entry per listed subject, and the engine
//! places the accepted draft beside its facts in the specification. A
//! requirement whose incumbent still stands keeps the incumbent's scenarios,
//! and a run in which every requirement does is placed without a turn.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_source::claims::DOTTED_KEBAB_PATTERN;
use omnia_guest::{Error, Model};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::artifact::{Cited, EMERY, Requirement, Scenario, Spec, Status};
use crate::specify::Extract;
use crate::specify::basis::{self, Basis};
use crate::specify::brief::{Brief, Review};
use crate::specify::synthesis::ClaimsSection;

/// What the engine needs to ask the model for `spec.md` and to verify its
/// draft: the extracts, the requirement bases derived from them, and the
/// specification the run continues.
pub struct SpecBrief<'a> {
    extracts: &'a [Extract],
    bases: &'a [Basis],
    prior: Option<&'a Spec>,
}

impl<'a> SpecBrief<'a> {
    /// Creates the brief for `spec.md` from the `extracts`, the requirement
    /// `bases` derived from them, and the `prior` specification they were
    /// numbered from.
    #[must_use]
    pub const fn new(extracts: &'a [Extract], bases: &'a [Basis], prior: Option<&'a Spec>) -> Self {
        Self {
            extracts,
            bases,
            prior,
        }
    }

    /// Resolves the specification: asks the model for the drafted content
    /// of every requirement without an incumbent, or — when every requirement
    /// has one — keeps the prior preamble and places the incumbents
    /// without a turn.
    ///
    /// # Errors
    ///
    /// A model failure is `bad_gateway`; an answer outside the schema, or a
    /// draft the backend could not repair within its rounds, is `bad_request`.
    pub async fn resolve<M: Model>(self, model: &M) -> Result<Spec, Error> {
        if let Some(prior) = self.prior
            && self.drafted().next().is_none()
        {
            tracing::info!("every requirement stands; the specification is carried");
            return Ok(self.place(prior.preamble.clone(), BTreeMap::new()));
        }

        self.judge(model).await
    }

    // The bases the model drafts: those without an incumbent.
    fn drafted(&self) -> impl Iterator<Item = &Basis> {
        self.bases.iter().filter(|basis| basis.incumbent.is_none())
    }

    // Places the specification: every requirement in id order, each the
    // engine's facts beside its scenarios — drafted, or the incumbent's.
    fn place(&self, preamble: Vec<String>, mut drafts: BTreeMap<String, Vec<Scenario>>) -> Spec {
        let mut requirements: Vec<Requirement> = self
            .bases
            .iter()
            .map(|basis| {
                let scenarios = basis.incumbent.as_ref().map_or_else(
                    || {
                        drafts
                            .remove(basis.subject.as_str())
                            .expect("verify held the draft to the requirements")
                    },
                    |incumbent| incumbent.scenarios.clone(),
                );
                basis.requirement(scenarios)
            })
            .collect();
        requirements.sort_by_key(|requirement| requirement.id);

        Spec {
            emery: EMERY,
            next_id: basis::next_id(self.bases, self.prior),
            preamble,
            requirements,
        }
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

    // Tightens the derived schema to this run: exactly one entry per drafted
    // requirement, each `subject` drawn from their subjects, and at least one
    // scenario per entry.
    fn tighten(&self, schema: &mut Value) {
        let count = self.drafted().count();
        schema["properties"]["requirements"]["minItems"] = json!(count);
        schema["properties"]["requirements"]["maxItems"] = json!(count);
        schema["$defs"]["Draft"]["properties"]["subject"]["enum"] =
            json!(self.drafted().map(|basis| &basis.subject).collect::<Vec<_>>());
        schema["$defs"]["Draft"]["properties"]["scenarios"]["minItems"] = json!(1);
    }

    // Verifies a candidate draft against the requirements: every drafted
    // requirement exactly once and nothing else, at least one scenario per
    // entry with one-line fields, no reserved opener in the preamble.
    fn verify(&self, answer: &SpecAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");

        let by_subject: BTreeMap<&str, &Basis> =
            self.bases.iter().map(|basis| (basis.subject.as_str(), basis)).collect();
        let mut seen = BTreeSet::new();
        for draft in &answer.requirements {
            let subject = draft.subject.as_str();
            if !seen.insert(subject) {
                review.note(format_args!("`{subject}` is drafted more than once"));
                continue;
            }

            match by_subject.get(subject) {
                None => {
                    review.note(format_args!("`{subject}` is not a requirement"));
                    continue;
                }
                Some(basis) if basis.incumbent.is_some() => {
                    review.note(format_args!("`{subject}` is unchanged and not to be drafted"));
                    continue;
                }
                Some(_) => {}
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

        for basis in self.drafted().filter(|basis| !seen.contains(basis.subject.as_str())) {
            review.note(format_args!("requirement `{}` is not drafted", basis.subject));
        }
    }

    // Places the accepted draft in the specification beside the incumbents.
    fn into_output(self, answer: SpecAnswer) -> Self::Output {
        let drafts =
            answer.requirements.into_iter().map(|draft| (draft.subject, draft.scenarios)).collect();
        self.place(answer.preamble, drafts)
    }
}

// Renders the user turn of the prompt: every claim in every extract, then
// every requirement to draft with its id, status, sources, and coverage, each
// contributing claim labelled winner / loser / contributor, then the
// requirements that stand unchanged, for the preamble's sake.
impl Display for SpecBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Draft `spec.md`.\n\n{claims}", claims = ClaimsSection(self.extracts))?;

        f.write_str("\n## Requirements (draft one entry per subject)\n\n")?;
        for basis in self.drafted() {
            let sources = basis.contributors().map(Cited::from).map(|cited| cited.to_string());
            let coverage = if basis.covered { "evidenced" } else { "not evidenced" };
            writeln!(
                f,
                "- {id} `{subject}` — Status: {status} — Sources: [{sources}] — acceptance criteria {coverage}",
                id = basis.id,
                subject = basis.subject,
                status = basis.status,
                sources = sources.collect::<Vec<_>>().join(", "),
            )?;

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

        let standing: Vec<&Basis> =
            self.bases.iter().filter(|basis| basis.incumbent.is_some()).collect();
        if !standing.is_empty() {
            f.write_str(
                "\n## Unchanged requirements (already drafted; do not answer for these)\n\n",
            )?;
            for basis in standing {
                writeln!(f, "- {id} `{subject}`", id = basis.id, subject = basis.subject)?;
            }
        }

        Ok(())
    }
}

/// The `spec.md` draft: preamble paragraphs and one entry per requirement to
/// draft. Only what needs synthesis is asked for; every heading, provenance
/// line, body, and note is the renderer's.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery spec draft")]
pub struct SpecAnswer {
    /// Markdown paragraphs before the first requirement.
    pub preamble: Vec<String>,
    /// One draft per listed requirement, keyed by subject; any order.
    pub requirements: Vec<Draft>,
}

/// The drafted content of one requirement.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    /// The requirement's subject, exactly as listed.
    #[schemars(regex(pattern = DOTTED_KEBAB_PATTERN))]
    pub subject: String,
    /// At least one scenario.
    pub scenarios: Vec<Scenario>,
}
