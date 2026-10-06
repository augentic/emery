//! Implements the `emery` engine guest and the runtime that ships it.
//!
//! On `wasm32` the crate is the engine guest: it passes process arguments to
//! the command interface and supplies host-provided model, storage, source,
//! target, and plugin capabilities, so every external effect stays subject
//! to the runtime's grants. Natively it is the [`runtime`] the `emery` binary
//! runs, reachable by a suite that drives the same wiring.

#[cfg(not(target_arch = "wasm32"))]
pub mod runtime;

#[cfg(target_arch = "wasm32")]
mod guest {
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
    impl emery_adapter::target::Target for Provider {}

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
}
