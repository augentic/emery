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

// Empty impls retain the WASI defaults selected by the runtime's grants.
struct Provider;

impl Model for Provider {}
impl StateStore for Provider {}
impl BlobStore for Provider {}
impl Plugins for Provider {}
impl emery_adapter::source::Source for Provider {}

struct CliGuest;

wasip3::cli::command::export!(CliGuest);

impl Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        command::execute_wasi(dispatch()).await;
        Ok(())
    }
}

async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments(), on_verbosity).await
}

fn on_verbosity(verbosity: Verbosity) {
    if let Err(error) = omnia_wasi_otel::set_filter(verbosity.directives()) {
        eprintln!("tracing filter not reloaded: {error:#}");
    }
}
