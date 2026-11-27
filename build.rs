//! Builds the engine guest and mock adapters for `wasm32-wasip3`.
//!
//! Emits `EMERY_GUEST` (precompiled in release builds via `emery-precompile`),
//! `EMERY_MOCK_SOURCE`, and `EMERY_MOCK_TARGET`. Check-only runs emit empty
//! stand-ins instead of spawning nested builds.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // skip the nested wasm32 build this script itself spawns
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    let build = Build::from_env();

    for tracked in ["src", "crates", "examples", "wit", "Cargo.toml", "Cargo.lock"] {
        println!("cargo::rerun-if-changed={}", build.manifest_dir.join(tracked).display());
    }
    for var in ["EMERY_SKIP_GUEST", "RUSTC_WORKSPACE_WRAPPER", "RA_RUSTC_WRAPPER"] {
        println!("cargo::rerun-if-env-changed={var}");
    }

    // a check-only invocation embeds an empty stand-in for every guest
    if check_only() {
        let skipped = build.out_dir.join("emery-guest-skipped.wasm");
        std::fs::write(&skipped, [])
            .unwrap_or_else(|err| panic!("failed to write {}: {err}", skipped.display()));
        for name in ["EMERY_GUEST", "EMERY_MOCK_SOURCE", "EMERY_MOCK_TARGET"] {
            println!("cargo::rustc-env={name}={}", skipped.display());
        }
        return;
    }

    let guest = build.engine_guest();
    println!("cargo::rustc-env=EMERY_GUEST={}", guest.display());

    let examples = build.mock_adapters();
    println!("cargo::rustc-env=EMERY_MOCK_SOURCE={}", examples.join("source.wasm").display());
    println!("cargo::rustc-env=EMERY_MOCK_TARGET={}", examples.join("target.wasm").display());
}

// A check never runs what it compiles, so the guests it embeds may be empty:
// under clippy, which cargo names through `RUSTC_WORKSPACE_WRAPPER`, under
// rust-analyzer's check, which sets `RA_RUSTC_WRAPPER`, or wherever
// `EMERY_SKIP_GUEST` is set.
fn check_only() -> bool {
    let skipped = std::env::var_os("EMERY_SKIP_GUEST").is_some_and(|value| !value.is_empty());
    let clippy = std::env::var_os("RUSTC_WORKSPACE_WRAPPER").is_some_and(|wrapper| {
        Path::new(&wrapper).file_stem().is_some_and(|stem| stem == "clippy-driver")
    });
    let analyzer = std::env::var_os("RA_RUSTC_WRAPPER").is_some();
    skipped || clippy || analyzer
}

struct Build {
    manifest_dir: PathBuf,
    out_dir: PathBuf,
    target_dir: PathBuf,
    release: bool,
}

#[derive(Clone, Copy)]
enum Target {
    Wasm32,
    Host,
}

impl Build {
    fn from_env() -> Self {
        let manifest_dir =
            PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
        let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env"));
        let target_dir = nested_dir(&out_dir);
        let release = std::env::var("PROFILE").as_deref() == Ok("release");
        Self {
            manifest_dir,
            out_dir,
            target_dir,
            release,
        }
    }

    fn wasm_out_dir(&self) -> PathBuf {
        self.target_dir.join("wasm32-wasip3").join(if self.release { "release" } else { "debug" })
    }

    // Builds the engine lib as a `cdylib`, which the manifest's `rlib` is not,
    // and precompiles it in release builds.
    fn engine_guest(&self) -> PathBuf {
        self.cargo(Target::Wasm32, &["rustc", "--lib", "--crate-type", "cdylib"]);
        let wasm = self.wasm_out_dir().join("emery.wasm");
        if self.release { self.precompile(&wasm) } else { wasm }
    }

    fn mock_adapters(&self) -> PathBuf {
        self.cargo(Target::Wasm32, &["build", "--example", "source", "--example", "target"]);
        self.wasm_out_dir().join("examples")
    }

    // Compiles the engine guest for the binary's own target through
    // `emery-precompile`, so the compiler is built once in the dev profile
    // (its Cranelift at the `[profile.dev.package]` opt-level) and never under
    // the release profile's LTO.
    fn precompile(&self, wasm: &Path) -> PathBuf {
        let target = std::env::var("TARGET").expect("cargo env");
        let compiled = self.out_dir.join("emery.cwasm");

        self.cargo(Target::Host, &["build", "-p", "emery-precompile"]);
        let tool = self.target_dir.join("debug").join("emery-precompile");
        let status = Command::new(&tool)
            .arg(wasm)
            .arg(&compiled)
            .args(["--target", &target])
            .status()
            .unwrap_or_else(|err| panic!("failed to spawn {}: {err}", tool.display()));
        assert!(
            status.success(),
            "precompiling {} for `{target}` failed: {status}",
            wasm.display()
        );

        compiled
    }

    fn cargo(&self, target: Target, args: &[&str]) {
        let cargo = std::env::var_os("CARGO").expect("cargo env");

        let mut child = Command::new(cargo);
        child.current_dir(&self.manifest_dir).args(args).arg("--locked");
        if matches!(target, Target::Wasm32) {
            child.args(["--target", "wasm32-wasip3"]);
            if self.release {
                child.arg("--release");
            }
        }

        // engine build environment
        sanitize(&mut child);
        child.env("CARGO_TARGET_DIR", &self.target_dir);
        child.env("CARGO_PROFILE_DEV_DEBUG", "0");
        child.env("CARGO_PROFILE_DEV_OPT_LEVEL", "1");

        let status =
            child.status().unwrap_or_else(|err| panic!("failed to spawn nested build: {err}"));
        assert!(status.success(), "nested build `cargo {}` failed: {status}", args.join(" "));
    }
}

fn nested_dir(out_dir: &Path) -> PathBuf {
    // the nearest `build` ancestor is Cargo's
    out_dir
        .ancestors()
        .find(|dir| dir.file_name().is_some_and(|name| name == "build"))
        .and_then(|build| build.parent()?.parent())
        .map_or_else(|| out_dir.join("engine"), |target| target.join("engine"))
}

fn sanitize(child: &mut Command) {
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
}
