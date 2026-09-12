//! The source axis
//!
//! The `emery:adapter/source` seam: the `source-adapter` WIT world, the
//! records that cross it — what an adapter is given ([`SourceInput`]) and
//! what it returns ([`Evidence`], the spec IR) — the claim gate those records
//! must pass, and the [`Source`] capability the engine calls source adapters
//! through.
//!
//! Every public item of the axis is exported here from three private
//! modules: `capability` (the import-side trait and the inbound records),
//! `evidence` (the outbound document and its gate), and, on `wasm32`,
//! `bindings` (the one WIT generation both sides ride).

#[cfg(target_arch = "wasm32")]
mod bindings;
mod capability;
mod evidence;

// The SDK's `source!` macro expands against these; no adapter names them.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use bindings::export;
pub use capability::{AdapterMetadata, Source, SourceContent, SourceInput};
pub use evidence::{Authority, Backing, CLAIM_ID_REGEX, Claim, ClaimKind, Evidence};
