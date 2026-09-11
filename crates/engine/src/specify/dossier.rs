//! The dossier
//!
//! Synthesises the requirements and the extracted claims into the dossier: the
//! typed specification and design masters. The model is asked up to two typed
//! questions in turn — the drafted content of `spec.md`, then of `design.md`
//! — each put as a brief that verifies every candidate answer against the
//! requirements, the section plan, and the evidence before the engine places
//! the accepted answer beside its own facts in the master. Every heading,
//! provenance line, body, tag, note, and signature is the engine's, so the
//! stored master is a function of the facts and the draft alone, and a
//! changed value re-ids every revision.
//!
//! What the master the run continues already settled is not asked again: a
//! requirement whose facts stand keeps its scenarios, a specification in
//! which every requirement stands is placed without a turn, and a design that
//! still verifies over an unchanged specification is kept whole.
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
use crate::specify::basis::Basis;
use crate::specify::brief::{Brief as _, Review};

/// Takes the extracts of every source, the requirement bases derived from
/// them, and the `master` the run continues, then synthesises the dossier's
/// specification and design masters, asking the model only for what the
/// master did not settle.
///
/// # Errors
///
/// A model failure is `bad_gateway`; an answer outside the schema, or a draft
/// the backend could not repair within its rounds, is `bad_request`.
pub async fn synthesise<M: Model>(
    model: &M, extracts: &[Extract], bases: &[Basis], master: Option<&Dossier>,
) -> Result<Dossier, Error> {
    let spec =
        SpecBrief::new(extracts, bases, master.map(|master| &master.spec)).resolve(model).await?;

    let brief = DesignBrief::new(extracts, &spec);
    let design = match master {
        Some(master) if master.spec == spec && brief.accepts(&master.design) => {
            tracing::info!("the specification stands; the design is carried");
            master.design.clone()
        }
        _ => brief.judge(model).await?,
    };

    Ok(Dossier { spec, design })
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
