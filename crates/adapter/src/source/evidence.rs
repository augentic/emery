//! Defines evidence documents, claims, and their validation rules.
//!
//! An [`Evidence`] document contains [`Claim`]s from the closed [`ClaimKind`]
//! taxonomy. The adapter declares the document's [`SourceKind`] separately,
//! so evidence cannot assign its own authority.
//!
//! [`Evidence::findings`] implements the
//! [claim gate](crate#vocabulary). It validates claim identifiers, the
//! fields required by each claim kind, and the grammar of `path` anchors.

use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{BadPath, beneath, is_kebab};

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

/// A parsed `path` anchor: the file a claim cites and the lines within it.
///
/// The grammar is `<path>`, `<path>#L<n>`, or `<path>#L<start>-L<end>`,
/// with the path relative to the source root and the lines 1-indexed.
///
/// # Examples
///
/// ```
/// use emery_adapter::source::Anchor;
///
/// let anchor = Anchor::parse("src/orders.ts#L12-L34")?;
/// assert_eq!(anchor.path, "src/orders.ts");
/// assert_eq!(anchor.lines, Some((12, 34)));
/// assert_eq!(anchor.to_string(), "src/orders.ts#L12-L34");
///
/// assert!(Anchor::parse("../secret.ts").is_err());
/// assert!(Anchor::parse("src/orders.ts#L34-L12").is_err());
/// # Ok::<(), emery_adapter::source::BadAnchor>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Anchor<'a> {
    /// The cited file, relative to the source root.
    pub path: &'a str,
    /// The cited line range, inclusive, when the anchor names one.
    pub lines: Option<(u64, u64)>,
}

// The anchor in the grammar `parse` reads: one line as `#L<n>`, never as a
// range of one.
impl Display for Anchor<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.path)?;
        match self.lines {
            None => Ok(()),
            Some((start, end)) if start == end => write!(f, "#L{start}"),
            Some((start, end)) => write!(f, "#L{start}-L{end}"),
        }
    }
}

impl<'a> Anchor<'a> {
    /// Parses `anchor` under the grammar of the claim rules.
    ///
    /// # Errors
    ///
    /// Returns [`BadAnchor`] describing the first rule the anchor breaks: a
    /// path [`beneath`] refuses, a fragment outside the grammar, or a range
    /// that ends before it starts.
    pub fn parse(anchor: &'a str) -> Result<Self, BadAnchor> {
        let (path, fragment) = anchor
            .split_once('#')
            .map_or((anchor, None), |(path, fragment)| (path, Some(fragment)));
        beneath(path)?;

        let lines = fragment.map(lines).transpose()?;
        if let Some((start, end)) = lines
            && end < start
        {
            return Err(BadAnchor::Reversed { start, end });
        }

        Ok(Self { path, lines })
    }
}

// The fragment after `#`: `L<n>` or `L<start>-L<end>`, each a positive
// decimal with no sign, padding, or whitespace.
fn lines(fragment: &str) -> Result<(u64, u64), BadAnchor> {
    let number = |text: &str| -> Option<u64> {
        let digits = text.strip_prefix('L')?;
        if digits.is_empty()
            || digits.starts_with('0')
            || !digits.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        digits.parse().ok()
    };
    let parsed = match fragment.split_once('-') {
        Some((start, end)) => number(start).zip(number(end)),
        None => number(fragment).map(|line| (line, line)),
    };
    parsed.ok_or(BadAnchor::Grammar)
}

/// The rule a `path` anchor breaks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BadAnchor {
    /// The path breaks a rule of [`beneath`].
    Path(BadPath),
    /// The fragment is not `#L<n>` or `#L<start>-L<end>`.
    Grammar,
    /// The range ends before it starts.
    Reversed {
        /// The first cited line.
        start: u64,
        /// The last cited line, before the first.
        end: u64,
    },
}

impl From<BadPath> for BadAnchor {
    fn from(bad: BadPath) -> Self {
        Self::Path(bad)
    }
}

impl Display for BadAnchor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(bad) => bad.fmt(f),
            Self::Grammar => f.write_str("is not `<path>`, `<path>#L<n>`, or `<path>#L<n>-L<n>`"),
            Self::Reversed { start, end } => {
                write!(f, "ends at line {end}, before it starts at line {start}")
            }
        }
    }
}

impl std::error::Error for BadAnchor {}

/// A collection of claims extracted from one source.
///
/// The source kind is declared in adapter metadata and is not part of this
/// document. Unknown document fields are rejected during deserialisation, and
/// a document serialises back to the JSON it was answered as, with absent
/// optional fields omitted and a malformed `synopsis` or `backing` dropped
/// (see [`Claim`]).
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
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
    /// result means the document passes the claim gate: every id is in the
    /// grammar, every required extra is present, and every `path` parses as
    /// an [`Anchor`].
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_adapter::source::Evidence;
    ///
    /// let evidence: Evidence = serde_json::from_str(
    ///     r#"{ "claims": [{ "kind": "requirement", "id": "Orders", "path": "../x.ts" }] }"#,
    /// )?;
    ///
    /// // A malformed id, a requirement without its `statement` extra, and a
    /// // path that escapes the source root.
    /// assert_eq!(evidence.findings().len(), 3);
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
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Claim {
    /// The taxonomy variant controlling the claim's required fields.
    pub kind: ClaimKind,
    /// A stable dotted, kebab-case identifier.
    ///
    /// Requirements, criteria, and examples require an identifier.
    #[schemars(regex(pattern = CLAIM_ID_REGEX))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The source location as `<path>`, `<path>#L<n>`, or
    /// `<path>#L<n>-L<n>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// An optional one-line summary.
    #[serde(default, deserialize_with = "lenient", skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,
    /// Optional supporting material.
    #[serde(default, deserialize_with = "lenient", skip_serializing_if = "Option::is_none")]
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

    fn findings(&self, index: usize) -> Vec<String> {
        let kind = self.kind;
        let mut findings = Vec::new();

        // the id rule
        match self.id.as_deref() {
            Some(id) if !is_claim_id(id) => {
                findings
                    .push(format!("- claim {index}: id `{id}` does not match `{CLAIM_ID_REGEX}`"));
            }
            None if kind.requires_id() => {
                findings.push(format!("- claim {index}: `{kind}` claims require an id"));
            }
            _ => {}
        }

        // the extras the kind requires
        let label = self.id.as_deref().unwrap_or("<unnamed>");
        for key in kind.required_extras() {
            if !self.extras.contains_key(*key) {
                findings
                    .push(format!("- claim {index}: `{kind}` `{label}` is missing extra `{key}`"));
            }
        }

        // the anchor grammar
        if let Some(path) = self.path.as_deref()
            && let Err(bad) = Anchor::parse(path)
        {
            findings.push(format!("- claim {index}: path `{path}` {bad}"));
        }

        findings
    }

    /// Returns the claim's `path` parsed as an [`Anchor`].
    ///
    /// `None` when the claim carries no path; `Some(Err(_))` when it carries
    /// one outside the grammar, which [`Evidence::findings`] reports.
    #[must_use]
    pub fn anchor(&self) -> Option<Result<Anchor<'_>, BadAnchor>> {
        self.path.as_deref().map(Anchor::parse)
    }
}

/// The supported kinds of extracted claims.
///
/// The taxonomy is closed. Unknown kinds are rejected during deserialisation.
#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    JsonSchema,
    strum::Display,
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
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Backing {
    /// Verbatim data stored in the evidence document.
    Payload(String),
    /// A path to supporting material within the source.
    Path(String),
}

fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}
