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

use omnia_sdk::Error;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use strum::VariantArray as _;

use crate::brief::{Brief, Review};
use crate::revision::{self, Change, ChangeClass, Design, EMERY, ReqId, Section, SectionKind};
use crate::specify::Extract;
use crate::specify::basis::Basis;
use crate::specify::design::{Block, DesignBrief, Outline};
use crate::specify::sections::{BasesSection, ClaimsSection};

// The brief of an amendment's design turn: the sections the change informs,
// redrafted over the current design.
pub struct DesignAmend<'a> {
    extracts: &'a [Extract],
    // The requirements the change adds, revises, or leaves in conflict, each
    // under the id it takes.
    drafted: &'a [Basis<'a>],
    // The current design less the types the change's retirements orphaned,
    // its `parent` the base revision.
    base: &'a Design,
    outline: Outline<'a>,
    // The sections the turn redrafts, in vocabulary order: `## Overview`,
    // which describes the whole, every section a change claim informs, and
    // every section a paragraph of which describes a retired requirement.
    redrafted: Vec<SectionKind>,
    // The requirements the change leaves as they stand.
    unchanged: BTreeSet<ReqId>,
    // The requirements the change retires, by id and subject.
    retired: BTreeMap<ReqId, &'a str>,
}

impl<'a> DesignAmend<'a> {
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
        extracts: &'a [Extract], drafted: &'a [Basis<'a>], changes: &'a [Change], base: &'a Design,
    ) -> Option<Self> {
        let outline = Outline::amend(extracts, base);
        let retired: BTreeMap<ReqId, &'a str> = changes
            .iter()
            .filter(|change| change.class == ChangeClass::Retired)
            .map(|change| (change.id, change.subject.as_str()))
            .collect();
        let touches = |kind| outline.informed(kind) || describes(base, kind, &retired);
        if !SectionKind::VARIANTS.iter().copied().any(touches) {
            return None;
        }
        let redrafted = SectionKind::VARIANTS
            .iter()
            .copied()
            .filter(|&kind| kind == SectionKind::Overview || touches(kind))
            .collect();

        Some(Self {
            extracts,
            drafted,
            base,
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

    // Whether `text` describes requirements the change retires, and no
    // other: such a paragraph describes nothing the specification states any
    // longer, so the turn may drop or reword it without naming it.
    fn withdrawn(&self, text: &str) -> bool {
        let cited = cited(text);
        !cited.is_empty() && cited.iter().all(|id| self.retired.contains_key(id))
    }

    // Whether the base's `kind` section may leave the design: it stands,
    // every block of it is a paragraph describing retired requirements
    // alone, and it is not `## Overview`, which every design holds.
    fn withdrawable(&self, kind: SectionKind) -> bool {
        kind != SectionKind::Overview
            && self.base.section(kind).is_some_and(|section| {
                section.blocks.iter().all(|block| match block {
                    revision::Block::Text(text) => self.withdrawn(text),
                    revision::Block::Type { .. } => false,
                })
            })
    }

    // Whether a paragraph of `kind` citing `cited` describes unchanged
    // requirements alone and must stay verbatim: outside `## Overview`, in a
    // change that drafts a requirement.
    fn untouched(&self, kind: SectionKind, cited: &BTreeSet<ReqId>) -> bool {
        kind != SectionKind::Overview
            && !self.drafted.is_empty()
            && !cited.is_empty()
            && cited.is_subset(&self.unchanged)
    }

    // The loss check over one redrafted section against the base's, three
    // cases over a base paragraph by what it cites:
    // - it is kept verbatim or named `revised`, and a named one is reworded,
    //   never dropped: every id it cites that the change keeps is cited by
    //   some paragraph of the answered section, and, outside `## Overview`
    //   where the change drafts a requirement, one citing unchanged
    //   requirements alone is kept verbatim, since a change that drafts
    //   none carries technology alone, which lands in the paragraphs
    //   describing the behaviour it serves;
    // - citing retired requirements alone, it is withdrawn: dropped or
    //   reworded, named or not;
    // - a number naming a verbatim paragraph, a type block, or no block of
    //   the section is a finding.
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
            match (present.contains(&verbatim(text)), named) {
                (true, true) => review.note(format_args!(
                    "{label} [{number}] is named in `revised` but is kept verbatim; drop it from \
                     `revised`"
                )),
                (false, false) if !self.withdrawn(text) => review.note(format_args!(
                    "{label} [{number}] is dropped or reworded without being named in `revised`; \
                     keep it verbatim, or name it"
                )),
                (false, true) => {
                    let cited = cited(text);
                    if self.untouched(section.kind, &cited) {
                        review.note(format_args!(
                            "{label} [{number}] describes {}, which the change leaves \
                             unchanged; keep it verbatim",
                            listed(&cited)
                        ));
                        continue;
                    }
                    let undescribed: Vec<ReqId> = cited
                        .into_iter()
                        .filter(|id| !self.retired.contains_key(id) && !described.contains(id))
                        .collect();
                    if !undescribed.is_empty() {
                        review.note(format_args!(
                            "{label} [{number}] is revised and describes {}, which the change \
                             keeps, yet no paragraph of the section describes {} now; reword it \
                             rather than dropping it",
                            listed(&undescribed),
                            if undescribed.len() == 1 { "it" } else { "them" }
                        ));
                    }
                }
                _ => {}
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
}

impl Brief for DesignAmend<'_> {
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
        let mut references: BTreeMap<&str, usize> = BTreeMap::new();
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

        for kind in self.redrafted.iter().filter(|kind| !seen.contains(kind)) {
            review.note(format_args!("`## {kind}` is required but absent"));
        }

        // the type blocks are the redrafted `## Domain model`'s alone; a
        // kept one carries the base's
        if self.redrafts(SectionKind::DomainModel) {
            self.outline.references(review, &references);
        }
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
            parent: self.base.parent.clone(),
            preamble: self.base.preamble.clone(),
            sections,
        })
    }
}

