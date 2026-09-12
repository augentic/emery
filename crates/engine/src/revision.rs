//! # The revision artifacts
//!
//! The typed specification and design a `specify` run commits, serialised as
//! canonical JSON and identified by the digest of those bytes. The two
//! Markdown documents an operator reads, `spec.md` and `design.md`, are
//! projections rendered from the revision on demand, so a stored revision is
//! never parsed back from prose.
//!
//! This module carries the model both documents share — the revision, its
//! id, and the [`Document`] contract each of its files meets: a fixed file
//! name, the canonical bytes, and the projection — together with the
//! vocabulary the renderer and the drafting checks agree on: the heading
//! markers, the provenance and note keys, and the line openers a drafted
//! paragraph may not use because the renderer owns them.

mod design;
mod spec;

use std::fmt::{self, Display, Formatter};

use omnia_guest::{Error, server_error};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};

use self::design::TYPE;
pub use self::design::{Block, Design, Section, SectionKind, citations};
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
/// the store files under a fixed name and the projection renders by
/// `Display`.
pub trait Document: Serialize + DeserializeOwned + Display {
    /// The file name the store commits the document under.
    const FILE: &'static str;

    /// Deserialises one document from its JSON.
    fn from_json(value: Value) -> Result<Self, Error> {
        if value["emery"] != EMERY {
            return Err(Error::BadRequest {
                code: "spec-outdated".into(),
                description: format!(
                    "`{}` was not written under emery grammar {EMERY}",
                    Self::FILE
                ),
            });
        }
        serde_json::from_value(value)
            .map_err(|err| server_error!("`{}` is not a revision: {err}", Self::FILE))
    }

    /// The canonical JSON the store hashes and writes: pretty, declaration
    /// order, one trailing newline.
    #[must_use]
    fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self)
            .expect("the revision serialises: no maps with non-string keys, no floats");
        text.push('\n');
        text
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
    /// Every file name a revision holds, in digest order.
    pub const FILES: [&'static str; 2] = [Spec::FILE, Design::FILE];

    /// Reads a revision from its two JSON documents, refusing another
    /// grammar's before the shape is checked.
    ///
    /// # Errors
    ///
    /// `spec-outdated` when either document's `emery` stamp is missing or
    /// another grammar's; `server_error` when a document under this grammar
    /// does not fit the revision, since this engine did not write it.
    pub fn read(spec: Value, design: Value) -> Result<Self, Error> {
        Ok(Self {
            spec: Spec::from_json(spec)?,
            design: Design::from_json(design)?,
        })
    }

    /// Each document as canonical JSON under its file name, in digest order.
    #[must_use]
    pub fn files(&self) -> Vec<(&'static str, String)> {
        vec![(Spec::FILE, self.spec.to_json()), (Design::FILE, self.design.to_json())]
    }

    /// The content id: the digest of every file, in digest order.
    #[must_use]
    pub fn id(&self) -> String {
        let files = self.files();
        digest(files.iter().map(|(name, body)| (*name, body.as_bytes())))
    }
}

// Writes one document body: the title, each preamble paragraph, and each
// typed block, with every line right-trimmed.
fn write<T: Display>(
    f: &mut Formatter<'_>, title: &str, preamble: &[String], blocks: &[T],
) -> fmt::Result {
    write!(f, "# {title}")?;
    for block in preamble {
        write_block(f, block)?;
    }
    for block in blocks {
        write_block(f, &block.to_string())?;
    }
    f.write_str("\n")
}

fn write_block(f: &mut Formatter<'_>, block: &str) -> fmt::Result {
    for (position, line) in block.lines().enumerate() {
        f.write_str(if position == 0 { "\n\n" } else { "\n" })?;
        f.write_str(line.trim_end())?;
    }
    Ok(())
}

// // Deserialises one revision document after checking its grammar stamp: the
// // stamp is the one field every grammar shares, so it is read before the shape.
// fn stamped<D: Document + DeserializeOwned>(value: Value) -> Result<D, Error> {
//     let name = D::FILE;
//     let stamp = &value["emery"];
//     if *stamp != EMERY {
//         return Err(Error::BadRequest {
//             code: "spec-outdated".into(),
//             description: format!(
//                 "`{name}` was written under emery grammar {stamp}; this engine reads {EMERY}"
//             ),
//         });
//     }

//     serde_json::from_value(value).map_err(|err| server_error!("`{name}` is not a revision: {err}"))
// }

/// Hashes stored files as SHA-256 over the length-prefixed names and bodies,
/// in the order given; the id of the revision whose files they are.
pub fn digest<'a>(files: impl Iterator<Item = (&'a str, &'a [u8])>) -> String {
    let mut hasher = Sha256::new();
    for (name, body) in files {
        hasher.update((name.len() as u64).to_be_bytes());
        hasher.update(name.as_bytes());
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}
