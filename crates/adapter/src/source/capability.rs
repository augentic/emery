//! The [`Source`] capability and the records that cross into an adapter.
//!
//! [`Source`] is how the engine reaches a loaded adapter: it addresses the
//! adapter by id and asks it to extract evidence or report its metadata. It
//! has the shape of omnia's other capability traits, so one provider carries
//! it beside `Model`, storage, and plugin loading.
//!
//! In a wasm guest the trait dispatches over the WIT import by default. In a
//! native build the methods are left to the implementor, so a test can script
//! exactly what an adapter would have returned.

use std::future::Future;

use omnia_sdk::Error;
use serde::{Deserialize, Serialize};

use crate::source::{Evidence, SourceKind};

/// The capability the engine calls source adapters through.
///
/// Adapters implement the export side — the world's `Guest`, through
/// `emery-sdk` — not this trait. An extract failure arrives classified: an
/// adapter refusing its input is [`Error::BadRequest`], and any other failure
/// is [`Error::BadGateway`].
pub trait Source: Send + Sync {
    /// Asks the adapter registered as `id` to extract `input`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] when the adapter refuses its input, and
    /// [`Error::BadGateway`] for any other adapter failure.
    #[cfg(not(target_arch = "wasm32"))]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;

    /// Asks the adapter registered as `id` to extract `input`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] when the adapter refuses its input, and
    /// [`Error::BadGateway`] for any other adapter failure.
    #[cfg(target_arch = "wasm32")]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        crate::source::bindings::import::extract(id, input)
    }

    /// Returns the metadata the adapter registered as `id` declares.
    #[cfg(not(target_arch = "wasm32"))]
    fn metadata(&self, id: &str) -> AdapterMetadata;

    /// Returns the metadata the adapter registered as `id` declares.
    #[cfg(target_arch = "wasm32")]
    fn metadata(&self, id: &str) -> AdapterMetadata {
        crate::source::bindings::import::metadata(id)
    }
}

/// The input to one `extract` call: the source's key and its content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceInput {
    /// The key the specification cites the source by.
    pub key: String,
    /// The workspace or inline value to read.
    pub content: SourceContent,
}

impl SourceInput {
    /// Returns the input lending the read-only directory at `root`, cited as `key`.
    #[must_use]
    pub fn workspace(key: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            content: SourceContent::Workspace(root.into()),
        }
    }

    /// Returns the input carrying `text` inline, cited as `key`, lending nothing.
    #[must_use]
    pub fn value(key: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            content: SourceContent::Value(text.into()),
        }
    }
}

/// The content of a source: a directory to read, or an inline value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceContent {
    /// A read-only directory, named as the guest sees it.
    Workspace(String),
    /// Text given inline; no directory is lent.
    Value(String),
}

/// What an adapter declares about itself, read once before any extract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterMetadata {
    /// The minimum Emery version the adapter requires, if it states one.
    pub emery_version: Option<String>,
    /// The kind of source the adapter reads, which ranks its evidence
    /// against other sources'.
    pub kind: SourceKind,
}
