//! Seals the staged candidate as the current revision in the target repository.
//!
//! [`apply`] reads the candidate [`specify`](crate::specify::specify)
//! staged, seals it under `spec/` in the repository the target names as
//! one commit labelled `emery/<id>`, and clears the staging slot, so `show`
//! and `build` read it from then on.

use omnia_sdk::api::Context;
use omnia_sdk::{BlobStore, Error, StateStore, Vcs};
use serde::{Deserialize, Serialize};

use crate::Files;
use crate::revision::Diff;
pub use crate::store::StagedKind;
use crate::store::{self, Standing};
use crate::vcs::{BuildRepo, TargetRef};

/// Seals the staged candidate as the current revision in the target's repository.
///
/// The candidate's three documents are written under
/// [`crate::vcs::SPECIFICATION`] in a working copy cut at the commit the current
/// revision stands at, else at the repository's head, sealed as one commit
/// whose message is a [`crate::vcs::RevisionMessage`], and labelled `emery/<id>`.
/// A repository the engine creates ([`BuildRepo::Target`] with no URL) is
/// created on the first seal, which is its root commit. A revision the
/// repository already holds seals nothing and stands as it was. The
/// checkout the repository's head is on is never touched.
///
/// # Errors
///
/// - Returns [`Error::NotFound`]:
///   - with code `nothing-staged`, when no candidate is staged;
///   - with code `revision-not-found`, when the repository the target
///     names does not exist or has no default branch, or what the run
///     already holds under the target's name was cloned or created under
///     another origin.
/// - Returns [`Error::BadRequest`]:
///   - when the target's name is not kebab-case;
///   - with code `spec-outdated`, when the candidate or the revision the
///     repository holds uses a different grammar;
///   - with code `spec-mismatch`, when `spec/` in the repository is not the
///     three documents of a revision, or does not match the revision its
///     label names;
///   - with code `repository-required`, when the target names the project
///     checkout ([`crate::vcs::PROJECT`]) and the project is no repository;
///   - with code `base-not-sealed`, when the project repository has no
///     commit to seal over;
///   - with code `secret-detected`, when a document carries a credential.
/// - Returns [`Error::BadGateway`] when the remote refuses, cannot be
///   reached, or wants credentials.
/// - Returns [`Error::ServerError`] when storage or version control fails,
///   or the candidate no longer matches its identifier.
pub async fn apply<P: StateStore + BlobStore + Vcs + Files>(
    input: ApplyInput, context: Context<P>,
) -> Result<ApplyOutput, Error> {
    let provider = context.provider();
    let repo = input.target.repo()?;
    let standing = store::current(provider, &repo).await?;
    seal(provider, &repo, standing).await
}

// Seals the staged candidate over what `standing` read of `repo`; the one
// sealing step `apply` and `specify --apply` share.
pub(crate) async fn seal<P: StateStore + BlobStore + Vcs + Files>(
    provider: &P, repo: &BuildRepo, standing: Standing,
) -> Result<ApplyOutput, Error> {
    let candidate = store::staged(provider).await?.ok_or_else(store::nothing_staged)?;
    let sealed = store::commit(provider, repo, &candidate.revision, standing).await?;
    tracing::info!(
        revision = %sealed.id,
        parent = sealed.parent.as_deref().unwrap_or("none"),
        commit = %sealed.commit,
        label = %sealed.label,
        kind = %candidate.kind,
        "sealed"
    );

    Ok(ApplyOutput {
        revision: sealed.id,
        parent: sealed.parent,
        commit: sealed.commit,
        label: sealed.label,
        repository: repo.path(),
        kind: candidate.kind,
        diff: sealed.diff,
    })
}

/// Where [`apply`] seals the staged candidate: the target whose repository holds the specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ApplyInput {
    /// The target whose repository the specification lives in.
    pub target: TargetRef,
}

/// The revision a successful [`apply`] sealed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ApplyOutput {
    /// The content identifier of the revision, now current.
    pub revision: String,
    /// The revision it displaced, by id; `None` on the first seal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The commit the revision stands at in the repository.
    pub commit: String,
    /// The label set on that commit, `emery/<revision>`.
    pub label: String,
    /// The repository the revision was sealed in, as a path beneath the
    /// project root: `.` for the project's own, or [`crate::vcs::target_path`] of
    /// the target's name.
    pub repository: String,
    /// What kind of candidate was sealed.
    pub kind: StagedKind,
    /// Changes from the displaced revision.
    ///
    /// Absent on the first seal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
}
