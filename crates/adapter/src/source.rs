//! The source axis of the contract.
//!
//! A source adapter is given a [`SourceInput`] — a key and a workspace or
//! inline value — and returns [`Evidence`], a document of typed claims. This
//! module carries those records, the claim gate every document must pass
//! ([`Evidence::findings`]), and the [`Source`] capability the engine calls
//! source adapters through.
//!
//! On `wasm32` the module also carries the WIT bindings both sides ride. The
//! export side is reached only through the SDK's `source!` macro.

#[cfg(target_arch = "wasm32")]
mod bindings;
mod capability;
mod evidence;

// The SDK's `source!` macro expands against these; no adapter names them.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use bindings::export;
pub use capability::{AdapterMetadata, Source, SourceContent, SourceInput};
pub use evidence::{Backing, CLAIM_ID_REGEX, Claim, ClaimKind, Evidence, SourceKind};
