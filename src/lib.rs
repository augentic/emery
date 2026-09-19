//! Implements the WebAssembly engine guest used by the shipped runtime.
//!
//! The guest passes process arguments to the command interface and supplies
//! host-provided model, storage, source, and plugin capabilities. All external
//! effects therefore remain subject to the runtime's grants.

#![cfg(target_arch = "wasm32")]

use emery_cli::Verbosity;
use omnia_sdk::api::command::{self, Response};
use omnia_sdk::{BlobStore, Model, Plugins, StateStore};
use wasip3::cli::environment;
use wasip3::exports::cli::run::Guest;

// The bare provider: every capability keeps its WASI-backed default body, so
// each impl is empty and the runtime's grants decide what the engine can do.
struct Provider;

impl Model for Provider {}
impl StateStore for Provider {}
impl BlobStore for Provider {}
impl Plugins for Provider {}
impl emery_adapter::source::Source for Provider {}

// What `omnia_sdk::command!(dispatch)` expands to, written out so the export
// reads in place: `wasi:cli/run` is exported on a private type whose `run`
// hands the entry to `execute_wasi`, the one owner of telemetry
// initialization, flushing, channel writes, and process exit.
struct CliGuest;

wasip3::cli::command::export!(CliGuest);

impl Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        command::execute_wasi(dispatch()).await;
        Ok(())
    }
}

async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments(), set_filter).await
}

// Reloads the guest tracing filter to the level the flags selected; the
// guest's `RUST_LOG` refines whatever the level sets.
fn set_filter(verbosity: Verbosity) {
    if let Err(error) = omnia_wasi_otel::set_filter(verbosity.directives()) {
        eprintln!("tracing filter not reloaded: {error:#}");
    }
}
