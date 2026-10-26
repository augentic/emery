//! Defines the typed data and Markdown rendering for `design.md`.
//!
//! A [`Design`] contains introductory paragraphs and a fixed vocabulary of
//! sections. Section blocks contain drafted prose or verbatim type
//! signatures, each signature labelled with the [`Role`] its type plays.

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::revision;

const CITATION: &str = "(from ";

// The `Type:` key written before a type-signature fence.
pub const TYPE: &str = "Type:";

// A rebuild design in its stored form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Design {
    // The grammar the document was written under.
    pub emery: u32,
    // The revision this one amends, or `None` when drafted from sources alone.
    pub parent: Option<String>,
    // Markdown paragraphs preceding the first section.
    pub preamble: Vec<String>,
    // The sections, in vocabulary order.
    pub sections: Vec<Section>,
}

impl Design {
    // Returns the section of `kind`, if the design has one.
    #[must_use]
    pub fn section(&self, kind: SectionKind) -> Option<&Section> {
        self.sections.iter().find(|section| section.kind == kind)
    }

    // Returns the keys of the `Role::Data` types, in key order.
    //
    // These are the types a build plan assigns to slices. A
    // `Role::Mechanism` type is owned by no slice.
    #[must_use]
    pub fn data_types(&self) -> Vec<String> {
        let keys: BTreeSet<&str> = self
            .sections
            .iter()
            .flat_map(|section| &section.blocks)
            .filter_map(|block| match block {
                Block::Type {
                    key,
                    role: Role::Data,
                    ..
                } => Some(key.as_str()),
                Block::Text(_) | Block::Type { .. } => None,
            })
            .collect();
        keys.into_iter().map(str::to_owned).collect()
    }

    // Returns the signature and role of the type `key` names, if the design has one.
    #[must_use]
    pub fn type_block(&self, key: &str) -> Option<(&str, Role)> {
        self.sections.iter().flat_map(|section| &section.blocks).find_map(|block| match block {
            Block::Type {
                key: held,
                signature,
                role,
            } if held == key => Some((signature.as_str(), *role)),
            Block::Text(_) | Block::Type { .. } => None,
        })
    }

    // Returns the signature of the type `key` names, if the design has one.
    #[must_use]
    pub fn signature(&self, key: &str) -> Option<&str> {
        self.type_block(key).map(|(signature, _)| signature)
    }

    // Returns this design less the type blocks `keys` name.
    //
    // A section left with no block goes with them, since a design never
    // holds an empty section. Every text block and the `parent` stand as
    // they are.
    #[must_use]
    pub fn without_types(&self, keys: &BTreeSet<String>) -> Self {
        let sections = self
            .sections
            .iter()
            .filter_map(|section| {
                let blocks: Vec<Block> = section
                    .blocks
                    .iter()
                    .filter(|block| match block {
                        Block::Type { key, .. } => !keys.contains(key),
                        Block::Text(_) => true,
                    })
                    .cloned()
                    .collect();
                (!blocks.is_empty()).then_some(Section {
                    kind: section.kind,
                    blocks,
                })
            })
            .collect();
        Self {
            emery: self.emery,
            parent: self.parent.clone(),
            preamble: self.preamble.clone(),
            sections,
        }
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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

// A paragraph or type signature in a design section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Block {
    // One Markdown paragraph.
    Text(String),
    // A type claim's verbatim signature.
    Type {
        // The source claim's key.
        key: String,
        // The claim's signature.
        signature: String,
        // What the type stands for.
        role: Role,
    },
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => f.write_str(text),
            Self::Type {
                key,
                signature,
                role: Role::Data,
            } => write!(f, "{TYPE} {key}\n```\n{signature}\n```"),
            Self::Type {
                key,
                signature,
                role: Role::Mechanism,
            } => write!(f, "{TYPE} {key} (mechanism)\n```\n{signature}\n```"),
        }
    }
}

/// What a type in the domain model stands for.
///
/// A target declares a [`Data`](Role::Data) type as a shape its code reads
/// or produces. It declares nothing for a [`Mechanism`](Role::Mechanism): the
/// behaviour its requirements state is built through the target's own
/// platform, and no slice owns the type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Role {
    /// A shape that crosses a boundary or is stored: a message, a record, a
    /// response, an enumeration.
    Data,
    /// A class that is a client, a retriever, a bootstrap, a settings
    /// carrier, or another part of the source's own machinery.
    Mechanism,
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

// Returns every source name cited as `(from <source>)` in `text`.
//
// The parenthesised text must be one token: a phrase such as `(from the
// browser)` is prose, not a citation.
pub fn citations(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices(CITATION).filter_map(|(at, _)| {
        let (key, _) = text[at + CITATION.len()..].split_once(')')?;
        (!key.is_empty() && !key.contains(char::is_whitespace)).then_some(key)
    })
}
