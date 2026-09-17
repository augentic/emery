//! Reads documents from the current specification revision.
//!
//! [`show`] returns the selected document as Markdown and structured JSON,
//! together with the revision identifier. Storage details remain private to
//! the engine.

use anyhow::Context as _;
use omnia_sdk::api::Context;
use omnia_sdk::{BlobStore, Error, StateStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, EnumString, VariantArray};

use crate::revision::Document;
use crate::store;

/// Returns one document from the current revision.
///
/// # Errors
///
/// - Returns [`Error::NotFound`] with code `spec-not-generated` when no
///   revision has been committed.
/// - Returns [`Error::BadRequest`] with code `spec-outdated` when the stored
///   revision uses a different grammar.
/// - Returns [`Error::ServerError`] when storage, validation, or serialisation
///   fails.
pub async fn show<P: StateStore + BlobStore>(
    input: ShowInput, context: Context<P>,
) -> Result<ShowOutput, Error> {
    let Some((id, revision)) = store::current(context.provider()).await? else {
        return Err(Error::NotFound {
            code: "spec-not-generated".into(),
            description: "no specification revision has been committed".into(),
        });
    };

    match input.artifact {
        Artifact::Spec => ShowOutput::new(&revision.spec, id),
        Artifact::Design => ShowOutput::new(&revision.design, id),
    }
}

/// Selects the document returned by [`show`].
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// The revision artifact projected as Markdown and structured JSON.
    pub artifact: Artifact,
}

/// A document available from a specification revision.
///
/// String parsing, [`AsRef::as_ref`], and Serde use the lowercase names `spec`
/// and `design`.
///
/// # Examples
///
/// ```
/// use emery_engine::show::Artifact;
///
/// let artifact: Artifact = "spec".parse()?;
/// assert!(matches!(artifact, Artifact::Spec));
/// assert_eq!(Artifact::Design.as_ref(), "design");
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
}

/// A rendered revision document and its structured representation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowOutput {
    /// The identifier of the current revision.
    pub revision: String,
    /// The Markdown projection, including revision front matter.
    pub body: String,
    /// The typed document serialised as JSON.
    pub document: Value,
}

impl ShowOutput {
    // The document serialises under the same derive the store wrote it
    // with, so a failure here is the engine's own defect: `server_error`.
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
