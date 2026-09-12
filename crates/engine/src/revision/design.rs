//! # The design
//!
//! The typed form of `design.md`: a preamble and the sections of a closed
//! vocabulary in a fixed order, each a run of drafted paragraphs and the type
//! signatures the engine placed verbatim. `Display` renders the Markdown
//! projection an operator reads.

use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::revision;

const CITATION: &str = "(from ";

/// The `Type:` key: the engine's own line labelling a signature fence.
pub const TYPE: &str = "Type:";

/// The design.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Design {
    /// The grammar the document was written under.
    pub emery: u32,
    /// Markdown paragraphs before the first section.
    pub preamble: Vec<String>,
    /// The sections, in vocabulary order.
    pub sections: Vec<Section>,
}

impl Design {
    /// Finds the section of `kind`.
    #[must_use]
    pub fn section(&self, kind: SectionKind) -> Option<&Section> {
        self.sections.iter().find(|section| section.kind == kind)
    }
}

impl revision::Document for Design {
    const FILE: &'static str = "design.json";
}

// Renders `design.md`: the preamble, then every section under its heading.
impl Display for Design {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        revision::write(f, "Design", &self.preamble, &self.sections)
    }
}

/// One `## ` section: the revision's, over its placed [`Block`]s, or a
/// draft's, over the blocks a draft answers in.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "Section")]
pub struct Section<B = Block> {
    /// The heading, from the closed vocabulary.
    pub kind: SectionKind,
    /// The blocks, in reading order.
    pub blocks: Vec<B>,
}

// Renders the heading, then each block.
impl Display for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "## {}", self.kind)?;
        for block in &self.blocks {
            write!(f, "\n\n{block}")?;
        }
        Ok(())
    }
}

/// One design block: a drafted paragraph, or a `type` claim's signature.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Block {
    /// One Markdown paragraph.
    Text(String),
    /// A `type` claim's signature, placed verbatim.
    Type {
        /// The claim's key.
        key: String,
        /// The claim's signature.
        signature: String,
    },
}

// Writes a drafted paragraph, or a `Type:` fence with the claim's signature.
impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => f.write_str(text),
            Self::Type { key, signature } => {
                write!(f, "{TYPE} {key}\n```\n{}\n```", signature.trim_end())
            }
        }
    }
}

/// The closed `## ` vocabulary, in document order. A draft names a section
/// by its kebab-case key (`as_ref()`, `domain-model`); the document by its
/// title (`Display`, `Domain model`).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    JsonSchema,
    strum::AsRefStr,
    strum::VariantArray,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum SectionKind {
    /// `## Overview`: what the system is and why.
    Overview,
    /// `## Domain model`: types and identifiers.
    DomainModel,
    /// `## APIs and integrations`: external surfaces.
    Apis,
    /// `## Technical logic`: delegation, validation, errors.
    TechnicalLogic,
    /// `## UI / layout`: the spatial tree.
    UiLayout,
    /// `## Observability`: metrics, traces, logs.
    Observability,
}

// Writes the section title as the document spells it.
impl Display for SectionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Overview => "Overview",
            Self::DomainModel => "Domain model",
            Self::Apis => "APIs and integrations",
            Self::TechnicalLogic => "Technical logic",
            Self::UiLayout => "UI / layout",
            Self::Observability => "Observability",
        })
    }
}

/// Yields every source key cited as `(from <key>)` in `text`. The
/// parenthesised text must be one token: a phrase such as `(from the
/// browser)` is prose, not a citation.
pub fn citations(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices(CITATION).filter_map(|(at, _)| {
        let (key, _) = text[at + CITATION.len()..].split_once(')')?;
        (!key.is_empty() && !key.contains(char::is_whitespace)).then_some(key)
    })
}
