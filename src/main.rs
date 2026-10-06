//! Defines the shipped `emery` runtime: the engine over the operator's package store.
//!
//! The runtime embeds the engine guest under grants fixed at compile time.
//! The invocation directory is mounted as `.`, writable: the tree a run reads
//! its sources from and builds into. Every adapter is a package, read from
//! the store `~/.emery/adapters` before anything is fetched — the file
//! `namespace_name@version.wasm` there is that release on this machine — and
//! fetched through the one route compiled in here otherwise: the `emery`
//! namespace to `augentic.io`. A release the binary routes nowhere arrives by
//! `wkg get <reference> -o ~/.emery/adapters/`. The store lies apart from the
//! project, so a component is never loaded from a tree a run can write, and
//! a run from the operator's home is refused at startup.
//!
//! The binary is host-only; on `wasm32` it compiles to an empty `main` so the
//! workspace-wide wasm32 clippy pass can include it.

cfg_select! {
    not(target_arch = "wasm32") => {
        use omnia_cursor::Client as Cursor;
        use omnia_filesystem::{Client as Filesystem, ConnectOptions};
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

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
                WasiModel: Cursor,
                WasiKeyValue: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
                WasiBlobstore: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
            },
        });
    }
    _ => {
        fn main() {}
    }
}
