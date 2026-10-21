//! Reads documents from the current specification revision, or the staged candidate.
//!
//! [`show`] returns the selected document as Markdown and structured JSON,
//! together with the revision identifier. The current revision is read from
//! the target's repository, the staged candidate from the engine's own
//! storage.

use anyhow::Context as _;
use omnia_sdk::api::Context;
use omnia_sdk::{BlobStore, Error, StateStore, Vcs};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, EnumString, VariantArray};

use crate::revision::Document;
use crate::vcs::TargetRef;
use crate::{Files, store};

/// Returns one document from the current revision, or from the staged candidate.
///
/// The current revision is read from the repository [`ShowInput::target`]
/// names, as [`apply`](crate::apply::apply) sealed it; the staged candidate
/// needs no target.
///
/// # Errors
///
/// - Returns [`Error::NotFound`] with code `spec-not-generated` when the
///   repository holds no revision, naming a staged candidate where one
///   stands, and with code `nothing-staged` when [`ShowInput::staged`] is
///   set and no candidate is staged.
/// - Returns [`Error::BadRequest`] with code `build-target-required` when
///   no target names the repository and the candidate is not asked for, and
///   with code `spec-outdated` when the revision read uses a different
///   grammar, and the classes [`apply`](crate::apply::apply) reads the
///   repository under otherwise.
/// - Returns [`Error::ServerError`] when storage, validation, or serialisation
///   fails.
pub async fn show<P: StateStore + BlobStore + Vcs + Files>(
    input: ShowInput, context: Context<P>,
) -> Result<ShowOutput, Error> {
    let store = context.provider();
    let (id, revision) = if input.staged {
        let candidate = store::staged(store).await?.ok_or_else(store::nothing_staged)?;
        (candidate.id, candidate.revision)
    } else {
        let target = input.target.ok_or_else(|| store::target_required("`emery show`"))?;
        let repo = target.repo()?;
        let standing = store::current(store, &repo).await?;
        let Some(current) = standing.current else {
            return Err(store::not_generated(store, &standing).await);
        };
        (current.id, current.revision?)
    };
    tracing::debug!(
        revision = %id,
        staged = input.staged,
        artifact = input.artifact.as_ref(),
        "revision read"
    );

    match input.artifact {
        Artifact::Spec => ShowOutput::new(&revision.spec, id),
        Artifact::Design => ShowOutput::new(&revision.design, id),
        Artifact::Plan => ShowOutput::new(&revision.plan, id),
    }
}

/// Selects the document returned by [`show`].
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// The revision artifact projected as Markdown and structured JSON.
    pub artifact: Artifact,
    /// Whether to read the staged candidate in place of the current revision.
    #[serde(default)]
    pub staged: bool,
    /// The target whose repository the current revision is read from.
    ///
    /// Required unless [`staged`](Self::staged) is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetRef>,
}

/// A document available from a specification revision.
///
/// String parsing, [`AsRef::as_ref`], and Serde use the lowercase names `spec`,
/// `design`, and `plan`.
///
/// # Examples
///
/// ```
/// use emery_engine::show::Artifact;
///
/// let artifact: Artifact = "spec".parse()?;
/// assert!(matches!(artifact, Artifact::Spec));
/// assert_eq!(Artifact::Design.as_ref(), "design");
/// assert_eq!(Artifact::Plan.as_ref(), "plan");
/// # Ok::<(), strum::ParseError>(())
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantArray)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Artifact {
    /// The behavioural specification, rendered as `spec.md`.
    Spec,
    /// The rebuild design, rendered as `design.md`.
    Design,
    /// The build plan, rendered as `plan.md`.
    Plan,
}

/// A rendered revision document and its structured representation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowOutput {
    /// The identifier of the revision read: the current one, or the staged candidate.
    pub revision: String,
    /// The Markdown projection, including revision front matter.
    pub body: String,
    /// The typed document serialised as JSON.
    pub document: Value,
}

impl ShowOutput {
    // The store wrote the document under this derive, so a failure here is
    // the engine's own defect.
    fn new<D: Document>(document: &D, revision: String) -> Result<Self, Error> {
        let value = serde_json::to_value(document)
            .with_context(|| format!("`{}` does not serialise", D::NAME))?;
        Ok(Self {
            body: document.to_markdown(&revision),
            revision,
            document: value,
        })
    }
}
