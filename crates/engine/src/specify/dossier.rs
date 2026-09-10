//! The dossier
//!
//! Synthesises the requirements and the extracted claims into the dossier: the
//! two specification documents. The model is asked two typed questions in turn —
//! the content of `spec.md`, then the content of `design.md` — each put as a
//! brief that verifies every candidate answer against the requirements, the
//! section plan, and the evidence before the engine renders the accepted answer
//! into the canonical document. Every heading, provenance line, tag, note, and
//! signature is the engine's, so the stored bytes are a function of the facts
//! and the draft alone, and a changed byte re-ids every revision.
//!
//! Nothing the engine already knows is asked of the model: it never writes a
//! heading, an id, a `Sources:` list, a status, a note, or a type signature,
//! so it cannot drop, reorder, or quietly rewrite a requirement, invent or
//! omit a section, cite an unbound source, or paraphrase a signature.

mod design;
mod spec;

use std::fmt::{self, Display, Formatter};

use omnia_guest::{Error, Model};
use serde_json::Value;

use self::design::DesignBrief;
use self::spec::SpecBrief;
use crate::artifact::{Dossier, RESERVED};
use crate::specify::Extract;
use crate::specify::brief::{Brief as _, Review};
use crate::specify::requirement::Requirement;

/// Takes the extracts of every source and the requirements derived from them,
/// and asks the model to synthesise them into the dossier: the specification
/// and design documents.
///
/// # Errors
///
/// A model failure is `bad_gateway`; an answer outside the schema, or a draft
/// the backend could not repair within its rounds, is `bad_request`.
pub async fn synthesise<M: Model>(
    model: &M, extracts: &[Extract], requirements: &[Requirement],
) -> Result<Dossier, Error> {
    let spec = SpecBrief::new(extracts, requirements).judge(model).await?;
    let design = DesignBrief::new(extracts, &spec).judge(model).await?;

    Ok(Dossier { spec, design })
}

// A document under construction: the blocks the renderer emits in order,
// joined by one blank line, every line right-trimmed, one trailing newline —
// the shape `artifact::Text` reads back.
struct Markdown(Vec<String>);

impl Markdown {
    fn new(title: &str) -> Self {
        Self(vec![format!("# {title}")])
    }

    // Adds one block, every line right-trimmed; `finish` joins the blocks.
    fn push(&mut self, text: impl Into<String>) {
        self.0.push(text.into().lines().map(str::trim_end).collect::<Vec<_>>().join("\n"));
    }

    fn extend(&mut self, texts: &[String]) {
        for text in texts {
            self.push(text);
        }
    }

    fn finish(self) -> String {
        let mut text = self.0.join("\n\n");
        text.push('\n');
        text
    }
}

// The `## Claims` section of a brief's prompt: every claim in every extract,
// under its source key and authority, so the model sees the whole body it
// must draft from.
struct ClaimsSection<'a>(&'a [Extract]);

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

// The prose checks both document briefs run: synthesis is where a draft is
// placed into a document, so a draft may not carry the document's own markup.
impl Review {
    // A drafted paragraph may not be blank or open a line with a reserved
    // marker.
    fn paragraph(&mut self, text: &str, label: impl Display) {
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

    fn paragraphs(&mut self, texts: &[String], label: impl Display) {
        for text in texts {
            self.paragraph(text, &label);
        }
    }

    // A scenario field is one non-blank line.
    fn line(&mut self, text: &str, label: impl Display) {
        if text.trim().is_empty() {
            self.note(format_args!("{label} is blank"));
        } else if text.contains('\n') {
            self.note(format_args!("{label} spans more than one line"));
        }
    }
}
