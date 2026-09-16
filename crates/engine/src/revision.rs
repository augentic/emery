//! The typed specification and design a run commits.
//!
//! A revision is serialised as canonical JSON and identified by the digest of
//! those bytes. The Markdown an operator reads — `spec.md`, `design.md` — is
//! projected from the revision on demand, so a stored revision is never parsed
//! back from prose.
//!
//! This module carries what both documents share: the [`Revision`] and its
//! id, the [`Document`] contract each meets (a name, the canonical bytes, the
//! projection), the [`Diff`] between two revisions, and the vocabulary the
//! renderer and the drafting checks agree on — the heading markers, the
//! provenance and note keys, and the line openers a drafted paragraph may not
//! use because the renderer owns them.

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

/// The grammar this engine writes and reads.
///
/// A stored revision stamped with another grammar is outdated.
pub const EMERY: u32 = 2;

/// The line openers a drafted paragraph may not use.
///
/// `#`, so no draft line reads as a heading, and the engine's own line keys,
/// so no draft line passes as provenance, a note, or a type label.
pub const RESERVED: &[&str] = &["#", ID, SOURCES, STATUS, NOTE, TYPE];

/// One document of a revision, stored as JSON and projected as Markdown.
pub trait Document: Serialize + DeserializeOwned + Display {
    /// The document's name: `spec` or `design`.
    const NAME: &'static str;

    /// Reads the document from its stored JSON.
    ///
    /// The grammar stamp is checked before the shape, so another grammar's
    /// document is reported as outdated rather than malformed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] with code `spec-outdated` when the `emery`
    /// stamp is missing or another grammar's, and [`Error::ServerError`] when
    /// the bytes are not JSON or a document under this grammar does not fit —
    /// this engine did not write it.
    fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        let value: Value = serde_json::from_slice(bytes)
            .with_context(|| format!("`{}` is not JSON", Self::NAME))?;

        // The stamp is the one field every grammar shares, so it is read
        // before the shape.
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
    /// Returns [`Error::ServerError`] when the document does not serialise;
    /// this engine built it, so a failure is a defect.
    fn to_json(&self) -> Result<String, Error> {
        let mut text = serde_json::to_string_pretty(self)
            .with_context(|| format!("`{}` does not serialise", Self::NAME))?;
        text.push('\n');
        Ok(text)
    }

    /// Renders the Markdown projection, with front matter naming revision `id`.
    ///
    /// `id` is the id of the revision the document belongs to, which the
    /// caller has already computed.
    #[must_use]
    fn to_markdown(&self, id: &str) -> String {
        format!("---\nemery: {EMERY}\nrevision: {id}\n---\n\n{self}")
    }
}

/// The specification and design one `specify` run produces.
///
/// A revision is committed under the id of its canonical bytes. The id is a
/// function of the content alone, so identical runs are byte-stable, and a
/// revision read back from storage is verified against the id it was stored
/// under.
#[derive(Debug)]
pub struct Revision {
    /// The behavioural specification.
    pub spec: Spec,
    /// The rebuild design.
    pub design: Design,
}

impl Revision {
    /// Reads the revision stored under `id` from the bytes of its two documents.
    ///
    /// The bytes are verified against `id` before either document is read.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the bytes do not hash to `id` — the
    /// store is content-addressed, so a mismatch is corruption — and otherwise
    /// each document's own refusals, `spec-outdated` first.
    pub fn read(id: &str, spec: &[u8], design: &[u8]) -> Result<Self, Error> {
        if digest(spec, design) != id {
            return Err(server_error!("revision `{id}` does not match its content"));
        }

        Ok(Self {
            spec: Spec::from_json(spec)?,
            design: Design::from_json(design)?,
        })
    }

    /// Returns the content id: the digest of the specification and design bytes.
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

// Writes one document body: the title, each preamble paragraph, and each
// typed block, one blank line apart.
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
