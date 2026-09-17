//! Defines the contract between the Emery engine and source adapters.
//!
//! A source adapter receives a [`source::SourceInput`] and returns
//! [`source::Evidence`] containing typed [`source::Claim`]s. The
//! [`source::Source`] capability is the engine-facing side of that exchange.
//!
//! This crate supplies the shared Rust types for the `emery:adapter` WIT
//! package. On WebAssembly targets, `source::export` also exposes the guest
//! interface implemented by adapters.
//!
//! # Examples
//!
//! Create an input and validate an adapter response:
//!
//! ```
//! use emery_adapter::source::{Evidence, SourceInput};
//!
//! let input = SourceInput::workspace("orders", ".");
//! assert_eq!(input.key, "orders");
//!
//! let evidence: Evidence = serde_json::from_str(
//!     r#"{
//!         "claims": [{
//!             "kind": "requirement",
//!             "id": "orders.create",
//!             "statement": "POST /orders creates an order."
//!         }]
//!     }"#,
//! )?;
//! assert!(evidence.findings().is_empty());
//! # Ok::<(), serde_json::Error>(())
//! ```
//!
//! # Vocabulary
//!
//! - **Source adapter**: a component that extracts claims from one source.
//! - **Evidence**: the complete set of [`source::Claim`]s returned for one
//!   input.
//! - **Claim gate**: the validation performed by
//!   [`source::Evidence::findings`] before evidence is accepted.

pub mod source;

/// Returns whether `value` is lowercase kebab-case.
///
/// A valid value matches `[a-z0-9]+(-[a-z0-9]+)*`.
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
