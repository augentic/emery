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

use crate::{BadMessage, BadPath, BadRoute, BadTrigger, Route, Trigger, beneath, is_kebab};

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
    /// grammar, every required extra is present, no refused extra is carried,
    /// every `path` parses as an [`Anchor`], and every `contract`'s `route`
    /// or `message` extra parses as a [`Trigger`], one of the two apiece.
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
    /// Requirements, criteria, retirements, and examples require an
    /// identifier.
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
    /// [`ClaimKind::required_extras`] and [`ClaimKind::refused_extras`] list
    /// the fields the claim gate requires and refuses.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

impl Claim {
    /// The extra a `call` names what it reaches in, as `<package>:<member>`.
    pub const CALLEE: &'static str = "callee";
    /// The extra a `criterion` states its condition in.
    pub const CRITERION: &'static str = "criterion";
    /// The extra a `decision` carries its directive in.
    pub const DECISION: &'static str = "decision";
    /// The extra a `contract` carries its inbound message's name in.
    pub const MESSAGE: &'static str = "message";
    /// The extra a `type` carries its declared identifier in.
    pub const NAME: &'static str = "name";
    /// The extra an `example` carries the digest of its capture in.
    pub const REPLAY_DIGEST: &'static str = "replay-digest";
    /// The extra a `contract` carries its route in, as `METHOD /path`.
    pub const ROUTE: &'static str = "route";
    /// The extra a `type` carries its declaration in.
    pub const SIGNATURE: &'static str = "signature";
    /// The extra a `requirement` or `retire` states its behaviour or withdrawal in.
    pub const STATEMENT: &'static str = "statement";

    /// Returns a claim of `kind` carrying nothing else.
    ///
    /// The common fields are set through [`with_id`](Self::with_id),
    /// [`at`](Self::at), and [`with_synopsis`](Self::with_synopsis), and an
    /// extra through [`with_extra`](Self::with_extra).
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_adapter::source::{Claim, ClaimKind};
    ///
    /// let claim = Claim::new(ClaimKind::Requirement)
    ///     .with_id("orders.create")
    ///     .at("src/orders.ts#L12")
    ///     .with_extra(Claim::STATEMENT, "Creates an order.");
    ///
    /// assert_eq!(claim.id.as_deref(), Some("orders.create"));
    /// assert_eq!(claim.statement(), "Creates an order.");
    /// ```
    #[must_use]
    pub fn new(kind: ClaimKind) -> Self {
        Self {
            kind,
            id: None,
            path: None,
            synopsis: None,
            backing: None,
            extras: serde_json::Map::new(),
        }
    }

    /// Returns a `type` claim declaring `name` with `signature`.
    ///
    /// The claim carries [`NAME`](Self::NAME) and
    /// [`SIGNATURE`](Self::SIGNATURE) and nothing else.
    #[must_use]
    pub fn type_decl(name: &str, signature: &str) -> Self {
        Self::new(ClaimKind::Type)
            .with_extra(Self::NAME, name)
            .with_extra(Self::SIGNATURE, signature)
    }

    /// Returns a `contract` claim reached through `trigger`.
    ///
    /// The claim carries the extra of the trigger's kind,
    /// [`ROUTE`](Self::ROUTE) or [`MESSAGE`](Self::MESSAGE), spelled as
    /// [`Trigger`] displays it, so [`trigger`](Self::trigger) reads it back.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_adapter::source::Claim;
    /// use emery_adapter::{Route, Trigger};
    ///
    /// let trigger = Trigger::Http(Route::parse("post /orders/:id/pay")?);
    /// let claim = Claim::contract(&trigger).with_id("orders");
    ///
    /// assert_eq!(claim.extra(Claim::ROUTE), Some("POST /orders/{id}/pay"));
    /// assert_eq!(claim.trigger(), Some(Ok(trigger)));
    /// # Ok::<(), emery_adapter::BadRoute>(())
    /// ```
    #[must_use]
    pub fn contract(trigger: &Trigger) -> Self {
        let key = match trigger {
            Trigger::Http(_) => Self::ROUTE,
            Trigger::Message(_) => Self::MESSAGE,
        };
        Self::new(ClaimKind::Contract).with_extra(key, trigger.to_string())
    }

    /// Returns the claim with its `id` set.
    #[must_use]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Returns the claim anchored at `path`, in the grammar of [`Anchor`].
    #[must_use]
    pub fn at(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Returns the claim with its `synopsis` set.
    #[must_use]
    pub fn with_synopsis(mut self, synopsis: impl Into<String>) -> Self {
        self.synopsis = Some(synopsis.into());
        self
    }

    /// Returns the claim with the extra `key` set to `value`.
    #[must_use]
    pub fn with_extra(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.extras.insert(key.to_owned(), value.into());
        self
    }

    /// Returns the extra `key` as text.
    ///
    /// `None` when the claim carries no such extra, or one that is not a
    /// string.
    #[must_use]
    pub fn extra(&self, key: &str) -> Option<&str> {
        self.extras.get(key)?.as_str()
    }

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
        match self.extras.get(Self::STATEMENT) {
            Some(serde_json::Value::String(text)) => normalise(text),
            Some(other) => normalise(&other.to_string()),
            None => String::new(),
        }
    }

