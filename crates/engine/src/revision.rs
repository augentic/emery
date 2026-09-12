//! # The revision
//!
//! The typed specification and design a `specify` run commits, serialised as
//! canonical JSON and identified by the digest of those bytes. The two
//! Markdown documents an operator reads, `spec.md` and `design.md`, are
//! projections rendered from the revision on demand, so a stored revision is
//! never parsed back from prose.
//!
//! This module carries the model both documents share — the revision, its
//! id, the [`Document`] contract each meets (a name, the canonical bytes,
//! the projection), and the [`Diff`] between two revisions — together with
//! the vocabulary the renderer and the drafting checks agree on: the heading
//! markers, the provenance and note keys, and the line openers a drafted
//! paragraph may not use because the renderer owns them.

mod design;
mod diff;
mod spec;

use std::fmt::{self, Display, Formatter};

use anyhow::Context;
use omnia_guest::{Error, server_error};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};

use self::design::TYPE;
pub use self::design::{Block, Design, Section, SectionKind, citations};
pub use self::diff::{Changed, DesignDiff, Diff, Entry, SpecDiff};
pub use self::spec::{Cited, Loser, ReqId, Requirement, Scenario, Spec, Status};
use self::spec::{ID, NOTE, SOURCES, STATUS};

/// The grammar this engine writes and reads; a stored revision stamped with
/// another is outdated.
pub const EMERY: u32 = 2;

/// Line openers a drafted paragraph may not use: `#`, so no draft line reads
/// as a heading (a `## ` section, `HEADING`, `SCENARIO`); and the engine's own
/// line keys, so no draft line passes as provenance, a note, or a type label.
pub const RESERVED: &[&str] = &["#", ID, SOURCES, STATUS, NOTE, TYPE];

/// One document of a revision: a serde shape under the [`EMERY`] stamp that
/// the projection renders by `Display`.
pub trait Document: Serialize + DeserializeOwned + Display {
    /// This document's name (`spec`, `design`).
    const NAME: &'static str;

    /// Reads one document from its stored JSON, refusing another grammar's
    /// before the shape is checked.
    ///
    /// # Errors
    ///
    /// `spec-outdated` when the `emery` stamp is missing or another
    /// grammar's; `server_error` when the bytes are not JSON or a document
    /// under this grammar does not fit, since this engine did not write it.
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

    /// The canonical JSON the store hashes and writes: pretty, declaration
    /// order, one trailing newline.
    ///
    /// # Errors
    ///
    /// `server_error` when the document does not serialise — this engine
    /// built it, so a failure is a defect, not a revision.
    fn to_json(&self) -> Result<String, Error> {
        let mut text = serde_json::to_string_pretty(self)
            .with_context(|| format!("`{}` does not serialise", Self::NAME))?;
        text.push('\n');
        Ok(text)
    }

    /// The Markdown projection: the front matter stamping the grammar and
    /// `revision` — the id of the revision this document belongs to, which
    /// the caller has already computed — then the document body.
    #[must_use]
    fn to_markdown(&self, id: &str) -> String {
        format!("---\nemery: {EMERY}\nrevision: {id}\n---\n\n{self}")
    }
}

/// The specification and design one `specify` run produces, committed under
/// the id of their canonical bytes.
///
/// The id is a function of the content alone, so identical runs are
/// byte-stable and a revision read back from storage is verified against the
/// id it was stored under.
#[derive(Debug)]
pub struct Revision {
    /// The behavioural specification.
    pub spec: Spec,
    /// The rebuild design.
    pub design: Design,
}

impl Revision {
    /// Reads the revision stored under `id` from the bytes of its two
    /// documents, refusing bytes that no longer hash to the id before either
    /// document is read.
    ///
    /// # Errors
    ///
    /// `server_error` when the bytes do not match the id — the store is
    /// content-addressed, so a mismatch is corruption, not a revision — then
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

    /// The content id: the digest of the specification and design bytes, in
    /// that order.
    ///
    /// # Errors
    ///
    /// `server_error` when a document does not serialise.
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
