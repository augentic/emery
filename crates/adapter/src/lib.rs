#![warn(missing_docs, clippy::missing_errors_doc)]

//! Defines the contract between the Emery engine and its adapters.
//!
//! A source adapter receives a [`source::SourceInput`] and returns
//! [`source::Evidence`] containing typed [`source::Claim`]s. The
//! [`source::Source`] capability is the engine-facing side of that exchange.
//! A target adapter runs a [`target::Task`] in a workspace: an init of its
//! layout, a build of one [`target::Slice`] of the plan answered by a
//! [`target::Report`], or a verify of the integrated tree answered by a
//! [`target::Verdict`], each an [`target::Outcome`]; [`target::Target`] is
//! the engine-facing side.
//!
//! This crate supplies the shared Rust types for the `emery:adapter` WIT
//! package. On WebAssembly targets, `source::export` and `target::export`
//! also expose the guest interfaces implemented by adapters.
//!
//! # Examples
//!
//! Create an input and validate an adapter response:
//!
//! ```
//! use emery_adapter::source::{Evidence, SourceInput};
//!
//! let input = SourceInput::workspace("orders", ".");
//! assert_eq!(input.name, "orders");
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
//! - **Target adapter**: a component that initialises a workspace with its
//!   layout, builds one slice of the plan into it, and verifies the
//!   integrated tree.
//! - **Report gate**: the validation performed by
//!   [`target::Report::findings`] before a report is accepted.
//! - **Root-relative path**: a `/`-separated path beneath the root a run
//!   lends, which both gates hold to [`beneath`].
//! - **Technology**: a vendor, product, protocol, framework, or host a
//!   requirement names in place of the role it plays, which a [`Lexicon`]
//!   reads a text for, over its fixed names and those of a source's
//!   packages.

mod path;
pub mod source;
pub mod target;
mod technology;

pub use path::{BadPath, SKIP_DIRS, SKIP_FILES, beneath};
pub use technology::Lexicon;

/// Returns whether `value` is lowercase kebab-case.
///
/// A valid value matches `[a-z0-9]+(-[a-z0-9]+)*`.
///
/// Claim-id segments, source names, and adapter names all follow this grammar.
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
