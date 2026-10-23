//! Synthesises the drafted content of `spec.md`.
//!
//! Drafts introductory paragraphs and acceptance scenarios in chunks of at
//! most [`SPEC_CHUNK`] requirements, holding all drafted prose against the
//! run's technology [`Lexicon`]. An amendment drafts the requirements its
//! change adds or revises alone, each whole under the id it keeps, and
//! takes the rest of the specification from the base.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{CLAIM_ID_REGEX, Claim, ClaimKind};
use emery_lexicon::Lexicon;
use omnia_sdk::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::brief::{Brief, Review};
use crate::revision::{self, Change, ChangeClass, EMERY, Requirement, Scenario, Spec};
use crate::specify::basis::{Basis, Contributor};
use crate::specify::sections::{self, BasesSection, ClaimsSection};
use crate::specify::{Extract, shape};

// The outcome a scenario states where no criterion evidences one.
const UNKNOWN: &str = "[unknown]";

// What a chunk opens with: a specify's first chunk drafts the preamble and
// lists every claim, a later chunk drafts no preamble, and an amendment's
// chunk drafts none either, since the base's stands.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lead {
    Preamble,
    Chunk,
    Amendment,
}

/// How many requirements one `spec-draft` turn drafts at most.
///
/// A larger run is drafted in chunks, each stem kept whole where it fits
/// and smaller stems merged up to the cap, so a refused candidate costs one
/// chunk's regeneration rather than the whole draft's.
pub const SPEC_CHUNK: usize = 25;

// A synthesis brief for the drafted portions of `spec.md`.
//
// The brief combines extracted claims with one chunk of their reconciled
// requirement bases.
pub struct SpecBrief<'a> {
    extracts: &'a [Extract],
    bases: Vec<&'a Basis<'a>>,
    lead: Lead,
    lexicon: Lexicon,
}

impl<'a> SpecBrief<'a> {
    // Returns the briefs `bases` are drafted under, in requirement order:
    // one per chunk of at most `SPEC_CHUNK` requirements, grouped by
    // stem — a stem past the cap split, smaller ones merged up to it — the
    // first alone asking for the preamble. Every chunk holds its draft to
    // one `Lexicon`: the fixed names, and the packages the run's `call`
    // claims spell.
    #[must_use]
    pub fn chunked(extracts: &'a [Extract], bases: &'a [Basis<'a>]) -> Vec<Self> {
        Self::cut(extracts, bases, false)
    }

    // Returns the briefs an amendment drafts `bases` under, in requirement order.
    //
    // `bases` are the requirements the change adds, revises, or leaves in
    // conflict, each under the id it keeps, chunked as
    // `chunked` chunks them. No chunk drafts a preamble,
    // since the base's stands, and each is told it amends the current
    // specification. An empty slice yields no brief and spends no turn.
    #[must_use]
    pub fn amending(extracts: &'a [Extract], bases: &'a [Basis<'a>]) -> Vec<Self> {
        Self::cut(extracts, bases, true)
    }

    fn cut(extracts: &'a [Extract], bases: &'a [Basis<'a>], amending: bool) -> Vec<Self> {
        let lexicon = Lexicon::from_packages(packages(extracts));
        let mut stems: Vec<(&str, Vec<&'a Basis<'a>>)> = Vec::new();
        for basis in bases {
            let stem = basis.stem();
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
                lead: match (amending, index) {
                    (true, _) => Lead::Amendment,
                    (false, 0) => Lead::Preamble,
                    (false, _) => Lead::Chunk,
                },
                lexicon: lexicon.clone(),
            })
            .collect()
    }

    // Returns the specification the chunks' accepted answers make together:
    // the preamble the first carried, and every requirement in id order.
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
            parent: None,
            preamble,
            requirements,
        }
    }

