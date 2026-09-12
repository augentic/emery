//! The `show` operation
//!
//! Renders one document — `spec.md` or `design.md` — from the current
//! specification revision so an operator, or a skill acting for one, can
//! review what the last `specify` committed.
//!
//! Review goes through this operation rather than the filesystem so the
//! revision store stays the engine's own: callers see a document rendered
//! from the stored revision, paired with the revision id it belongs to and the
//! typed document itself, and never the storage layout beneath it.

use omnia_guest::api::Context;
use omnia_guest::{BlobStore, Error, StateStore, server_error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, EnumString, VariantArray};

use crate::artifact::Document;
use crate::store;

/// Reads one document of the current revision over the context's provider,
/// returning it with the revision id it belongs to.
///
/// # Errors
///
/// Returns `NotFound` (`spec-not-generated`) when no revision has been
/// committed, and passes through the store's failures.
pub async fn show<P: StateStore + BlobStore>(
    input: ShowInput, context: Context<P>,
) -> Result<ShowOutput, Error> {
    let ShowInput { artifact } = input;

    let Some((id, revision)) = store::current(context.provider()).await? else {
        return Err(Error::NotFound {
            code: "spec-not-generated".into(),
            description: "no specification revision has been committed".into(),
        });
    };

    match artifact {
        Artifact::Spec => reviewed(&revision.spec, id),
        Artifact::Design => reviewed(&revision.design, id),
    }
}

/// Read one artifact of the current revision.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// Which artifact to read.
    pub artifact: Artifact,
}

/// The reviewable artifacts of a revision. A caller names one by its
/// kebab-case key (`as_ref()` / `parse()`, `spec`), the same spelling serde
/// uses.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, AsRefStr, EnumString, VariantArray)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Artifact {
    /// The behavioural specification.
    Spec,
    /// The rebuild design.
    Design,
}

/// Successful review result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowOutput {
    /// Current revision id.
    pub revision: String,
    /// The rendered Markdown projection.
    pub body: String,
    /// The stored document the projection was rendered from.
    pub document: Value,
}

// Pairs one document's projection and stored shape with its revision id.
fn reviewed<D: Document>(document: &D, revision: String) -> Result<ShowOutput, Error> {
    let value = serde_json::to_value(document)
        .map_err(|err| server_error!("`{}` did not serialise: {err}", D::FILE))?;

    Ok(ShowOutput {
        body: document.to_markdown(&revision),
        revision,
        document: value,
    })
}
