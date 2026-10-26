//! Synthesises the drafted content of `design.md`.
//!
//! Extracted claim kinds determine which design sections are required,
//! permitted, or omitted. The model supplies introductory paragraphs,
//! section prose, and the role each type plays. The engine validates source
//! citations and inserts type signatures verbatim from evidence. An
//! amendment redrafts the sections its change informs alone, over the
//! current design, through [`DesignAmend`].

mod amend;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{Claim, ClaimKind};
use omnia_sdk::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use strum::VariantArray as _;

pub use self::amend::DesignAmend;
use crate::brief::{Brief, Review};
use crate::revision::{self, Design, EMERY, Role, Section, SectionKind, citations};
use crate::specify::basis::Basis;
use crate::specify::sections::{BasesSection, ClaimsSection};
use crate::specify::{Extract, shape};

// A synthesis brief for the drafted portions of `design.md`.
//
// The brief contains extracted claims, the requirement bases the
// specification is drafted from, and the section outline derived from
// evidence. It runs from the bases rather than the drafted specification, so
// the design and the specification are drafted together.
pub struct DesignBrief<'a> {
    extracts: &'a [Extract],
    bases: &'a [Basis<'a>],
    outline: Outline<'a>,
}

impl<'a> DesignBrief<'a> {
    // Returns a design brief for `extracts` and `bases`.
    #[must_use]
    pub fn new(extracts: &'a [Extract], bases: &'a [Basis<'a>]) -> Self {
        Self {
            extracts,
            bases,
            outline: Outline::new(extracts),
        }
    }
}

impl Brief for DesignBrief<'_> {
    type Answer = DesignAnswer;
    type Output = Design;

    const NAME: &'static str = "design-draft";
    const PROSE: &'static [&'static str] =
        &["synthesise.md", "design-format.md", "claim-landing.md"];

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        Self::at(schema, "/properties/sections")?
            .insert("minItems".to_owned(), json!(self.outline.required().count()));

        // replace the derived `kind` reference with this run's subset
        let kinds = SectionKind::VARIANTS
            .iter()
            .filter(|kind| self.outline.presence(**kind) != Presence::Omitted)
            .map(AsRef::as_ref)
            .collect::<Vec<_>>();
        let kind = Self::at(schema, "/$defs/Section/properties/kind")?;
        kind.remove("$ref");
        kind.insert("type".to_owned(), json!("string"));
        kind.insert("enum".to_owned(), json!(kinds));
        Self::at(schema, "/$defs")?.remove("SectionKind");

        // restrict a type block's key to this run's type keys
        if !self.outline.types.is_empty() {
            let keys = json!(self.outline.keys().collect::<Vec<_>>());
            Self::at(schema, "/$defs/TypeRef/properties/key")?.insert("enum".to_owned(), keys);
        }
        Ok(())
    }

    fn verify(&self, answer: &DesignAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");

        let mut seen = BTreeSet::new();
        let mut references: BTreeMap<&str, usize> = BTreeMap::new();
        for section in &answer.sections {
            let kind = section.kind;
            let label = format!("`## {kind}`");
            if !seen.insert(kind) {
                review.note(format_args!("{label} is drafted more than once"));
            }
            if self.outline.presence(kind) == Presence::Omitted {
                review.note(format_args!("{label} is present but no claim informs it"));
            }
            self.outline.blocks(review, kind, &label, &section.blocks, &mut references);
        }

        for kind in self.outline.required().filter(|kind| !seen.contains(kind)) {
            review.note(format_args!("`## {kind}` is required but absent"));
        }
        self.outline.references(review, &references);
    }

    fn into_output(self, answer: DesignAnswer) -> Result<Design, Error> {
        let mut drafted = answer.sections;
        drafted.sort_by_key(|section| section.kind);
        let mut sections = Vec::with_capacity(drafted.len());

        for section in drafted {
            sections.push(Section {
                kind: section.kind,
                blocks: self.outline.lower(section.blocks)?,
            });
        }

        Ok(Design {
            emery: EMERY,
            parent: None,
            preamble: answer.preamble,
            sections,
        })
    }
}

