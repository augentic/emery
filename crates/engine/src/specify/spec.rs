//! Synthesises the drafted content of `spec.md`.
//!
//! The model supplies introductory paragraphs and acceptance scenarios. The
//! engine retains ownership of requirement identifiers, provenance, status,
//! and body text. A run is drafted in chunks of at most [`SPEC_CHUNK`]
//! requirements, grouped by stem, each its own turn; every response must
//! contain exactly one draft for each requirement subject of its chunk, and
//! only the first chunk's carries the preamble.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::CLAIM_ID_REGEX;
use omnia_sdk::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::revision::{EMERY, Requirement, Scenario, Spec};
use crate::specify::basis::Basis;
use crate::specify::brief::{BasesSection, Brief, ClaimsSection, Review};
use crate::specify::{Extract, shape};

// The outcome a scenario states where no criterion evidences one.
const UNKNOWN: &str = "[unknown]";

/// How many requirements one `spec-draft` turn drafts at most.
///
/// A larger run is drafted in chunks, each stem kept whole where it fits
/// and smaller stems merged up to the cap, so a refused candidate costs one
/// chunk's regeneration rather than the whole draft's.
pub const SPEC_CHUNK: usize = 25;

/// A synthesis brief for the drafted portions of `spec.md`.
///
/// The brief combines extracted claims with one chunk of their reconciled
/// requirement bases.
pub struct SpecBrief<'a> {
    extracts: &'a [Extract],
    bases: Vec<&'a Basis<'a>>,
    // whether this chunk's answer carries the preamble: the first's alone
    preamble: bool,
}

impl<'a> SpecBrief<'a> {
    /// Returns the briefs `bases` are drafted under, in requirement order:
    /// one per chunk of at most [`SPEC_CHUNK`] requirements, grouped by
    /// stem — a stem past the cap split, smaller ones merged up to it — the
    /// first alone asking for the preamble.
    #[must_use]
    pub fn chunked(extracts: &'a [Extract], bases: &'a [Basis<'a>]) -> Vec<Self> {
        let mut stems: Vec<(&str, Vec<&'a Basis<'a>>)> = Vec::new();
        for basis in bases {
            let stem = shape::stem(basis.subject);
            match stems.iter_mut().find(|(known, _)| *known == stem) {
                Some((_, under)) => under.push(basis),
                None => stems.push((stem, vec![basis])),
            }
        }

        let mut chunks: Vec<Vec<&'a Basis<'a>>> = Vec::new();
        let mut current: Vec<&'a Basis<'a>> = Vec::new();
        for (_, under) in stems {
            if under.len() > SPEC_CHUNK {
                if !current.is_empty() {
                    chunks.push(std::mem::take(&mut current));
                }
                chunks.extend(under.chunks(SPEC_CHUNK).map(<[_]>::to_vec));
                continue;
            }
            if current.len() + under.len() > SPEC_CHUNK {
                chunks.push(std::mem::take(&mut current));
            }
            current.extend(under);
        }
        if !current.is_empty() {
            chunks.push(current);
        }

        chunks
            .into_iter()
            .enumerate()
            .map(|(index, bases)| Self {
                extracts,
                bases,
                preamble: index == 0,
            })
            .collect()
    }

    /// Returns the specification the chunks' accepted answers make together:
    /// the preamble the first carried, and every requirement in id order.
    #[must_use]
    pub fn assemble(drafts: Vec<(Vec<String>, Vec<Requirement>)>) -> Spec {
        let mut preamble = Vec::new();
        let mut requirements = Vec::new();
        for (paragraphs, drafted) in drafts {
            preamble.extend(paragraphs);
            requirements.extend(drafted);
        }
        requirements.sort_by_key(|requirement| requirement.id);

        Spec {
            emery: EMERY,
            preamble,
            requirements,
        }
    }
}

impl Brief for SpecBrief<'_> {
    type Answer = SpecAnswer;
    type Output = (Vec<String>, Vec<Requirement>);

