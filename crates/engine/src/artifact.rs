//! # The revision artifacts
//!
//! The two documents a `specify` run commits, `spec.md` and `design.md`, are
//! rendered by the engine from validated drafts, so a stored document is
//! canonical output. Together they are the dossier a revision commits under
//! the id of its content. This module reads them back — for the re-mine diff,
//! which compares two revisions by requirement subject and section heading —
//! and carries the vocabulary the renderer and the reader share: the heading
//! markers, the provenance and note keys, the positional requirement id, the
//! closed status and tag sets, and the closed section vocabulary — and, drawn
//! from it, the line openers a drafted paragraph may not use.
//!
//! A stored document that does not fit its grammar was not rendered by this
//! engine; the reader reports it as corruption, not as a grammar finding.

mod design;
mod spec;

use std::ops::Deref;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strum::VariantArray as _;

pub use self::design::{Design, SectionKind, citations};
pub use self::spec::{HEADING, ID, NOTE, ReqId, SCENARIO, SOURCES, STATUS, Spec, Status};

/// Line openers a drafted paragraph may not use: `#`, so no draft line reads
/// as a heading (a `## ` section, `HEADING`, `SCENARIO`) and splits a block on
/// read; and the engine's own line keys, so no draft line passes as
/// provenance or a note.
pub const RESERVED: &[&str] = &["#", ID, SOURCES, STATUS, NOTE];

/// The reviewable documents of one revision, in digest order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::VariantArray)]
#[serde(rename_all = "kebab-case")]
pub enum Document {
    /// The behavioural specification document.
    Spec,
    /// The rebuild design document.
    Design,
}

impl Document {
    /// The document's file name in the revision store.
    #[must_use]
    pub const fn file(self) -> &'static str {
        match self {
            Self::Spec => "spec.md",
            Self::Design => "design.md",
        }
    }
}

/// The documents one `specify` run produces, which a revision commits under
/// the id of their content.
///
/// The id is a function of the documents alone, so identical runs are
/// byte-stable and a dossier read back from storage is verified against the
/// id it was stored under.
#[derive(Debug)]
pub struct Dossier {
    /// The behavioural specification document.
    pub spec: String,
    /// The rebuild design document.
    pub design: String,
}

impl Dossier {
    /// Computes the revision id this dossier commits as: SHA-256 over the
    /// length-prefixed document names and bodies, in digest order.
    #[must_use]
    pub fn revision(&self) -> String {
        let mut hasher = Sha256::new();
        for (name, body) in self.files() {
            hasher.update((name.len() as u64).to_be_bytes());
            hasher.update(name.as_bytes());
            hasher.update((body.len() as u64).to_be_bytes());
            hasher.update(body.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Consumes the dossier for one document's body.
    #[must_use]
    pub fn into_body(self, document: Document) -> String {
        match document {
            Document::Spec => self.spec,
            Document::Design => self.design,
        }
    }

    /// Pairs each document's file name with its body, in digest order.
    pub fn files(&self) -> impl Iterator<Item = (&'static str, &str)> {
        Document::VARIANTS.iter().map(|document| (document.file(), self.body(*document)))
    }

    // Selects the field that holds `document`'s body.
    fn body(&self, document: Document) -> &str {
        match document {
            Document::Spec => &self.spec,
            Document::Design => &self.design,
        }
    }
}

/// A document's text as right-trimmed lines.
#[derive(Debug)]
pub struct Text<'a> {
    lines: Vec<Line<'a>>,
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(text: &'a str) -> Self {
        let lines = text.lines().map(|raw| Line(raw.trim_end())).collect();
        Self { lines }
    }
}

impl<'a> Text<'a> {
    /// Splits the text into blocks, one per `marker` heading, yielding the
    /// heading line with its marker stripped and then the body lines. The
    /// preamble before the first heading is skipped.
    pub fn blocks(&'a self, marker: &'a str) -> impl Iterator<Item = (Line<'a>, Lines<'a>)> {
        self.lines.chunk_by(move |_, next| next.heading(marker).is_none()).filter_map(move |run| {
            let [first, body @ ..] = run else { return None };
            Some((first.heading(marker)?, Lines(body)))
        })
    }
}

/// One right-trimmed line.
#[derive(Debug, Clone, Copy)]
pub struct Line<'a>(pub &'a str);

impl Line<'_> {
    /// Whether the line is empty.
    #[must_use]
    pub const fn is_blank(self) -> bool {
        self.0.is_empty()
    }

    // Strips `marker` from a heading line and returns the trimmed heading
    // text; `None` for any other line.
    fn heading(self, marker: &str) -> Option<Self> {
        Some(Self(self.0.strip_prefix(marker)?.trim()))
    }
}

/// A run of lines; derefs to the slice.
#[derive(Debug, Clone, Copy)]
pub struct Lines<'a>(pub &'a [Line<'a>]);

impl<'a> Deref for Lines<'a> {
    type Target = [Line<'a>];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Lines<'_> {
    /// Joins the lines into one text with its blank edges trimmed.
    #[must_use]
    pub fn text(self) -> String {
        let text: Vec<&str> = self.iter().map(|line| line.0).collect();
        text.join("\n").trim_matches('\n').to_string()
    }
}
