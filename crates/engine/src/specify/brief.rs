//! Briefs
//!
//! One judgment put to the model: what the engine asks, how it steers the
//! answer, and what it accepts. A brief carries a run's facts, names the
//! prose that instructs the model, renders the turn, tightens the answer's
//! derived schema to the run, and verifies every candidate against the facts;
//! only an answer it accepts becomes output — requirements or a document — and
//! the brief alone produces that output.
//!
//! A run puts up to three briefs in turn — how the requirement claims group
//! (on a run over two or more sources), the drafted content of `spec.md`,
//! then of `design.md` — and places each accepted answer beside the engine's
//! facts in the revision. Nothing the engine already knows is asked of the
//! model: it never writes a heading, an id, a `Sources:` list, a status, a
//! note, or a type signature, so it cannot drop, reorder, or quietly rewrite
//! a requirement, invent or omit a section, cite an unbound source, or
//! paraphrase a signature. The stored revision is a function of the facts
//! and the accepted drafts alone.
//!
//! This module carries what every brief shares: the trait, the [`Review`]
//! each verification records on, and the [`ClaimsSection`] of the prompt the
//! document briefs open with.

use std::fmt::{self, Display, Formatter};

use omnia_guest::model::{Findings, Question};
use omnia_guest::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::revision::RESERVED;
use crate::specify::Extract;

// `Sync`: the verify closure `Question::ask` takes is `Send`, and it
// borrows the brief.
pub trait Brief: Display + Sync + Sized {
    /// The typed answer the brief asks for.
    type Answer: JsonSchema + DeserializeOwned + Send;

    /// What the judgment yields: requirements, a document.
    type Output;

    /// The question's name.
    const NAME: &'static str;

    /// The synthesis prose, in prompt order.
    const PROSE: &'static [&'static str];

    /// Tightens the derived `schema` toward this run. Steering for the
    /// provider; [`Self::verify`] is the gate.
    fn tighten(&self, schema: &mut Value);

    /// Verifies a candidate answer against the run's facts, recording every
    /// finding on `review` for repair.
    fn verify(&self, answer: &Self::Answer, review: &mut Review);

    /// Transforms the answer into output specific to the brief.
    ///
    /// The answer passed [`Self::verify`], so a fact it names that the brief
    /// cannot place is the engine's own defect: `server_error`.
    fn into_output(self, answer: Self::Answer) -> Result<Self::Output, Error>;

    /// Puts the brief to `model` and turns the answer its verification
    /// accepted into this brief's output.
    ///
    /// # Errors
    ///
    /// A model failure is `bad_gateway`; a candidate outside the answer's
    /// shape or the backend's spent rounds is `bad_request`; synthesis prose
    /// the build did not embed is `server_error`.
    async fn judge<M: Model>(self, model: &M) -> Result<Self::Output, Error> {
        tracing::info!(question = Self::NAME, "asking the model");
        let mut system = Vec::with_capacity(Self::PROSE.len());
        for path in Self::PROSE {
            let prose = crate::prose::body(path)
                .ok_or_else(|| server_error!("synthesis prose `{path}` is not embedded"))?;
            system.push(prose);
        }
        let answer = Question::<Self::Answer>::new(Self::NAME)
            .system(system.join("\n\n---\n\n"))
            .schema(|schema| self.tighten(schema))
            .ask(model, self.to_string(), None, |answer| {
                let mut review = Review::default();
                self.verify(answer, &mut review);
                review.verdict()
            })
            .await?;

        self.into_output(answer)
    }
}

// What a brief records against one candidate. One type, so the bullet every
// finding carries and the accept-or-reject verdict are decided here rather
// than by each brief.
#[derive(Default)]
pub struct Review(Findings);

impl Review {
    // Records one finding as a bullet: omnia joins the findings with newlines
    // under `## Findings`, so the list markup is the engine's.
    pub fn note(&mut self, finding: impl Display) {
        self.0.push(format!("- {finding}"));
    }

    // Accepts a candidate nothing was found against; rejects one with the
    // findings the backend feeds back as the correction.
    pub fn verdict(self) -> Result<(), Findings> {
        if self.0.is_empty() { Ok(()) } else { Err(self.0) }
    }

    // The prose checks the document briefs share. A draft is placed into a
    // document the engine renders, so it may not carry the document's own
    // markup: a paragraph may not be blank or open a line with a reserved
    // marker.
    pub fn paragraph(&mut self, text: &str, label: impl Display) {
        if text.trim().is_empty() {
            self.note(format_args!("{label} has a blank paragraph"));
            return;
        }

        for line in text.lines() {
            let line = line.trim_start();
            if let Some(marker) = RESERVED.iter().copied().find(|marker| line.starts_with(marker)) {
                self.note(format_args!(
                    "{label}: a paragraph line opens with the reserved marker `{marker}`"
                ));
            }
        }
    }

    pub fn paragraphs(&mut self, texts: &[String], label: impl Display) {
        for text in texts {
            self.paragraph(text, &label);
        }
    }

    // A scenario field is one non-blank line.
    pub fn line(&mut self, text: &str, label: impl Display) {
        if text.trim().is_empty() {
            self.note(format_args!("{label} is blank"));
        } else if text.contains('\n') {
            self.note(format_args!("{label} spans more than one line"));
        }
    }
}

// The `## Claims` section of a document brief's turn: every claim in every
// extract, under its source key and authority, so the model sees the whole
// body it must draft from.
pub struct ClaimsSection<'a>(pub &'a [Extract]);

impl Display for ClaimsSection<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("## Claims\n")?;

        for extract in self.0 {
            write!(
                f,
                "\n### source `{key}` ({authority})\n\n",
                key = extract.key,
                authority = extract.evidence.authority
            )?;

            for claim in &extract.evidence.claims {
                let id = claim.id.as_deref().unwrap_or("-");
                let synopsis = claim.synopsis.as_deref().unwrap_or("");
                writeln!(
                    f,
                    "- {kind} `{id}` — {synopsis} — {extras}",
                    kind = claim.kind,
                    extras = Value::Object(claim.extras.clone()),
                )?;
            }
        }

        Ok(())
    }
}
