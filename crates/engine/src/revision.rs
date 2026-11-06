//! Defines stored revisions and their Markdown projections.
//!
//! A [`Revision`] stores a typed specification, design, and plan as canonical
//! JSON. Its content digest is the revision identifier. Markdown is rendered
//! on demand and is never parsed back into revision data.
//!
//! [`Document`] defines shared serialisation and rendering behaviour.
//! [`Diff`] describes changes between revisions, and [`Amendment`] a change
//! staged against the current one.

mod amendment;
mod design;
mod diff;
mod id;
mod layout;
mod plan;
mod spec;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use anyhow::Context;
use omnia_sdk::{Error, server_error};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub use self::amendment::{
    Amendment, AmendmentBase, BriefState, Change, ChangeClass, SliceChange, SliceClass,
};
pub use self::design::{Block, Design, Role, Section, SectionKind, TYPE, citations};
use self::diff::BRIEF;
pub use self::diff::{Changed, DesignDiff, Diff, Entry, PlanDiff, Retired, SliceEntry, SpecDiff};
pub use self::id::{BadId, ReqId, SliceId, leads, stem, substem};
pub use self::layout::{LAYOUT, Layout, LayoutDigest, LayoutView};
use self::plan::{DEPENDS_ON, REQUIREMENTS, TYPES};
pub use self::plan::{Plan, Slice, Waves, ranks};
pub use self::spec::{Cited, Loser, Requirement, Scenario, Spec, Status};
use self::spec::{HEADING, ID, MESSAGE, NOTE, ROUTE, SOURCES, STATUS};

// The revision grammar written and accepted by this engine.
//
// A stored revision stamped with another grammar is outdated.
pub const EMERY: u32 = 9;

// Line prefixes reserved for engine-generated Markdown.
//
// `#`, so no draft line reads as a heading, and the engine's own line keys,
// so no draft line passes as provenance, a trigger, a note, a type or
// layout label, or a slice's lists.
pub const RESERVED: &[&str] = &[
    "#",
    ID,
    SOURCES,
    STATUS,
    ROUTE,
    MESSAGE,
    NOTE,
    TYPE,
    LAYOUT,
    REQUIREMENTS,
    TYPES,
    DEPENDS_ON,
];

// A typed revision document stored as JSON and rendered as Markdown.
pub trait Document: Serialize + DeserializeOwned + Display {
    const NAME: &'static str;

    // The grammar stamp is checked before the shape, so another grammar's
    // document is reported as outdated rather than malformed.
    fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        let value: Value = serde_json::from_slice(bytes)
            .with_context(|| format!("`{}` is not JSON", Self::NAME))?;

        // check the grammar stamp before the shape
        let stamp = &value["emery"];
        if *stamp != EMERY {
            return Err(Error::BadRequest {
                code: "spec-outdated".into(),
                description: format!(
                    "`{}` was written under emery grammar {stamp}; this engine reads {EMERY}",
                    Self::NAME
                ),
            });
        }

        Ok(serde_json::from_value(value)
            .with_context(|| format!("`{}` is not a revision", Self::NAME))?)
    }

    // Pretty-printed, in declaration order, with one trailing newline.
    fn to_json(&self) -> Result<String, Error> {
        let mut text = serde_json::to_string_pretty(self)
            .with_context(|| format!("`{}` does not serialise", Self::NAME))?;
        text.push('\n');
        Ok(text)
    }

    #[must_use]
    fn to_markdown(&self, id: &str) -> String {
        format!("---\nemery: {EMERY}\nrevision: {id}\n---\n\n{self}")
    }
}

