//! The kebab grammar
//!
//! The one spelling every name in the contract follows: source keys, adapter
//! names, and each segment of a claim id are kebab-case, and a claim id is
//! kebab segments joined by `.`. Declaring the grammar once keeps the engine's
//! checks on keys and names and the claim gate's check on ids from drifting
//! apart.

/// Claim-id grammar. Rides the derived `Claim.id` schema as a steering
/// `pattern` and is enforced again in code.
pub const CLAIM_ID_REGEX: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

/// Tells whether `value` follows the kebab grammar shared by claim-id
/// segments, source keys, and adapter names: `[a-z0-9]+(-[a-z0-9]+)*`.
#[must_use]
pub fn is_kebab(value: &str) -> bool {
    value.split('-').all(|segment| {
        !segment.is_empty() && segment.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9'))
    })
}

// Tells whether `value` is kebab segments joined by `.`; `is_kebab` refuses
// the empty segment an empty value or a doubled dot leaves.
pub fn is_valid(value: &str) -> bool {
    value.split('.').all(is_kebab)
}
