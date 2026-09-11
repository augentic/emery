//! WIT bindings
//!
//! The generated Rust bindings for the `source-adapter` WIT world, plus the
//! conversions between the generated wire records and the contract types the
//! rest of the workspace uses.
//!
//! Both directions come from one generation: adapters export through it via
//! the SDK's `source!` macro over [`export`], and the engine guest calls into
//! it through [`import`]. The records live in the WIT `types` interface, so
//! the export side and the caller side bind the same Rust types and each
//! conversion is written once, here at the module root — `From` where the
//! wire form always lifts, `TryFrom` where an extra's canonical JSON must
//! parse.
//!
//! The WIT `error` variant lives here alone: an adapter's `omnia_guest::Error`
//! is lowered onto it for [`export`], and [`import::extract`] lifts it back
//! into the same classes, so neither side of the seam names the wire variant.

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
        // The WIT marks `extract` alone as `async func`, so the bindings need
        // no `async:` list here.
        generate_all,
        pub_export_macro: true,
    });
}

use self::generated::emery::adapter::types as wit;
use crate::types;

impl From<types::AdapterMetadata> for wit::AdapterMetadata {
    fn from(metadata: types::AdapterMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
        }
    }
}

impl From<wit::AdapterMetadata> for types::AdapterMetadata {
    fn from(metadata: wit::AdapterMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
        }
    }
}

impl From<types::SourceContent> for wit::Content {
    fn from(content: types::SourceContent) -> Self {
        match content {
            types::SourceContent::Workspace(root) => Self::Workspace(root),
            types::SourceContent::Value(value) => Self::Value(value),
        }
    }
}

impl From<wit::Content> for types::SourceContent {
    fn from(content: wit::Content) -> Self {
        match content {
            wit::Content::Workspace(root) => Self::Workspace(root),
            wit::Content::Value(value) => Self::Value(value),
        }
    }
}

impl From<types::SourceInput> for wit::Input {
    fn from(input: types::SourceInput) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
        }
    }
}

impl From<wit::Input> for types::SourceInput {
    fn from(input: wit::Input) -> Self {
        Self {
            key: input.key,
            content: input.content.into(),
        }
    }
}

impl From<types::Authority> for wit::Authority {
    fn from(authority: types::Authority) -> Self {
        match authority {
            types::Authority::Intent => Self::Intent,
            types::Authority::Documentation => Self::Documentation,
            types::Authority::Behaviour => Self::Behaviour,
        }
    }
}

impl From<wit::Authority> for types::Authority {
    fn from(authority: wit::Authority) -> Self {
        match authority {
            wit::Authority::Intent => Self::Intent,
            wit::Authority::Documentation => Self::Documentation,
            wit::Authority::Behaviour => Self::Behaviour,
        }
    }
}

impl From<types::ClaimKind> for wit::ClaimKind {
    fn from(kind: types::ClaimKind) -> Self {
        match kind {
            types::ClaimKind::Intent => Self::Intent,
            types::ClaimKind::Requirement => Self::Requirement,
            types::ClaimKind::Criterion => Self::Criterion,
            types::ClaimKind::Decision => Self::Decision,
            types::ClaimKind::Section => Self::Section,
            types::ClaimKind::Diagram => Self::Diagram,
            types::ClaimKind::Contract => Self::Contract,
            types::ClaimKind::Example => Self::Example,
            types::ClaimKind::Excerpt => Self::Excerpt,
            types::ClaimKind::Type => Self::Type,
            types::ClaimKind::Call => Self::Call,
            types::ClaimKind::Region => Self::Region,
            types::ClaimKind::Container => Self::Container,
            types::ClaimKind::Leaf => Self::Leaf,
        }
    }
}

impl From<wit::ClaimKind> for types::ClaimKind {
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

impl From<types::Backing> for wit::Backing {
    fn from(backing: types::Backing) -> Self {
        match backing {
            types::Backing::Payload(payload) => Self::Payload(payload),
            types::Backing::Path(path) => Self::Path(path),
        }
    }
}

impl From<wit::Backing> for types::Backing {
    fn from(backing: wit::Backing) -> Self {
        match backing {
            wit::Backing::Payload(payload) => Self::Payload(payload),
            wit::Backing::Path(path) => Self::Path(path),
        }
    }
}

impl From<types::Claim> for wit::Claim {
    fn from(claim: types::Claim) -> Self {
        // Open body fields ride the wire as canonical JSON text (A8);
        // `serde_json::Value` always encodes.
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

// Lifts a claim off the wire, parsing each extra back from its canonical
// JSON (A8); an extra that fails to parse is a typed error rather than a
// dropped key.
impl TryFrom<wit::Claim> for types::Claim {
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

impl From<types::Evidence> for wit::Evidence {
    fn from(evidence: types::Evidence) -> Self {
        Self {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<wit::Evidence> for types::Evidence {
    type Error = String;

    fn try_from(evidence: wit::Evidence) -> Result<Self, String> {
        Ok(Self {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(TryInto::try_into).collect::<Result<_, _>>()?,
        })
    }
}

// Lowers an adapter failure onto the wire, which carries the description
// alone: a refusal of the input becomes `invalid-request`, every other
// class `internal`; the lift restores the class. `io` is lifted but never
// produced.
impl From<omnia_guest::Error> for wit::Error {
    fn from(error: omnia_guest::Error) -> Self {
        let description = error.description();
        match error {
            omnia_guest::Error::BadRequest { .. } | omnia_guest::Error::NotFound { .. } => {
                Self::InvalidRequest(description)
            }
            omnia_guest::Error::ServerError { .. } | omnia_guest::Error::BadGateway { .. } => {
                Self::Internal(description)
            }
        }
    }
}

/// The export side: the bindings an adapter's `source!` macro wires into.
pub mod export {
    // The root glob carries the bindgen support items the `export!` macro
    // expands against; the second names the world's records and `Guest`.
    pub use super::generated::exports::emery::adapter::source::*;
    pub use super::generated::*;
}

/// The import side: the engine guest's caller over the wire.
pub mod import {
    use omnia_guest::{Error, bad_gateway, bad_request};

    use super::generated::emery::adapter::source as imported;
    use super::wit;
    use crate::types;

    /// Returns resolve-time metadata for `id`.
    #[must_use]
    pub fn metadata(id: &str) -> types::AdapterMetadata {
        imported::metadata(id).into()
    }

    /// Dispatches `extract` to `id`.
    ///
    /// # Errors
    ///
    /// An adapter refusing its input is `BadRequest`; any other adapter
    /// failure, or an extra that is not canonical JSON, is `BadGateway`.
    pub async fn extract(id: &str, input: &types::SourceInput) -> Result<types::Evidence, Error> {
        let answer = imported::extract(id.to_string(), input.clone().into()).await.map_err(
            |err| match err {
                wit::Error::InvalidRequest(detail) => bad_request!("source `{id}`: {detail}"),
                wit::Error::Io(detail) | wit::Error::Internal(detail) => {
                    bad_gateway!("source `{id}`: {detail}")
                }
            },
        )?;
        types::Evidence::try_from(answer).map_err(|detail| bad_gateway!("source `{id}`: {detail}"))
    }
}
