//! Defines the shipped `emery` runtime.
//!
//! The runtime embeds the engine guest under capability policies fixed at
//! compile time. The invocation directory is mounted as `.`, writable: the
//! tree a run reads its sources from and builds into. The adapters root,
//! `~/.emery/adapters`, is mounted read-only as `adapters`, apart from the
//! project, and is the one root a local adapter loads from: a component is
//! never loaded from a directory a run can write. A package adapter fetches
//! from the registry `~/.emery/wasm-pkg.toml` routes its namespace to, in
//! the schema of wkg's `config.toml`. That file is read natively at startup
//! and never mounted, so no guest reaches it. The `emery` namespace is
//! `augentic.io` unless a line there re-routes it, and a commented template
//! is written on first use. Every external effect stays within those grants.
//!
//! The binary is host-only; on `wasm32` it compiles to an empty `main` so the
//! workspace-wide wasm32 clippy pass can include it.

cfg_select! {
    not(target_arch = "wasm32") => {
        use std::io::ErrorKind;
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
                { name: "adapters", path: adapters() },
            ],
            registries: registries(),
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
        #[expect(clippy::print_stderr, reason = "runs before logging is initialised")]
        fn adapters() -> PathBuf {
            let dir = emery_home().join("adapters");
            if let Err(error) = std::fs::create_dir_all(&dir) {
                eprintln!("There was an issue creating `~/.emery/adapters`: {error}.");
                std::process::exit(1)
            }
            dir
        }

        // The routing is read natively, never mounted, so no guest reaches
        // it. The macro evaluates it before the mounts, so the directory is
        // created here too.
        #[expect(clippy::print_stderr, reason = "runs before logging is initialised")]
        fn registries() -> String {
            let home = emery_home();
            let path = home.join("wasm-pkg.toml");

            match std::fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    // no file exists; write out template
                    const TEMPLATE: &str = include_str!("wasm-pkg.toml");
                    let write_result = std::fs::create_dir_all(&home)
                        .and_then(|()| std::fs::write(&path, TEMPLATE));
                    if let Err(error) = write_result {
                        eprintln!("There was an issue writing `~/.emery/wasm-pkg.toml`: {error}.");
                        std::process::exit(1)
                    }
                    TEMPLATE.to_owned()
                }
                Err(error) => {
                    eprintln!("There was an issue reading `~/.emery/wasm-pkg.toml`: {error}.");
                    std::process::exit(1)
                }
            }
        }

        #[expect(clippy::print_stderr, reason = "runs before logging is initialised")]
        fn emery_home() -> PathBuf {
            let Some(home) = std::env::home_dir() else {
                eprintln!(
                    "emery needs $HOME set: adapters and wasm-pkg.toml live under `~/.emery`."
                );
                std::process::exit(1)
            };
            home.join(".emery")
        }
    }
    _ => {
        fn main() {}
    }
}
