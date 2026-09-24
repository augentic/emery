//! Builds the engine component and names it for the runtime to embed.
//!
//! The build script compiles the engine guest for `wasm32-wasip2` and emits
//! the artifact's path as `EMERY_GUEST`, which the `runtime!` invocation in
//! `src/main.rs` reads with `env!` and embeds with `include_bytes!`. The
//! artifact is the raw component (`emery.wasm`), JIT-compiled at startup: the
//! runtime plans each run's deployment itself and mounts it through the
//! macro's `run`, whose safe build admits raw components alone.
//!
//! The resulting `emery` binary is self-contained and does not load its engine
//! component from disk at run time.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // prevent recursion: skip this script if the target is wasm32
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env"));
    let release = std::env::var("PROFILE").as_deref() == Ok("release");

    let guest = build_engine(&manifest_dir, &out_dir, release);
    println!("cargo:rustc-env=EMERY_GUEST={}", guest.display());
}

// Build the engine wasm32 guest.
fn build_engine(manifest_dir: &Path, out_dir: &Path, release: bool) -> PathBuf {
    for tracked in ["src", "crates", "wit", "Cargo.toml", "Cargo.lock"] {
        println!("cargo::rerun-if-changed={}", manifest_dir.join(tracked).display());
    }

    // reuse parent's Cargo build
    let cargo = std::env::var_os("CARGO").expect("cargo env");
    let target_dir = nested_dir(out_dir);

    let mut child = Command::new(cargo);
    child.current_dir(manifest_dir).args([
        "build",
        "--lib",
        "--locked",
        "--target",
        "wasm32-wasip2",
    ]);
    if release {
        child.arg("--release");
    }

    // engine build environment
    child = sanitize(child);
    child.env("CARGO_TARGET_DIR", &target_dir);
    child.env("CARGO_PROFILE_DEV_DEBUG", "0");

    let status = child.status().unwrap_or_else(|err| panic!("failed to spawn engine build: {err}"));
    assert!(status.success(), "engine could not be built: {status}");

    target_dir
        .join("wasm32-wasip2")
        .join(if release { "release" } else { "debug" })
        .join("emery.wasm")
}

// Child build's target directory.
fn nested_dir(out_dir: &Path) -> PathBuf {
    // the nearest `build` ancestor is Cargo's
    out_dir
        .ancestors()
        .find(|dir| dir.file_name().is_some_and(|name| name == "build"))
        .and_then(|build| build.parent()?.parent())
        .map_or_else(|| out_dir.join("engine"), |target| target.join("engine"))
}

// Strip host's env vars from the child's environment.
fn sanitize(mut child: Command) -> Command {
    for (key, _) in std::env::vars_os() {
        let Some(key) = key.to_str() else { continue };

        let cargo = key.starts_with("CARGO_") && !matches!(key, "CARGO_HOME" | "CARGO_NET_OFFLINE");
        let rustc = matches!(
            key,
            "RUSTFLAGS" | "RUSTDOCFLAGS" | "RUSTC" | "RUSTC_WRAPPER" | "RUSTC_WORKSPACE_WRAPPER"
        );

        if cargo || rustc {
            child.env_remove(key);
        }
    }
    child
}
