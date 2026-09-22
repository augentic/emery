//! Defines the shipped `emery` runtime.
//!
//! The runtime embeds the engine guest under capability and registry policies
//! fixed at compile time. Every external effect remains limited to those
//! grants.

use omnia_cursor::Client as Cursor;
use omnia_filesystem::{Client as Filesystem, ConnectOptions};
use omnia_wasi_blobstore::WasiBlobstore;
use omnia_wasi_keyvalue::WasiKeyValue;
use omnia_wasi_model::WasiModel;
use omnia_wasi_otel::{OtelDefault, WasiOtel};

omnia::runtime!({
    mode: command,
    mounts: [{ name: ".", path: "." }],
    guests: [{
        id: "emery",
        source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
    }],
    // Target-scoped: a bare level would reach the engine guest too and defeat `-q`.
    env: { RUST_LOG: "emery_sdk=info" },
    link: {
        interfaces: ["emery:adapter/source@0.1.0"],
    },
    plugin: {
        locations: [
            { name: ".", path: "." },
            { registry: "omnia.host", config: include_str!("wasm-pkg.toml") },
        ],
    },
    hosts: {
        WasiOtel: OtelDefault,
        WasiModel: Cursor,
        WasiKeyValue: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
        WasiBlobstore: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
    }
});
