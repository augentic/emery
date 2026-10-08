//! Ranks the sources of a run by authority.
//!
//! Every source carries one [`Rank`]: the rank the project's config gives
//! it, or its kind's default. Reconciliation reads nothing else of a source's
//! authority.

use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;

use emery_adapter::source::SourceKind;
use serde::{Deserialize, Serialize};

/// The authority rank of a source: `1` is the highest, and equal ranks tie.
///
/// A source is ranked by the `rank` its `[[source]]` entry states, or by its
/// kind's default when it states none. A requirement's agreeing
/// classes are ordered by their leading contributor's rank: a class alone at
/// the top rank wins as a divergence, and two classes tied there are a
/// conflict. Zero is not a rank, so `rank = 0` is refused where the entry is
/// read.
///
/// # Examples
///
/// ```
/// use emery_adapter::source::SourceKind;
/// use emery_engine::Rank;
///
/// assert!(Rank::of(SourceKind::Intent) < Rank::of(SourceKind::Documentation));
/// assert_eq!(Rank::of(SourceKind::Behaviour).to_string(), "3");
/// assert!(serde_json::from_str::<Rank>("0").is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Rank(NonZeroU32);

/// Converts a source kind to its default rank.
impl From<SourceKind> for Rank {
    fn from(value: SourceKind) -> Self {
        Self(match value {
            SourceKind::Intent => NonZeroU32::MIN,
            SourceKind::Documentation => NonZeroU32::MIN.saturating_add(1),
            SourceKind::Behaviour => NonZeroU32::MIN.saturating_add(2),
        })
    }
}

impl Display for Rank {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}
