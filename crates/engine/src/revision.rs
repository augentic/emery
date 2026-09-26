//! Defines stored revisions and their Markdown projections.
//!
//! A [`Revision`] stores a typed specification and design as canonical JSON.
//! Its content digest is the revision identifier. Markdown is rendered on
//! demand and is never parsed back into revision data.
//!
//! [`Document`] defines shared serialisation and rendering behaviour.
//! [`Diff`] describes changes between revisions.

mod design;
mod diff;
mod spec;

use std::fmt::{self, Display, Formatter};

use anyhow::Context;
use omnia_sdk::{Error, server_error};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};

use self::design::TYPE;
pub use self::design::{Block, Design, Section, SectionKind, citations};
pub use self::diff::{Changed, DesignDiff, Diff, Entry, SpecDiff};
pub use self::spec::{Cited, Loser, ReqId, Requirement, Scenario, Spec, Status};
use self::spec::{ID, NOTE, SOURCES, STATUS};

/// The revision grammar written and accepted by this engine.
///
/// A stored revision stamped with another grammar is outdated.
pub const EMERY: u32 = 2;

/// Line prefixes reserved for engine-generated Markdown.
///
/// `#`, so no draft line reads as a heading, and the engine's own line keys,
/// so no draft line passes as provenance, a note, or a type label.
pub const RESERVED: &[&str] = &["#", ID, SOURCES, STATUS, NOTE, TYPE];

/// A typed revision document stored as JSON and rendered as Markdown.
pub trait Document: Serialize + DeserializeOwned + Display {
    /// The storage and projection name of the document.
    const NAME: &'static str;

    /// Reads the document from its stored JSON.
    ///
    /// The grammar stamp is checked before the shape, so another grammar's
    /// document is reported as outdated rather than malformed.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] with code `spec-outdated` when the
    ///   `emery` stamp is missing or uses another grammar.
    /// - Returns [`Error::ServerError`] when the bytes are not valid JSON or
    ///   do not match the document shape for the current grammar.
    fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        let value: Value = serde_json::from_slice(bytes)
            .with_context(|| format!("`{}` is not JSON", Self::NAME))?;

        // check the grammar stamp before the shape
        let stamp = &value["emery"];
        if *stamp != EMERY {
            return Err(Error::BadRequest {
                code: "spec-outdated".into(),
                description: format!(
                    "`{}` was written under emery grammar {stamp}; this engine reads {EMERY}",
                    Self::NAME
                ),
            });
        }

        Ok(serde_json::from_value(value)
            .with_context(|| format!("`{}` is not a revision", Self::NAME))?)
    }

    /// Returns the canonical JSON the store hashes and writes.
    ///
    /// Pretty-printed, in declaration order, with one trailing newline.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the document cannot be serialised.
    fn to_json(&self) -> Result<String, Error> {
        let mut text = serde_json::to_string_pretty(self)
            .with_context(|| format!("`{}` does not serialise", Self::NAME))?;
        text.push('\n');
        Ok(text)
    }

    /// Returns the Markdown projection for revision `id`.
    ///
    /// The projection begins with front matter containing the grammar and
    /// revision identifier.
    #[must_use]
    fn to_markdown(&self, id: &str) -> String {
        format!("---\nemery: {EMERY}\nrevision: {id}\n---\n\n{self}")
    }
}

/// The specification and design committed as one unit.
///
/// The identifier depends only on canonical document content. Reading a
/// revision verifies its bytes against that identifier.
#[derive(Debug)]
pub struct Revision {
    /// The behavioural specification.
    pub spec: Spec,
    /// The rebuild design.
    pub design: Design,
}

impl Revision {
    /// Reads and validates the revision stored under `id`.
    ///
    /// The content digest is checked before either document is deserialised.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] with code `spec-outdated` when either
    ///   document uses a different grammar.
    /// - Returns [`Error::ServerError`] when the content does not match `id`,
    ///   is not valid JSON, or does not match the current document shape.
    pub fn read(id: &str, spec: &[u8], design: &[u8]) -> Result<Self, Error> {
        if digest(spec, design) != id {
            return Err(server_error!("revision `{id}` does not match its content"));
        }

        Ok(Self {
            spec: Spec::from_json(spec)?,
            design: Design::from_json(design)?,
        })
    }

    /// Returns the content identifier for this revision.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when a document does not serialise.
    pub fn id(&self) -> Result<String, Error> {
        Ok(digest(self.spec.to_json()?.as_bytes(), self.design.to_json()?.as_bytes()))
    }
}

fn digest(spec: &[u8], design: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for body in [spec, design] {
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}

fn write<T: Display>(
    f: &mut Formatter<'_>, title: &str, preamble: &[String], blocks: &[T],
) -> fmt::Result {
    write!(f, "# {title}")?;
    for paragraph in preamble {
        write!(f, "\n\n{paragraph}")?;
    }
    for block in blocks {
        write!(f, "\n\n{block}")?;
    }
    f.write_str("\n")
}
