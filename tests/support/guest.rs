//! Precompiled `emery` engine guest shared across test invocations.

use std::fs;
use std::sync::OnceLock;

use omnia::SourceSpec;
use omnia::wasmtime::Engine;

const GUEST: &str = env!("EMERY_GUEST");

static PRECOMPILED: OnceLock<Vec<u8>> = OnceLock::new();

#[must_use]
pub fn engine_guest() -> SourceSpec {
    let bytes: &'static [u8] = PRECOMPILED.get_or_init(|| {
        let raw = fs::read(GUEST).expect("reading the engine guest");
        if Engine::detect_precompiled(&raw).is_some() {
            return raw;
        }
        let SourceSpec::Bytes(bytes) =
            omnia_test::host::precompiled(GUEST).expect("precompiling the engine guest")
        else {
            unreachable!("omnia_test::host::precompiled returns SourceSpec::Bytes");
        };
        bytes.into_owned()
    });
    SourceSpec::from(bytes)
}
