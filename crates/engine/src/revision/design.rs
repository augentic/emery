//! Defines the typed data and Markdown rendering for `design.md`.
//!
//! A [`Design`] contains introductory paragraphs and a fixed vocabulary of
//! sections. Section blocks contain drafted prose or verbatim type
//! signatures.

use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::revision;

const CITATION: &str = "(from ";

/// The `Type:` key written before a type-signature fence.
pub const TYPE: &str = "Type:";

/// A rebuild design in its stored form.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Design {
    /// The grammar the document was written under.
    pub emery: u32,
    /// Markdown paragraphs preceding the first section.
    pub preamble: Vec<String>,
    /// The sections, in vocabulary order.
    pub sections: Vec<Section>,
}

impl Design {
    /// Returns the section of `kind`, if the design has one.
    #[must_use]
    pub fn section(&self, kind: SectionKind) -> Option<&Section> {
        self.sections.iter().find(|section| section.kind == kind)
    }
}

impl revision::Document for Design {
    const NAME: &'static str = "design";
}

impl Display for Design {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        revision::write(f, "Design", &self.preamble, &self.sections)
    }
}

/// A section of a rebuild design.
///
/// `B` allows the same section shape to hold either draft blocks or stored
/// [`Block`] values.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "Section")]
pub struct Section<B = Block> {
    /// The section heading.
    pub kind: SectionKind,
    /// The blocks, in reading order.
    pub blocks: Vec<B>,
}

impl Display for Section {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "## {}", self.kind)?;
        for block in &self.blocks {
            write!(f, "\n\n{block}")?;
        }
        Ok(())
    }
}

/// A paragraph or type signature in a design section.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Block {
    /// One Markdown paragraph.
    Text(String),
    /// A type claim's verbatim signature.
    Type {
        /// The source claim's key.
        key: String,
        /// The claim's signature.
        signature: String,
    },
}

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

/// A section in the rebuild design.
///
/// Variants are declared in document order. [`AsRef::as_ref`] returns the
/// kebab-case key used in structured data, while [`Display`] returns the
/// Markdown heading.
///
/// # Examples
///
/// ```
/// use emery_engine::specify::SectionKind;
///
/// assert_eq!(SectionKind::DomainModel.as_ref(), "domain-model");
/// assert_eq!(SectionKind::DomainModel.to_string(), "Domain model");
/// ```
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

/// Returns every source key cited as `(from <key>)` in `text`.
///
/// The parenthesised text must be one token: a phrase such as `(from the
/// browser)` is prose, not a citation.
pub fn citations(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices(CITATION).filter_map(|(at, _)| {
        let (key, _) = text[at + CITATION.len()..].split_once(')')?;
        (!key.is_empty() && !key.contains(char::is_whitespace)).then_some(key)
    })
}
