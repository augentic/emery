//! The claims an adapter returns, and the gate they must pass.
//!
//! An adapter answers with one [`Evidence`] document of [`Claim`]s. Each claim
//! has a kind from the closed [`ClaimKind`] taxonomy, and each kind states the
//! extras a complete claim of that kind carries. The [`SourceKind`] a document
//! ranks under is not part of it: the adapter declares its kind in its
//! metadata, so the model that writes the claims never chooses their
//! authority.
//!
//! The claim gate, [`Evidence::findings`], applies the rules every claim must
//! satisfy: the id grammar ([`CLAIM_ID_REGEX`]), which kinds must carry an id,
//! and the extras each kind requires. Both parties run it — an adapter before
//! its answer leaves the guest, so a bad claim can be repaired, and the
//! engine again on receipt.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::is_kebab;

/// The claim-id grammar: kebab-case segments joined by `.`.
///
/// The derived schema for [`Claim::id`] carries it as a `pattern`, and the
/// claim gate enforces it again in code.
pub const CLAIM_ID_REGEX: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

// `is_kebab` refuses the empty segment an empty value or a doubled dot
// leaves, so the split needs no further check.
fn is_claim_id(value: &str) -> bool {
    value.split('.').all(is_kebab)
}

/// The document an adapter returns: its claims, and nothing else.
///
/// This is also the shape a model answers in. It carries claims alone — the
/// kind of source is declared by the adapter, not answered by the model — so
/// a document-level `kind` is refused as an unknown field.
///
/// # Examples
///
/// ```
/// use emery_adapter::source::Evidence;
///
/// let evidence: Evidence = serde_json::from_str(
///     r#"{
///         "claims": [{
///             "kind": "requirement",
///             "id": "orders.create",
///             "path": "docs/orders.md#L3",
///             "statement": "POST /orders creates an order."
///         }]
///     }"#,
/// )?;
/// assert!(evidence.findings().is_empty());
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = "Emery evidence answer")]
pub struct Evidence {
    /// The extracted claims.
    pub claims: Vec<Claim>,
}

impl Evidence {
    /// Returns every rule the document's claims break, one line each.
    ///
    /// Findings come in claim order and name the claim by its index. An empty
    /// result means the document passes the claim gate.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_adapter::source::Evidence;
    ///
    /// let evidence: Evidence =
    ///     serde_json::from_str(r#"{ "claims": [{ "kind": "requirement", "id": "Orders" }] }"#)?;
    ///
    /// // A malformed id, and a requirement without its `statement` extra.
    /// assert_eq!(evidence.findings().len(), 2);
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    #[must_use]
    pub fn findings(&self) -> Vec<String> {
        self.claims.iter().enumerate().flat_map(|(index, claim)| claim.findings(index)).collect()
    }
}

/// The kind of source an adapter reads, which ranks its evidence.
///
/// An adapter declares its kind in its metadata, and the engine applies it to
/// every [`Evidence`] document the adapter returns. The variants are declared
/// in authority order — `Intent` outranks `Documentation`, which outranks
/// `Behaviour` — so the derived `Ord` is the precedence a cross-source
/// disagreement is resolved under.
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
pub enum SourceKind {
    /// Operator directives.
    Intent,
    /// Specifications and documentation.
    Documentation,
    /// Observed behaviour.
    Behaviour,
}

/// One typed statement about a source.
///
/// The fields every kind shares are named; anything else a kind carries
/// flattens into [`Claim::extras`]. `synopsis` and `backing` are lenient: a
/// malformed value becomes `None` rather than failing the document.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Claim {
    /// The claim's kind, from the closed taxonomy.
    pub kind: ClaimKind,
    /// A stable dotted kebab-case id; required for requirements, criteria,
    /// and examples.
    #[schemars(regex(pattern = CLAIM_ID_REGEX))]
    pub id: Option<String>,
    /// Where in the source the claim comes from: `<path>`, `<path>#L<n>`, or
    /// `<path>#L<n>-L<n>`.
    pub path: Option<String>,
    /// A one-line headline.
    #[serde(default, deserialize_with = "lenient")]
    pub synopsis: Option<String>,
    /// What backs the claim: inline data, or a path to it.
    #[serde(default, deserialize_with = "lenient")]
    pub backing: Option<Backing>,
    /// The kind-specific fields, kept for synthesis.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

impl Claim {
    /// Returns the `statement` extra as text, with whitespace collapsed.
    ///
    /// Runs of whitespace become one space, so a reflowed statement still
    /// compares equal. A non-string value is rendered as JSON rather than
    /// dropped, and a missing extra is the empty string.
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
            Some(id) if !is_claim_id(id) => {
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

/// The closed taxonomy of claim kinds.
///
/// Adding a kind is a contract change: the WIT package, the prompts, and the
/// schema move together.
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
    /// Returns `true` if a claim of this kind must carry an id.
    ///
    /// Requirements, criteria, and examples must: they are the kinds a
    /// specification cites by id.
    #[must_use]
    pub const fn requires_id(self) -> bool {
        matches!(self, Self::Requirement | Self::Criterion | Self::Example)
    }

    /// Returns the extras a complete claim of this kind must carry.
    ///
    /// Widening this table is a contract change.
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

/// What backs a claim: data carried inline, or a path to it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Backing {
    /// Verbatim data carried inline.
    Payload(String),
    /// A path within the source.
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
