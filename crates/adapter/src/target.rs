//! Defines target-adapter inputs, outputs, and the engine-facing capability.
//!
//! A [`Slice`] is what the engine hands a target to build. An adapter returns
//! a [`Report`], which [`Report::findings`] holds to the slice. The engine
//! invokes adapters through [`Target`].
//!
//! On WebAssembly targets, `export` contains the guest interface implemented
//! by an adapter.

#[cfg(target_arch = "wasm32")]
mod bindings;
mod capability;
mod report;

#[cfg(target_arch = "wasm32")]
pub use bindings::export;
pub use capability::{Slice, Target, TargetMetadata};
pub use report::Report;
