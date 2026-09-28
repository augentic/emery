//! Synthesises the drafted content of `design.md`.
//!
//! Extracted claim kinds determine which design sections are required,
//! permitted, or omitted. The model supplies introductory paragraphs and
//! section prose. The engine validates source citations and inserts type
//! signatures verbatim from evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{Claim, ClaimKind};
use omnia_sdk::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use strum::VariantArray as _;

use crate::revision::{self, Design, EMERY, Section, SectionKind, Spec, citations};
use crate::specify::Extract;
use crate::specify::brief::{Brief, ClaimsSection, Review};

/// A synthesis brief for the drafted portions of `design.md`.
///
/// The brief contains extracted claims, the specification being implemented,
/// and the section plan derived from evidence.
pub struct DesignBrief<'a> {
    extracts: &'a [Extract],
    spec: &'a Spec,
    plan: Plan<'a>,
}

impl<'a> DesignBrief<'a> {
    /// Returns a design brief for `extracts` and `spec`.
    #[must_use]
    pub fn new(extracts: &'a [Extract], spec: &'a Spec) -> Self {
        Self {
            extracts,
            spec,
            plan: Plan::new(extracts),
        }
    }
}

impl Brief for DesignBrief<'_> {
    type Answer = DesignAnswer;
    type Output = Design;

    const NAME: &'static str = "design-draft";
    const PROSE: &'static [&'static str] = &["synthesise.md", "design-format.md"];

    fn tighten(&self, schema: &mut Value) {
        schema["properties"]["sections"]["minItems"] = json!(self.plan.required().count());

        // replace the derived `kind` reference with this run's subset
        let kinds = SectionKind::VARIANTS
            .iter()
            .filter(|kind| self.plan.presence(**kind) != Presence::Omitted)
            .map(AsRef::as_ref)
            .collect::<Vec<_>>();
        if let Some(kind) = schema["$defs"]["Section"]["properties"]["kind"].as_object_mut() {
            kind.remove("$ref");
            kind.insert("type".to_string(), json!("string"));
            kind.insert("enum".to_string(), json!(kinds));
        }
        if let Some(defs) = schema["$defs"].as_object_mut() {
            defs.remove("SectionKind");
        }

        // restrict the `{"type": …}` arm to this run's type keys
        if self.plan.types.is_empty() {
            return;
        }
        if let Some(block) = type_block(schema) {
            block["properties"]["type"]["enum"] = json!(self.plan.keys().collect::<Vec<_>>());
        }
    }

    fn verify(&self, answer: &DesignAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");

        let sources = &self.plan.sources;
        let mut seen = BTreeSet::new();
        let mut references: BTreeMap<&str, usize> = BTreeMap::new();
        for section in &answer.sections {
            let kind = section.kind;
            let label = format!("`## {kind}`");
            if !seen.insert(kind) {
                review.note(format_args!("{label} is drafted more than once"));
            }
            if self.plan.presence(kind) == Presence::Omitted {
                review.note(format_args!("{label} is present but no claim informs it"));
            }
            if section.blocks.is_empty() {
                review.note(format_args!("{label} has no block"));
            }

            for block in &section.blocks {
                match block {
                    Block::Text(text) => {
                        review.paragraph(text, &label);
                        for key in citations(text).filter(|key| !sources.contains(key)) {
                            review.note(format_args!(
                                "{label} cites source `{key}`, which is not bound"
                            ));
                        }
                    }
                    Block::Type(key) => {
                        if kind != SectionKind::DomainModel {
                            review.note(format_args!(
                                "{label} references type `{key}`; type blocks belong under \
                                 `## Domain model`"
                            ));
                        }
                        *references.entry(key.as_str()).or_default() += 1;
                    }
                }
            }
        }

        for kind in self.plan.required().filter(|kind| !seen.contains(kind)) {
            review.note(format_args!("`## {kind}` is required but absent"));
        }

        for key in self.plan.keys() {
            match references.get(key).copied().unwrap_or_default() {
                1 => {}
                0 => review.note(format_args!("type `{key}` is never referenced")),
                n => review.note(format_args!("type `{key}` is referenced {n} times")),
            }
        }

        for key in references.keys().filter(|key| !self.plan.types.contains_key(**key)) {
            review.note(format_args!("type `{key}` is not a type claim"));
        }
    }

    fn into_output(self, answer: DesignAnswer) -> Result<Design, Error> {
        let mut drafted = answer.sections;
        drafted.sort_by_key(|section| section.kind);
        let mut sections = Vec::with_capacity(drafted.len());

        for section in drafted {
            let mut blocks = Vec::with_capacity(section.blocks.len());
            for block in section.blocks {
                blocks.push(match block {
                    Block::Text(text) => revision::Block::Text(text),
                    Block::Type(key) => {
                        let signature =
                            self.plan.types.get(key.as_str()).copied().ok_or_else(|| {
                                server_error!("type `{key}` was accepted without a type claim")
                            })?;
                        revision::Block::Type {
                            key,
                            signature: signature.to_string(),
                        }
                    }
                });
            }

            sections.push(Section {
                kind: section.kind,
                blocks,
            });
        }

        Ok(Design {
            emery: EMERY,
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
            let presence = self.plan.presence(kind);
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

        if !self.plan.types.is_empty() {
            f.write_str(
                "\n## Type blocks\n\nReference each `type` claim exactly once under \
                 `domain-model` as a `{\"type\": \"<key>\"}` block; the engine inserts its \
                 signature verbatim.\n\n",
            )?;
            for key in self.plan.keys() {
                writeln!(f, "- `{key}`")?;
            }
        }

        write!(f, "\n## The rendered `spec.md`\n\n{spec}", spec = self.spec)
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
    /// The key of a `type` claim.
    Type(String),
}

// What the evidence lets the design say: the claim kinds present, the source
// names a citation may name, and the type blocks on offer. `types` maps each
// `type` claim's key, the name the model references it by, to the signature
// the engine renders under it. A `type` claim without a string `signature`
// has nothing to render and is not offered.
struct Plan<'a> {
    kinds: BTreeSet<ClaimKind>,
    sources: BTreeSet<&'a str>,
    types: BTreeMap<String, &'a str>,
}

impl<'a> Plan<'a> {
    fn new(extracts: &'a [Extract]) -> Self {
        let claims = || extracts.iter().flat_map(|extract| &extract.evidence.claims);
        let mut types: BTreeMap<String, &'a str> = BTreeMap::new();

        for claim in claims().filter(|claim| claim.kind == ClaimKind::Type) {
            let Some(Value::String(signature)) = claim.extras.get("signature") else { continue };
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

    fn keys(&self) -> impl Iterator<Item = &str> {
        self.types.keys().map(String::as_str)
    }

    fn presence(&self, kind: SectionKind) -> Presence {
        let informed = kind.informants().iter().any(|claim| self.kinds.contains(claim));
        if informed {
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

fn type_block(schema: &mut Value) -> Option<&mut Value> {
    let variants = schema.pointer_mut("/$defs/Block/oneOf")?.as_array_mut()?;
    variants.iter_mut().find(|variant| variant["required"] == json!(["type"]))
}

// The declared name of a `type` claim: its id, its `name` extra, or its path
// with any `#L…` anchor stripped, so a re-anchored claim keeps its key.
fn declared(claim: &Claim) -> Option<&str> {
    if let Some(id) = claim.id.as_deref() {
        return Some(id);
    }
    if let Some(Value::String(name)) = claim.extras.get("name") {
        return Some(name);
    }

    let path = claim.path.as_deref()?;
    let (file, _anchor) = path.split_once('#').unwrap_or((path, ""));
    Some(file)
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
    // only ever permitted.
    const fn informants(self) -> &'static [ClaimKind] {
        match self {
            Self::Overview | Self::Observability => &[],
            Self::DomainModel => &[ClaimKind::Type],
            Self::Apis => &[ClaimKind::Call, ClaimKind::Contract],
            Self::TechnicalLogic => &[ClaimKind::Excerpt],
            Self::UiLayout => &[ClaimKind::Region, ClaimKind::Container, ClaimKind::Leaf],
        }
    }
}