    // Returns the specification `base` amended by `changes` and `drafts`.
    //
    // The preamble is the base's. Each unchanged
    // requirement is the base's as it stands, each
    // retired one is left out, and every other is the
    // one `drafts` drafted under its id. The requirements are ordered by id,
    // and `parent` names the base revision.
    #[must_use]
    pub fn amend(
        base: Spec, parent: String, changes: &[Change],
        drafts: Vec<(Vec<String>, Vec<Requirement>)>,
    ) -> Spec {
        let kept: BTreeSet<_> = changes
            .iter()
            .filter(|change| change.class == ChangeClass::Unchanged)
            .map(|change| change.id)
            .collect();
        let mut requirements: Vec<Requirement> = base
            .requirements
            .into_iter()
            .filter(|requirement| kept.contains(&requirement.id))
            .chain(drafts.into_iter().flat_map(|(_, drafted)| drafted))
            .collect();
        requirements.sort_by_key(|requirement| requirement.id);

        Spec {
            emery: EMERY,
            parent: Some(parent),
            preamble: base.preamble,
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

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        let count = self.bases.len();
        let requirements = Self::at(schema, "/properties/requirements")?;
        requirements.insert("minItems".to_owned(), json!(count));
        requirements.insert("maxItems".to_owned(), json!(count));
        let subjects = json!(self.bases.iter().map(|basis| &basis.subject).collect::<Vec<_>>());
        Self::at(schema, "/$defs/Draft/properties/subject")?.insert("enum".to_owned(), subjects);
        Self::at(schema, "/$defs/Draft/properties/scenarios")?
            .insert("minItems".to_owned(), json!(1));
        if self.lead != Lead::Preamble {
            Self::at(schema, "/properties/preamble")?.insert("maxItems".to_owned(), json!(0));
        }
        Ok(())
    }

    fn verify(&self, answer: &SpecAnswer, review: &mut Review) {
        match self.lead {
            Lead::Preamble => {
                review.paragraphs(&answer.preamble, "preamble");
                for text in &answer.preamble {
                    review.technology(&self.lexicon, text, "preamble");
                }
            }
            Lead::Chunk if !answer.preamble.is_empty() => {
                review.note(
                    "the preamble is drafted with the first requirements; leave it empty here",
                );
            }
            Lead::Amendment if !answer.preamble.is_empty() => {
                review.note("the preamble is the current specification's; leave it empty");
            }
            Lead::Chunk | Lead::Amendment => {}
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
            for outcome in draft.verify(basis, &self.lexicon, review) {
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
                let scenarios = draft.scenarios.into_iter().map(Scenario::trimmed).collect();
                (draft.subject, scenarios)
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

// The packages the run's `call` claims spell: the part of each `callee`
// before its first `:`. A bare global (`fetch`) spells none, and a wrapper
// of the tree (`src/lib/http.ts:HttpClient.post`) spells a path the lexicon
// reads as a plain word and keeps nothing of.
fn packages(extracts: &[Extract]) -> impl Iterator<Item = &str> {
    extracts
        .iter()
        .flat_map(|extract| &extract.evidence.claims)
        .filter(|claim| claim.kind == ClaimKind::Call)
        .filter_map(Claim::callee)
        .filter_map(|callee| callee.split_once(':').map(|(package, _)| package))
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

// The claims one chunk past the first is drafted from: every claim whose id
// leads with one of the chunk's stems — its requirements, the criteria and
// examples under them — and every unidentified claim anchored in a file one
// of those requirement claims is anchored in, the detail beside them. The
// first chunk carries the preamble, which describes the whole, so it lists
// every claim instead.
struct Scope<'a> {
    stems: BTreeSet<&'a str>,
    files: BTreeSet<&'a str>,
}

impl<'a> Scope<'a> {
    fn of(extracts: &'a [Extract], bases: &[&Basis<'a>]) -> Self {
        let stems: BTreeSet<&str> = bases
            .iter()
            .flat_map(|basis| {
                std::iter::once(basis.stem())
                    .chain(basis.classes.iter().flatten().map(Contributor::stem))
            })
            .collect();
        let files = extracts
            .iter()
            .flat_map(|extract| &extract.evidence.claims)
            .filter(|claim| claim.kind == ClaimKind::Requirement)
            .filter(|claim| {
                claim.id.as_deref().is_some_and(|id| stems.contains(revision::stem(id)))
            })
            .filter_map(|claim| claim.path.as_deref())
            .map(shape::file)
            .collect();
        Self { stems, files }
    }

    fn keeps(&self, claim: &Claim) -> bool {
        claim.id.as_deref().map_or_else(
            || claim.path.as_deref().is_none_or(|path| self.files.contains(shape::file(path))),
            |id| self.stems.contains(revision::stem(id)),
        )
    }
}

impl Display for SpecBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.lead {
            Lead::Preamble => {
                write!(f, "Draft `spec.md`.\n\n{claims}", claims = ClaimsSection(self.extracts))?;
            }
            Lead::Chunk => {
                f.write_str(
                    "Draft `spec.md`: the requirements below, one chunk of the specification's. \
                     The preamble is drafted with the first chunk, so leave it empty. The claims \
                     listed are the ones under this chunk's stems and the detail anchored beside \
                     them; the source's other claims are other chunks' to draft from.\n\n",
                )?;
                let scope = Scope::of(self.extracts, &self.bases);
                sections::claims(f, self.extracts, |claim| scope.keeps(claim))?;
            }
            Lead::Amendment => {
                f.write_str(
                    "Draft `spec.md`: the requirements below, which amend the current \
                     specification. Each is one the change sources add to it or revise in it, \
                     drafted whole, scenarios included; the specification's other requirements \
                     stand as they are and are not listed. The preamble is the current \
                     specification's and is kept, so leave it empty. The claims listed are the \
                     ones under this chunk's stems and the detail anchored beside them; the \
                     sources' other claims are other chunks' to draft from, or restate \
                     requirements that stand.\n\n",
                )?;
                let scope = Scope::of(self.extracts, &self.bases);
                sections::claims(f, self.extracts, |claim| scope.keeps(claim))?;
            }
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

impl Draft {
    // Returns each evidenced `then` outcome the scenarios state, normalised,
    // for the check across requirements.
    fn verify(&self, basis: &Basis<'_>, lexicon: &Lexicon, review: &mut Review) -> Vec<String> {
        let label = format!("`{}`", self.subject);
        if self.scenarios.is_empty() {
            review.note(format_args!("{label} has no scenario"));
        }

        // the requirement's own statements, which no scenario line may restate
        let statements: BTreeSet<String> =
            basis.classes.iter().flatten().map(|member| normalised(&member.statement)).collect();

        let mut outcomes = Vec::new();
        for scenario in &self.scenarios {
            review.line(&scenario.name, format_args!("{label} scenario `name`"));
            review.technology(lexicon, &scenario.name, format_args!("{label} scenario `name`"));
            for (field, text) in scenario.lines() {
                review.line(text, format_args!("{label} scenario `{field}`"));
                review.technology(lexicon, text, format_args!("{label} scenario `{field}`"));
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
                    "{label} scenario `then` is `{UNKNOWN}` but the requirement is covered; \
                     state the evidenced outcome"
                ));
            }
            if !then.is_empty() && then != UNKNOWN {
                outcomes.push(then);
            }
        }
        outcomes
    }
}
