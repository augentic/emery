//! Defines the shipped `emery` runtime.
//!
//! The runtime embeds the engine guest under capability policies fixed at
//! compile time: the invocation directory is the one mount, so it is also the
//! root a local adapter loads from, and a package adapter fetches from the
//! registry the project's `emery.toml` routes its namespace to. Every
//! external effect remains limited to those grants.

use omnia_cursor::Client as Cursor;
use omnia_filesystem::{Client as Filesystem, ConnectOptions};
use omnia_wasi_blobstore::WasiBlobstore;
use omnia_wasi_keyvalue::WasiKeyValue;
use omnia_wasi_model::WasiModel;
use omnia_wasi_otel::{OtelDefault, WasiOtel};

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