impl Display for DesignBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Draft `design.md`.\n\n{claims}", claims = ClaimsSection(self.extracts))?;

        f.write_str("\n## Sections\n\n")?;
        for &kind in SectionKind::VARIANTS {
            let presence = self.outline.presence(kind);
            let informants = kind
                .informants()
                .iter()
                .map(|kind| format!("`{kind}`"))
                .collect::<Vec<_>>()
                .join(" / ");
            let reason = match presence {
                Presence::Required if informants.is_empty() => String::new(),
                Presence::Required => format!(": {informants} claims are present"),
                Presence::Permitted => " where claims inform it".to_string(),
                Presence::Omitted => format!(": no {informants} claim"),
            };
            writeln!(f, "- `{key}` (`## {kind}`) — {presence}{reason}", key = kind.as_ref())?;
        }

        if !self.outline.types.is_empty() {
            f.write_str(
                "\n## Type blocks\n\nReference each `type` claim exactly once under \
                 `domain-model` as a `{\"type\": {\"key\": \"<key>\", \"role\": \"data\" | \
                 \"mechanism\"}}` block; the engine inserts its signature verbatim. A `data` \
                 type is a shape the program reads, stores, or produces; a `mechanism` is a \
                 class that is a client, a retriever, a bootstrap, a settings carrier, or an \
                 error or exception class, which a build declares nothing for and no slice \
                 owns.\n\n",
            )?;
            for key in self.outline.keys() {
                writeln!(f, "- `{key}`")?;
            }
        }

        write!(
            f,
            "\n## Requirements (the specification's, drafted beside this design)\n\n{bases}",
            bases = BasesSection(self.bases)
        )
    }
}

/// Model-authored content for a rebuild design.
///
/// The engine supplies section headings and type signatures.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery design draft")]
pub struct DesignAnswer {
    /// Markdown paragraphs before the first section.
    pub preamble: Vec<String>,
    /// One entry per rendered section, in any order.
    pub sections: Vec<Section<Block>>,
}

/// A drafted paragraph or reference to a type claim.
///
/// The renderer inserts a referenced claim's signature verbatim.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Block {
    /// One Markdown paragraph; inline `(from <source>)` citations allowed.
    Text(String),
    /// A `type` claim by its key, with the role its type plays.
    Type(TypeRef),
}

/// A design draft's reference to one `type` claim.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TypeRef {
    /// The claim's key: its declared name, else its id, else its path.
    pub key: String,
    /// What the type stands for.
    pub role: Role,
}

// What the evidence lets the design say: the claim kinds present, the source
// names a citation may name, and the type blocks on offer. `types` maps each
// `type` claim's key, the name the model references it by, to the signature
// the engine renders under it. A `type` claim without a string `signature`
// has nothing to render and is not offered.
struct Outline<'a> {
    kinds: BTreeSet<ClaimKind>,
    sources: BTreeSet<&'a str>,
    types: BTreeMap<String, &'a str>,
}

impl<'a> Outline<'a> {
    fn new(extracts: &'a [Extract]) -> Self {
        let claims = || extracts.iter().flat_map(|extract| &extract.evidence.claims);
        let mut types: BTreeMap<String, &'a str> = BTreeMap::new();

        for claim in claims().filter(|claim| claim.kind == ClaimKind::Type) {
            let Some(signature) = claim.signature() else { continue };
            let Some(name) = declared(claim) else { continue };

            let key = type_key(&types, name, claim.path.as_deref());
            types.insert(key, signature.trim_end());
        }

        Self {
            kinds: claims().map(|claim| claim.kind).collect(),
            sources: extracts.iter().map(|extract| extract.source.as_str()).collect(),
            types,
        }
    }

    // The outline of an amendment's design turn over the change `extracts`
    // and the current design `base`: the change's claim kinds, so a section
    // is informed as a specify's is; the change's sources and every source
    // the base's text cites, so a kept citation is no finding; and the
    // base's type blocks beneath the change's `type` claims, a key the
    // change redeclares taking the change's signature.
    fn amend(extracts: &'a [Extract], base: &'a Design) -> Self {
        let mut outline = Self::new(extracts);
        for block in base.sections.iter().flat_map(|section| &section.blocks) {
            match block {
                revision::Block::Text(text) => outline.sources.extend(citations(text)),
                revision::Block::Type { key, signature, .. } => {
                    outline.types.entry(key.clone()).or_insert(signature.as_str());
                }
            }
        }
        outline
    }

    fn keys(&self) -> impl Iterator<Item = &str> {
        self.types.keys().map(String::as_str)
    }

    // Whether a claim kind that informs `kind` is present.
    fn informed(&self, kind: SectionKind) -> bool {
        kind.informants().iter().any(|claim| self.kinds.contains(claim))
    }

    fn presence(&self, kind: SectionKind) -> Presence {
        if self.informed(kind) {
            return Presence::Required;
        }

        match kind {
            SectionKind::Overview => Presence::Required,
            SectionKind::Observability | SectionKind::TechnicalLogic => Presence::Permitted,
            _ => Presence::Omitted,
        }
    }

