//! Defines the shipped `emery` runtime.
//!
//! The runtime embeds the engine guest under capability policies fixed at
//! compile time. The invocation directory is the one mount, read-only, so it
//! is also the root a local adapter loads from. A package adapter fetches from
//! the registry the project's `emery.toml` routes its namespace to. Every
//! external effect stays within those grants.
//!
//! The binary is host-only; on `wasm32` it compiles to an empty `main` so the
//! workspace-wide wasm32 clippy pass can include it.

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
    mounts: [{ name: ".", path: "." }],
    guests: [{ path: env!("EMERY_GUEST") }],
    hosts: {
        WasiOtel: OtelDefault,
        WasiModel: Cursor,
        WasiKeyValue: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
        WasiBlobstore: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
    },
});
