//! Redrafts the sections of `design.md` a change informs.
//!
//! An amendment spends one design turn only when a change claim informs a
//! section, or a section's paragraph describes a requirement the change
//! retires. The turn redrafts those sections and `## Overview` with the
//! current text in the brief, and the engine copies every other section and
//! the preamble from the base. The answer is held to the base: a kept
//! paragraph is verbatim or named `revised`, a named one is reworded and
//! never dropped while it describes a requirement the change keeps, a
//! paragraph describing retired requirements alone may go unnamed, and the
//! type blocks are the base's kept keys and the change's.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{Claim, ClaimKind};
use omnia_sdk::Error;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use strum::VariantArray as _;

use crate::brief::{Brief, Review};
use crate::revision::{
    self, Change, ChangeClass, Cited, Design, EMERY, ReqId, Section, SectionKind,
};
use crate::specify::Extract;
use crate::specify::basis::Basis;
use crate::specify::design::{Block, DesignBrief, Outline, declared, type_key};
use crate::specify::sections::{BasesSection, ClaimsSection};

// The brief of an amendment's design turn: the sections the change informs,
// redrafted over the current design.
pub struct DesignAmend {
    extracts: Vec<Extract>,
    // The requirements the change adds, revises, or leaves in conflict, each
    // under the id it takes.
    drafted: Vec<Basis>,
    // The current design less the types the change's retirements orphaned,
    // its `parent` the base revision.
    base: Design,
    outline: Outline,
    // The sections the turn redrafts, in vocabulary order: `## Overview`,
    // which describes the whole, every section a change claim informs, and
    // every section a paragraph of which describes a retired requirement.
    redrafted: BTreeSet<SectionKind>,
    // The requirements the change leaves as they stand.
    unchanged: BTreeSet<ReqId>,
    // The requirements the change retires, by id and subject.
    retired: BTreeMap<ReqId, String>,
}

// How a base paragraph stands against an amendment's requirements.
enum ParagraphPolicy {
    // Outside `## Overview`, in a change that drafts requirements and retires
    // none, cites unchanged requirements alone and must stay verbatim.
    LockedVerbatim(BTreeSet<ReqId>),
    // Cites retired requirements alone and may be dropped or reworded unnamed.
    Withdrawn,
    // May stay verbatim unnamed, or be named in `revised` while still
    // describing every `kept` requirement it cited.
    Revisable { kept: Vec<ReqId> },
}

impl DesignAmend {
    // Returns the brief, or `None` when no change claim informs a section
    // and no section describes a retired requirement, so the design is
    // `base` as it stands and no turn is spent.
    //
    // `drafted` are the bases the amended specification drafts anew and
    // `changes` the change set, read for the requirements it retires and
    // leaves unchanged. `base` is the current design as the retirements
    // pruned it.
    #[must_use]
    pub fn new(
        extracts: &[Extract], drafted: &[Basis], changes: &[Change], base: &Design,
    ) -> Option<Self> {
        let retired: BTreeMap<ReqId, String> = changes
            .iter()
            .filter(|change| change.class == ChangeClass::Retired)
            .map(|change| (change.id, change.subject.clone()))
            .collect();
        let scoped = ScopedExtracts::new(extracts, drafted, !retired.is_empty(), base);
        let outline = Outline::amend(&scoped.extracts, scoped.types, base);
        let mut redrafted: BTreeSet<SectionKind> = SectionKind::VARIANTS
            .iter()
            .copied()
            .filter(|&kind| outline.informed(kind) || describes(base, kind, &retired))
            .collect();
        if redrafted.is_empty() {
            return None;
        }
        redrafted.insert(SectionKind::Overview);

        Some(Self {
            extracts: scoped.extracts,
            drafted: drafted.to_vec(),
            base: base.clone(),
            outline,
            redrafted,
            unchanged: changes
                .iter()
                .filter(|change| change.class == ChangeClass::Unchanged)
                .map(|change| change.id)
                .collect(),
            retired,
        })
    }

