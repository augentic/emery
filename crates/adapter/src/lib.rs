#![warn(missing_docs, clippy::missing_errors_doc)]

//! Defines the contract between the Emery engine and its adapters.
//!
//! Supplies the shared Rust types and `wasm32` guest bindings for the
//! `emery:adapter` WIT package: [`source`] for claim extraction and
//! [`target`] for workspace initialisation, slice builds, and wave
//! verification.
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
//! - **Source adapter**: a component that extracts [`source::Evidence`] from a
//!   [`source::SourceInput`].
//! - **Claim gate**: validation performed by [`source::Evidence::findings`].
//! - **Target adapter**: a component that runs [`target::Task`]s and returns
//!   [`target::Outcome`]s.
//! - **Report gate**: validation performed by [`target::Report::findings`].
//! - **Root-relative path**: a `/`-separated path validated by [`beneath`].
//! - **Trigger**: what reaches a requirement, the surface its source
//!   registered it at: an HTTP [`Route`], `POST /orders/{id}`, in the one
//!   grammar [`Route::parse`] reads, or a message by name, a broker topic or
//!   a socket event, as [`Trigger`] carries either.

mod path;
mod route;
pub mod source;
pub mod target;
mod trigger;

pub use path::{
    BadPath, NoFollow, SKIP_DIRS, SKIP_FILES, Standing, beneath, make_dirs, standing, write_whole,
};
pub use route::{BadRoute, METHODS, Route};
pub use trigger::{BadMessage, BadTrigger, Trigger};

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
