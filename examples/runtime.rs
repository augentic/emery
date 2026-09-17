//! Runs a development runtime for the end-to-end example.
//!
//! This runtime matches the shipped deployment but loads adapters only from
//! local paths. Build the example adapter first, then run `specify` with
//! `examples/emery.toml`:
//!
//! ```text
//! cargo build --example adapter --target wasm32-wasip2 --release
//! cargo run --example runtime -- specify --config examples/emery.toml
//! ```

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        fn main() {}
    } else {
        use omnia_cursor::Client as Cursor;
        use omnia_filesystem::{Client as Filesystem, ConnectOptions};
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            guests: [{
                id: "emery",
                source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
            }],
            mounts: [{ name: ".", path: "." }],
            link: {
                interfaces: ["emery:adapter/source@0.1.0"],
            },
            plugin: {
                locations: [
                    { name: ".", path: "." },
                ],
            },
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: Cursor,
                WasiKeyValue: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
                WasiBlobstore: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
            }
        });
    }
}