// The three canonical JSON documents of a revision, `(name, body)` in digest order.
pub type Documents = [(&'static str, String); 3];

// The specification, design, and plan committed as one unit.
#[derive(Debug, Clone)]
pub struct Revision {
    pub spec: Spec,
    pub design: Design,
    pub plan: Plan,
}

impl Revision {
    // The storage names of the three documents, in digest order.
    pub const DOCUMENTS: [&str; 3] = [Spec::NAME, Design::NAME, Plan::NAME];

    // The content digest is checked before any document is deserialised.
    pub fn read(id: &str, spec: &[u8], design: &[u8], plan: &[u8]) -> Result<Self, Error> {
        if Self::identify(spec, design, plan) != id {
            return Err(server_error!("revision `{id}` does not match its content"));
        }

        Ok(Self {
            spec: Spec::from_json(spec)?,
            design: Design::from_json(design)?,
            plan: Plan::from_json(plan)?,
        })
    }

    // Serialises the three documents to canonical JSON, in digest order, with their content id.
    pub fn documents(&self) -> Result<(String, Documents), Error> {
        let documents = [
            (Spec::NAME, self.spec.to_json()?),
            (Design::NAME, self.design.to_json()?),
            (Plan::NAME, self.plan.to_json()?),
        ];
        let id = Self::identify(
            documents[0].1.as_bytes(),
            documents[1].1.as_bytes(),
            documents[2].1.as_bytes(),
        );
        Ok((id, documents))
    }

    // The content identifier of the three documents as stored.
    #[must_use]
    pub fn identify(spec: &[u8], design: &[u8], plan: &[u8]) -> String {
        digest(&[spec, design, plan])
    }

    // What tells the three documents apart as one revision's: the plan
    // places each requirement of the specification in one slice, owns each
    // data type of the design in one, and names nothing else. Every seal
    // satisfies it, so a finding names a mix of two revisions' documents,
    // which a merge resolved one file at a time leaves.
    #[must_use]
    pub fn findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        let stated: BTreeSet<ReqId> =
            self.spec.requirements.iter().map(|requirement| requirement.id).collect();
        let keys = self.design.data_types();
        let mut placed: BTreeMap<ReqId, SliceId> = BTreeMap::new();
        let mut owned: BTreeMap<&str, SliceId> = BTreeMap::new();
        for slice in &self.plan.slices {
            for &id in &slice.requirements {
                if !stated.contains(&id) {
                    findings.push(format!(
                        "`{id}` is placed by `{}` and is no requirement of the specification",
                        slice.id
                    ));
                } else if let Some(other) = placed.insert(id, slice.id) {
                    findings.push(format!("`{id}` is placed by `{other}` and `{}`", slice.id));
                }
            }
            for key in &slice.types {
                if !keys.contains(key) {
                    findings.push(format!(
                        "`{key}` is owned by `{}` and is no data type of the design",
                        slice.id
                    ));
                } else if let Some(other) = owned.insert(key, slice.id) {
                    findings.push(format!("`{key}` is owned by `{other}` and `{}`", slice.id));
                }
            }
        }
        for id in stated.iter().filter(|id| !placed.contains_key(id)) {
            findings.push(format!("`{id}` is placed by no slice"));
        }
        for key in keys.iter().filter(|key| !owned.contains_key(key.as_str())) {
            findings.push(format!("`{key}` is owned by no slice"));
        }
        findings
    }
}

// The content id of one or more document bodies: SHA-256 over each body
// behind its length, so no boundary between bodies is ambiguous.
fn digest(bodies: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for body in bodies {
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}

fn write<T: Display>(
    f: &mut Formatter<'_>, title: &str, preamble: &[String], blocks: &[T],
) -> fmt::Result {
    write!(f, "# {title}")?;
    for paragraph in preamble {
        write!(f, "\n\n{paragraph}")?;
    }
    for block in blocks {
        write!(f, "\n\n{block}")?;
    }
    f.write_str("\n")
}

// A `Key: [a, b]` line with no trailing newline.
fn write_list<T: Display>(f: &mut Formatter<'_>, key: &str, items: &[T]) -> fmt::Result {
    write!(f, "{key} [")?;
    for (position, item) in items.iter().enumerate() {
        if position > 0 {
            f.write_str(", ")?;
        }
        write!(f, "{item}")?;
    }
    f.write_str("]")
}
