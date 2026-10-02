//! Defines the identifiers a revision's records carry.
//!
//! [`ReqId`] numbers requirements and [`SliceId`] slices. Each is assigned
//! from one in the order the engine fixes, rendered behind its own prefix,
//! and stored as that rendering. A requirement's subject is a dotted claim
//! id, and its stem is the first segment.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

// One identifier type over a `u32` behind a fixed prefix: rendered padded to
// three digits, parsed only from that rendering, and stored as the string.
macro_rules! numbered {
    ($(#[$doc:meta])* $name:ident, $prefix:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(u32);

        impl $name {
            const PREFIX: &str = $prefix;

            /// Returns the id numbered `number`.
            ///
            /// Numbers produced by the engine begin at one.
            #[must_use]
            pub const fn new(number: u32) -> Self {
                Self(number)
            }
        }

        impl FromStr for $name {
            type Err = String;

            // An id is well formed exactly when it renders back to itself.
            fn from_str(text: &str) -> Result<Self, String> {
                let malformed = || format!("malformed id `{text}`");
                let digits = text.strip_prefix(Self::PREFIX).ok_or_else(malformed)?;
                let number: u32 = digits.parse().ok().ok_or_else(malformed)?;
                let id = Self(number);
                if number == 0 || id.to_string() != text {
                    return Err(malformed());
                }
                Ok(id)
            }
        }

        impl TryFrom<String> for $name {
            type Error = String;

            fn try_from(text: String) -> Result<Self, String> {
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
    /// assert_eq!(ReqId::new(7).to_string(), "REQ-007");
    /// assert_eq!("REQ-007".parse::<ReqId>()?, ReqId::new(7));
    /// assert!("REQ-7".parse::<ReqId>().is_err());
    /// # Ok::<(), String>(())
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
    /// assert_eq!(SliceId::new(2).to_string(), "SLICE-002");
    /// assert_eq!("SLICE-002".parse::<SliceId>()?, SliceId::new(2));
    /// assert!("SLICE-2".parse::<SliceId>().is_err());
    /// # Ok::<(), String>(())
    /// ```
    SliceId, "SLICE-"
}

pub fn stem(id: &str) -> &str {
    id.split_once('.').map_or(id, |(stem, _)| stem)
}
