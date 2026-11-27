//! Defines the shipped `emery` runtime binary.
//!
//! Embeds `EMERY_GUEST` with the current directory mounted writable as `.`,
//! adapters loaded from `~/.emery/adapters` or fetched from `augentic.io`,
//! state under `.emery/storage`, version control via host `git`, and model
//! workers confined to `.emery/vcs/worktrees` and `.emery/vcs/integration`.

cfg_select! {
    not(target_arch = "wasm32") => {
        use omnia_filesystem::{Client as Filesystem, ConnectOptions as FilesystemOptions};
        use omnia_git::Client as Git;
        use omnia_wasi_blobstore::WasiBlobStore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        use omnia_wasi_vcs::WasiVcs;

        const STORAGE_ROOT: &str = ".emery/storage";

        cfg_select! {
            feature = "genai" => {
                use omnia_genai::{Client as Model, ConnectOptions as ModelOptions};
            }
            not(feature = "genai") => {
                use omnia_cursor::{Client as Model, ConnectOptions as ModelOptions};
            }
        }

        fn storage() -> FilesystemOptions {
            FilesystemOptions {
                root: STORAGE_ROOT.into(),
                buckets: vec!["default".into()],
            }
        }

        fn model() -> omnia::anyhow::Result<ModelOptions> {
            Ok(ModelOptions {
                shell_roots: vec![".emery/vcs/worktrees".into(), ".emery/vcs/integration".into()],
                ..ModelOptions::load_env()?
            })
        }

        omnia::runtime!({
            mode: command,
            guests: [{
                path: env!("EMERY_GUEST"),
                command: true,
                grants: {
                    env: ["EMERY_*", "CURSOR_*", "GENAI_*", "OPENAI_API_KEY", "GIT_BINARY"],
                    mounts: ["."],
                    interfaces: [
                        "wasi:cli", "wasi:clocks", "wasi:random", "wasi:filesystem",
                        "wasi:io", "omnia:otel", "omnia:keyvalue", "omnia:blobstore",
                        "omnia:vcs", "omnia:plugins", "omnia:model", "emery:adapter",
                    ],
                },
            }],
            plugins: {
                store: "~/.emery/adapters",
                registries: include_str!("wasm-pkg.toml"),
                grants: {
                    interfaces: [
                        "wasi:cli", "wasi:clocks", "wasi:random", "wasi:filesystem",
                        "wasi:io", "omnia:otel", "emery:adapter",
                    ],
                },
            },
            mounts: [{ name: ".", path: ".", writable: true }],
            env: {
                inherit: [
                    "EMERY_*", "CURSOR_*", "GENAI_*", "OPENAI_API_KEY", "GIT_BINARY", "RUST_LOG",
                ],
            },
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: Model(model()?),
                WasiKeyValue: Filesystem(storage()),
                WasiBlobStore: Filesystem(storage()),
                WasiVcs: Git,
            },
        });
    }
    _ => {
        // Host-only; empty so the workspace wasm32 lint pass compiles this binary.
        fn main() {}
    }
}
