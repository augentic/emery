//! Implements the WebAssembly engine guest used by the shipped runtime.
//!
//! The guest passes process arguments to the command interface and supplies
//! host-provided model, storage, source, and plugin capabilities. All external
//! effects therefore remain subject to the runtime's grants.

#![cfg(target_arch = "wasm32")]

use emery_cli::Verbosity;
use omnia_sdk::api::command::Response;
use omnia_sdk::{BlobStore, Model, Plugins, StateStore};
use tracing::Level;
use wasip3::cli::environment;

// The bare provider: every capability keeps its WASI-backed default body, so
// each impl is empty and the runtime's grants decide what the engine can do.
struct Provider;

impl Model for Provider {}
impl StateStore for Provider {}
impl BlobStore for Provider {}
impl Plugins for Provider {}
impl emery_adapter::source::Source for Provider {}

omnia_sdk::command!(dispatch);

// The root span of a run. `command!` owns the telemetry lifecycle around it,
// so the export flushes before any exit, a non-zero one included.
#[omnia_wasi_otel::instrument(name = "cli_guest_run", level = Level::DEBUG)]
async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments(), trace).await
}

// Reloads the guest tracing filter to the level the invocation selects.
fn trace(verbosity: Verbosity) {
    let preset = verbosity.directives();
    let ambient = match verbosity {
        Verbosity::Quiet => None,
        Verbosity::Progress | Verbosity::Debug => std::env::var("RUST_LOG").ok(),
    };

    let refined = ambient
        .and_then(|ambient| omnia_wasi_otel::set_filter(&format!("{preset},{ambient}")).ok());
    if refined.is_none() {
        let _ = omnia_wasi_otel::set_filter(preset);
    }
}
