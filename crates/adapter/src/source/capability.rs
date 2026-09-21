//! Defines the [`Source`] capability and records supplied to an adapter.
//!
//! The engine uses [`Source`] to query a loaded adapter's metadata and extract
//! evidence from an input. WebAssembly builds dispatch through the imported
//! adapter interface; native builds require an implementation.

use std::future::Future;

use omnia_sdk::Error;
use serde::{Deserialize, Serialize};

use crate::source::{Evidence, SourceKind};

/// The engine capability for querying and invoking source adapters.
///
/// Adapter components implement the guest interface rather than this trait.
/// The WebAssembly implementation classifies an input refusal as
/// [`Error::BadRequest`] and any other adapter failure as
/// [`Error::BadGateway`].
///
/// # Examples
///
/// A native host can provide adapters directly:
///
/// ```
/// use std::future::{Future, ready};
///
/// use emery_adapter::source::{
///     AdapterMetadata, Claim, ClaimKind, Evidence, Source, SourceInput, SourceKind,
/// };
/// use omnia_sdk::Error;
///
/// struct Host;
///
/// impl Source for Host {
///     fn extract(
///         &self, _id: &str, _input: &SourceInput,
///     ) -> impl Future<Output = Result<Evidence, Error>> + Send {
///         ready(Ok(Evidence {
///             claims: vec![Claim {
///                 kind: ClaimKind::Decision,
///                 id: None,
///                 path: None,
///                 synopsis: None,
///                 backing: None,
///                 extras: serde_json::Map::new(),
///             }],
///         }))
///     }
///
///     fn metadata(&self, _id: &str) -> AdapterMetadata {
///         AdapterMetadata {
///             emery_version: None,
///             kind: SourceKind::Documentation,
///         }
///     }
/// }
/// ```
pub trait Source: Send + Sync {
    /// Extracts evidence from `input` using the adapter registered as `id`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] for any other adapter failure.
    #[cfg(not(target_arch = "wasm32"))]
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;

    /// Extracts evidence from `input` using the adapter registered as `id`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] for any other adapter failure.
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

/// The source identifier and content supplied to one extraction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceInput {
    /// The key used to cite the source in a specification.
    pub key: String,
    /// The workspace or inline text presented to the adapter.
    pub content: SourceContent,
}

impl SourceInput {
    /// Returns an input backed by the read-only directory at `root`.
    #[must_use]
    pub fn workspace(key: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            content: SourceContent::Workspace(root.into()),
        }
    }

    /// Returns an input containing `text` without an associated workspace.
    #[must_use]
    pub fn value(key: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            content: SourceContent::Value(text.into()),
        }
    }
}

/// The content presented to a source adapter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceContent {
    /// A read-only directory, named as the adapter sees it.
    Workspace(String),
    /// Text supplied inline without a workspace.
    Value(String),
}

/// Metadata declared by a source adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterMetadata {
    /// The minimum compatible Emery version, if the adapter declares one.
    pub emery_version: Option<String>,
    /// The source kind used to rank this adapter's evidence.
    pub kind: SourceKind,
}
