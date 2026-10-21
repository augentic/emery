//! Cuts the design to a slice for its build brief: a type the adapter laid
//! reduced to a pointer, a type no text of the slice speaks of left out.

use std::borrow::Cow;
use std::fmt::Write as _;

use emery_adapter::target::Slice;
use emery_lexicon::words;

use super::Scaffold;

// The engine's rendering of a type claim's signature: this line over a fence.
const TYPE: &str = "Type: ";
const MECHANISM: &str = " (mechanism)";

// The design as the brief carries it: a data type the adapter laid reduced
// to a line naming it, one owned by no entry of the slice's `types` and
// spelled by none of the slice's text left out and listed once at the end,
// every mechanism type and every paragraph as the design holds them.
pub(super) fn cut(slice: &Slice, scaffold: Option<&Scaffold<'_>>) -> String {
    let laid = scaffold.map_or(&[][..], |scaffold| scaffold.types);
    let spelled = spelled(slice, scaffold);
    let mut kept: Vec<Cow<'_, str>> = Vec::new();
    let mut left_out: Vec<&str> = Vec::new();
    for block in blocks(&slice.design) {
        let Some((key, mechanism)) = type_block(block) else {
            kept.push(Cow::Borrowed(block));
            continue;
        };
        if laid.iter().any(|laid| laid == key) {
            kept.push(Cow::Owned(format!(
                "{TYPE}{key} — laid in the tree by the adapter before this turn, as the scaffold \
                 section says."
            )));
        } else if mechanism
            || slice.types.iter().any(|owned| owned.key == key)
            || names(key, &spelled)
        {
            kept.push(Cow::Borrowed(block));
        } else {
            left_out.push(key);
        }
    }
    let mut text = kept.join("\n\n");
    if !left_out.is_empty() {
        let keys: Vec<String> = left_out.iter().map(|key| format!("`{key}`")).collect();
        let _ = write!(
            text,
            "\n\nDesign types left out, owned by other slices and named by none of this \
             slice's text: {}.",
            keys.join(", ")
        );
    }
    text
}

// The words of each text of the slice in their order: its plan entry, its
// cut of the specification, the signatures of the types it owns, and the
// scaffold's brief, never its passages, since a reference quoted for the
// turn speaks of no type of this design.
fn spelled(slice: &Slice, scaffold: Option<&Scaffold<'_>>) -> Vec<Vec<String>> {
    [slice.plan.as_str(), slice.spec.as_str()]
        .into_iter()
        .chain(slice.types.iter().map(|owned| owned.signature.as_str()))
        .chain(scaffold.map(|scaffold| scaffold.brief))
        .map(words)
        .collect()
}

// Whether the slice's text spells the type: the words of the key's
// identifier, its stem and its anchor set aside, run together somewhere in
// one text, each matching a plural as its singular, so `order line` is
// named by `OrderLine` or `order lines` and not by `order` here and `line`
// there.
fn names(key: &str, spelled: &[Vec<String>]) -> bool {
    let identifier = key.split(" (").next().unwrap_or(key);
    let identifier = identifier.rsplit('.').next().unwrap_or(identifier);
    let words = words(identifier);
    !words.is_empty()
        && spelled.iter().any(|text| {
            text.windows(words.len())
                .any(|run| run.iter().zip(&words).all(|(spelt, word)| same(spelt, word)))
        })
}

fn same(spelt: &str, word: &str) -> bool {
    spelt == word || spelt.strip_suffix('s') == Some(word) || word.strip_suffix('s') == Some(spelt)
}

// The key of a `Type:` block and whether the design labels it a mechanism;
// `None` for any other block.
fn type_block(block: &str) -> Option<(&str, bool)> {
    let (first, rest) = block.split_once('\n')?;
    let key = first.strip_prefix(TYPE)?;
    if !rest.starts_with("```") {
        return None;
    }
    let (key, mechanism) = key.strip_suffix(MECHANISM).map_or((key, false), |key| (key, true));
    Some((key.trim(), mechanism))
}

// The design's blocks: the runs of lines between blank lines outside a
// fence, so a signature's blank lines stay within its block.
fn blocks(design: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut in_fence = false;
    let mut start: Option<usize> = None;
    let mut at = 0;
    for line in design.split_inclusive('\n') {
        if line.starts_with("```") {
            in_fence = !in_fence;
        }
        if line.trim().is_empty() && !in_fence {
            if let Some(from) = start.take() {
                blocks.push(design[from..at].trim_end());
            }
        } else if start.is_none() {
            start = Some(at);
        }
        at += line.len();
    }
    if let Some(from) = start {
        blocks.push(design[from..].trim_end());
    }
    blocks
}
