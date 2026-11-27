//! Host harness and fixtures for the SDK component suites.

#![cfg(not(target_arch = "wasm32"))]
#![expect(dead_code, reason = "each integration test binary uses a different subset")]

pub mod build_fixtures;
pub mod by_file;
pub mod harness;
