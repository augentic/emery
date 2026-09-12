//! Briefs
//!
//! One judgment put to the model: what the engine asks, how it steers the
//! answer, and what it accepts. A brief carries a run's facts, names the
//! prose that instructs the model, renders the turn, tightens the answer's
//! derived schema to the run, and verifies every candidate against the facts;
//! only an answer it accepts becomes output — requirements or a document — and
//! the brief alone produces that output.
//!
//! The model is never asked for anything the engine can decide itself, and
//! nothing the engine renders comes from an unchecked answer.

use std::fmt::Display;

use omnia_guest::model::{Findings, Question};
use omnia_guest::{Error, Model};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::revision::RESERVED;

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
    fn into_output(self, answer: Self::Answer) -> Self::Output;

    /// Puts the brief to `model` and turns the answer its verification
    /// accepted into this brief's output.
    ///
    /// # Errors
    ///
    /// A model failure is `bad_gateway`; a candidate outside the answer's
    /// shape or the backend's spent rounds is `bad_request`.
    async fn judge<M: Model>(self, model: &M) -> Result<Self::Output, Error> {
        tracing::info!(question = Self::NAME, "asking the model");
        let system = Self::PROSE
            .iter()
            .map(|path| crate::prose::body(path))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        let answer = Question::<Self::Answer>::new(Self::NAME)
            .system(system)
            .schema(|schema| self.tighten(schema))
            .ask(model, self.to_string(), None, |answer| {
                let mut review = Review::default();
                self.verify(answer, &mut review);
                review.verdict()
            })
            .await?;

        Ok(self.into_output(answer))
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
