//! Stages an amendment against the current revision from change sources.
//!
//! [`amend`] extracts claims from each change source as
//! [`specify`](crate::specify::specify) does, matches them against the
//! requirements the current revision holds, derives what the change did to
//! each requirement, and stages the result as an [`Amendment`] for review.
//! Nothing is sealed, and the current revision is read and never moved.

mod change;
mod standing;

use emery_adapter::source::Source;
use omnia_sdk::api::Context;
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, Vcs, bad_request};
use serde::Serialize;

use self::change::ChangeSet;
use self::standing::Standing;
pub use crate::revision::{Amendment, AmendmentBase, Change, ChangeClass};
use crate::specify::{BASELINE, Bound, Extracted, GroupingBrief, RepositorySource, SourceConfig};
use crate::vcs::TargetRef;
use crate::{Files, store};

/// Prepares and stages an amendment against the current revision from `input`.
///
/// The current revision is read from the repository [`AmendInput::target`]
/// names before any adapter loads, and its requirements stand as the
/// baseline the change is matched against, under the reserved source name
/// `baseline`: a claim restating or revising one keeps the requirement's
/// id, a claim no requirement covers is added under the next free id, a
/// `retire` claim retires the requirement stating the behaviour it
/// withdraws, and a requirement no claim touches stands unchanged. The
/// sources are extracted as `specify` extracts them, a repository source
/// read in a working copy removed once every source answered.
///
/// A change that leaves every requirement as it stands stages nothing. One
/// that changes anything is staged as the one amendment, replacing an
/// amendment staged before it; a staged revision candidate is never
/// replaced, since it awaits `emery apply`.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the run is refused:
///   - an empty source list, with code `specify-source-required`;
///   - a source named `baseline`, the reserved name;
///   - no target, with code `build-target-required`;
///   - a revision candidate standing staged, with code `amendment-pending`;
///   - a `retire` claim withdrawing a behaviour no requirement of the
///     current revision states, with code `retire-unmatched`;
///   - the classes [`specify`](crate::specify::specify) refuses a source
///     list, an adapter, or a model answer under;
///   - the classes [`apply`](crate::apply::apply) reads the repository
///     under.
/// - Returns [`Error::NotFound`] with code `spec-not-generated` when the
///   repository holds no revision, naming a staged candidate where one
///   stands, and with code `revision-not-found` as `specify` does.
/// - Returns [`Error::ServerError`] and [`Error::BadGateway`] as `specify`
///   does.
pub async fn amend<P: Model + Source + StateStore + BlobStore + Plugins + Vcs + Files>(
    input: AmendInput, context: Context<P>,
) -> Result<AmendOutput, Error> {
    let provider = context.provider();

    // the sources, the revision they amend, and the slot the amendment goes to,
    // each refused before any load
    let bound = Bound::all(&input.sources)?;
    if bound.iter().any(|source| source.name() == BASELINE) {
        return Err(bad_request!(
            "source `{BASELINE}` is the current revision's own name in an amendment; name the \
             source otherwise"
        ));
    }
    let repo = input.target.ok_or_else(|| store::target_required("`emery amend`"))?.repo()?;
    let repository = store::current(provider, &repo).await?;
    let Some(current) = repository.current else {
        return Err(store::not_generated(provider, &repository).await);
    };
    let revision = current.revision?;
    store::no_candidate(provider).await?;

    let Extracted {
        extracts,
        repositories,
    } = Bound::extract_all(provider, &bound).await?;

    // the change matched against the base, and what it did to each requirement
    let standing = Standing::new(&revision.spec, &extracts);
    let bases =
        GroupingBrief::with_standing(&extracts, standing.contributors(), standing.criteria())
            .derive(provider)
            .await?;
    let base = AmendmentBase {
        revision: current.id,
        commit: current.commit,
    };
    let amendment = ChangeSet::derive(&standing, &bases)?.into_amendment(base);

    let tally = Tally::of(&amendment);
    let conflicts = amendment.conflicts().map(str::to_owned).collect();
    let stems = amendment.stems().into_iter().map(str::to_owned).collect();
    let staged = if amendment.is_change() {
        let id = store::stage_amendment(provider, &amendment).await?;
        tracing::info!(amendment = %id, base = %amendment.base.revision, ?tally, "staged");
        Some(id)
    } else {
        tracing::info!(base = %amendment.base.revision, "no change against the revision");
        None
    };

    Ok(AmendOutput {
        amendment: staged,
        base: amendment.base,
        tally,
        conflicts,
        stems,
        repositories,
    })
}

/// The change sources an amendment is prepared from, and the target whose revision it amends.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AmendInput {
    /// The change sources in declaration order, bound as `specify` binds
    /// its sources; none may be named `baseline`.
    pub sources: Vec<SourceConfig>,
    /// The target whose repository the current revision lives in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetRef>,
}

/// What a successful [`amend`] prepared, and whether it staged it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AmendOutput {
    /// The content identifier of the staged amendment; absent when the
    /// change left every requirement as it stands, so nothing was staged.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amendment: Option<String>,
    /// The revision the amendment was prepared against, at the commit it
    /// was read at.
    pub base: AmendmentBase,
    /// How many requirements fall under each class of change.
    pub tally: Tally,
    /// The subjects of the requirements the change sources disagree over,
    /// in id order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<String>,
    /// The stems of the requirements the change touches, once each, sorted.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stems: Vec<String>,
    /// The sources read from a repository, in declaration order, each at the
    /// commit its revision resolved to.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub repositories: Vec<RepositorySource>,
}

/// How many requirements an [`Amendment`] holds under each class of change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Tally {
    /// Requirements the base does not hold, stated by the change.
    pub added: usize,
    /// Requirements of the base the change restates, their ids kept.
    pub changed: usize,
    /// Requirements of the base the change withdraws.
    pub retired: usize,
    /// Requirements the change sources state and disagree over.
    pub conflict: usize,
    /// Requirements of the base the change leaves as they stand.
    pub unchanged: usize,
}

impl Tally {
    /// Counts the entries of `amendment` by class.
    #[must_use]
    pub fn of(amendment: &Amendment) -> Self {
        Self {
            added: amendment.count(ChangeClass::Added),
            changed: amendment.count(ChangeClass::Changed),
            retired: amendment.count(ChangeClass::Retired),
            conflict: amendment.count(ChangeClass::Conflict),
            unchanged: amendment.count(ChangeClass::Unchanged),
        }
    }
}
