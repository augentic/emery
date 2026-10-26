//! Defines the identifiers a revision's records carry.
//!
//! [`ReqId`] numbers requirements and [`SliceId`] slices. Each is assigned
//! from one in the order the engine fixes, rendered behind its own prefix,
//! and stored as that rendering. A requirement's subject is a dotted claim
//! id, and its stem is the first segment.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The error of parsing a requirement or slice id outside its grammar.
///
/// An id is well formed exactly when it renders back to itself: the prefix,
/// then a positive number padded to at least three digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadId(String);

impl Display for BadId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "malformed id `{}`", self.0)
    }
}

impl Error for BadId {}

// One identifier type over a `NonZeroU32` behind a fixed prefix: rendered
// padded to three digits, parsed only from that rendering, and stored as the
// string.
macro_rules! numbered {
    ($(#[$doc:meta])* $name:ident, $prefix:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(NonZeroU32);

        impl $name {
            const PREFIX: &str = $prefix;

            /// The first id, numbered one.
            pub const FIRST: Self = Self(NonZeroU32::MIN);

            /// Returns the id numbered `number`.
            #[must_use]
            pub const fn new(number: NonZeroU32) -> Self {
                Self(number)
            }

            /// Returns the id's number.
            #[must_use]
            pub const fn number(self) -> NonZeroU32 {
                self.0
            }

            /// Returns the id after this one, or `None` at [`u32::MAX`].
            #[must_use]
            pub const fn next(self) -> Option<Self> {
                match self.0.checked_add(1) {
                    Some(next) => Some(Self(next)),
                    None => None,
                }
            }

            /// Returns the ids in order from [`FIRST`](Self::FIRST), one per record.
            pub fn sequence() -> impl Iterator<Item = Self> {
                std::iter::successors(Some(Self::FIRST), |&id| id.next())
            }

            // The ids in order above the highest id `used` holds, or from
            // `FIRST` when `used` is empty.
            pub(crate) fn free_after(
                used: impl IntoIterator<Item = Self>,
            ) -> impl Iterator<Item = Self> {
                let first = used.into_iter().max().map_or(Some(Self::FIRST), Self::next);
                std::iter::successors(first, |&id| id.next())
            }
        }

        impl FromStr for $name {
            type Err = BadId;

            fn from_str(text: &str) -> Result<Self, BadId> {
                let malformed = || BadId(text.to_owned());
                let digits = text.strip_prefix(Self::PREFIX).ok_or_else(malformed)?;
                let number: NonZeroU32 = digits.parse().ok().ok_or_else(malformed)?;
                let id = Self(number);
                if id.to_string() != text {
                    return Err(malformed());
                }
                Ok(id)
            }
        }

        impl TryFrom<String> for $name {
            type Error = BadId;

            fn try_from(text: String) -> Result<Self, BadId> {
                text.parse()
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.to_string()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                write!(f, "{}{:03}", Self::PREFIX, self.0)
            }
        }
    };
}

numbered! {
    /// A requirement identifier rendered as `REQ-NNN`.
    ///
    /// Parsing accepts positive numbers in canonical form, padded to at least
    /// three digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_engine::specify::ReqId;
    ///
    /// let seventh = ReqId::sequence().nth(6).expect("ids run on");
    /// assert_eq!(seventh.to_string(), "REQ-007");
    /// assert_eq!("REQ-007".parse::<ReqId>()?, seventh);
    /// assert_eq!(ReqId::FIRST.to_string(), "REQ-001");
    /// assert!("REQ-7".parse::<ReqId>().is_err());
    /// assert!("REQ-000".parse::<ReqId>().is_err());
    /// # Ok::<(), emery_engine::specify::BadId>(())
    /// ```
    ReqId, "REQ-"
}

numbered! {
    /// A slice identifier rendered as `SLICE-NNN`.
    ///
    /// Parsing accepts positive numbers in canonical form, padded to at least
    /// three digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_engine::specify::SliceId;
    ///
    /// let second = SliceId::sequence().nth(1).expect("ids run on");
    /// assert_eq!(second.to_string(), "SLICE-002");
    /// assert_eq!("SLICE-002".parse::<SliceId>()?, second);
    /// assert!("SLICE-2".parse::<SliceId>().is_err());
    /// # Ok::<(), emery_engine::specify::BadId>(())
    /// ```
    SliceId, "SLICE-"
}

pub fn stem(id: &str) -> &str {
    id.split_once('.').map_or(id, |(stem, _)| stem)
}

// The first two segments of a dotted id; an id of fewer is whole.
pub fn substem(id: &str) -> &str {
    id.match_indices('.').nth(1).map_or(id, |(end, _)| &id[..end])
}
