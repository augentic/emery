//! Builds the engine component and the two mock adapters, and names them for the crate.
//!
//! The engine guest is compiled for `wasm32-wasip2` and its path is emitted as
//! `EMERY_GUEST`, which the `runtime!` invocation in `src/main.rs` embeds and
//! the runtime suite deploys.
//! A debug build names the raw `emery.wasm`, which the runtime compiles at
//! startup. A release build precompiles it to `emery.cwasm` for the binary's
//! own target under the runtime's default compile settings, so the build
//! shell's environment steers neither. The runtime loads either format.
//!
//! The mock source and target under `examples/` are built beside it, raw wasm
//! in every profile, and named `EMERY_MOCK_SOURCE` and `EMERY_MOCK_TARGET` for
//! the runtime suite to stage in its scratch store and registry.
//!
//! The `emery` binary is self-contained and never loads its engine from disk.
//! Nothing of an adapter is fetched or embedded.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // skip the nested wasm32 build this script itself spawns
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env"));
    let release = std::env::var("PROFILE").as_deref() == Ok("release");

    for tracked in ["src", "crates", "examples", "wit", "Cargo.toml", "Cargo.lock"] {
        println!("cargo::rerun-if-changed={}", manifest_dir.join(tracked).display());
    }
    let target_dir = nested_dir(&out_dir);
    let built = target_dir.join("wasm32-wasip2").join(if release { "release" } else { "debug" });

    // the engine: the lib as a cdylib, which the manifest's `rlib` is not
    nested(&manifest_dir, &target_dir, release, &["rustc", "--lib", "--crate-type", "cdylib"]);
    let wasm = built.join("emery.wasm");
    let guest = if release {
        let target = std::env::var("TARGET").expect("cargo env");
        let compiled = out_dir.join("emery.cwasm");
        omnia::compile::compile(
            &wasm,
            Some(compiled.clone()),
            Some(&target),
            &omnia::CompileOptions::default(),
        )
        .expect("should compile the wasm component");
        compiled
    } else {
        wasm
    };
    println!("cargo:rustc-env=EMERY_GUEST={}", guest.display());

    // the mocks
    nested(
        &manifest_dir,
        &target_dir,
        release,
        &["build", "--example", "source", "--example", "target"],
    );
    let examples = built.join("examples");
    println!("cargo:rustc-env=EMERY_MOCK_SOURCE={}", examples.join("source.wasm").display());
    println!("cargo:rustc-env=EMERY_MOCK_TARGET={}", examples.join("target.wasm").display());
}

// Spawns one wasm32 build of this package with the parent's cargo.
fn nested(manifest_dir: &Path, target_dir: &Path, release: bool, args: &[&str]) {
    let cargo = std::env::var_os("CARGO").expect("cargo env");

    let mut child = Command::new(cargo);
    child.current_dir(manifest_dir).args(args).args(["--locked", "--target", "wasm32-wasip2"]);
    if release {
        child.arg("--release");
    }

    // engine build environment
    child = sanitize(child);
    child.env("CARGO_TARGET_DIR", target_dir);
    child.env("CARGO_PROFILE_DEV_DEBUG", "0");

    let status = child.status().unwrap_or_else(|err| panic!("failed to spawn wasm32 build: {err}"));
    assert!(status.success(), "wasm32 build `cargo {}` failed: {status}", args.join(" "));
}

fn nested_dir(out_dir: &Path) -> PathBuf {
    // the nearest `build` ancestor is Cargo's
    out_dir
        .ancestors()
        .find(|dir| dir.file_name().is_some_and(|name| name == "build"))
        .and_then(|build| build.parent()?.parent())
        .map_or_else(|| out_dir.join("engine"), |target| target.join("engine"))
}

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