    fn redrafts(&self, kind: SectionKind) -> bool {
        self.redrafted.contains(&kind)
    }

    fn paragraph_policy(&self, kind: SectionKind, text: &str) -> ParagraphPolicy {
        let cited = cited(text);
        if !cited.is_empty() && cited.iter().all(|id| self.retired.contains_key(id)) {
            return ParagraphPolicy::Withdrawn;
        }
        if kind != SectionKind::Overview
            && !self.drafted.is_empty()
            && self.retired.is_empty()
            && !cited.is_empty()
            && cited.is_subset(&self.unchanged)
        {
            return ParagraphPolicy::LockedVerbatim(cited);
        }
        let kept = cited.into_iter().filter(|id| !self.retired.contains_key(id)).collect();
        ParagraphPolicy::Revisable { kept }
    }

    // Whether the base's `kind` section may leave the design: it stands,
    // every block of it is a paragraph describing retired requirements
    // alone, and it is not `## Overview`, which every design holds.
    fn withdrawable(&self, kind: SectionKind) -> bool {
        kind != SectionKind::Overview
            && self.base.section(kind).is_some_and(|section| {
                section.blocks.iter().all(|block| {
                    matches!(
                        block,
                        revision::Block::Text(text)
                            if matches!(
                                self.paragraph_policy(kind, text),
                                ParagraphPolicy::Withdrawn
                            )
                    )
                })
            })
    }

    // Holds one redrafted section against the base's paragraphs and `revised`
    // indices.
    fn loss(&self, section: &Redraft, label: &str, review: &mut Review) {
        let Some(base) = self.base.section(section.kind) else {
            if !section.revised.is_empty() {
                review.note(format_args!(
                    "{label}: `revised` names paragraphs, but the current design has no such \
                     section; leave it empty"
                ));
            }
            return;
        };
        let present: BTreeSet<String> = section.texts().map(verbatim).collect();
        let described: BTreeSet<ReqId> = section.texts().flat_map(cited).collect();
        let revised: BTreeSet<usize> = section.revised.iter().copied().collect();

        for (position, block) in base.blocks.iter().enumerate() {
            let number = position + 1;
            let named = revised.contains(&number);
            let revision::Block::Text(text) = block else {
                if named {
                    review.note(format_args!(
                        "{label} [{number}] is a type block, kept or dropped by its key under \
                         `## Type blocks`, not a paragraph to revise"
                    ));
                }
                continue;
            };
            let policy = self.paragraph_policy(section.kind, text);
            match (present.contains(&verbatim(text)), named, policy) {
                (true, true, _) => review.note(format_args!(
                    "{label} [{number}] is named in `revised` but is kept verbatim; drop it from \
                     `revised`"
                )),
                (true, false, _) | (false, _, ParagraphPolicy::Withdrawn) => {}
                (false, _, ParagraphPolicy::LockedVerbatim(unchanged)) => review.note(format_args!(
                    "{label} [{number}] describes {}, which the change leaves unchanged; keep it \
                     verbatim, omit [{number}] from `revised`, and place added detail in a new \
                     paragraph",
                    listed(&unchanged)
                )),
                (false, false, ParagraphPolicy::Revisable { .. }) => review.note(format_args!(
                    "{label} [{number}] is dropped or reworded without being named in `revised`; \
                     keep it verbatim, or name it"
                )),
                (false, true, ParagraphPolicy::Revisable { kept }) => {
                    let undescribed: Vec<ReqId> =
                        kept.into_iter().filter(|id| !described.contains(id)).collect();
                    if !undescribed.is_empty() {
                        let pronoun = if undescribed.len() == 1 { "it" } else { "them" };
                        review.note(format_args!(
                            "{label} [{number}] is revised and describes {}, which the change \
                             keeps, yet no paragraph of the section describes {pronoun} now; \
                             reword it rather than dropping it",
                            listed(&undescribed),
                        ));
                    }
                }
            }
        }

        for number in revised.iter().filter(|&&number| number == 0 || number > base.blocks.len()) {
            review.note(format_args!(
                "{label}: `revised` names [{number}], which the current section does not hold; \
                 it has {} blocks",
                base.blocks.len()
            ));
        }
    }

