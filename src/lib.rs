//! Implements the WebAssembly engine guest the shipped runtime embeds.
//!
//! Exports `emery_cli::router` as the `wasi:cli/run` entrypoint.

#![cfg(target_arch = "wasm32")]

omnia_sdk::command::export!(emery_cli::router);
