//! Defines target-adapter inputs, outputs, and the engine-facing capability.
//!
//! The engine invokes a target through [`Target`], running a [`Task`] and
//! validating the returned [`Outcome`] with [`Report::findings`] or
//! [`Verdict::findings`].
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
    DesignType, MergeRule, MergeStrategy, Outcome, Requirement, Route, Scenario, Slice, Target,
    TargetMetadata, Task,
};
pub use report::Report;
pub use verdict::Verdict;