    fn write_current_design(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            "\n## Current design\n\nEach section to redraft is listed with its blocks numbered. \
             Keep a paragraph verbatim unless the change touches what it describes, and name \
             every paragraph you reword or drop in that section's `revised` list by its number: \
             a paragraph neither kept verbatim nor named is refused, and so is one named but \
             kept verbatim. Any `[n]` marked `(keep verbatim — unchanged …)` is locked because \
             it cites unchanged requirements alone: copy its text verbatim without the marker, \
             omit `n` from `revised`, and put routes, queries, or technical rules for added or \
             revised requirements in new paragraphs rather than editing a locked summary \
             paragraph. Name a paragraph `revised` to reword it, never to drop it: every \
             requirement it describes that the change keeps is still described by a paragraph \
             of the section, so a paragraph you drop describes retired requirements alone, or \
             none. A type block is listed as its `Type:` line and is kept or dropped by its key \
             under `## Type blocks`, never named in `revised`.\n",
        )?;
        if !self.retired.is_empty() {
            f.write_str(
                "\nA paragraph describing requirements the change retires, and no other, \
                 describes nothing the specification states any longer: drop it, or reword it \
                 around what stands, named or not. A section other than `## Overview` whose \
                 every paragraph describes retired requirements alone may be answered with no \
                 block, and leaves the design.\n",
            )?;
        }
        for &kind in &self.redrafted {
            write!(f, "\n### {kind}\n\n")?;
            let Some(section) = self.base.section(kind) else {
                f.write_str("The current design has no such section; draft it.\n")?;
                continue;
            };
            for (position, block) in section.blocks.iter().enumerate() {
                self.write_block(f, kind, position + 1, block)?;
            }
        }
        Ok(())
    }

    fn write_block(
        &self, f: &mut Formatter<'_>, kind: SectionKind, number: usize, block: &revision::Block,
    ) -> fmt::Result {
        match block {
            revision::Block::Text(text) => match self.paragraph_policy(kind, text) {
                ParagraphPolicy::LockedVerbatim(unchanged) => {
                    let unchanged = listed(&unchanged);
                    writeln!(f, "[{number}] (keep verbatim — unchanged {unchanged}) {text}")
                }
                ParagraphPolicy::Withdrawn | ParagraphPolicy::Revisable { .. } => {
                    writeln!(f, "[{number}] {text}")
                }
            },
            revision::Block::Type { key, role, .. } => {
                writeln!(f, "[{number}] {} {key} ({role})", revision::TYPE)
            }
        }
    }

    fn write_type_blocks(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if !self.redrafts(SectionKind::DomainModel) || self.outline.types.is_empty() {
            return Ok(());
        }
        f.write_str(
            "\n## Type blocks\n\nReference each key exactly once under `domain-model` as a \
             `{\"type\": {\"key\": \"<key>\", \"role\": \"data\" | \"mechanism\"}}` block; \
             the engine inserts its signature verbatim. A kept type keeps its role unless \
             the signature the change redeclares changes what it stands for; a new one is \
             `data`, a shape the program reads, stores, or produces, or `mechanism`, a class \
             that is a client, a retriever, a bootstrap, a settings carrier, or an error or \
             exception class, which a build declares nothing for and no part owns.\n\n",
        )?;
        for (key, signature) in &self.outline.types {
            match self.base.type_block(key) {
                Some((held, role)) if held == signature => {
                    writeln!(f, "- `{key}` — kept, {role}")?;
                }
                Some((_, role)) => {
                    writeln!(f, "- `{key}` — kept, {role}; the change redeclares its signature")?;
                }
                None => writeln!(f, "- `{key}` — new")?,
            }
        }
        Ok(())
    }

    fn write_requirements(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("\n## Requirements\n\n")?;
        if self.drafted.is_empty() {
            f.write_str(
                "The change adds or revises no requirement; the specification's stand as they \
                 are. What it carries lands in the paragraphs that already describe the \
                 behaviour it serves, each one you revise named in `revised`.\n",
            )?;
        } else {
            write!(
                f,
                "The requirements the change adds or revises, drafted beside this \
                 design:\n\n{bases}",
                bases = BasesSection(&self.drafted)
            )?;
        }
        if !self.retired.is_empty() {
            f.write_str(
                "\nThe requirements the change retires, which nothing of the design describes \
                 any longer:\n\n",
            )?;
            for (id, subject) in &self.retired {
                writeln!(f, "- {id} `{subject}`")?;
            }
        }
        Ok(())
    }
}

