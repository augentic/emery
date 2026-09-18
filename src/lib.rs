//! Implements the WebAssembly engine guest used by the shipped runtime.
//!
//! The guest passes process arguments to the command interface and supplies
//! host-provided model, storage, source, and plugin capabilities. All external
//! effects therefore remain subject to the runtime's grants.

#![cfg(target_arch = "wasm32")]

use std::io::Write;

use omnia_sdk::api::command::{self, IntoExit, Response};
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

struct CliGuest;

wasip3::cli::command::export!(CliGuest);

impl wasip3::exports::cli::run::Guest for CliGuest {
    #[omnia_wasi_otel::instrument(name = "cli_guest_run", level = Level::DEBUG)]
    async fn run() -> Result<(), ()> {
        command::execute_wasi(dispatch()).await;
        Ok(())
    }
}

async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments()).await
}
