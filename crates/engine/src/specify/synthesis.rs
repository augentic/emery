//! Synthesis
//!
//! Synthesises the requirements and the extracted claims into a revision: the
//! typed specification and design. The model is asked up to two typed
//! questions in turn — the drafted content of `spec.md`, then of `design.md`
//! — each put as a brief that verifies every candidate answer against the
//! requirements, the section plan, and the evidence before the engine places
//! the accepted answer beside its own facts in the revision. Every heading,
//! provenance line, body, tag, note, and signature is the engine's, so the
//! stored revision is a function of the facts and the draft alone, and a
//! changed value re-ids every revision.
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
use crate::artifact::Revision;
use crate::specify::brief::Brief as _;
use crate::specify::{Extract, basis};

/// Takes the extracts of every source and the requirement bases derived from
/// them, then synthesises the specification and design.
///
/// # Errors
///
/// A model failure is `bad_gateway`; an answer outside the schema, or a draft
/// the backend could not repair within its rounds, is `bad_request`.
pub async fn synthesise<M: Model>(model: &M, extracts: &[Extract]) -> Result<Revision, Error> {
    let bases = basis::derive(model, extracts).await?;
    let spec = SpecBrief::new(extracts, &bases).judge(model).await?;
    let design = DesignBrief::new(extracts, &spec).judge(model).await?;

    Ok(Revision { spec, design })
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
