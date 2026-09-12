//! The source adapter contract
//!
//! The agreement between the Emery engine and every source adapter: the
//! `emery:adapter/source` WIT world, the Rust types that mirror its records,
//! the rules a claim must satisfy, and the [`Source`] capability the engine
//! calls adapters through.
//!
//! Both sides depend on this one crate so they cannot drift apart. The engine
//! consumes it directly; adapters receive it re-exported through the
//! `emery-adapter` SDK.

#[cfg(target_arch = "wasm32")]
mod bindings;

mod capability;
mod evidence;
mod grammar;

// The SDK's `source!` macro expands against these; no adapter names them.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use bindings::export;
pub use capability::{AdapterMetadata, Source, SourceContent, SourceInput};
pub use evidence::{Authority, Backing, Claim, ClaimKind, Evidence};
pub use grammar::{CLAIM_ID_REGEX, is_kebab};
