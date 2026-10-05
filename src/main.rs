//! Defines the shipped `emery` runtime.
//!
//! The runtime embeds the engine guest under capability policies fixed at
//! compile time. The invocation directory is mounted as `.`, writable: the
//! tree a run reads its sources from and builds into. The adapters root,
//! `~/.emery/adapters`, is mounted read-only as `adapters`, apart from the
//! project, and is the one root a local adapter loads from: a component is
//! never loaded from a directory a run can write. A package adapter fetches
//! from the registry the project's `emery.toml` routes its namespace to.
//! Every external effect stays within those grants.
//!
//! The binary is host-only; on `wasm32` it compiles to an empty `main` so the
//! workspace-wide wasm32 clippy pass can include it.

cfg_select! {
    not(target_arch = "wasm32") => {
        use std::path::PathBuf;

        use omnia_cursor::Client as Cursor;
        use omnia_filesystem::{Client as Filesystem, ConnectOptions};
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            mounts: [
                { name: ".", path: ".", writable: true },
                // adapters must be loaded from outside emery's writable mount (W^X rule)
                { name: "adapters", path: adapters_dir() },
            ],
            guests: [{ path: env!("EMERY_GUEST") }],
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: Cursor,
                WasiKeyValue: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
                WasiBlobstore: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
            },
        });

        // The adapters directory is created on first use so the mount opens.
        // It must be outside emery's writable mount so a malicious guest
        // cannot write to it (following the W^X rule).
        #[allow(
            clippy::print_stderr,
            reason = "runs before logging is initialised",
        )]
        fn adapters_dir() -> PathBuf {
            let Some(home) = std::env::home_dir() else {
                eprintln!(
                    "The adapters directory needs $HOME set.",
                );
                std::process::exit(1)
            };
            let dir = home.join(".emery").join("adapters");
            if let Err(error) = std::fs::create_dir_all(&dir) {
                eprintln!(
                    "There was an issue creating `~/.emery/adapters`, which emery uses to load
                    local adapters: {error}. You will need to create it before continuing."
                );
                std::process::exit(1)
            }
            dir
        }
    }
    _ => {
        fn main() {}
    }
}