impl Brief for DesignAmend {
    type Answer = DesignAmendAnswer;
    type Output = Design;

    const NAME: &'static str = DesignBrief::NAME;
    const PROSE: &'static [&'static str] = DesignBrief::PROSE;

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        let count = self.redrafted.len();
        let sections = Self::at(schema, "/properties/sections")?;
        sections.insert("minItems".to_owned(), json!(count));
        sections.insert("maxItems".to_owned(), json!(count));

        // replace the derived `kind` reference with the redrafted sections
        let kinds = self.redrafted.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        let kind = Self::at(schema, "/$defs/Redraft/properties/kind")?;
        kind.remove("$ref");
        kind.insert("type".to_owned(), json!("string"));
        kind.insert("enum".to_owned(), json!(kinds));
        Self::at(schema, "/$defs")?.remove("SectionKind");

        // a `revised` number names a block of the longest current section;
        // none where the current design holds no redrafted section
        let longest = self
            .redrafted
            .iter()
            .filter_map(|kind| self.base.section(*kind))
            .map(|section| section.blocks.len())
            .max()
            .unwrap_or_default();
        if longest == 0 {
            Self::at(schema, "/$defs/Redraft/properties/revised")?
                .insert("maxItems".to_owned(), json!(0));
        } else {
            let number = Self::at(schema, "/$defs/Redraft/properties/revised/items")?;
            number.insert("minimum".to_owned(), json!(1));
            number.insert("maximum".to_owned(), json!(longest));
        }

        // restrict a type block's key to the kept keys and the change's
        if self.redrafts(SectionKind::DomainModel) && !self.outline.types.is_empty() {
            let keys = json!(self.outline.keys().collect::<Vec<_>>());
            Self::at(schema, "/$defs/TypeRef/properties/key")?.insert("enum".to_owned(), keys);
        }
        Ok(())
    }

    fn verify(&self, answer: &DesignAmendAnswer, review: &mut Review) {
        let mut seen = BTreeSet::new();
        let mut references: BTreeMap<String, usize> = BTreeMap::new();
        for section in &answer.sections {
            let kind = section.kind;
            let label = format!("`## {kind}`");
            if !seen.insert(kind) {
                review.note(format_args!("{label} is drafted more than once"));
            }
            if !self.redrafts(kind) {
                review.note(format_args!(
                    "{label} is not redrafted by this amendment; the current section stands as \
                     it is"
                ));
            }
            // a section answered with no block leaves the design, where every
            // paragraph of it described retired requirements alone
            if !section.blocks.is_empty() || !self.withdrawable(kind) {
                self.outline.blocks(review, kind, &label, &section.blocks, &mut references);
            }
            self.loss(section, &label, review);
        }

        for kind in self.redrafted.difference(&seen) {
            review.note(format_args!("`## {kind}` is required but absent"));
        }

        // the type blocks are the redrafted `## Domain model`'s alone; a
        // kept one carries the base's
        if self.redrafts(SectionKind::DomainModel) {
            self.outline.references(review, &references);
        }

        // the change's packages are named by the design as it will stand:
        // the redrafted sections, the kept ones, and the preamble
        let kept = self
            .base
            .sections
            .iter()
            .filter(|section| !self.redrafts(section.kind))
            .flat_map(|section| &section.blocks)
            .filter_map(|block| match block {
                revision::Block::Text(text) => Some(text.as_str()),
                revision::Block::Type { .. } => None,
            });
        let texts = answer
            .sections
            .iter()
            .flat_map(Redraft::texts)
            .chain(kept)
            .chain(self.base.preamble.iter().map(String::as_str));

        self.outline.packages(review, texts);
    }

    fn into_output(self, answer: DesignAmendAnswer) -> Result<Design, Error> {
        let mut drafted: BTreeMap<SectionKind, Vec<Block>> =
            answer.sections.into_iter().map(|section| (section.kind, section.blocks)).collect();
        let mut sections = Vec::with_capacity(self.base.sections.len() + drafted.len());
        for &kind in SectionKind::VARIANTS {
            if let Some(blocks) = drafted.remove(&kind) {
                if blocks.is_empty() {
                    continue;
                }
                sections.push(Section {
                    kind,
                    blocks: self.outline.lower(blocks)?,
                });
            } else if let Some(section) = self.base.section(kind) {
                sections.push(section.clone());
            }
        }

        Ok(Design {
            emery: EMERY,
            parent: self.base.parent,
            preamble: self.base.preamble,
            sections,
        })
    }
}

