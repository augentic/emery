//! The contract between the Emery engine and its source adapters.
//!
//! The engine asks a source adapter to read one source — a document tree, a
//! codebase, a written brief — and to answer with typed claims about it. This
//! crate is the Rust side of the `emery:adapter` WIT package, compiled into
//! both parties so neither can drift from the other. The engine depends on it
//! directly; adapters receive it re-exported through the `emery-sdk` crate.
//!
//! The package is organised by axis, one module each. Today there is one,
//! [`source`], carrying:
//!
//! - [`source::SourceInput`], what an adapter is given: a key and a workspace
//!   or inline value.
//! - [`source::Evidence`], what it returns: a document of typed
//!   [`source::Claim`]s, checked by the claim gate
//!   [`source::Evidence::findings`].
//! - [`source::Source`], the capability the engine calls adapters through.
//!
//! Every name in the contract — a claim-id segment, a source key, an adapter
//! name — follows the kebab grammar [`is_kebab`] checks.

pub mod source;

/// Returns `true` if `value` is kebab-case: `[a-z0-9]+(-[a-z0-9]+)*`.
///
/// Claim-id segments, source keys, and adapter names all follow this grammar.
///
/// # Examples
///
/// ```
/// use emery_adapter::is_kebab;
///
/// assert!(is_kebab("orders-api"));
/// assert!(!is_kebab("Orders"));
/// assert!(!is_kebab("orders--api"));
/// assert!(!is_kebab(""));
/// ```
#[must_use]
pub fn is_kebab(value: &str) -> bool {
    value.split('-').all(|segment| {
        !segment.is_empty() && segment.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9'))
    })
}
