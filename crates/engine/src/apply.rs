//! Seals the staged candidate as the current revision.
//!
//! [`apply`] reads the candidate [`specify`](crate::specify::specify)
//! staged, commits it as the current revision, and clears the staging
//! slot, so `show` and `build` read it from then on.

use omnia_sdk::api::Context;
use omnia_sdk::{BlobStore, Error, StateStore};
use serde::{Deserialize, Serialize};

use crate::revision::Diff;
use crate::store;
pub use crate::store::StagedKind;

/// Seals the staged candidate as the current revision.
///
/// # Errors
///
/// - Returns [`Error::NotFound`] with code `nothing-staged` when no
///   candidate is staged.
/// - Returns [`Error::BadRequest`] with code `spec-outdated` when the
///   candidate uses a different grammar.
/// - Returns [`Error::ServerError`] when storage fails, or the candidate no
///   longer matches its identifier.
pub async fn apply<P: StateStore + BlobStore>(
    input: ApplyInput, context: Context<P>,
) -> Result<ApplyOutput, Error> {
    let ApplyInput {} = input;
    seal(context.provider()).await
}

// Commits the staged candidate and clears the slot; the one sealing step
// `apply` and `specify --apply` share.
pub(crate) async fn seal<S: StateStore + BlobStore>(store: &S) -> Result<ApplyOutput, Error> {
    let candidate = store::staged(store).await?.ok_or_else(store::nothing_staged)?;
    let committed = store::commit(store, &candidate.revision).await?;
    store::clear(store).await?;
    tracing::info!(
        revision = %committed.id,
        parent = committed.parent.as_deref().unwrap_or("none"),
        kind = %candidate.kind,
        "sealed"
    );

    Ok(ApplyOutput {
        revision: committed.id,
        parent: committed.parent,
        kind: candidate.kind,
        diff: committed.diff,
    })
}

/// What [`apply`] seals: the staged candidate, which it names nothing of.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ApplyInput {}

/// The revision a successful [`apply`] sealed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ApplyOutput {
    /// The content identifier of the revision, now current.
    pub revision: String,
    /// The revision it displaced, by id; `None` on the first seal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// What kind of candidate was sealed.
    pub kind: StagedKind,
    /// Changes from the displaced revision.
    ///
    /// Absent on the first seal, and when the outgoing revision was
    /// unreadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
}