    const NAME: &'static str = "spec-draft";
    const PROSE: &'static [&'static str] = &[
        "synthesise.md",
        "authority.md",
        "claim-landing.md",
        "requirement-block.md",
        "spec-format.md",
        "tags.md",
    ];

    fn tighten(&self, schema: &mut Value) {
        let count = self.bases.len();
        schema["properties"]["requirements"]["minItems"] = json!(count);
        schema["properties"]["requirements"]["maxItems"] = json!(count);
        schema["$defs"]["Draft"]["properties"]["subject"]["enum"] =
            json!(self.bases.iter().map(|basis| &basis.subject).collect::<Vec<_>>());
        schema["$defs"]["Draft"]["properties"]["scenarios"]["minItems"] = json!(1);
        if !self.preamble {
            schema["properties"]["preamble"]["maxItems"] = json!(0);
        }
    }

    fn verify(&self, answer: &SpecAnswer, review: &mut Review) {
        if self.preamble {
            review.paragraphs(&answer.preamble, "preamble");
        } else if !answer.preamble.is_empty() {
            review.note("the preamble is drafted with the first requirements; leave it empty here");
        }

        // each draft against its requirement
        let by_subject: BTreeMap<&str, &Basis<'_>> =
            self.bases.iter().map(|basis| (basis.subject, *basis)).collect();
        let mut seen = BTreeSet::new();
        let mut outcomes: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
        for draft in &answer.requirements {
            let subject = draft.subject.as_str();
            if !seen.insert(subject) {
                review.note(format_args!("`{subject}` is drafted more than once"));
                continue;
            }
            let Some(basis) = by_subject.get(subject) else {
                review.note(format_args!("`{subject}` is not a requirement"));
                continue;
            };
            for outcome in verify_draft(basis, draft, review) {
                outcomes.entry(outcome).or_default().insert(subject);
            }
        }

        // requirements no draft covers
        for basis in self.bases.iter().filter(|basis| !seen.contains(basis.subject)) {
            review.note(format_args!("requirement `{}` is not drafted", basis.subject));
        }

        // one outcome across requirements: two behaviours can share one, so
        // it is noted for the log, not held against the draft
        for (then, subjects) in outcomes.iter().filter(|(_, subjects)| subjects.len() > 1) {
            tracing::debug!(
                then = then.as_str(),
                requirements = subjects.len(),
                "one `then` repeats across requirements"
            );
        }
    }

    fn into_output(self, answer: SpecAnswer) -> Result<Self::Output, Error> {
        let mut drafts: BTreeMap<String, Vec<Scenario>> = answer
            .requirements
            .into_iter()
            .map(|draft| {
                (draft.subject, draft.scenarios.into_iter().map(Scenario::trimmed).collect())
            })
            .collect();
        let mut requirements = Vec::with_capacity(self.bases.len());
        for basis in self.bases {
            let scenarios = drafts.remove(basis.subject).ok_or_else(|| {
                server_error!("requirement `{}` was accepted without a draft", basis.subject)
            })?;
            requirements.push(basis.requirement(scenarios));
        }

        Ok((answer.preamble, requirements))
    }
}

// One draft against its requirement. Returns each evidenced `then` outcome
// its scenarios state, normalised, for the check across requirements.
fn verify_draft(basis: &Basis<'_>, draft: &Draft, review: &mut Review) -> Vec<String> {
    let label = format!("`{}`", draft.subject);
    if draft.scenarios.is_empty() {
        review.note(format_args!("{label} has no scenario"));
    }

    // the requirement's own statements, which no scenario line may restate
    let statements: BTreeSet<String> =
        basis.classes.iter().flatten().map(|member| normalised(&member.statement)).collect();

    let mut outcomes = Vec::new();
    for scenario in &draft.scenarios {
        review.line(&scenario.name, format_args!("{label} scenario `name`"));
        for (field, text) in scenario.lines() {
            review.line(text, format_args!("{label} scenario `{field}`"));
        }

        let when = normalised(&scenario.when);
        if statements.contains(&when) {
            review.note(format_args!(
                "{label} scenario `when` restates the requirement; state the trigger"
            ));
        }

        let then = normalised(&scenario.then);
        if statements.contains(&then) {
            review.note(format_args!(
                "{label} scenario `then` restates the requirement; state the outcome the \
                 scenario observes"
            ));
        }
        if then == UNKNOWN && basis.covered {
            review.note(format_args!(
                "{label} scenario `then` is `{UNKNOWN}` but the requirement is covered; state \
                 the evidenced outcome"
            ));
        }
        if !then.is_empty() && then != UNKNOWN {
            outcomes.push(then);
        }
    }
    outcomes
}

// Whitespace collapsed, trailing punctuation dropped, lowercased: the shape
// two lines are compared in.
fn normalised(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(['.', '!', '?', ';', ':', ','])
        .to_lowercase()
}

impl Display for SpecBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.preamble {
            write!(f, "Draft `spec.md`.\n\n{claims}", claims = ClaimsSection(self.extracts))?;
        } else {
            write!(
                f,
                "Draft `spec.md`: the requirements below, one chunk of the specification's. \
                 The preamble is drafted with the first chunk, so leave it empty.\n\n{claims}",
                claims = ClaimsSection(self.extracts)
            )?;
        }

        write!(
            f,
            "\n## Requirements (draft one entry per subject)\n\n{bases}",
            bases = BasesSection(&self.bases)
        )
    }
}

/// Model-authored content for a specification.
///
/// The engine supplies headings, provenance, requirement bodies, and notes.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery spec draft")]
pub struct SpecAnswer {
    /// Markdown paragraphs before the first requirement.
    pub preamble: Vec<String>,
    /// One draft per requirement subject, in any order.
    pub requirements: Vec<Draft>,
}

/// Drafted acceptance scenarios for one requirement.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    /// The requirement subject exactly as supplied by the engine.
    #[schemars(regex(pattern = CLAIM_ID_REGEX))]
    pub subject: String,
    /// At least one scenario.
    pub scenarios: Vec<Scenario>,
}
