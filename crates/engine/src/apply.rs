//! Seals the staged candidate or amendment as the current revision in the target repository.
//!
//! [`apply`] reads what stands in the staging slot — the candidate
//! [`specify`](crate::specify::specify) staged, or the amendment
//! [`amend`](crate::amend::amend) staged — seals its three documents under
//! `spec/` in the repository the target names as one commit labelled
//! `emery/<id>`, and clears the slot, so `show` and `build` read it from
//! then on.

use omnia_sdk::api::Context;
use omnia_sdk::{BlobStore, Error, StateStore, Vcs};
use serde::{Deserialize, Serialize};

use crate::Files;
use crate::revision::Diff;
pub use crate::store::StagedKind;
use crate::store::{self, Sealing, StagedItem, Standing};
use crate::vcs::{BuildRepo, TargetRef};

/// Seals the staged candidate or amendment as the current revision in the target's repository.
///
/// A candidate's three documents are written under
/// [`crate::vcs::SPECIFICATION`] in a working copy cut at the commit the current
/// revision stands at, else at the repository's head, sealed as one commit
/// whose message is a [`crate::vcs::RevisionMessage`] naming the displaced
/// revision as parent, and labelled `emery/<id>`; where the current revision
/// amends an earlier one, only a candidate `specify` staged fresh seals,
/// with no parent and over the repository's head, every earlier label left
/// standing. An amendment's three documents are sealed the same way over
/// the commit its base was read at, the base its parent and what it changed
/// the message's [`Summary`](crate::vcs::Summary), and only while that base
/// is still the current revision at that commit; the base's label stands. A
/// repository the engine creates ([`BuildRepo::Target`] with no URL) is
/// created on the first seal, which is its root commit. A revision the
/// repository already holds seals nothing and stands as it was. The checkout
/// the repository's head is on is never touched.
///
/// # Errors
///
/// - Returns [`Error::NotFound`]:
///   - with code `nothing-staged`, when nothing is staged;
///   - with code `revision-not-found`, when the repository the target
///     names does not exist or has no default branch, or what the run
///     already holds under the target's name was cloned or created under
///     another origin.
/// - Returns [`Error::BadRequest`]:
///   - with code `lineage-exists`, when the staged candidate was not staged
///     fresh and the current revision amends an earlier one; the candidate
///     stays staged;
///   - with code `amendment-stale`, when the staged amendment was prepared
///     against a revision, or a commit, other than the one the repository
///     now holds as current; the amendment stays staged;
///   - when the target's name is not kebab-case;
///   - with code `spec-outdated`, when the candidate or the revision the
///     repository holds uses a different grammar;
///   - with code `spec-mismatch`, when `spec/` in the repository is not the
///     three documents of a revision, does not match the revision its
///     label names, or, at a head no label names, holds documents that
///     read as no revision, edited or merged by hand;
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

// Seals whatever is staged over what `standing` read of `repo`; the one
// sealing step `apply`, `specify --apply`, and `amend --apply` share.
pub(crate) async fn seal<P: StateStore + BlobStore + Vcs + Files>(
    provider: &P, repo: &BuildRepo, standing: Standing,
) -> Result<ApplyOutput, Error> {
    let (kind, amendment, sealing) =
        match store::staged_item(provider).await?.ok_or_else(store::nothing_staged)? {
            StagedItem::Revision(staged) if staged.fresh => {
                (StagedKind::Revision, None, Sealing::fresh(staged.revision, &standing))
            }
            StagedItem::Revision(staged) => {
                (StagedKind::Revision, None, Sealing::candidate(staged.revision, &standing)?)
            }
            StagedItem::Amendment(staged) => (
                StagedKind::Amendment,
                Some(staged.id),
                Sealing::amendment(staged.amendment, &standing)?,
            ),
        };
    let sealed = store::commit(provider, repo, sealing, standing).await?;
    tracing::info!(
        revision = %sealed.id,
        parent = sealed.parent.as_deref().unwrap_or("none"),
        commit = %sealed.commit,
        label = %sealed.label,
        %kind,
        amendment = amendment.as_deref().unwrap_or("none"),
        "sealed"
    );

    Ok(ApplyOutput {
        revision: sealed.id,
        parent: sealed.parent,
        commit: sealed.commit,
        label: sealed.label,
        repository: repo.path(),
        kind,
        amendment,
        diff: sealed.diff,
    })
}

/// Where [`apply`] seals what is staged: the target whose repository holds the specification.
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
    ///
    /// For a sealed amendment, the base it was prepared against.
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
    /// What kind of staged item was sealed.
    pub kind: StagedKind,
    /// The content identifier of the amendment sealed; absent for a
    /// revision candidate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amendment: Option<String>,
    /// Changes from the displaced revision.
    ///
    /// Absent on the first seal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
}