impl Display for DesignAmend {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Amend `design.md`: redraft the sections listed under `## Current design`, which \
             the change's claims inform or whose paragraphs describe a requirement the change \
             retires, and `## Overview`, which describes the whole. Every other section and the \
             preamble are the current design's and are kept as they stand, so answer those \
             sections alone.\n\n{claims}",
            claims = ClaimsSection(&self.extracts)
        )?;
        self.write_current_design(f)?;
        self.write_type_blocks(f)?;
        self.write_requirements(f)
    }
}

// Narrows whole-scope behaviour extracts to the claims that inform this design
// amendment: unchanged `type` claims are omitted, drafted amendments keep only
// claims matching drafted contributors or their cited files, pure retirements
// omit surviving whole-scope `call`, `contract`, and `excerpt` claims, and
// non-drafted runs on a snapshot-narrowed source omit `contract` claims from
// unchanged files.
struct ScopedExtracts {
    extracts: Vec<Extract>,
    types: BTreeMap<String, String>,
}

struct DraftedClaims {
    ids: BTreeSet<String>,
    citations: Vec<Cited>,
}

impl DraftedClaims {
    fn of(drafted: &[Basis]) -> Self {
        let mut ids = BTreeSet::new();
        let mut citations = Vec::new();
        for member in drafted.iter().flat_map(Basis::change_contributors) {
            ids.insert(member.id.clone());
            citations.push(Cited::from(member));
        }
        Self { ids, citations }
    }

    fn contains(&self, source: &str, claim: &Claim) -> bool {
        let cited = Cited::from_claim(source, claim);
        if let Some(claim_id) = claim.id.as_deref() {
            return self.ids.iter().any(|id| revision::leads(id, claim_id))
                || self.citations.iter().any(|member| member.overlaps(&cited));
        }
        if claim.kind == ClaimKind::Contract {
            return self.citations.iter().any(|member| member.overlaps(&cited));
        }
        self.citations.iter().any(|member| member.same_file(&cited))
    }
}

enum WholeClaims {
    Drafted(DraftedClaims),
    RetiredOnly,
    All,
}

impl WholeClaims {
    fn new(drafted: &[Basis], has_retired: bool) -> Self {
        if !drafted.is_empty() {
            Self::Drafted(DraftedClaims::of(drafted))
        } else if has_retired {
            Self::RetiredOnly
        } else {
            Self::All
        }
    }

