//! The `source-adapter` world an adapter exports, and its `metadata` answer.
//!
//! A guest implements [`Guest`] on a unit type and invokes [`export!`] for
//! it once, at its crate root. `metadata` answers with [`metadata`];
//! `extract` lifts the WIT input into a [`SourceInput`](crate::SourceInput),
//! surveys it, and hands the seams to [`mine`](crate::mine) — the outcome
//! lowers back through `into` and `?`. The crate-level example is a complete
//! guest.

pub use emery_adapter::source::export::*;

use crate::SourceKind;

/// Returns the `metadata` answer for an adapter reading `kind` sources.
///
/// The `emery-version` pin is this SDK's own version: the contract the
/// adapter compiled against.
#[must_use]
pub fn metadata(kind: SourceKind) -> AdapterMetadata {
    crate::AdapterMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        kind,
    }
    .into()
}
