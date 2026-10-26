//! Defines the shared workflow for typed model questions.
//!
//! A [`Brief`] combines engine facts, prompt documents, an answer schema, and
//! fact-based validation to produce verified engine output.

use std::borrow::Cow;
use std::fmt::Display;

use emery_lexicon::Lexicon;
use omnia_sdk::model::{Findings, Question};
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::revision::RESERVED;
use crate::{RETRIES, is_retryable};

// A typed question to the model and the checks its answer must satisfy.
// `Sync`: the verify closure `Question::ask` takes is `Send`, and it
// borrows the brief.
pub trait Brief: Display + Sync + Sized {
    // The typed answer the brief asks for.
    type Answer: JsonSchema + DeserializeOwned + Send;

    // The engine value produced from an accepted answer.
    type Output;

    // The stable name of the model request.
    const NAME: &'static str;

    // Paths of prompt documents, in assembly order.
    const PROSE: &'static [&'static str];

    // The name this question is put under.
    //
    // `Self::NAME` unless a run puts the brief more than once, when each
    // question carries what tells it apart.
    fn label(&self) -> Cow<'static, str> {
        Cow::Borrowed(Self::NAME)
    }

    // Restricts the derived `schema` using facts from this run.
    //
    // Schema changes guide generation; `Self::verify` remains the
    // authoritative check.
    //
    // Returns `Error::ServerError` when the derived schema lacks a node the
    // brief steers, since the shape is the answer type's own.
    fn tighten(&self, schema: &mut Value) -> Result<(), Error>;

    // Returns the object of `schema` at `pointer`, a JSON pointer.
    //
    // Returns `Error::ServerError` when nothing stands at `pointer`, or
    // what stands there is no object.
    fn at<'s>(schema: &'s mut Value, pointer: &str) -> Result<&'s mut Map<String, Value>, Error> {
        schema
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| server_error!("the `{}` answer schema has no `{pointer}`", Self::NAME))
    }

    // Validates a candidate answer against facts from this run.
    //
    // Every violation is recorded in `review` for a possible correction
    // round.
    fn verify(&self, answer: &Self::Answer, review: &mut Review);

    // Converts an accepted answer into engine output.
    //
    // Returns `Error::ServerError` when the accepted answer cannot be
    // reconciled with the brief's facts.
    fn into_output(self, answer: Self::Answer) -> Result<Self::Output, Error>;

    // Submits the brief to `model` and returns validated engine output.
    //
    // - Returns `Error::BadRequest` when no valid answer is produced within
    //   the available rounds, or the backend's time budget ends the
    //   question twice over.
    // - Returns `Error::ServerError` when a required prompt document is not
    //   embedded or an accepted answer cannot be converted.
    // - Returns `Error::BadGateway` when the model operation fails twice
    //   over: a first failure is put once more before it is reported.
    #[tracing::instrument(skip_all, fields(question = %self.label()))]
    async fn judge<M: Model>(self, model: &M) -> Result<Self::Output, Error> {
        tracing::info!("asking the model");
        let mut system = Vec::with_capacity(Self::PROSE.len());
        for path in Self::PROSE {
            let prose = emery_prose::body(PROSE, path)
                .ok_or_else(|| server_error!("synthesis prose `{path}` is not embedded"))?;
            system.push(prose);
        }
        let mut tightened = Ok(());
        let question = Question::<Self::Answer>::new(&self.label())
            .system(system.join("\n\n---\n\n"))
            .schema(|schema| tightened = self.tighten(schema));
        tightened?;
        let check = |answer: &Self::Answer| {
            let mut review = Review::default();
            self.verify(answer, &mut review);
            review.verdict()
        };

        // a failure of the transport or the backend's time, not of the
        // answer, is put once more, fresh, before the run is given up
        let turn = self.to_string();
        let mut attempt = 0;
        let answer = loop {
            attempt += 1;
            match question.ask(model, &turn, None, check).await {
                Ok(answer) => break answer,
                Err(failure) => {
                    let error = Error::from(failure);
                    if attempt < RETRIES && is_retryable(&error) {
                        tracing::warn!(
                            attempt,
                            description = %error.description(),
                            "the model failed upstream or timed out; asking once more"
                        );
                        continue;
                    }
                    return Err(error);
                }
            }
        };
        tracing::info!("answered");

        self.into_output(answer)
    }
}

// The findings a brief records against one candidate.
#[derive(Default)]
pub struct Review(Findings);

impl Review {
    // Records one finding.
    //
    // Each finding is a bullet: omnia joins them with newlines under
    // `## Findings`, so the list markup is the engine's.
    pub fn note(&mut self, finding: impl Display) {
        self.0.push(format!("- {finding}"));
    }

    // Returns whether the candidate is accepted.
    //
    // A candidate nothing was found against is accepted. Otherwise the
    // findings are logged at DEBUG under the brief's `judge` span and
    // returned for the backend to feed back as the correction.
    pub fn verdict(self) -> Result<(), Findings> {
        if self.0.is_empty() {
            return Ok(());
        }
        tracing::debug!(findings = ?self.0, "candidate rejected");
        Err(self.0)
    }

    // Checks that a drafted paragraph carries none of the document's own markup.
    //
    // A paragraph may not be blank or open a line with a reserved marker,
    // since the draft is placed into a document the engine renders.
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

    // Checks every paragraph in `texts` as `Review::paragraph` does.
    pub fn paragraphs(&mut self, texts: &[String], label: impl Display) {
        for text in texts {
            self.paragraph(text, &label);
        }
    }

    // Checks that `text` is one non-blank line.
    pub fn line(&mut self, text: &str, label: impl Display) {
        if text.trim().is_empty() {
            self.note(format_args!("{label} is blank"));
        } else if text.contains('\n') {
            self.note(format_args!("{label} spans more than one line"));
        }
    }

    // Checks that `text` names no technology of `lexicon`.
    //
    // A specification states what the system reaches by its role; the
    // product, protocol, library, framework, or host behind it is the
    // design's. A text naming one is a finding listing the names.
    pub fn technology(&mut self, lexicon: &Lexicon, text: &str, label: impl Display) {
        let named = lexicon.names(text);
        if named.is_empty() {
            return;
        }
        let quoted = named.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ");
        self.note(format_args!(
            "{label} names the technology {quoted}; name what the system reaches by its role, \
             since the design holds the product"
        ));
    }
}

static PROSE: &[emery_prose::Doc] = emery_prose::prose![
    "../prose/authority.md",
    "../prose/claim-landing.md",
    "../prose/design-format.md",
    "../prose/grouping.md",
    "../prose/merge.md",
    "../prose/requirement-block.md",
    "../prose/slicing.md",
    "../prose/spec-format.md",
    "../prose/synthesise.md",
    "../prose/tags.md",
];

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Brief as _;
    use crate::build::merge::MergeBrief;
    use crate::specify::{DesignAmend, DesignBrief, GroupingBrief, SliceBrief, SpecBrief};

    #[test]
    fn corpus() {
        let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("prose");
        let prompts = [
            GroupingBrief::PROSE,
            SpecBrief::PROSE,
            DesignBrief::PROSE,
            DesignAmend::PROSE,
            SliceBrief::PROSE,
            MergeBrief::PROSE,
        ]
        .concat();
        let findings = emery_prose::check(super::PROSE, &tree, &prompts, &[]);
        assert!(findings.is_empty(), "{}", findings.join("\n"));
    }
}
