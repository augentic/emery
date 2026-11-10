//! Derives inter-part dependency edges from design data types and their signatures.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Formatter};

use crate::revision::{Design, Part, PartId};
use crate::specify::plan::{Draft, quoted};

// Keywords that introduce a type or value's declared identifier in a signature.
const KEYWORDS: &[&str] = &["interface", "type", "class", "struct", "enum", "const", "def"];

// The design's `Role::Data` types and the data types each signature
// references.
#[derive(Debug, Clone, Default)]
pub struct Types {
    // The `Role::Data` type keys in design order.
    pub keys: Vec<String>,
    // For each key, the other keys whose declared identifier appears as a
    // whole token in its signature, in design order.
    references: BTreeMap<String, Vec<String>>,
}

impl Types {
    // Reads the `Role::Data` type keys, declared names, and cross-type
    // references from `design`.
    #[must_use]
    pub fn of(design: &Design) -> Self {
        let keys = design.data_types();
        let declared: BTreeMap<&str, &str> = keys
            .iter()
            .map(|key| {
                let signature = design.signature(key).unwrap_or("");
                (key.as_str(), declared_name(signature, key))
            })
            .collect();

        // two keys sharing one declared identifier reference neither
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for &name in declared.values() {
            *counts.entry(name).or_default() += 1;
        }

        let references = keys
            .iter()
            .map(|key| {
                let signature = design.signature(key).unwrap_or("");
                let tokens: BTreeSet<&str> = tokens(signature).collect();
                let named = keys
                    .iter()
                    .filter(|&other| other != key)
                    .filter(|&other| {
                        let target = declared[other.as_str()];
                        !target.is_empty()
                            && counts.get(target) == Some(&1)
                            && tokens.contains(target)
                    })
                    .cloned()
                    .collect();
                (key.clone(), named)
            })
            .collect();

        Self { keys, references }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    // Returns the keys of `chosen` that exist in the design, deduplicated in
    // design order.
    fn ordered(&self, chosen: &[String]) -> Vec<String> {
        self.keys.iter().filter(|key| chosen.contains(key)).cloned().collect()
    }

    // Orders and deduplicates `owned` and `uses` in design order, pruning
    // keys absent from the design and keys from `uses` that `owned` holds.
    pub fn settle(&self, owned: &mut Vec<String>, uses: &mut Vec<String>) {
        *owned = self.ordered(owned);
        uses.retain(|key| !owned.contains(key));
        *uses = self.ordered(uses);
    }

    // Writes one `- `<key>`` bullet line, listing the other keys its
    // signature names where any exist.
    pub fn write_key(&self, f: &mut Formatter<'_>, key: &str) -> fmt::Result {
        match self.references(key) {
            [] => writeln!(f, "- `{key}`"),
            named => writeln!(f, "- `{key}` — names {}", quoted(named)),
        }
    }

    // The keys whose declared identifier appears in `key`'s signature, in
    // design order.
    #[must_use]
    pub fn references(&self, key: &str) -> &[String] {
        self.references.get(key).map_or(&[], Vec::as_slice)
    }

    // Derives inter-part dependency edges across `drafts` from each draft's
    // owned `types` and named `uses`.
    #[must_use]
    pub fn owner_edges(&self, drafts: &[Draft]) -> Vec<Edge> {
        let mut edges = Vec::new();
        for from in drafts {
            let refs = from.types.iter().flat_map(|key| self.references(key));
            let uses = from.uses.iter().filter(|key| !from.types.contains(key));
            let needed: BTreeSet<&str> = refs.chain(uses).map(String::as_str).collect();
            if needed.is_empty() {
                continue;
            }

            for to in drafts {
                if to.name == from.name {
                    continue;
                }
                let via: Vec<String> = self
                    .keys
                    .iter()
                    .filter(|key| needed.contains(key.as_str()) && to.types.contains(key))
                    .cloned()
                    .collect();
                if !via.is_empty() {
                    edges.push(Edge {
                        from: from.name.clone(),
                        to: to.name.clone(),
                        via,
                    });
                }
            }
        }
        edges
    }

    // Sets each part's `depends_on` from the edges derived across `parts`,
    // sorted and deduplicated in `PartId` order.
    pub fn depend(&self, parts: &mut [Part]) {
        let edges = self.owner_edges(&Draft::all(parts));
        let ids: BTreeMap<String, PartId> =
            parts.iter().map(|part| (part.name.clone(), part.id)).collect();
        for part in parts {
            part.depends_on = edges
                .iter()
                .filter(|edge| edge.from == part.name)
                .filter_map(|edge| ids.get(&edge.to).copied())
                .collect();
            part.depends_on.sort_unstable();
            part.depends_on.dedup();
        }
    }
}

// The identifier `signature` declares after a declaration keyword, or `key`'s
// last segment when the signature names none.
fn declared_name<'a>(signature: &'a str, key: &'a str) -> &'a str {
    for line in signature.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || trimmed.starts_with('@')
        {
            continue;
        }
        let head = trimmed.split(['{', '=']).next().unwrap_or(trimmed);
        let mut iter = tokens(head);
        while let Some(token) = iter.next() {
            if KEYWORDS.contains(&token)
                && let Some(name) = iter.find(|candidate| {
                    !KEYWORDS.contains(candidate)
                        && candidate
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_alphabetic() || ch == '_')
                })
            {
                return name;
            }
        }
        if trimmed.contains(['{', '=']) {
            break;
        }
    }
    let bare = key.split(" (").next().unwrap_or(key).trim();
    bare.rsplit_once('.').map_or(bare, |(_, segment)| segment)
}

fn tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(|ch: char| !(ch.is_alphanumeric() || ch == '_')).filter(|token| !token.is_empty())
}

// A directed dependency from one drafted part to another, with the target's
// owned type keys that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    // The keys owned by `to` that `from`'s owned types name or `from` lists
    // in `uses`, in design order.
    pub via: Vec<String>,
}
