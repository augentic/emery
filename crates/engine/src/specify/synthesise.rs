//! Synthesis
//!
//! Turns the requirement rows and the extracted claims into the two
//! specification documents. The model is asked two typed questions in turn —
//! the content of `spec.md`, then the content of `design.md` — each put as a
//! brief that verifies every candidate answer against the rows, the section
//! plan, and the evidence before the engine renders the accepted answer into
//! the canonical document. Every heading, provenance line, tag, note, and
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
use crate::artifact::RESERVED;
use crate::specify::SourceEvidence;
use crate::specify::brief::{Brief as _, Review};
use crate::specify::provenance::Provenance;
use crate::store::Revision;

/// Takes evidence from all queried sources, with the requirement rows derived
/// from it, and asks the model to synthesise them into a single specification
/// set containing both the specification and design documents.
///
/// # Errors
///
/// A model failure is `bad_gateway`; an answer outside the schema, or a draft
/// the backend could not repair within its rounds, is `bad_request`.
pub async fn synthesise<M: Model>(
    model: &M, sources: &[SourceEvidence], rows: &[Provenance],
) -> Result<Revision, Error> {
    let spec = SpecBrief::new(sources, rows).judge(model).await?;
    let design = DesignBrief::new(&spec, sources).judge(model).await?;
    Ok(Revision { spec, design })
}

// A document under construction: the blocks the renderer emits in order,
// joined by one blank line, every line right-trimmed, one trailing newline —
// the shape `artifact::Text` reads back.
struct Rendering(Vec<String>);

impl Rendering {
    fn new(title: &str) -> Self {
        Self(vec![format!("# {title}")])
    }

    // An engine block: a heading, a provenance run, a note, a fence.
    fn push(&mut self, block: String) {
        self.0.push(block);
    }

    // A drafted paragraph, placed as the model wrote it but for blank edges.
    fn paragraph(&mut self, text: &str) {
        self.0.push(text.trim().to_string());
    }

    fn paragraphs(&mut self, texts: &[String]) {
        for text in texts {
            self.paragraph(text);
        }
    }

    fn finish(self) -> String {
        let mut text = self
            .0
            .iter()
            .map(|block| block.lines().map(str::trim_end).collect::<Vec<_>>().join("\n"))
            .collect::<Vec<_>>()
            .join("\n\n");
        text.push('\n');
        text
    }
}

// The `## Claims` section of a brief's prompt: every claim of every source
// under the source's key and authority, so the model sees the whole body of
// evidence it must draft from.
struct ClaimsSection<'a>(&'a [SourceEvidence]);

impl Display for ClaimsSection<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("## Claims\n")?;

        for source in self.0 {
            write!(
                f,
                "\n### source `{key}` ({authority})\n\n",
                key = source.key,
                authority = source.evidence.authority
            )?;

            for claim in &source.evidence.claims {
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
    fn paragraphs(&mut self, texts: &[String], label: impl Display) {
        for text in texts {
            self.paragraph(text, &label);
        }
    }

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

    // A scenario field is one non-blank line.
    fn line(&mut self, text: &str, label: impl Display) {
        if text.trim().is_empty() {
            self.note(format_args!("{label} is blank"));
        } else if text.contains('\n') {
            self.note(format_args!("{label} spans more than one line"));
        }
    }
}
