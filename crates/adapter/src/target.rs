//! Defines target-adapter inputs, outputs, and the host import surface.
//!
//! The engine guest invokes a target through [`metadata`] and [`run`],
//! running a [`Task`] and validating the returned [`Outcome`] with
//! [`Report::findings`] or [`Verdict::findings`].
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
    DesignType, MergeRule, MergeStrategy, Outcome, Requirement, Scenario, Slice, TargetMetadata,
    Task,
};
use omnia_sdk::Error;
pub use report::Report;
pub use verdict::Verdict;

/// Returns the metadata the adapter registered as `id` declares.
///
/// # Panics
///
/// Panics when called off `wasm32`, where the host import is not linked.
#[must_use]
pub fn metadata(id: &str) -> TargetMetadata {
    #[cfg(target_arch = "wasm32")]
    {
        bindings::metadata(id)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
        unreachable!("target adapters are invoked only from wasm32 guests")
    }
}

/// Runs `task` in the tree at `workspace` using the adapter registered as `id`.
///
/// An input refusal is [`omnia_sdk::ErrorKind::BadRequest`]; any other adapter failure is
/// [`omnia_sdk::ErrorKind::BadGateway`]. The outcome must answer `task`, or the caller
/// refuses it.
///
/// # Errors
///
/// Returns [`Error`] when the adapter refuses the call.
///
/// # Panics
///
/// Panics when called off `wasm32`, where the host import is not linked.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::unused_async,
        reason = "native builds keep the wasm32 signature until the host links the import"
    )
)]
pub async fn run(id: &str, workspace: &str, task: &Task) -> Result<Outcome, Error> {
    #[cfg(target_arch = "wasm32")]
    {
        bindings::run(id, workspace, task).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (id, workspace, task);
        unreachable!("target adapters are invoked only from wasm32 guests")
    }
}
