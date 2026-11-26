//! Engine guest path for deployment-based CLI suites.

use std::fs;
use std::path::{Path, PathBuf};

use omnia::RuntimeOptions;
use sha2::{Digest as _, Sha256};

const GUEST: &str = env!("EMERY_GUEST");

/// Resolves the engine guest, compiling raw wasm once per artifact under `CARGO_TARGET_TMPDIR`.
#[must_use]
pub fn engine_guest() -> PathBuf {
    let bytes = fs::read(GUEST).expect("reading the engine guest");
    if !bytes.starts_with(b"\0asm") {
        return PathBuf::from(GUEST);
    }
    let key = hex::encode(&Sha256::digest(&bytes)[..8]);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let compiled = dir.join(format!("emery-{key}.cwasm"));
    if compiled.exists() {
        return compiled;
    }

    fs::create_dir_all(dir).expect("creating the target tmpdir");
    let lock = fs::File::create(dir.join(format!("emery-{key}.lock"))).expect("creating the lock");
    lock.lock().expect("locking the artifact");
    if !compiled.exists() {
        let options = RuntimeOptions::load_env().expect("the runtime options").compile_options();
        let staged = dir.join(format!("emery-{key}.cwasm.tmp"));
        omnia::compile::compile(Path::new(GUEST), Some(staged.clone()), None, &options)
            .expect("compiling the engine guest");
        fs::rename(&staged, &compiled).expect("placing the artifact");
    }
    compiled
}
