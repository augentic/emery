//! Host harness and tree fixtures for the SDK suites.

#![cfg(not(target_arch = "wasm32"))]
#![allow(dead_code, reason = "each integration test binary uses a different subset")]

pub mod by_file;
pub mod harness;
pub mod stub;

#[allow(unused_imports, reason = "tree re-exports; other suites link this module alone")]
pub use stub::*;
