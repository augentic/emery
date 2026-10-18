//! Defines the shipped `emery` runtime binary.
//!
//! Embeds `EMERY_GUEST` with the current directory mounted writable as `.`,
//! adapters loaded from `~/.emery/adapters` or fetched from `augentic.io`,
//! state under `.emery/storage`, version control via host `git`, and model
//! workers confined to `.emery/vcs/worktrees` and `.emery/vcs/integration`.

cfg_select! {
    not(target_arch = "wasm32") => {
        use omnia::FromEnv as _;
        use omnia_cursor::{Client as Cursor, ConnectOptions as CursorOptions};
        use omnia_filesystem::{Client as Filesystem, ConnectOptions};
        use omnia_git::Client as Git;
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        use omnia_wasi_vcs::WasiVcs;

        omnia::runtime!({
            mode: command,
            guests: [{ path: env!("EMERY_GUEST") }],
            plugins: {
                store: "~/.emery/adapters",
                registries: include_str!("wasm-pkg.toml"),
            },
            mounts: [{ name: ".", path: ".", writable: true }],
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: Cursor(CursorOptions {
                    shell_roots: vec![".emery/vcs/worktrees".into(), ".emery/vcs/integration".into()],
                    ..CursorOptions::load_env()?
                }),
                WasiKeyValue: Filesystem(ConnectOptions { root: ".emery/storage".into() }),
                WasiBlobstore: Filesystem(ConnectOptions { root: ".emery/storage".into() }),
                WasiVcs: Git,
            },
        });
    }
    _ => {
        fn main() {}
    }
}
