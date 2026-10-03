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

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use omnia_cursor::Client as Cursor;
#[cfg(not(target_arch = "wasm32"))]
use omnia_filesystem::{Client as Filesystem, ConnectOptions};
#[cfg(not(target_arch = "wasm32"))]
use omnia_wasi_blobstore::WasiBlobstore;
#[cfg(not(target_arch = "wasm32"))]
use omnia_wasi_keyvalue::WasiKeyValue;
#[cfg(not(target_arch = "wasm32"))]
use omnia_wasi_model::WasiModel;
#[cfg(not(target_arch = "wasm32"))]
use omnia_wasi_otel::{OtelDefault, WasiOtel};

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
omnia::runtime!({
    mode: command,
    mounts: [
        { name: ".", path: ".", writable: true },
        { name: "adapters", path: adapters_root() },
    ],
    guests: [{ path: env!("EMERY_GUEST") }],
    hosts: {
        WasiOtel: OtelDefault,
        WasiModel: Cursor,
        WasiKeyValue: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
        WasiBlobstore: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
    },
});

// The adapters root is created on first use so the mount opens. It must lie
// apart from the project: with no home directory there is no such root, so
// the runtime stops before any guest runs rather than mount one inside the
// tree a run can write.
#[cfg(not(target_arch = "wasm32"))]
fn adapters_root() -> PathBuf {
    let home = std::env::home_dir()
        .expect("the adapters root is `~/.emery/adapters`, which needs a home directory: set HOME");
    let root = home.join(".emery").join("adapters");
    std::fs::create_dir_all(&root)
        .unwrap_or_else(|err| panic!("creating the adapters root {}: {err}", root.display()));
    root
}
