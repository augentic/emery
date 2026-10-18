//! Renders the shared requirement basis and claim sections for synthesis briefs.

use std::borrow::Borrow;
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::Claim;
use serde_json::Value;

use crate::revision::Status;
use crate::specify::Extract;
use crate::specify::basis::Basis;

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
