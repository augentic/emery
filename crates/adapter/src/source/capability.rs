//! The `Source` capability
//!
//! [`Source`] is how the engine reaches an adapter: it addresses a loaded
//! adapter by id and asks it to extract evidence or report its metadata.
//! It follows the shape of omnia's other capability traits so a provider
//! carries it alongside `Model`, storage, and plugin loading. The records
//! that cross the seam inward — what an adapter is given and what it reports
//! about itself — are declared beside it.
//!
//! In a wasm guest the trait dispatches over the WIT import automatically. In
//! a native build the methods are left for the caller to implement, so a test
//! can script exactly what an adapter would have returned.

use std::future::Future;

use omnia_guest::Error;
use serde::{Deserialize, Serialize};

use crate::source::Evidence;

/// Import-side source dispatch over the `emery:adapter/source` contract.
///
/// Adapters implement the export-side `SourceAdapter` from `emery-sdk`
/// instead. An extract failure arrives classified: an adapter refusing its
/// input is `BadRequest`, any other failure `BadGateway`.
pub trait Source: Send + Sync {
    /// Dispatches `extract` to `id`.
    #[cfg(not(target_arch = "wasm32"))]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;

    /// Dispatches `extract` to `id`.
    #[cfg(target_arch = "wasm32")]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        crate::source::bindings::import::extract(id, input)
    }

    /// Returns resolve-time metadata for `id`.
    #[cfg(not(target_arch = "wasm32"))]
    fn metadata(&self, id: &str) -> AdapterMetadata;

    /// Returns resolve-time metadata for `id`.
    #[cfg(target_arch = "wasm32")]
    fn metadata(&self, id: &str) -> AdapterMetadata {
        crate::source::bindings::import::metadata(id)
    }
}

/// Source operation input: the key the specification cites the source by and
/// what the adapter extracts from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceInput {
    /// Binding key.
    pub key: String,
    /// Workspace or inline content.
    pub content: SourceContent,
}

/// Workspace or inline source content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceContent {
    /// Deployment-local root of a read-only source view.
    Workspace(String),
    /// Inline value without a filesystem lend.
    Value(String),
}

/// Resolve-time source adapter metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterMetadata {
    /// Exact minimum Emery version, if any.
    pub emery_version: Option<String>,
}
