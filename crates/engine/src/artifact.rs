//! # The revision artifacts
//!
//! The master of one revision: the typed specification and design a `specify`
//! run commits, serialised as canonical JSON and identified by the digest of
//! those bytes. The two Markdown documents an operator reads, `spec.md` and
//! `design.md`, are projections rendered from the master on demand, so a
//! stored revision is never parsed back from prose.
//!
//! This module carries the model both documents share — the dossier, its
//! revision id, and the projection renderer — together with the vocabulary the
//! renderer and the drafting checks agree on: the heading markers, the
//! provenance and note keys, and the line openers a drafted paragraph may not
//! use because the renderer owns them.

mod design;
mod spec;

use omnia_guest::Error;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use strum::VariantArray as _;

pub use self::design::{Block, Design, Section, SectionKind, TYPE, citations};
pub use self::spec::{
    Cited, ID, Loser, NOTE, ReqId, Requirement, SOURCES, STATUS, Scenario, Spec, Status,
};

/// The master grammar this engine writes and reads; a stored or carried
/// master stamped with another is outdated.
pub const EMERY: u32 = 2;

/// Line openers a drafted paragraph may not use: `#`, so no draft line reads
/// as a heading (a `## ` section, `HEADING`, `SCENARIO`); and the engine's own
/// line keys, so no draft line passes as provenance, a note, or a type label.
pub const RESERVED: &[&str] = &["#", ID, SOURCES, STATUS, NOTE, TYPE];

/// The two documents of one revision, in digest order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::VariantArray)]
#[serde(rename_all = "kebab-case")]
pub enum Document {
    /// The behavioural specification.
    Spec,
    /// The rebuild design.
    Design,
}

impl Document {
    /// The master's file name in the revision store.
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

/// The master one `specify` run produces, which a revision commits under the
/// id of its canonical bytes.
///
/// The id is a function of the content alone, so identical runs are
/// byte-stable and a dossier read back from storage is verified against the
/// id it was stored under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dossier {
    /// The behavioural specification.
    pub spec: Spec,
    /// The rebuild design.
    pub design: Design,
}

impl Dossier {
    /// Reads a dossier from its two JSON documents, refusing another
    /// grammar's before the shape is checked.
    ///
    /// # Errors
    ///
    /// `spec-outdated` when either document's `emery` stamp is missing or
    /// another grammar's; `master-invalid` when a document does not fit the
    /// master.
    pub fn read(spec: Value, design: Value) -> Result<Self, Error> {
        Ok(Self {
            spec: stamped(spec, Document::Spec)?,
            design: stamped(design, Document::Design)?,
        })
    }

    /// Computes the revision id this dossier commits as: the digest of its
    /// canonical files.
    #[must_use]
    pub fn revision(&self) -> String {
        digest(self.files().iter().map(|(name, body)| (*name, body.as_bytes())))
    }

    /// Serialises each document as canonical JSON under its file name, in
    /// digest order.
    #[must_use]
    pub fn files(&self) -> Vec<(&'static str, String)> {
        Document::VARIANTS
            .iter()
            .map(|document| (document.file(), self.canonical(*document)))
            .collect()
    }

    /// Renders `document`'s Markdown projection: the front matter stamping
    /// the grammar and the revision id, then the document body.
    #[must_use]
    pub fn render(&self, document: Document) -> String {
        let body = match document {
            Document::Spec => self.spec.to_string(),
            Document::Design => self.design.to_string(),
        };
        format!("---\nemery: {EMERY}\nrevision: {}\n---\n\n{body}", self.revision())
    }

    // The one canonical form: pretty JSON in declaration order, one trailing
    // newline. Both the hash and the store write read it from here.
    fn canonical(&self, document: Document) -> String {
        let mut text = match document {
            Document::Spec => serde_json::to_string_pretty(&self.spec),
            Document::Design => serde_json::to_string_pretty(&self.design),
        }
        .expect("the master serialises: no maps with non-string keys, no floats");
        text.push('\n');
        text
    }
}

// Deserialises one master after checking its grammar stamp: the stamp is
// the one field every grammar shares, so it is read before the shape.
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

    serde_json::from_value(value).map_err(|err| Error::BadRequest {
        code: "master-invalid".into(),
        description: format!("`{name}` is not a master: {err}"),
    })
}

/// Hashes stored files as SHA-256 over the length-prefixed names and bodies,
/// in the order given; the revision id of the dossier whose files they are.
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

// A projection under construction: the blocks the renderer emits in order,
// joined by one blank line, every line right-trimmed, one trailing newline.
struct Markdown(Vec<String>);

impl Markdown {
    fn new(title: &str) -> Self {
        Self(vec![format!("# {title}")])
    }

    // Adds one block, every line right-trimmed; `finish` joins the blocks.
    fn push(&mut self, text: impl Into<String>) {
        self.0.push(text.into().lines().map(str::trim_end).collect::<Vec<_>>().join("\n"));
    }

    fn extend(&mut self, texts: &[String]) {
        for text in texts {
            self.push(text);
        }
    }

    fn finish(self) -> String {
        let mut text = self.0.join("\n\n");
        text.push('\n');
        text
    }
}
