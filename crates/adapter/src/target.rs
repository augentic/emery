//! Defines target-adapter inputs, outputs, and the engine-facing capability.
//!
//! The engine puts a [`Task`] to a target through [`Target`]: an init over
//! a tree a build lands in, a [`Slice`] to build, or a verify over the
//! integrated tree, each answered by its [`Outcome`]. A build answers a
//! [`Report`], which [`Report::findings`] holds to the slice, and a verify a
//! [`Verdict`], which [`Verdict::findings`] holds to itself. The engine
//! merges every slice under the [`MergeRule`]s the adapter's
//! [`TargetMetadata`] declares, and holds the commit a build starts from to
//! the [`Base`] it declares.
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
pub use capability::{
    Base, MergeRule, MergeStrategy, Outcome, Slice, Target, TargetMetadata, Task,
};
pub use report::Report;
pub use verdict::Verdict;
