//! Defines the shared workflow for typed synthesis requests.
//!
//! A [`Brief`] combines engine facts, prompt documents, an answer schema, and
//! validation. Only a response that passes both deserialisation and
//! fact-based checks can produce engine output.
//!
//! Synthesis may use briefs for claim grouping, specification drafting,
//! design drafting, and slicing. Facts already known to the engine are
//! validated or inserted directly rather than requested from the model.

use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::Claim;
use omnia_sdk::model::{Error as ModelError, Findings, Question};
use omnia_sdk::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::revision::{RESERVED, Status};
use crate::specify::basis::Basis;
use crate::specify::{Extract, PROSE};

/// A typed synthesis question and the checks its answer must satisfy.
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
    /// - Returns [`Error::BadGateway`] when the model operation fails twice
    ///   over: a first failure is put once more before it is reported.
    #[tracing::instrument(skip_all, fields(question = Self::NAME))]
    async fn judge<M: Model>(self, model: &M) -> Result<Self::Output, Error> {
        tracing::info!(question = Self::NAME, "asking the model");
        let mut system = Vec::with_capacity(Self::PROSE.len());
        for path in Self::PROSE {
            let prose = emery_prose::body(PROSE, path)
                .ok_or_else(|| server_error!("synthesis prose `{path}` is not embedded"))?;
            system.push(prose);
        }
        let question = Question::<Self::Answer>::new(Self::NAME)
            .system(system.join("\n\n---\n\n"))
            .schema(|schema| self.tighten(schema));
        let check = |answer: &Self::Answer| {
            let mut review = Review::default();
            self.verify(answer, &mut review);
            review.verdict()
        };
        let answer = match question.ask(model, self.to_string(), None, check).await {
            // the transport or a tool failed, not the answer — the failures the
            // run would report as `bad_gateway` — so the question is put once
            // more, fresh, before the run is given up
            Err(failure @ (ModelError::Backend(_) | ModelError::ToolFailed(_))) => {
                tracing::warn!(question = Self::NAME, %failure, "the model failed; asking once more");
                question.ask(model, self.to_string(), None, check).await?
            }
            outcome => outcome?,
        };
        tracing::info!(question = Self::NAME, "answered");

        self.into_output(answer)
    }
}

/// The findings a brief records against one candidate.
#[derive(Default)]
pub struct Review(Findings);

impl Review {
    /// Records one finding.
    ///
    /// Each finding is a bullet: omnia joins them with newlines under
    /// `## Findings`, so the list markup is the engine's.
    pub fn note(&mut self, finding: impl Display) {
        self.0.push(format!("- {finding}"));
    }

    /// Returns whether the candidate is accepted.
    ///
    /// A candidate nothing was found against is accepted. Otherwise the
    /// findings are logged at DEBUG under the brief's `judge` span and
    /// returned for the backend to feed back as the correction.
    pub fn verdict(self) -> Result<(), Findings> {
        if self.0.is_empty() {
            return Ok(());
        }
        tracing::debug!(findings = ?self.0, "candidate rejected");
        Err(self.0)
    }

    /// Checks that a drafted paragraph carries none of the document's own markup.
    ///
    /// A paragraph may not be blank or open a line with a reserved marker,
    /// since the draft is placed into a document the engine renders.
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

    /// Checks every paragraph in `texts` as [`Review::paragraph`] does.
    pub fn paragraphs(&mut self, texts: &[String], label: impl Display) {
        for text in texts {
            self.paragraph(text, &label);
        }
    }

    /// Checks that `text` is one non-blank line.
    pub fn line(&mut self, text: &str, label: impl Display) {
        if text.trim().is_empty() {
            self.note(format_args!("{label} is blank"));
        } else if text.contains('\n') {
            self.note(format_args!("{label} spans more than one line"));
        }
    }
}

/// The requirement outline of a document brief's turn, one entry per basis.
///
/// Each entry carries the engine's facts about the requirement: its id,
/// subject, status, sources, whether a criterion covers it, and every
/// contributing claim with its role. The three drafts run from it together,
/// so none waits on another's answer. The bases are owned or borrowed, so a
/// brief over a chunk of the run's bases lists them as one over all of them
/// does.
pub struct BasesSection<'a, B>(pub &'a [B]);

impl<'a, B: Borrow<Basis<'a>>> Display for BasesSection<'a, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for basis in self.0 {
            let basis: &Basis<'a> = basis.borrow();
            let coverage = if basis.covered {
                "evidenced"
            } else {
                "not evidenced — `then` is the outcome the statements name, else `[unknown]`"
            };
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
                        "  - {role}: {source} ({kind}, rank {rank}, `{claim}`): {statement}",
                        source = member.source,
                        kind = member.kind,
                        rank = member.rank,
                        claim = member.id,
                        statement = member.statement,
                    )?;
                }
            }
        }

        Ok(())
    }
}

/// The `## Claims` section of a document brief's turn.
///
/// Every claim of every extract is listed under its source name and kind.
pub struct ClaimsSection<'a>(pub &'a [Extract]);

impl Display for ClaimsSection<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        claims(f, self.0, |_| true)
    }
}

/// Writes the `## Claims` section over the claims of `extracts` that `keep`
/// admits, each under its source's name and kind; a source none of whose
/// claims is kept is listed with none.
///
/// # Errors
///
/// Returns the formatter's error.
pub fn claims(
    f: &mut Formatter<'_>, extracts: &[Extract], keep: impl Fn(&Claim) -> bool,
) -> fmt::Result {
    f.write_str("## Claims\n")?;

    for extract in extracts {
        write!(
            f,
            "\n### source `{source}` ({kind})\n\n",
            source = extract.source,
            kind = extract.kind
        )?;

        for claim in extract.evidence.claims.iter().filter(|claim| keep(claim)) {
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
