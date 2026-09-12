//! Evidence
//!
//! What an adapter returns: an [`Evidence`] document of typed [`Claim`]s
//! under an [`Authority`] class — the spec IR every source is reduced to.
//! [`ClaimKind`] is the closed taxonomy the whole system agrees on, and each
//! kind's required extras are declared next to it, so the contract states in
//! one place what a complete claim of that kind looks like.
//!
//! The rules a claim must satisfy — the grammar its id follows, which kinds
//! must carry an id at all, and the extras each kind requires — are applied
//! by [`Evidence::findings`], which reports every violation as one line. Two
//! parties enforce them: an adapter checks its own answer so a bad claim can
//! be repaired before it leaves the guest, and the engine checks again on
//! receipt, because it cannot assume every adapter did.
//!
//! Serde derives sit only on the shapes that cross a JSON boundary: the
//! document a model answer is parsed into, and the [`Authority`] a committed
//! requirement records.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grammar::{CLAIM_ID_REGEX, is_valid};

/// Extracted claims and their document-level authority.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Evidence {
    /// Document-level authority.
    pub authority: Authority,
    /// Extracted claims.
    pub claims: Vec<Claim>,
}

impl Evidence {
    /// Collects every finding over the document's claims, one line each, in
    /// claim order; empty when the document passes the gate.
    #[must_use]
    pub fn findings(&self) -> Vec<String> {
        self.claims.iter().enumerate().flat_map(|(index, claim)| claim.findings(index)).collect()
    }
}

/// Claim-set authority. The variants are declared in rank order, so the
/// derived `Ord` is the authority hierarchy: `Intent` outranks
/// `Documentation`, which outranks `Behaviour`.
#[derive(
    Clone,
    Copy,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    JsonSchema,
    strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Authority {
    /// Operator directives.
    Intent,
    /// Specifications and documentation.
    Documentation,
    /// Observed behaviour.
    Behaviour,
}

/// A claim extracted from a source.
///
/// Open per-kind fields flatten into [`Claim::extras`]. `synopsis` and
/// `backing` are lenient: malformed shapes become absent.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Claim {
    /// Kind from the closed taxonomy.
    pub kind: ClaimKind,
    /// Stable dotted-kebab ID; required for requirements, criteria, and examples.
    #[schemars(regex(pattern = CLAIM_ID_REGEX))]
    pub id: Option<String>,
    /// Source anchor: `<path>`, `<path>#L<n>`, or `<path>#L<n>-L<n>`.
    pub path: Option<String>,
    /// Semantic headline.
    #[serde(default, deserialize_with = "lenient")]
    pub synopsis: Option<String>,
    /// Path or inline backing.
    #[serde(default, deserialize_with = "lenient")]
    pub backing: Option<Backing>,
    /// Open per-kind fields preserved for synthesis.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

impl Claim {
    /// The `statement` extra as text, every run of whitespace collapsed to
    /// one space so a reflowed statement still matches. A non-string value
    /// is rendered rather than dropped: the claim gate requires the extra,
    /// not that it is a string.
    #[must_use]
    pub fn statement(&self) -> String {
        let normalise = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
        match self.extras.get("statement") {
            Some(serde_json::Value::String(text)) => normalise(text),
            Some(other) => normalise(&other.to_string()),
            None => String::new(),
        }
    }

    // The claim's findings as `claim {index}`: an id outside the dotted-kebab
    // grammar, or none on a kind that requires one; then every extra its kind
    // requires that it lacks.
    fn findings(&self, index: usize) -> impl Iterator<Item = String> + '_ {
        let kind = self.kind;
        let id = match self.id.as_deref() {
            Some(id) if !is_valid(id) => {
                Some(format!("- claim {index}: id `{id}` does not match `{CLAIM_ID_REGEX}`"))
            }
            None if kind.requires_id() => {
                Some(format!("- claim {index}: `{kind}` claims require an id"))
            }
            _ => None,
        };

        let extras =
            kind.required_extras().iter().filter(|&key| !self.extras.contains_key(*key)).map(
                move |key| {
                    format!(
                        "- claim {index}: `{kind}` `{label}` is missing extra `{key}`",
                        label = self.id.as_deref().unwrap_or("<unnamed>"),
                    )
                },
            );

        id.into_iter().chain(extras)
    }
}

/// Closed claim taxonomy; update the workflow contract and schema together.
#[derive(
    Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, JsonSchema, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ClaimKind {
    /// Operator intent.
    Intent,
    /// Behavioural requirement.
    Requirement,
    /// Acceptance criterion.
    Criterion,
    /// Recorded decision.
    Decision,
    /// Document section.
    Section,
    /// Diagram.
    Diagram,
    /// API contract.
    Contract,
    /// Runtime capture.
    Example,
    /// Verbatim excerpt.
    Excerpt,
    /// Type declaration.
    Type,
    /// Call site.
    Call,
    /// Code region.
    Region,
    /// Container such as a module or package.
    Container,
    /// Leaf item.
    Leaf,
}

impl ClaimKind {
    /// Tells whether a claim of this kind must carry an id: requirements,
    /// criteria, and examples, the kinds a specification cites by id.
    #[must_use]
    pub const fn requires_id(self) -> bool {
        matches!(self, Self::Requirement | Self::Criterion | Self::Example)
    }

    /// Returns the extras this kind must carry.
    ///
    /// Widening this closed table is a contract change.
    #[must_use]
    pub const fn required_extras(self) -> &'static [&'static str] {
        match self {
            Self::Requirement => &["statement"],
            Self::Criterion => &["criterion"],
            Self::Example => &["replay-digest"],
            _ => &[],
        }
    }
}

/// Claim backing.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Backing {
    /// Inline verbatim data.
    Payload(String),
    /// Filesystem path.
    Path(String),
}

// Deserializes an open field leniently: a malformed value becomes `None`
// instead of failing the whole document.
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}
