//! Defines the typed data and Markdown rendering for `design.md`.
//!
//! A [`Design`] contains introductory paragraphs and a fixed vocabulary of
//! sections. Section blocks contain drafted prose, verbatim type
//! signatures, each signature labelled with the [`Role`] its type plays, or
//! the content-addressed [`Layout`]s a `## UI / layout` section names.

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::revision;
use crate::revision::layout::Layout;

const CITATION: &str = "(from ";

pub const TYPE: &str = "Type:";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Design {
    pub emery: u32,
    // The revision this one amends, or `None` when drafted from sources alone.
    pub parent: Option<String>,
    pub preamble: Vec<String>,
    // The sections, in vocabulary order.
    pub sections: Vec<Section>,
}

impl Design {
    #[must_use]
    pub fn section(&self, kind: SectionKind) -> Option<&Section> {
        self.sections.iter().find(|section| section.kind == kind)
    }

    // Returns the keys of the `Role::Data` types, in key order: the types a
    // build plan assigns to slices (`Role::Mechanism` is owned by none).
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
                Block::Text(_) | Block::Type { .. } | Block::Layout(_) => None,
            })
            .collect();
        keys.into_iter().map(str::to_owned).collect()
    }

    #[must_use]
    pub fn type_block(&self, key: &str) -> Option<(&str, Role)> {
        self.sections.iter().flat_map(|section| &section.blocks).find_map(|block| match block {
            Block::Type {
                key: held,
                signature,
                role,
            } if held == key => Some((signature.as_str(), *role)),
            Block::Text(_) | Block::Type { .. } | Block::Layout(_) => None,
        })
    }

    #[must_use]
    pub fn signature(&self, key: &str) -> Option<&str> {
        self.type_block(key).map(|(signature, _)| signature)
    }

    // Returns this design less the type blocks `keys` name; a section left
    // with no block goes with them.
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
                        Block::Text(_) | Block::Layout(_) => true,
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

impl Design {
    // The layouts `## UI / layout` names, in the design's order.
    pub fn layouts(&self) -> impl Iterator<Item = &Layout> {
        self.section(SectionKind::UiLayout)
            .into_iter()
            .flat_map(|section| &section.blocks)
            .filter_map(|block| match block {
                Block::Layout(layout) => Some(layout),
                Block::Text(_) | Block::Type { .. } => None,
            })
    }

    // Appends to `## UI / layout` each of `layouts` it does not already
    // name, opening the section in vocabulary order where the design has none.
    // A layout recorded again by a source replaces every layout that source
    // already contributed, so an amendment carries the source's layouts as
    // they were recorded this time and not also the ones they succeed.
    pub fn attach(&mut self, layouts: Vec<Layout>) {
        let replacing: BTreeSet<String> =
            layouts.iter().map(|layout| layout.source.clone()).collect();
        for section in &mut self.sections {
            if section.kind != SectionKind::UiLayout {
                continue;
            }
            section.blocks.retain(|block| match block {
                Block::Layout(held) => !replacing.contains(&held.source),
                Block::Text(_) | Block::Type { .. } => true,
            });
        }
        for layout in layouts {
            if self.layouts().any(|held| held.digest == layout.digest) {
                continue;
            }
            let at = match self.sections.iter().position(|s| s.kind >= SectionKind::UiLayout) {
                Some(at) if self.sections[at].kind == SectionKind::UiLayout => at,
                Some(at) => {
                    self.sections.insert(at, Section::empty(SectionKind::UiLayout));
                    at
                }
                None => {
                    self.sections.push(Section::empty(SectionKind::UiLayout));
                    self.sections.len() - 1
                }
            };
            self.sections[at].blocks.push(Block::Layout(layout));
        }
    }

    // Returns this design with each layout's images left out: what a model
    // reads, which takes the layout whole from the slice instead.
    #[must_use]
    pub fn without_views(&self) -> Self {
        let mut design = self.clone();
        for block in design.sections.iter_mut().flat_map(|section| &mut section.blocks) {
            if let Block::Layout(layout) = block {
                layout.views.clear();
            }
        }
        design
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

impl Section {
    const fn empty(kind: SectionKind) -> Self {
        Self {
            kind,
            blocks: Vec::new(),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Block {
    Text(String),
    Type { key: String, signature: String, role: Role },
    Layout(Layout),
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
            Self::Layout(layout) => layout.fmt(f),
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
