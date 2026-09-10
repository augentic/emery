//! WIT bindings
//!
//! The generated Rust bindings for the `source-adapter` WIT world, plus the
//! conversions between the generated wire records and the contract types the
//! rest of the workspace uses.
//!
//! Both directions come from one generation: adapters export through it via
//! the SDK's `source!` macro over [`export`], and the engine guest calls into
//! it through [`import`]. A single generation guarantees the two sides agree
//! on the wire shape by construction.
//!
//! The WIT `error` variant lives here alone: an adapter's `omnia_guest::Error`
//! is lowered onto it in [`export`], and [`import::extract`] lifts it back into
//! the same classes, so neither side of the seam names the wire variant.

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

/// The export side: the bindings an adapter's `source!` macro wires into,
/// and the lowering of contract types onto its records.
pub mod export {
    // The root glob carries the bindgen support items the `export!` macro
    // expands against; the second names the world's records and `Guest`.
    pub use super::generated::exports::emery::adapter::source::*;
    pub use super::generated::*;
    use crate::types;

    impl From<types::AdapterMetadata> for AdapterMetadata {
        fn from(metadata: types::AdapterMetadata) -> Self {
            Self {
                emery_version: metadata.emery_version,
            }
        }
    }

    impl From<types::SourceContent> for Content {
        fn from(content: types::SourceContent) -> Self {
            match content {
                types::SourceContent::Workspace(root) => Self::Workspace(root),
                types::SourceContent::Value(value) => Self::Value(value),
            }
        }
    }

    impl From<Content> for types::SourceContent {
        fn from(content: Content) -> Self {
            match content {
                Content::Workspace(root) => Self::Workspace(root),
                Content::Value(value) => Self::Value(value),
            }
        }
    }

    impl From<types::SourceInput> for Input {
        fn from(input: types::SourceInput) -> Self {
            Self {
                key: input.key,
                content: input.content.into(),
            }
        }
    }

    impl From<Input> for types::SourceInput {
        fn from(input: Input) -> Self {
            Self {
                key: input.key,
                content: input.content.into(),
            }
        }
    }

    impl From<types::Authority> for Authority {
        fn from(authority: types::Authority) -> Self {
            match authority {
                types::Authority::Intent => Self::Intent,
                types::Authority::Documentation => Self::Documentation,
                types::Authority::Behaviour => Self::Behaviour,
            }
        }
    }

    impl From<types::ClaimKind> for ClaimKind {
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

    impl From<types::Backing> for Backing {
        fn from(backing: types::Backing) -> Self {
            match backing {
                types::Backing::Payload(payload) => Self::Payload(payload),
                types::Backing::Path(path) => Self::Path(path),
            }
        }
    }

    impl From<types::Claim> for Claim {
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

    impl From<types::Evidence> for Evidence {
        fn from(evidence: types::Evidence) -> Self {
            Self {
                authority: evidence.authority.into(),
                claims: evidence.claims.into_iter().map(Into::into).collect(),
            }
        }
    }

    // Lowers an adapter failure onto the wire, which carries the description
    // alone: a refusal of the input becomes `invalid-request`, every other
    // class `internal`; the lift restores the class. `io` is lifted but never
    // produced.
    impl From<omnia_guest::Error> for Error {
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
}

/// The import side: the engine guest's caller, and the lifting of its records
/// into contract types.
pub mod import {
    use omnia_guest::{Error, bad_gateway, bad_request};

    use super::generated::emery::adapter::source as imported;
    use crate::types;

    /// Returns resolve-time metadata for `id`.
    #[must_use]
    pub fn metadata(id: &str) -> types::AdapterMetadata {
        let record = imported::metadata(id);
        types::AdapterMetadata {
            emery_version: record.emery_version,
        }
    }

    /// Dispatches `extract` to `id`.
    ///
    /// Open extras are parsed from their canonical JSON (A8); an extra that
    /// fails to parse is a typed error rather than a dropped key.
    ///
    /// # Errors
    ///
    /// An adapter refusing its input is `BadRequest`; any other adapter
    /// failure, or an extra that is not canonical JSON, is `BadGateway`.
    pub async fn extract(id: &str, input: &types::SourceInput) -> Result<types::Evidence, Error> {
        let answer = imported::extract(id.to_string(), input.clone().into()).await.map_err(
            |err| match err {
                imported::Error::InvalidRequest(detail) => bad_request!("source `{id}`: {detail}"),
                imported::Error::Io(detail) | imported::Error::Internal(detail) => {
                    bad_gateway!("source `{id}`: {detail}")
                }
            },
        )?;
        evidence(answer).map_err(|detail| bad_gateway!("source `{id}`: {detail}"))
    }

    impl From<types::SourceContent> for imported::Content {
        fn from(content: types::SourceContent) -> Self {
            match content {
                types::SourceContent::Workspace(root) => Self::Workspace(root),
                types::SourceContent::Value(value) => Self::Value(value),
            }
        }
    }

    impl From<types::SourceInput> for imported::Input {
        fn from(input: types::SourceInput) -> Self {
            Self {
                key: input.key,
                content: input.content.into(),
            }
        }
    }

    impl From<imported::Authority> for types::Authority {
        fn from(authority: imported::Authority) -> Self {
            match authority {
                imported::Authority::Intent => Self::Intent,
                imported::Authority::Documentation => Self::Documentation,
                imported::Authority::Behaviour => Self::Behaviour,
            }
        }
    }

    impl From<imported::ClaimKind> for types::ClaimKind {
        fn from(kind: imported::ClaimKind) -> Self {
            match kind {
                imported::ClaimKind::Intent => Self::Intent,
                imported::ClaimKind::Requirement => Self::Requirement,
                imported::ClaimKind::Criterion => Self::Criterion,
                imported::ClaimKind::Decision => Self::Decision,
                imported::ClaimKind::Section => Self::Section,
                imported::ClaimKind::Diagram => Self::Diagram,
                imported::ClaimKind::Contract => Self::Contract,
                imported::ClaimKind::Example => Self::Example,
                imported::ClaimKind::Excerpt => Self::Excerpt,
                imported::ClaimKind::Type => Self::Type,
                imported::ClaimKind::Call => Self::Call,
                imported::ClaimKind::Region => Self::Region,
                imported::ClaimKind::Container => Self::Container,
                imported::ClaimKind::Leaf => Self::Leaf,
            }
        }
    }

    impl From<imported::Backing> for types::Backing {
        fn from(backing: imported::Backing) -> Self {
            match backing {
                imported::Backing::Payload(payload) => Self::Payload(payload),
                imported::Backing::Path(path) => Self::Path(path),
            }
        }
    }

    fn evidence(evidence: imported::Evidence) -> Result<types::Evidence, String> {
        Ok(types::Evidence {
            authority: evidence.authority.into(),
            claims: evidence.claims.into_iter().map(claim).collect::<Result<_, _>>()?,
        })
    }

    fn claim(claim: imported::Claim) -> Result<types::Claim, String> {
        let mut extras = serde_json::Map::new();
        for (key, encoded) in claim.extras {
            let value = serde_json::from_str(&encoded)
                .map_err(|err| format!("extra `{key}` is not canonical JSON ({err}): {encoded}"))?;
            extras.insert(key, value);
        }
        Ok(types::Claim {
            kind: claim.kind.into(),
            id: claim.id,
            path: claim.path,
            synopsis: claim.synopsis,
            backing: claim.backing.map(Into::into),
            extras,
        })
    }
}
