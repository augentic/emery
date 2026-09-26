//! Implements the WebAssembly interface shared by adapters and the engine.
//!
//! Adapters implement [`export::Guest`], while the engine invokes them
//! through [`import`]. Contract records and errors are converted at this
//! boundary.

mod generated {
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    wit_bindgen::generate!({
        world: "source-adapter",
        path: "../../wit",
        // `extract` alone is `async func` in the WIT, so no `async:` list is needed
        generate_all,
        pub_export_macro: true,
    });
}

use self::generated::emery::adapter::types as wit;
use crate::source::{
    AdapterMetadata, Backing, Claim, ClaimKind, Evidence, SourceContent, SourceInput, SourceKind,
};

impl From<AdapterMetadata> for wit::AdapterMetadata {
    fn from(metadata: AdapterMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
            kind: metadata.kind.into(),
        }
    }
}

impl From<wit::AdapterMetadata> for AdapterMetadata {
    fn from(metadata: wit::AdapterMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
            kind: metadata.kind.into(),
        }
    }
}

impl From<SourceContent> for wit::Content {
    fn from(content: SourceContent) -> Self {
        match content {
            SourceContent::Workspace(root) => Self::Workspace(root),
            SourceContent::Value(value) => Self::Value(value),
        }
    }
}

impl From<wit::Content> for SourceContent {
    fn from(content: wit::Content) -> Self {
        match content {
            wit::Content::Workspace(root) => Self::Workspace(root),
            wit::Content::Value(value) => Self::Value(value),
        }
    }
}

impl From<SourceInput> for wit::Input {
    fn from(input: SourceInput) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
        }
    }
}

impl From<wit::Input> for SourceInput {
    fn from(input: wit::Input) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
        }
    }
}

impl From<SourceKind> for wit::SourceKind {
    fn from(kind: SourceKind) -> Self {
        match kind {
            SourceKind::Intent => Self::Intent,
            SourceKind::Documentation => Self::Documentation,
            SourceKind::Behaviour => Self::Behaviour,
        }
    }
}

impl From<wit::SourceKind> for SourceKind {
    fn from(kind: wit::SourceKind) -> Self {
        match kind {
            wit::SourceKind::Intent => Self::Intent,
            wit::SourceKind::Documentation => Self::Documentation,
            wit::SourceKind::Behaviour => Self::Behaviour,
        }
    }
}

impl From<ClaimKind> for wit::ClaimKind {
    fn from(kind: ClaimKind) -> Self {
        match kind {
            ClaimKind::Intent => Self::Intent,
            ClaimKind::Requirement => Self::Requirement,
            ClaimKind::Criterion => Self::Criterion,
            ClaimKind::Decision => Self::Decision,
            ClaimKind::Section => Self::Section,
            ClaimKind::Diagram => Self::Diagram,
            ClaimKind::Contract => Self::Contract,
            ClaimKind::Example => Self::Example,
            ClaimKind::Excerpt => Self::Excerpt,
            ClaimKind::Type => Self::Type,
            ClaimKind::Call => Self::Call,
            ClaimKind::Region => Self::Region,
            ClaimKind::Container => Self::Container,
            ClaimKind::Leaf => Self::Leaf,
        }
    }
}

impl From<wit::ClaimKind> for ClaimKind {
    fn from(kind: wit::ClaimKind) -> Self {
        match kind {
            wit::ClaimKind::Intent => Self::Intent,
            wit::ClaimKind::Requirement => Self::Requirement,
            wit::ClaimKind::Criterion => Self::Criterion,
            wit::ClaimKind::Decision => Self::Decision,
            wit::ClaimKind::Section => Self::Section,
            wit::ClaimKind::Diagram => Self::Diagram,
            wit::ClaimKind::Contract => Self::Contract,
            wit::ClaimKind::Example => Self::Example,
            wit::ClaimKind::Excerpt => Self::Excerpt,
            wit::ClaimKind::Type => Self::Type,
            wit::ClaimKind::Call => Self::Call,
            wit::ClaimKind::Region => Self::Region,
            wit::ClaimKind::Container => Self::Container,
            wit::ClaimKind::Leaf => Self::Leaf,
        }
    }
}

