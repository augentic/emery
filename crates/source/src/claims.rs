//! Claim rules
//!
//! What makes a claim well formed: the grammar its id must follow, which
//! claim kinds must carry an id at all, and the extra fields each kind is
//! required to include. [`Evidence::findings`] applies all of them to a whole
//! document and reports every violation as one line.
//!
//! The rules live in the contract crate because two parties enforce them.
//! An adapter checks its own answer so a bad claim can be repaired before it
//! leaves the guest; the engine checks again on receipt, because it cannot
//! assume every adapter did.

use crate::types::{Claim, ClaimKind, Evidence};

/// Claim-id grammar. Rides the derived `Claim.id` schema as a steering
/// `pattern` and is enforced again in code.
pub const DOTTED_KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

impl Evidence {
    /// Collects every finding over the document's claims, one line each, in
    /// claim order; empty when the document passes the gate.
    #[must_use]
    pub fn findings(&self) -> Vec<String> {
        self.claims.iter().enumerate().flat_map(|(index, claim)| claim.findings(index)).collect()
    }
}

impl Claim {
    // The claim's findings as `claim {index}`: an id outside the dotted-kebab
    // grammar, or none on a kind that requires one; then every extra its kind
    // requires that it lacks.
    fn findings(&self, index: usize) -> impl Iterator<Item = String> + '_ {
        let kind = self.kind;
        let id = match self.id.as_deref() {
            Some(id) if !is_dotted_kebab(id) => {
                Some(format!("- claim {index}: id `{id}` does not match `{DOTTED_KEBAB_PATTERN}`"))
            }
            None if !kind.required_extras().is_empty() => {
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

impl ClaimKind {
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

// Tells whether `value` is kebab segments joined by `.`; `is_kebab` refuses
// the empty segment an empty value or a doubled dot leaves.
fn is_dotted_kebab(value: &str) -> bool {
    value.split('.').all(is_kebab)
}

/// Tells whether `value` follows the kebab grammar shared by claim-id
/// segments, source keys, and adapter names: `[a-z0-9]+(-[a-z0-9]+)*`.
#[must_use]
pub fn is_kebab(value: &str) -> bool {
    value.split('-').all(|segment| {
        !segment.is_empty() && segment.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9'))
    })
}
