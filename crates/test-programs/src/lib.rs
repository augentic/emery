//! Guest SDK drivers and, natively, the artifact table the suites run.
//!
//! On `wasm32` the crate holds shared helpers for the drivers; natively it is
//! the generated path constants and the `foreach_*!` completeness macros.

pub mod protocol;

#[cfg(target_arch = "wasm32")]
pub mod helpers;
#[cfg(target_arch = "wasm32")]
pub mod seams;

#[cfg(not(target_arch = "wasm32"))]
include!(concat!(env!("OUT_DIR"), "/gen.rs"));
