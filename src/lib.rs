//! Implements the WebAssembly engine guest the shipped runtime embeds.
//!
//! The guest passes process arguments to the command interface and supplies
//! host-provided model, storage, version-control, source, target, and plugin
//! capabilities, so every external effect stays subject to the runtime's
//! grants. The one capability it answers itself, the engine's `Files`, reads
//! a working copy's files through the `.` preopen.

#![cfg(target_arch = "wasm32")]

use std::future::Future;
use std::io;
use std::path::Path;

use emery_engine::Files;
use omnia_sdk::api::command::{self, Response};
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, Vcs, server_error};
use wasip3::cli::environment;
use wasip3::exports::cli::run::Guest;

// Empty impls retain the WASI defaults selected by the runtime's grants.
struct Provider;

impl Model for Provider {}
impl StateStore for Provider {}
impl BlobStore for Provider {}
impl Plugins for Provider {}
impl Vcs for Provider {}
impl emery_adapter::source::Source for Provider {}
impl emery_adapter::target::Target for Provider {}

impl Files for Provider {
    fn read(
        &self, at: &str, path: &str,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Error>> + Send {
        let outcome = match std::fs::read(Path::new(at).join(path)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::IsADirectory
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(server_error!("reading `{path}` under `{at}`: {error}")),
        };
        async move { outcome }
    }
}

struct CliGuest;

wasip3::cli::command::export!(CliGuest);

impl Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        command::execute_wasi(dispatch()).await;
        Ok(())
    }
}

async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments()).await
}