    /// Returns the `criterion` extra as text, a `criterion`'s condition.
    #[must_use]
    pub fn criterion(&self) -> Option<&str> {
        self.extra(Self::CRITERION)
    }

    /// Returns the `callee` extra as text, what a `call` reaches as `<package>:<member>`.
    #[must_use]
    pub fn callee(&self) -> Option<&str> {
        self.extra(Self::CALLEE)
    }

    /// Returns the `signature` extra as text, a `type`'s declaration.
    #[must_use]
    pub fn signature(&self) -> Option<&str> {
        self.extra(Self::SIGNATURE)
    }

    /// Returns the `name` extra as text, a `type`'s declared identifier.
    #[must_use]
    pub fn type_name(&self) -> Option<&str> {
        self.extra(Self::NAME)
    }

    /// Returns the `decision` extra as text, a `decision`'s directive.
    #[must_use]
    pub fn decision(&self) -> Option<&str> {
        self.extra(Self::DECISION)
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

        // the extras the kind refuses
        for key in kind.refused_extras() {
            if self.extras.contains_key(*key) {
                findings.push(format!(
                    "- claim {index}: `{kind}` `{label}` carries extra `{key}`, which belongs to \
                     another kind"
                ));
            }
        }

        // the anchor grammar
        if let Some(path) = self.path.as_deref()
            && let Err(bad) = Anchor::parse(path)
        {
            findings.push(format!("- claim {index}: path `{path}` {bad}"));
        }

        // the trigger grammar
        if kind == ClaimKind::Contract
            && let Some(Err(bad)) = self.trigger()
        {
            let spelled =
                |key: &str| self.extras.get(key).map(ToString::to_string).unwrap_or_default();
            findings.push(match &bad {
                BadTrigger::Route(_) => {
                    format!("- claim {index}: route {} {bad}", spelled(Self::ROUTE))
                }
                BadTrigger::Message(_) => {
                    format!("- claim {index}: message {} {bad}", spelled(Self::MESSAGE))
                }
                BadTrigger::Both => format!("- claim {index}: {bad}"),
            });
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

    /// Returns the claim's `route` or `message` extra parsed as a [`Trigger`].
    ///
    /// `None` when the claim carries neither; `Some(Err(_))` when it carries
    /// both, or one outside its grammar or that is not a string, which
    /// [`Evidence::findings`] reports for a `contract`.
    #[must_use]
    pub fn trigger(&self) -> Option<Result<Trigger, BadTrigger>> {
        let route = self.extras.get(Self::ROUTE);
        let message = self.extras.get(Self::MESSAGE);
        Some(match (route, message) {
            (None, None) => return None,
            (Some(_), Some(_)) => Err(BadTrigger::Both),
            (Some(serde_json::Value::String(text)), None) => {
                Route::parse(text).map(Trigger::Http).map_err(BadTrigger::Route)
            }
            (Some(_), None) => Err(BadTrigger::Route(BadRoute::Grammar)),
            (None, Some(serde_json::Value::String(text))) => {
                Trigger::message(text).map_err(BadTrigger::Message)
            }
            (None, Some(_)) => Err(BadTrigger::Message(BadMessage::Empty)),
        })
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
    /// Withdrawal of a behaviour.
    Retire,
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
    /// Requirements, criteria, retirements, and examples require one. Each
    /// names the subject it states, checks, or withdraws, or the capture it
    /// records.
    #[must_use]
    pub const fn requires_id(self) -> bool {
        matches!(self, Self::Requirement | Self::Criterion | Self::Retire | Self::Example)
    }

    /// Returns whether claims of this kind state, check, or withdraw a subject's behaviour.
    ///
    /// Requirements, criteria, and retirements carry a subject's `<stem>.<leaf>`
    /// identifier and a body stated by role. The SDK holds them to a seam's
    /// stems and lexicon.
    #[must_use]
    pub const fn is_subject(self) -> bool {
        matches!(self, Self::Requirement | Self::Criterion | Self::Retire)
    }

    /// Returns the extras a complete claim of this kind must carry.
    ///
    /// Requirements and retirements need [`Claim::STATEMENT`], criteria
    /// [`Claim::CRITERION`], examples [`Claim::REPLAY_DIGEST`], and other
    /// kinds none.
    #[must_use]
    pub const fn required_extras(self) -> &'static [&'static str] {
        match self {
            Self::Requirement | Self::Retire => &[Claim::STATEMENT],
            Self::Criterion => &[Claim::CRITERION],
            Self::Example => &[Claim::REPLAY_DIGEST],
            _ => &[],
        }
    }

    /// Returns the extras a claim of this kind must not carry.
    ///
    /// A retirement withdraws a behaviour and carries its `statement` alone:
    /// a [`Claim::CRITERION`], [`Claim::CALLEE`], [`Claim::SIGNATURE`],
    /// [`Claim::ROUTE`], or [`Claim::MESSAGE`] extra on it is another kind's
    /// claim. Other kinds refuse none.
    #[must_use]
    pub const fn refused_extras(self) -> &'static [&'static str] {
        match self {
            Self::Retire => {
                &[Claim::CRITERION, Claim::CALLEE, Claim::SIGNATURE, Claim::ROUTE, Claim::MESSAGE]
            }
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
