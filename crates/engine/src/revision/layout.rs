//! Defines the content-addressed layout a `## UI / layout` section names.
//!
//! A source may record a layout whole: a body the engine stores by its
//! [`LayoutDigest`] and never reads. The design holds a [`Layout`] block
//! naming the body by digest, with images rendered from it for review; the
//! body itself stands in the engine's storage and is delivered to every
//! slice a target builds.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

pub const LAYOUT: &str = "Layout:";

const SCHEME: &str = "sha256:";

/// The `sha256:<hex>` digest of a layout body, its address in storage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LayoutDigest(String);

impl LayoutDigest {
    /// The digest of `body`.
    #[must_use]
    pub fn of(body: &[u8]) -> Self {
        Self(format!("{SCHEME}{}", hex::encode(Sha256::digest(body))))
    }

    /// The digest as written, `sha256:` and 64 lowercase hex digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The 64 hex digits alone, the key the body is stored under.
    #[must_use]
    pub fn hex(&self) -> &str {
        &self.0[SCHEME.len()..]
    }
}

impl Display for LayoutDigest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for LayoutDigest {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, String> {
        let hex = value
            .strip_prefix(SCHEME)
            .ok_or_else(|| format!("layout digest `{value}` does not start with `{SCHEME}`"))?;
        if hex.len() != 64 || !hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(format!("layout digest `{value}` is not 64 lowercase hex digits"));
        }
        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<String> for LayoutDigest {
    type Error = String;

    fn try_from(value: String) -> Result<Self, String> {
        value.parse()
    }
}

impl From<LayoutDigest> for String {
    fn from(digest: LayoutDigest) -> Self {
        digest.0
    }
}

/// A layout block of `## UI / layout`: the body's digest, what it is, the
/// source that recorded it, and the images rendered from it for review.
///
/// Attached by the engine after the design turn, never drafted: a turn
/// neither sees nor drops one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Layout {
    pub digest: LayoutDigest,
    pub media_type: String,
    // The body's length in bytes, for review: the body is not in the design.
    pub size: u64,
    // The `[[source]]` name that recorded the layout.
    pub source: String,
    pub views: Vec<LayoutView>,
}

/// An image rendered from a layout, shown inline in `design.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LayoutView {
    pub label: String,
    pub media_type: String,
    pub body: String,
}

impl Display for Layout {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{LAYOUT} {} ({}, {} bytes, recorded by `{}`)",
            self.digest, self.media_type, self.size, self.source
        )?;
        for view in &self.views {
            write!(
                f,
                "\n\n![{}](data:{};base64,{})",
                view.label,
                view.media_type,
                STANDARD.encode(view.body.as_bytes())
            )?;
        }
        Ok(())
    }
}
