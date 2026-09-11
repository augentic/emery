//! The engine guest
//!
//! The wasm component the shipped runtime embeds and runs. It binds
//! the host's model, storage, and plugin capabilities into one provider and
//! hands the process arguments to the command façade.
//!
//! Running the engine as a guest is what gives Emery its sandbox: the
//! project is mounted read-only, and every effect the engine has goes through
//! a capability the runtime deliberately granted.

#![cfg(target_arch = "wasm32")]

use omnia_guest::api::command::Response;
use omnia_guest::{BlobStore, Model, Plugins, StateStore};
use wasip3::cli::environment;

// The bare provider: every capability keeps its WASI-backed default body, so
// each impl is empty and the runtime's grants decide what the engine can do.
#[derive(Clone)]
struct Provider;

impl Model for Provider {}
impl StateStore for Provider {}
impl BlobStore for Provider {}
impl Plugins for Provider {}
impl emery_source::Source for Provider {}

omnia_guest::command!(dispatch);

async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments()).await
}
