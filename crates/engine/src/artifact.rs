//! # The revision artifacts
//!
//! The typed specification and design a `specify` run commits, serialised as
//! canonical JSON and identified by the digest of those bytes. The two
//! Markdown documents an operator reads, `spec.md` and `design.md`, are
//! projections rendered from the revision on demand, so a stored revision is
//! never parsed back from prose.
//!
//! This module carries the model both documents share — the revision, its
//! id, and the projection renderer — together with the vocabulary the
//! renderer and the drafting checks agree on: the heading markers, the
//! provenance and note keys, and the line openers a drafted paragraph may not
//! use because the renderer owns them.

mod design;
mod spec;

use omnia_guest::{Error, server_error};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use strum::VariantArray as _;

pub use self::design::{Block, Design, Section, SectionKind, TYPE, citations};
pub use self::spec::{
    Cited, ID, Loser, NOTE, ReqId, Requirement, SOURCES, STATUS, Scenario, Spec, Status,
};

/// The grammar this engine writes and reads; a stored revision stamped with
/// another is outdated.
pub const EMERY: u32 = 2;

/// Line openers a drafted paragraph may not use: `#`, so no draft line reads
/// as a heading (a `## ` section, `HEADING`, `SCENARIO`); and the engine's own
/// line keys, so no draft line passes as provenance, a note, or a type label.
pub const RESERVED: &[&str] = &["#", ID, SOURCES, STATUS, NOTE, TYPE];

/// The two documents of one revision, in digest order. A caller names one by
/// its kebab-case key (`as_ref()` / `parse()`, `spec`), the same spelling
/// serde uses.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::AsRefStr,
    strum::EnumString,
    strum::VariantArray,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Document {
    /// The behavioural specification.
    Spec,
    /// The rebuild design.
    Design,
}

impl Document {
    /// The revision document's file name in the store.
    #[must_use]
    pub const fn file(self) -> &'static str {
        match self {
            Self::Spec => "spec.json",
            Self::Design => "design.json",
        }
    }

    /// The rendered projection's file name.
    #[must_use]
    pub const fn projection(self) -> &'static str {
        match self {
            Self::Spec => "spec.md",
            Self::Design => "design.md",
        }
    }
}

/// The specification and design one `specify` run produces, committed under
/// the id of their canonical bytes.
///
/// The id is a function of the content alone, so identical runs are
/// byte-stable and a revision read back from storage is verified against the
/// id it was stored under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    /// The behavioural specification.
    pub spec: Spec,
    /// The rebuild design.
    pub design: Design,
}

impl Revision {
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
            spec: stamped(spec, Document::Spec)?,
            design: stamped(design, Document::Design)?,
        })
    }

    /// Computes the id this revision commits as: the digest of its canonical
    /// files.
    #[must_use]
    pub fn id(&self) -> String {
        digest(self.files().iter().map(|(document, body)| (document.file(), body.as_bytes())))
    }

    /// Serialises each document as canonical JSON, in digest order: pretty,
    /// declaration order, one trailing newline.
    #[must_use]
    pub fn files(&self) -> Vec<(Document, String)> {
        Document::VARIANTS
            .iter()
            .copied()
            .map(|document| {
                let mut text = match document {
                    Document::Spec => serde_json::to_string_pretty(&self.spec),
                    Document::Design => serde_json::to_string_pretty(&self.design),
                }
                .expect("the revision serialises: no maps with non-string keys, no floats");
                text.push('\n');
                (document, text)
            })
            .collect()
    }

    /// Renders `document`'s Markdown projection: the front matter stamping
    /// the grammar and `revision` — this revision's id, which the caller has
    /// already computed — then the document body.
    #[must_use]
    pub fn render(&self, document: Document, id: &str) -> String {
        let body = match document {
            Document::Spec => self.spec.to_string(),
            Document::Design => self.design.to_string(),
        };
        format!("---\nemery: {EMERY}\nrevision: {id}\n---\n\n{body}")
    }
}

// Deserialises one revision document after checking its grammar stamp: the
// stamp is the one field every grammar shares, so it is read before the shape.
fn stamped<T: DeserializeOwned>(value: Value, document: Document) -> Result<T, Error> {
    let name = document.file();
    let stamp = &value["emery"];
    if *stamp != EMERY {
        return Err(Error::BadRequest {
            code: "spec-outdated".into(),
            description: format!(
                "`{name}` was written under emery grammar {stamp}; this engine reads {EMERY}"
            ),
        });
    }

    serde_json::from_value(value).map_err(|err| server_error!("`{name}` is not a revision: {err}"))
}

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
