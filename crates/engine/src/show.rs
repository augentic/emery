//! The `show` operation
//!
//! Renders one document — `spec.md` or `design.md` — from the current
//! specification revision so an operator, or a skill acting for one, can
//! review what the last `specify` committed.
//!
//! Review goes through this operation rather than the filesystem so the
//! revision store stays the engine's own: callers see a document rendered
//! from the stored master, paired with the revision id it belongs to and the
//! master itself, and never the storage layout beneath it. The JSON envelope
//! is what a project carries beside its code as `.emery/<document>.json`, so
//! the next `specify` can continue the revision.

use omnia_guest::api::Context;
use omnia_guest::{BlobStore, Error, StateStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::artifact::Document;
use crate::store;

/// Read one document of the current revision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// Which document to read.
    pub document: Document,
}

/// Successful review result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowOutput {
    /// Current revision id.
    pub revision: String,
    /// The rendered Markdown projection.
    pub body: String,
    /// The stored master the projection was rendered from.
    pub document: Value,
}

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
    let ShowInput { document } = input;

    let Some(dossier) = store::current(context.provider()).await? else {
        return Err(Error::NotFound {
            code: "spec-not-generated".into(),
            description: "no specification revision has been committed".into(),
        });
    };

    let master = match document {
        Document::Spec => serde_json::to_value(&dossier.spec),
        Document::Design => serde_json::to_value(&dossier.design),
    }
    .expect("the master serialises: no maps with non-string keys, no floats");

    Ok(ShowOutput {
        revision: dossier.revision(),
        body: dossier.render(document),
        document: master,
    })
}