    fn keeps(&self, extract: &Extract, claim: &Claim) -> bool {
        match self {
            Self::Drafted(drafted) => drafted.contains(&extract.source, claim),
            Self::RetiredOnly => false,
            Self::All => extract.delta().is_none_or(|delta| {
                claim.kind != ClaimKind::Contract
                    || claim.path.as_deref().is_none_or(|path| delta.is_changed(path))
            }),
        }
    }
}

impl ScopedExtracts {
    fn new(extracts: &[Extract], drafted: &[Basis], has_retired: bool, base: &Design) -> Self {
        let policy = WholeClaims::new(drafted, has_retired);
        let mut taken: BTreeMap<String, String> = BTreeMap::new();
        let mut types = BTreeMap::new();
        let mut scoped = Vec::with_capacity(extracts.len());

        for extract in extracts {
            let whole = extract.is_whole_behaviour();
            let mut claims = Vec::with_capacity(extract.evidence.claims.len());

            for claim in &extract.evidence.claims {
                if claim.kind == ClaimKind::Type {
                    let Some(signature) = claim.signature() else {
                        if !whole {
                            claims.push(claim.clone());
                        }
                        continue;
                    };
                    let Some(name) = declared(claim) else {
                        if !whole {
                            claims.push(claim.clone());
                        }
                        continue;
                    };
                    let key = type_key(&taken, name, claim.path.as_deref());
                    let trimmed = signature.trim_end().to_owned();
                    taken.insert(key.clone(), trimmed.clone());
                    if !whole || base.signature(&key) != Some(trimmed.as_str()) {
                        types.insert(key, trimmed);
                        claims.push(claim.clone());
                    }
                    continue;
                }

                if !whole || policy.keeps(extract, claim) {
                    claims.push(claim.clone());
                }
            }

            scoped.push(extract.with_claims(claims));
        }

        Self {
            extracts: scoped,
            types,
        }
    }
}

// Whether a paragraph of `base`'s `kind` section cites one of `retired`.
fn describes(base: &Design, kind: SectionKind, retired: &BTreeMap<ReqId, String>) -> bool {
    base.section(kind).is_some_and(|section| {
        section.blocks.iter().any(|block| match block {
            revision::Block::Text(text) => cited(text).iter().any(|id| retired.contains_key(id)),
            revision::Block::Type { .. } => false,
        })
    })
}

// The requirement ids `text` cites, each spelled `REQ-NNN`.
fn cited(text: &str) -> BTreeSet<ReqId> {
    text.match_indices("REQ-")
        .filter_map(|(at, prefix)| {
            let rest = &text[at + prefix.len()..];
            let end = rest.find(|ch: char| !ch.is_ascii_digit()).unwrap_or(rest.len());
            text[at..at + prefix.len() + end].parse().ok()
        })
        .collect()
}

// Whitespace collapsed: the shape a kept paragraph is compared in.
fn verbatim(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

// The items spelled in order, comma-separated, as a finding names them.
fn listed(items: impl IntoIterator<Item: Display>) -> String {
    items.into_iter().map(|item| item.to_string()).collect::<Vec<_>>().join(", ")
}

/// Model-authored content for an amended design: the sections the change informs, redrafted.
///
/// The engine keeps every other section and the preamble from the current
/// design, and inserts type signatures.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery design amendment")]
pub struct DesignAmendAnswer {
    /// One entry per section to redraft, in any order.
    pub sections: Vec<Redraft>,
}

/// One redrafted section of an amended design.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Redraft {
    /// The section heading.
    pub kind: SectionKind,
    /// The blocks, in reading order.
    pub blocks: Vec<Block>,
    /// The numbers of the current section's paragraphs this draft rewords or
    /// drops; every other is kept verbatim.
    pub revised: Vec<usize>,
}

impl Redraft {
    fn texts(&self) -> impl Iterator<Item = &str> {
        self.blocks.iter().filter_map(|block| match block {
            Block::Text(text) => Some(text.as_str()),
            Block::Type(_) => None,
        })
    }
}
