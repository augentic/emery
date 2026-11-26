//! Implements the WebAssembly engine guest the shipped runtime embeds.
//!
//! The host binds `emery_cli::router` as the `wasi:cli/run` export; engine
//! verbs call WASI host imports directly with no capability provider.

#![cfg(target_arch = "wasm32")]

omnia_sdk::command::export!(emery_cli::router);
