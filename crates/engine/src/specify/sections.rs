//! Renders the shared requirement basis and claim sections for synthesis briefs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::Trigger;
use emery_adapter::source::{Claim, ClaimKind};
use serde_json::Value;

use crate::revision::{self, Status};
use crate::specify::basis::{Basis, Contributor};
use crate::specify::{Extract, shape};

// The requirement outline of a document brief's turn, one entry per basis.
//
// Each entry carries the engine's facts about the requirement: its id,
// subject, status, sources, whether a criterion covers it, the trigger a
// contract binds to it, and every contributing claim with its role. The
// three drafts run from it together, so none waits on another's answer.
pub struct BasesSection<'a>(pub &'a [Basis]);

impl Display for BasesSection<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for basis in self.0 {
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
            write!(f, "] — acceptance criteria {coverage}")?;
            match &basis.trigger {
                Some(Trigger::Http(route)) => write!(f, " — Route: {route}")?,
                Some(Trigger::Message(name)) => write!(f, " — Message: {name}")?,
                None => {}
            }
            f.write_str("\n")?;

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

// The `## Claims` section of a document brief's turn.
//
// Every claim of every extract is listed under its source name and kind.
pub struct ClaimsSection<'a>(pub &'a [Extract]);

impl Display for ClaimsSection<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("## Claims\n")?;

        for extract in self.0 {
            write!(
                f,
                "\n### source `{source}` ({kind})\n\n",
                source = extract.source,
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

// The claims belonging to a subset of requirement `bases`: every claim whose id
// leads with one of their stems, and every unidentified claim anchored in a
// file where one of those requirement claims was read.
pub struct ClaimScope {
    stems: BTreeSet<String>,
    files: BTreeSet<String>,
}

impl ClaimScope {
    #[must_use]
    pub fn of(extracts: &[Extract], bases: &[Basis]) -> Self {
        let stems: BTreeSet<String> = bases
            .iter()
            .flat_map(|basis| {
                std::iter::once(basis.stem())
                    .chain(basis.classes.iter().flatten().map(Contributor::stem))
            })
            .map(str::to_owned)
            .collect();
        let files = extracts
            .iter()
            .flat_map(|extract| &extract.evidence.claims)
            .filter(|claim| claim.kind == ClaimKind::Requirement)
            .filter(|claim| {
                claim.id.as_deref().is_some_and(|id| stems.contains(revision::stem(id)))
            })
            .filter_map(|claim| claim.path.as_deref())
            .map(|path| shape::file(path).to_owned())
            .collect();
        Self { stems, files }
    }

    #[must_use]
    pub fn contains(&self, claim: &Claim) -> bool {
        claim.id.as_deref().map_or_else(
            || claim.path.as_deref().is_none_or(|path| self.files.contains(shape::file(path))),
            |id| self.stems.contains(revision::stem(id)),
        )
    }
}

// The `call` claims of `extracts` by the package each reaches through: the
// part of its `callee` before the first `:`. A bare global (`fetch`) reaches
// through none, and a wrapper of the tree (`src/lib/http.ts:HttpClient.post`)
// spells a path the lexicon reads as a plain word and keeps nothing of. The
// specification is held from naming these and the design to naming them.
pub fn packages(extracts: &[Extract]) -> BTreeMap<String, Vec<Claim>> {
    let mut packages: BTreeMap<String, Vec<Claim>> = BTreeMap::new();
    let calls = extracts
        .iter()
        .flat_map(|extract| &extract.evidence.claims)
        .filter(|claim| claim.kind == ClaimKind::Call);
    for claim in calls {
        if let Some((package, _)) = claim.callee().and_then(|callee| callee.split_once(':')) {
            packages.entry(package.to_owned()).or_default().push(claim.clone());
        }
    }
    packages
}