impl Display for DesignAmend<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Amend `design.md`: redraft the sections listed under `## Current design`, which \
             the change's claims inform or whose paragraphs describe a requirement the change \
             retires, and `## Overview`, which describes the whole. Every other section and the \
             preamble are the current design's and are kept as they stand, so answer those \
             sections alone.\n\n{claims}",
            claims = ClaimsSection(self.extracts)
        )?;

        f.write_str(
            "\n## Current design\n\nEach section to redraft is listed with its blocks numbered. \
             Keep a paragraph verbatim unless the change touches what it describes, and name \
             every paragraph you reword or drop in that section's `revised` list by its number: \
             a paragraph neither kept verbatim nor named is refused, and so is one named but \
             kept verbatim. Name a paragraph `revised` to reword it, never to drop it: every \
             requirement it describes that the change keeps is still described by a paragraph \
             of the section, so a paragraph you drop describes retired requirements alone, or \
             none. A type block is \
             listed as its `Type:` line and is kept or dropped by its key under `## Type \
             blocks`, never named in `revised`.\n",
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
        for kind in &self.redrafted {
            write!(f, "\n### {kind}\n\n")?;
            let Some(section) = self.base.section(*kind) else {
                f.write_str("The current design has no such section; draft it.\n")?;
                continue;
            };
            for (position, block) in section.blocks.iter().enumerate() {
                let number = position + 1;
                match block {
                    revision::Block::Text(text) => writeln!(f, "[{number}] {text}")?,
                    revision::Block::Type { key, role, .. } => {
                        writeln!(f, "[{number}] {} {key} ({role})", revision::TYPE)?;
                    }
                }
            }
        }

        if self.redrafts(SectionKind::DomainModel) && !self.outline.types.is_empty() {
            f.write_str(
                "\n## Type blocks\n\nReference each key exactly once under `domain-model` as a \
                 `{\"type\": {\"key\": \"<key>\", \"role\": \"data\" | \"mechanism\"}}` block; \
                 the engine inserts its signature verbatim. A kept type keeps its role unless \
                 the signature the change redeclares changes what it stands for; a new one is \
                 `data`, a shape the program reads, stores, or produces, or `mechanism`, a class \
                 that is a client, a retriever, a bootstrap, a settings carrier, or an error or \
                 exception class, which a build declares nothing for and no slice owns.\n\n",
            )?;
            for (key, &signature) in &self.outline.types {
                match self.base.type_block(key) {
                    Some((held, role)) if held == signature => {
                        writeln!(f, "- `{key}` — kept, {role}")?;
                    }
                    Some((_, role)) => writeln!(
                        f,
                        "- `{key}` — kept, {role}; the change redeclares its signature"
                    )?,
                    None => writeln!(f, "- `{key}` — new")?,
                }
            }
        }

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
                bases = BasesSection(self.drafted)
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

// Whitespace collapsed: the shape a kept paragraph is compared in.
fn verbatim(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

// The items spelled in order, comma-separated, as a finding names them.
fn listed(items: impl IntoIterator<Item: Display>) -> String {
    items.into_iter().map(|item| item.to_string()).collect::<Vec<_>>().join(", ")
}

// Whether a paragraph of `base`'s `kind` section cites one of `retired`.
fn describes(base: &Design, kind: SectionKind, retired: &BTreeMap<ReqId, &str>) -> bool {
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
