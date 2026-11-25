//! Records supplied to a source adapter and returned by its metadata export.

use serde::{Deserialize, Serialize};

use crate::source::SourceKind;

/// The source identifier and content supplied to one extraction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceInput {
    /// The name the specification cites the source by.
    ///
    /// Kebab-case, chosen by the caller, and distinct from the adapter's
    /// identifier.
    pub name: String,
    /// The workspace or inline text presented to the adapter.
    pub content: SourceContent,
}

impl SourceInput {
    /// Returns an input backed by the read-only directory at `root`.
    #[must_use]
    pub fn workspace(name: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: SourceContent::Workspace(root.into()),
        }
    }

    /// Returns an input containing `text` without an associated workspace.
    #[must_use]
    pub fn value(name: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            name: name.into(),
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
pub struct SourceMetadata {
    /// The minimum compatible Emery version, if the adapter declares one.
    pub emery_version: Option<String>,
    /// The source kind used to rank this adapter's evidence.
    pub kind: SourceKind,
}
