//! Defines source-adapter inputs, outputs, and the engine-facing capability.
//!
//! [`SourceInput`] identifies the source to read. An adapter returns
//! [`Evidence`], whose claims are validated by [`Evidence::findings`]. The
//! engine invokes adapters through [`Source`].
//!
//! On WebAssembly targets, `export` contains the guest interface implemented
//! by an adapter.

#[cfg(target_arch = "wasm32")]
mod bindings;
mod capability;
mod evidence;

#[cfg(target_arch = "wasm32")]
pub use bindings::export;
pub use capability::{AdapterMetadata, Source, SourceContent, SourceInput};
pub use evidence::{Backing, CLAIM_ID_REGEX, Claim, ClaimKind, Evidence, SourceKind};
