//! Reads one document of the current revision back for review.
//!
//! [`show`] renders `spec.md` or `design.md` from the current revision so an
//! operator, or a skill acting for one, can review what the last `specify`
//! committed. Review goes through the operation rather than the filesystem, so
//! the revision store stays the engine's own: a caller gets the rendered
//! document, the revision id it belongs to, and the typed document itself,
//! never the storage layout beneath them.

use anyhow::Context as _;
use omnia_sdk::api::Context;
use omnia_sdk::{BlobStore, Error, StateStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, EnumString, VariantArray};

use crate::revision::Document;
use crate::store;

/// Reads one document of the current revision over the context's provider.
///
/// # Errors
///
/// Returns [`Error::NotFound`] with code `spec-not-generated` when no revision
/// has been committed, and passes through the store's failures.
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

/// The input to [`show`]: which document to read.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// The document to read.
    pub artifact: Artifact,
}

/// A reviewable document of a revision.
///
/// A caller names one by its kebab-case key — `spec`, `design` — through
/// `parse()` and `as_ref()`, the same spelling serde uses.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantArray)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Artifact {
    /// The behavioural specification.
    Spec,
    /// The rebuild design.
    Design,
}

/// The rendered document, with the revision it belongs to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowOutput {
    /// The id of the current revision.
    pub revision: String,
    /// The document rendered as Markdown, with front matter naming the
    /// revision.
    pub body: String,
    /// The stored document the projection was rendered from, as JSON.
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
