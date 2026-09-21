//! Defines evidence documents, claims, and their validation rules.
//!
//! An [`Evidence`] document contains [`Claim`]s from the closed [`ClaimKind`]
//! taxonomy. The adapter declares the document's [`SourceKind`] separately,
//! so evidence cannot assign its own authority.
//!
//! [`Evidence::findings`] implements the
//! [claim gate](crate#vocabulary). It validates claim identifiers and the
//! fields required by each claim kind.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::is_kebab;

/// The regular expression for dotted, kebab-case claim identifiers.
///
/// The derived schema for [`Claim::id`] carries it as a `pattern`, and the
/// claim gate enforces it again in code.
pub const CLAIM_ID_REGEX: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

// `is_kebab` refuses the empty segment an empty value or a doubled dot
// leaves, so the split needs no further check.
fn is_claim_id(value: &str) -> bool {
    value.split('.').all(is_kebab)
}

/// A collection of claims extracted from one source.
///
/// The source kind is declared in adapter metadata and is not part of this
/// document. Unknown document fields are rejected during deserialisation.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = "Emery evidence answer")]
pub struct Evidence {
    /// The claims in source extraction order.
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

/// The authority class assigned to an adapter's evidence.
///
/// The derived ordering sorts highest authority first:
/// [`SourceKind::Intent`] outranks [`SourceKind::Documentation`], which
/// outranks [`SourceKind::Behaviour`].
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
    /// Directives supplied by an operator.
    Intent,
    /// Written specifications and documentation.
    Documentation,
    /// Behaviour extracted from an implementation.
    Behaviour,
}

/// A typed statement extracted from a source.
///
/// Fields common to every claim are represented directly. Kind-specific
/// fields are collected in [`Claim::extras`]. A malformed `synopsis` or
/// `backing` value is treated as absent rather than rejecting the document.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Claim {
    /// The taxonomy variant controlling the claim's required fields.
    pub kind: ClaimKind,
    /// A stable dotted, kebab-case identifier.
    ///
    /// Requirements, criteria, and examples require an identifier.
    #[schemars(regex(pattern = CLAIM_ID_REGEX))]
    pub id: Option<String>,
    /// The source location as `<path>`, `<path>#L<n>`, or
    /// `<path>#L<n>-L<n>`.
    pub path: Option<String>,
    /// An optional one-line summary.
    #[serde(default, deserialize_with = "lenient")]
    pub synopsis: Option<String>,
    /// Optional supporting material.
    #[serde(default, deserialize_with = "lenient")]
    pub backing: Option<Backing>,
    /// Additional fields specific to the claim's [`ClaimKind`].
    ///
    /// [`ClaimKind::required_extras`] lists the fields required by the claim
    /// gate.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

impl Claim {
    /// Returns the `statement` extra as text, with whitespace collapsed.
    ///
    /// Runs of whitespace become one space, so a reflowed statement still
    /// compares equal. A non-string value is rendered as JSON rather than
    /// dropped, and a missing extra is the empty string.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_adapter::source::Evidence;
    ///
    /// let evidence: Evidence = serde_json::from_str(
    ///     r#"{ "claims": [{
    ///         "kind": "requirement",
    ///         "id": "orders.create",
    ///         "statement": "  Creates\n an order. "
    ///     }] }"#,
    /// )?;
    ///
    /// assert_eq!(evidence.claims[0].statement(), "Creates an order.");
    /// # Ok::<(), serde_json::Error>(())
    /// ```
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

/// The supported kinds of extracted claims.
///
/// The taxonomy is closed. Unknown kinds are rejected during deserialisation.
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
    /// Returns whether claims of this kind require an identifier.
    ///
    /// Requirements, criteria, and examples must: they are the kinds a
    /// specification cites by id.
    #[must_use]
    pub const fn requires_id(self) -> bool {
        matches!(self, Self::Requirement | Self::Criterion | Self::Example)
    }

    /// Returns the extras a complete claim of this kind must carry.
    ///
    /// Requirements need `statement`, criteria need `criterion`, examples
    /// need `replay-digest`, and other kinds need none.
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

/// Supporting material attached to a claim.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Backing {
    /// Verbatim data stored in the evidence document.
    Payload(String),
    /// A path to supporting material within the source.
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