    fn required(&self) -> impl Iterator<Item = SectionKind> + '_ {
        SectionKind::VARIANTS
            .iter()
            .copied()
            .filter(|kind| self.presence(*kind) == Presence::Required)
    }

    // Checks the blocks of one drafted section: a section with none, each
    // paragraph's markup and citations, each type block's place under
    // `## Domain model`, counting the type keys referenced into
    // `references`.
    fn blocks<'b>(
        &self, review: &mut Review, kind: SectionKind, label: &str, blocks: &'b [Block],
        references: &mut BTreeMap<&'b str, usize>,
    ) {
        if blocks.is_empty() {
            review.note(format_args!("{label} has no block"));
        }
        for block in blocks {
            match block {
                Block::Text(text) => {
                    review.paragraph(text, label);
                    for key in citations(text).filter(|key| !self.sources.contains(key)) {
                        review
                            .note(format_args!("{label} cites source `{key}`, which is not bound"));
                    }
                }
                Block::Type(TypeRef { key, .. }) => {
                    if kind != SectionKind::DomainModel {
                        review.note(format_args!(
                            "{label} references type `{key}`; type blocks belong under `## \
                             Domain model`"
                        ));
                    }
                    *references.entry(key.as_str()).or_default() += 1;
                }
            }
        }
    }

    // Checks that `references` names every offered type key exactly once and
    // no other.
    fn references(&self, review: &mut Review, references: &BTreeMap<&str, usize>) {
        for key in self.keys() {
            match references.get(key).copied().unwrap_or_default() {
                1 => {}
                0 => review.note(format_args!("type `{key}` is never referenced")),
                n => review.note(format_args!("type `{key}` is referenced {n} times")),
            }
        }
        for key in references.keys().filter(|key| !self.types.contains_key(**key)) {
            review.note(format_args!("type `{key}` is not a type claim"));
        }
    }

    // Lowers accepted `blocks` into stored ones, each type reference taking
    // the signature offered under its key.
    //
    // Returns `Error::ServerError` for a key the outline does not offer,
    // which `references` refused before acceptance.
    fn lower(&self, blocks: Vec<Block>) -> Result<Vec<revision::Block>, Error> {
        blocks
            .into_iter()
            .map(|block| match block {
                Block::Text(text) => Ok(revision::Block::Text(text)),
                Block::Type(TypeRef { key, role }) => {
                    let signature = self.types.get(key.as_str()).copied().ok_or_else(|| {
                        server_error!("type `{key}` was accepted without a type claim")
                    })?;
                    Ok(revision::Block::Type {
                        key,
                        signature: signature.to_string(),
                        role,
                    })
                }
            })
            .collect()
    }
}

// The key a `type` claim is placed under, so no declaration displaces another:
// - its declared name;
// - where an earlier claim took that, the name with its anchored path in
//   parentheses;
// - where that is taken too, a counter from 2 beside the path.
fn type_key(taken: &BTreeMap<String, &str>, name: &str, path: Option<&str>) -> String {
    if !taken.contains_key(name) {
        return name.to_owned();
    }

    let anchor = path.unwrap_or("unanchored");
    let anchored = format!("{name} ({anchor})");
    if !taken.contains_key(&anchored) {
        return anchored;
    }

    // more candidates than taken keys, so one is free
    (2..=taken.len() + 2)
        .map(|n| format!("{name} ({anchor}, {n})"))
        .find(|key| !taken.contains_key(key))
        .unwrap_or(anchored)
}

// The declared name of a `type` claim: its `name` extra — the identifier the
// declaration spells, which several claims sharing one `id` never share — then
// its id, then its path with any `#L…` anchor stripped, so a re-anchored claim
// keeps its key.
fn declared(claim: &Claim) -> Option<&str> {
    claim.type_name().or(claim.id.as_deref()).or_else(|| claim.path.as_deref().map(shape::file))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "lowercase")]
enum Presence {
    Required,
    Permitted,
    Omitted,
}

impl SectionKind {
    // The claim kinds whose presence requires this section. `Overview` and
    // `Observability` have none: the first is always required, the second
    // only ever permitted. A `decision` requires `Apis` because the
    // technology a prose source prescribes lands there, and nowhere in the
    // specification.
    const fn informants(self) -> &'static [ClaimKind] {
        match self {
            Self::Overview | Self::Observability => &[],
            Self::DomainModel => &[ClaimKind::Type],
            Self::Apis => &[ClaimKind::Call, ClaimKind::Contract, ClaimKind::Decision],
            Self::TechnicalLogic => &[ClaimKind::Excerpt],
            Self::UiLayout => &[ClaimKind::Region, ClaimKind::Container, ClaimKind::Leaf],
        }
    }
}
