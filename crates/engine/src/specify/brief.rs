//! Defines the shared workflow for typed synthesis requests.
//!
//! A [`Brief`] combines engine facts, prompt documents, an answer schema, and
//! validation. Only a response that passes both deserialisation and
//! fact-based checks can produce engine output.
//!
//! Synthesis may use briefs for claim grouping, specification drafting, and
//! design drafting. Facts already known to the engine are validated or
//! inserted directly rather than requested from the model.

use std::fmt::{self, Display, Formatter};

use omnia_sdk::model::{Findings, Question};
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::revision::RESERVED;
use crate::specify::{Extract, PROSE};

// `Sync`: the verify closure `Question::ask` takes is `Send`, and it
// borrows the brief.
pub trait Brief: Display + Sync + Sized {
    /// The typed answer the brief asks for.
    type Answer: JsonSchema + DeserializeOwned + Send;

    /// The engine value produced from an accepted answer.
    type Output;

    /// The stable name of the model request.
    const NAME: &'static str;

    /// Paths of prompt documents, in assembly order.
    const PROSE: &'static [&'static str];

    /// Restricts the derived `schema` using facts from this run.
    ///
    /// Schema changes guide generation; [`Self::verify`] remains the
    /// authoritative check.
    fn tighten(&self, schema: &mut Value);

    /// Validates a candidate answer against facts from this run.
    ///
    /// Every violation is recorded in `review` for a possible correction
    /// round.
    fn verify(&self, answer: &Self::Answer, review: &mut Review);

    /// Converts an accepted answer into engine output.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the accepted answer cannot be
    /// reconciled with the brief's facts.
    fn into_output(self, answer: Self::Answer) -> Result<Self::Output, Error>;

    /// Submits the brief to `model` and returns validated engine output.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when no valid answer is produced within
    ///   the available rounds.
    /// - Returns [`Error::ServerError`] when a required prompt document is not
    ///   embedded or an accepted answer cannot be converted.
    /// - Returns [`Error::BadGateway`] when the model operation fails.
    async fn judge<M: Model>(self, model: &M) -> Result<Self::Output, Error> {
        tracing::info!(question = Self::NAME, "asking the model");
        let mut system = Vec::with_capacity(Self::PROSE.len());
        for path in Self::PROSE {
            let prose = emery_prose::body(PROSE, path)
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
        tracing::debug!(question = Self::NAME, "answered");

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
// extract, under its source key and kind, so the model sees the whole body it
// must draft from.
pub struct ClaimsSection<'a>(pub &'a [Extract]);

impl Display for ClaimsSection<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("## Claims\n")?;

        for extract in self.0 {
            write!(
                f,
                "\n### source `{key}` ({kind})\n\n",
                key = extract.source,
                kind = extract.kind
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
