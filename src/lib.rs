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

struct CliGuest;

wasip3::cli::command::export!(CliGuest);

impl Guest for CliGuest {
    #[omnia_wasi_otel::instrument(name = "cli_guest_run")]
    async fn run() -> Result<(), ()> {
        command::execute_wasi(dispatch()).await;
        Ok(())
    }
}

// The root span of a run. `execute_wasi` owns the telemetry lifecycle around it,
// so the export flushes before any exit, a non-zero one included.

async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments(), trace).await
}

// Reloads the guest tracing filter to the level the invocation selects.
fn trace(verbosity: Verbosity) {
    let filter = verbosity.into_filter();
    if verbosity != Verbosity::Quiet
        && let Ok(rust_log) = std::env::var("RUST_LOG")
        && omnia_wasi_otel::set_filter(&format!("{filter},{rust_log}")).is_ok()
    {
        return;
    }
    let _ = omnia_wasi_otel::set_filter(filter);
}
