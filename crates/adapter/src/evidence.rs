//! The evidence call
//!
//! The one model call an adapter makes: [`evidence`] asks the extract
//! question as an [`Evidence`]-typed [`Question`] and returns the accepted
//! document. [`EvidenceTurn`] owns the user-turn envelope so an adapter only
//! chooses its source kind and supplies either the bound material or a
//! prepared material note.
//!
//! The schema steers the answer's shape but cannot express every rule a
//! claim must satisfy, so each candidate the backend proposes is run through
//! the contract's claim gate before it is accepted; a miss goes back to the
//! model as findings and the backend asks again. The engine re-runs the same
//! gate on receipt, but an adapter that checks in place rarely hands it
//! evidence to reject.

use omnia_guest::model::Question;
use omnia_guest::{Error, Model};

use crate::references;
use crate::types::{Context, Evidence, SourceContent, SourceInput};

/// The source-specific material inside the SDK-owned evidence user turn.
#[derive(Debug)]
pub struct EvidenceTurn<'a> {
    source: &'a str,
    material: Material<'a>,
}

#[derive(Debug)]
enum Material<'a> {
    Bound(&'a str),
    Prepared(String),
}

impl<'a> EvidenceTurn<'a> {
    /// Uses the [`SourceInput`] directly; `tree` names workspace contents.
    #[must_use]
    pub const fn bound(source: &'a str, tree: &'a str) -> Self {
        Self {
            source,
            material: Material::Bound(tree),
        }
    }

    /// Uses an adapter-prepared note for a source needing custom handling.
    #[must_use]
    pub fn prepared(source: &'a str, note: impl Into<String>) -> Self {
        Self {
            source,
            material: Material::Prepared(note.into()),
        }
    }

    fn render(self, ctx: &Context<'_>, input: &SourceInput) -> String {
        let material = match self.material {
            Material::Bound(tree) => content_note(input, tree),
            Material::Prepared(note) => note,
        };
        let mut notes = vec![
            format!(
                "Extract the claim set of the {source} source bound to adapter `{id}` (source \
                 key `{key}`).",
                source = self.source,
                id = ctx.adapter_id,
                key = input.key,
            ),
            material,
        ];
        if !ctx.docs.is_empty() {
            notes.push(
                "The prompt's references are available through this call's `read_doc` tool \
                 (`list_docs` enumerates them); load referenced bodies on demand."
                    .to_string(),
            );
        }
        notes.push(
            "Answer with one JSON object matching the gated Evidence schema. The caller persists \
             the document; do not write it yourself."
                .to_string(),
        );
        notes.join("\n\n")
    }
}

/// Asks the extract question and returns the accepted [`Evidence`].
///
/// # Errors
///
/// A request the host refuses, or the last gate findings once the backend's
/// rounds are spent, is `BadRequest`; a tool or transport failure is
/// `BadGateway`.
pub async fn evidence<P: Model>(
    model: &P, ctx: &Context<'_>, input: &SourceInput, system: impl Into<String>,
    turn: EvidenceTurn<'_>,
) -> Result<Evidence, Error> {
    let user = turn.render(ctx, input);
    let mut question = Question::<Evidence>::new("evidence").system(system);
    // The tools are declared exactly when there is a corpus to answer them.
    let answering = references::answering(ctx.docs);
    if answering.is_some() {
        question = question.tools(references::tools());
    }
    if let Some(lend) = ctx.lend {
        question = question.workspace(lend);
    }

    let evidence = question
        .ask(model, user, answering, |evidence| {
            let findings = evidence.findings();
            if findings.is_empty() { Ok(()) } else { Err(findings) }
        })
        .await?;
    Ok(evidence)
}

/// Describes the bound source to the model; `tree` names what a workspace
/// holds (for example `the documentation tree`).
#[must_use]
pub fn content_note(input: &SourceInput, tree: &str) -> String {
    match &input.content {
        SourceContent::Workspace(root) => format!(
            "`$SOURCE_DIR` is the read-only view at `{root}` — {tree} the prompt walks. \
             Nothing outside it is reachable; extract mines only this source."
        ),
        SourceContent::Value(value) => format!(
            "The bound material is this inline value; no `$SOURCE_DIR` is lent:\n\n{value}\n\n\
             Nothing else is reachable; extract mines only this source."
        ),
    }
}
