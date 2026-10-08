//! Defines target-adapter inputs, outputs, and the engine-facing capability.
//!
//! A [`Slice`] is what the engine hands a target to build. An adapter returns
//! a [`Report`], which [`Report::findings`] holds to the slice, and a
//! [`Verdict`] over the integrated tree, which [`Verdict::findings`] holds to
//! itself. The engine invokes adapters through [`Target`] and merges every
//! slice under the [`MergeRule`]s its [`TargetMetadata`] declares.
//!
//! On WebAssembly targets, `export` contains the guest interface implemented
//! by an adapter.

#[cfg(target_arch = "wasm32")]
mod bindings;
mod capability;
mod report;
mod verdict;

#[cfg(target_arch = "wasm32")]
pub use bindings::export;
pub use capability::{MergeRule, MergeStrategy, Slice, Target, TargetMetadata};
pub use report::Report;
pub use verdict::Verdict;
