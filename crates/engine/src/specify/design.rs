//! The `design.md` brief
//!
//! Asks the model for the drafted content of `design.md`: the preamble and
//! the blocks of each section. Which sections of the closed vocabulary a run
//! calls for is decided by the claim kinds it extracted: the schema names that
//! subset, every candidate draft is verified against the plan, the bound
//! sources it may cite, and the `type` claims whose signatures the engine
//! inserts, and the engine places the accepted draft in the design.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_source::ClaimKind;
use omnia_guest::{Error, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use strum::VariantArray as _;

use crate::revision::{self, Design, EMERY, Section, SectionKind, Spec, citations};
use crate::specify::Extract;
use crate::specify::brief::{Brief, ClaimsSection, Review};

/// What the engine needs to ask the model for `design.md` and to verify its
/// draft: the extracts, the specification, and the section plan.
pub struct DesignBrief<'a> {
    extracts: &'a [Extract],
    spec: &'a Spec,
    plan: Plan<'a>,
}

impl<'a> DesignBrief<'a> {
    /// Creates the brief for `design.md` from the `extracts`, the
    /// specification `spec` the design follows, and a section plan derived
    /// from the claims in the extracts.
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
    const PROSE: &'static [&'static str] =
        &["synthesis/synthesise.md", "synthesis/design-format.md"];

    // Tightens the derived schema to this run's plan: at least as many
    // sections as the plan requires, `kind` limited to the kinds the plan does
    // not forbid, and `type` blocks limited to this run's `type` claim keys.
    fn tighten(&self, schema: &mut Value) {
        schema["properties"]["sections"]["minItems"] = json!(self.plan.required().count());

        // The derived `kind` refers to the whole vocabulary; the run's
        // subset replaces the reference in place.
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

        // The derived `Block` oneOf includes every variant; restrict the
        // `{"type": …}` arm to this run's type keys.
        if !self.plan.signatures.is_empty()
            && let Some(block) = schema
                .pointer_mut("/$defs/Block/oneOf")
                .and_then(Value::as_array_mut)
                .and_then(|variants| {
                    variants.iter_mut().find(|variant| variant["required"] == json!(["type"]))
                })
        {
            block["properties"]["type"]["enum"] = json!(self.plan.keys().collect::<Vec<_>>());
        }
    }

    // Verifies a candidate draft against the plan: every required section
    // present, none forbidden, duplicated, or empty; each `type` claim placed
    // once, only under `## Domain model`; citations bound; no reserved opener.
    fn verify(&self, answer: &DesignAnswer, review: &mut Review) {
        review.paragraphs(&answer.preamble, "preamble");

        let bound = &self.plan.bound;
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
                        for key in citations(text).filter(|key| !bound.contains(key)) {
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

        for key in references.keys().filter(|key| !self.plan.signatures.contains_key(*key)) {
            review.note(format_args!("type `{key}` is not a type claim"));
        }
    }

    // Places the draft in the design: the drafted sections in vocabulary
    // order, each `type` block carrying the claim's signature.
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
                            self.plan.signatures.get(key.as_str()).copied().ok_or_else(|| {
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

// Renders the user turn of the prompt: every claim in every extract, the
// plan's verdict on each section kind with its reason, the `type` claims to
// place, and the rendered `spec.md` the design must follow.
impl Display for DesignBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Draft `design.md`.\n\n{claims}", claims = ClaimsSection(self.extracts))?;

        f.write_str("\n## Sections\n\n")?;
        for &kind in SectionKind::VARIANTS {
            let presence = self.plan.presence(kind);
            let kinds = informants(kind).iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>();
            let reason = match (presence, kinds.is_empty()) {
                (Presence::Required, false) => {
                    format!(": {} claims are present", kinds.join(" / "))
                }
                (Presence::Omitted, false) => format!(": no {} claim", kinds.join(" / ")),
                (Presence::Permitted, _) => " where claims inform it".to_string(),
                _ => String::new(),
            };
            writeln!(f, "- `{key}` (`## {kind}`) — {presence}{reason}", key = kind.as_ref())?;
        }

        if !self.plan.signatures.is_empty() {
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

/// The `design.md` draft: preamble paragraphs and one entry per section.
/// Only what needs synthesis is asked for; every heading and signature is
/// the renderer's.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery design draft")]
pub struct DesignAnswer {
    /// Markdown paragraphs before the first section.
    pub preamble: Vec<String>,
    /// One entry per rendered section; any order.
    pub sections: Vec<Section<Block>>,
}

/// One design block: a paragraph, or a reference to a `type` claim whose
/// signature the renderer inserts verbatim.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Block {
    /// One Markdown paragraph; inline `(from <source>)` citations allowed.
    Text(String),
    /// The key of a `type` claim.
    Type(String),
}

// The facts a design draft is verified against: the kinds of every extracted
// claim (which decide the sections this run requires, permits, or forbids),
// the bound sources it may cite, and the `type` claims it must reference —
// each by key (its id, or its path when it has none), with the trimmed
// signature the engine places. A `type` claim without a string `signature`
// has nothing to place and is not planned.
struct Plan<'a> {
    kinds: BTreeSet<ClaimKind>,
    bound: BTreeSet<&'a str>,
    signatures: BTreeMap<&'a str, &'a str>,
}

impl<'a> Plan<'a> {
    fn new(extracts: &'a [Extract]) -> Self {
        let claims = || extracts.iter().flat_map(|extract| &extract.evidence.claims);
        let signatures = claims()
            .filter(|claim| claim.kind == ClaimKind::Type)
            .filter_map(|claim| {
                let key = claim.id.as_deref().or(claim.path.as_deref())?;
                let Some(Value::String(signature)) = claim.extras.get("signature") else {
                    return None;
                };
                Some((key, signature.trim_end()))
            })
            .collect();

        Self {
            kinds: claims().map(|claim| claim.kind).collect(),
            bound: extracts.iter().map(|extract| extract.key.as_str()).collect(),
            signatures,
        }
    }

    // Lists every `type` claim key the draft must reference, in key order.
    fn keys(&self) -> impl Iterator<Item = &str> {
        self.signatures.keys().copied()
    }

    // Decides whether section `kind` is required, permitted, or forbidden:
    // `Overview`, and any kind with an informant claim present, is required;
    // uninformed `Observability` / `TechnicalLogic` permitted; the rest forbidden.
    fn presence(&self, kind: SectionKind) -> Presence {
        let informed = informants(kind).iter().any(|claim| self.kinds.contains(claim));
        match (kind, informed) {
            (SectionKind::Overview, _) | (_, true) => Presence::Required,
            (SectionKind::Observability | SectionKind::TechnicalLogic, false) => {
                Presence::Permitted
            }
            (_, false) => Presence::Omitted,
        }
    }

    // Lists every section the plan requires, in vocabulary order.
    fn required(&self) -> impl Iterator<Item = SectionKind> + '_ {
        SectionKind::VARIANTS
            .iter()
            .copied()
            .filter(|kind| self.presence(*kind) == Presence::Required)
    }
}

// Whether the evidence calls for a section, tolerates it, or leaves it out;
// the lowercase name is the presence the prompt states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "lowercase")]
enum Presence {
    Required,
    Permitted,
    Omitted,
}

// Maps a section kind to the claim kinds whose presence requires it.
// `Overview` and `Observability` have none: the first is always required, the
// second only ever permitted.
const fn informants(kind: SectionKind) -> &'static [ClaimKind] {
    match kind {
        SectionKind::Overview | SectionKind::Observability => &[],
        SectionKind::DomainModel => &[ClaimKind::Type],
        SectionKind::Apis => &[ClaimKind::Call, ClaimKind::Contract],
        SectionKind::TechnicalLogic => &[ClaimKind::Excerpt],
        SectionKind::UiLayout => &[ClaimKind::Region, ClaimKind::Container, ClaimKind::Leaf],
    }
}