impl From<Backing> for wit::Backing {
    fn from(backing: Backing) -> Self {
        match backing {
            Backing::Payload(payload) => Self::Payload(payload),
            Backing::Path(path) => Self::Path(path),
        }
    }
}

impl From<wit::Backing> for Backing {
    fn from(backing: wit::Backing) -> Self {
        match backing {
            wit::Backing::Payload(payload) => Self::Payload(payload),
            wit::Backing::Path(path) => Self::Path(path),
        }
    }
}

impl From<Claim> for wit::Claim {
    fn from(claim: Claim) -> Self {
        // extras cross the bindings as canonical JSON text
        let extras =
            claim.extras.into_iter().map(|(key, value)| (key, value.to_string())).collect();
        Self {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(Into::into),
            extras,
        }
    }
}

impl TryFrom<wit::Claim> for Claim {
    type Error = String;

    fn try_from(claim: wit::Claim) -> Result<Self, String> {
        let extras = claim
            .extras
            .into_iter()
            .map(|(key, encoded)| match serde_json::from_str(&encoded) {
                Ok(value) => Ok((key, value)),
                Err(err) => Err(format!("extra `{key}` is not canonical JSON ({err}): {encoded}")),
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(Into::into),
            extras,
        })
    }
}

impl From<Evidence> for wit::Evidence {
    fn from(evidence: Evidence) -> Self {
        Self {
            claims: evidence.claims.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<wit::Evidence> for Evidence {
    type Error = String;

    fn try_from(evidence: wit::Evidence) -> Result<Self, String> {
        Ok(Self {
            claims: evidence.claims.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
        })
    }
}

// The WIT variant carries the description alone; the lift in `Source::extract`
// restores the class.
impl From<omnia_sdk::Error> for wit::Error {
    fn from(error: omnia_sdk::Error) -> Self {
        let description = error.description();
        match error {
            omnia_sdk::Error::BadRequest { .. } | omnia_sdk::Error::NotFound { .. } => {
                Self::InvalidRequest(description)
            }
            omnia_sdk::Error::ServerError { .. } | omnia_sdk::Error::BadGateway { .. } => {
                Self::Internal(description)
            }
        }
    }
}

/// The WebAssembly guest interface implemented by a source adapter.
///
/// This module exposes the generated `Guest` trait, its records, and the
/// `export!` macro.
pub mod export {
    // The root glob carries the bindgen support items the `export!` macro
    // expands against; the second names the world's records and `Guest`.
    pub use super::generated::exports::emery::adapter::source::*;
    pub use super::generated::*;
}

/// The WebAssembly client used to invoke a loaded source adapter.
pub mod import {
    use omnia_sdk::{Error, bad_gateway, bad_request};

    use super::generated::emery::adapter::source as imported;
    use super::wit;
    use crate::source::{AdapterMetadata, Evidence, SourceInput};

    /// Returns the metadata the adapter registered as `id` declares.
    #[must_use]
    pub fn metadata(id: &str) -> AdapterMetadata {
        imported::metadata(id).into()
    }

    /// Extracts evidence from `input` using the adapter registered as `id`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects the input.
    /// - Returns [`Error::BadGateway`] when the adapter fails internally or
    ///   returns an extra that is not canonical JSON.
    pub async fn extract(id: &str, input: &SourceInput) -> Result<Evidence, Error> {
        let answer = imported::extract(id.to_string(), input.clone().into()).await.map_err(
            |err| match err {
                wit::Error::InvalidRequest(detail) => bad_request!("source `{id}`: {detail}"),
                wit::Error::Internal(detail) => bad_gateway!("source `{id}`: {detail}"),
            },
        )?;
        Evidence::try_from(answer).map_err(|detail| bad_gateway!("source `{id}`: {detail}"))
    }
}
