//! Defines the shipped `emery` runtime.
//!
//! The runtime embeds the engine guest under capability policies fixed at
//! compile time: the invocation directory is the one mount, revision state
//! lives in `.omnia/storage`, and Cursor answers the model. The deployment's
//! guest list is the loader's allow-list, so what varies per run is declared
//! before the engine runs: the runtime reads the invocation the way the engine
//! will and declares every local component and package it names as an
//! on-demand guest — under the name the engine loads it by, pinned to its
//! `[[source]] digest`, its package routed by the project's `[registries]`
//! table — and the loader admits exactly what the operator named. A bare name
//! is a guest the runtime itself declares; this one declares none.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::process::ExitCode;

use emery_cli::Plan;
use emery_engine::{AdapterRef, Registries};
use omnia::{DeploymentBuilder, GuestEntry, LevelFilter, Manifest, Mode, SourceSpec};
use omnia_sdk::plugins::Digest;
use serde::Serialize;

mod host {
    use omnia_cursor::Client as Cursor;
    use omnia_filesystem::{Client as Filesystem, ConnectOptions};
    use omnia_wasi_blobstore::WasiBlobstore;
    use omnia_wasi_keyvalue::WasiKeyValue;
    use omnia_wasi_model::WasiModel;
    use omnia_wasi_otel::{OtelDefault, WasiOtel};

    omnia::runtime!({
        mode: command,
        guests: [{ path: env!("EMERY_GUEST") }],
        mounts: [{ name: ".", path: "." }],
        hosts: {
            WasiOtel: OtelDefault,
            WasiModel: Cursor,
            WasiKeyValue: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
            WasiBlobstore: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
        },
    });
}

fn main() -> ExitCode {
    // argv belongs to the engine; the runtime reads it for the run's adapters and level
    let args: Result<Vec<String>, OsString> =
        std::env::args_os().skip(1).map(OsString::into_string).collect();
    let args = match args {
        Ok(args) => args,
        Err(arg) => {
            eprintln!("argument `{}` is not valid UTF-8", arg.display());
            return ExitCode::FAILURE;
        }
    };
    let plan = emery_cli::plan(std::env::args_os());

    // declare the run's adapters on the compiled-in deployment
    let manifest = match host::manifest().into_manifest() {
        Ok(manifest) => declare(manifest, &plan),
        Err(error) => {
            eprintln!("{error:#}");
            return ExitCode::FAILURE;
        }
    };

    // run the engine over it
    let builder =
        DeploymentBuilder::new().manifest(manifest).args(args).program_name(env!("CARGO_PKG_NAME"));
    let builder = match level(plan.verbose, plan.quiet) {
        Some(level) => builder.level(level),
        None => builder,
    };
    match host::run(builder) {
        Ok(status) => status.into(),
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

// The run's deployment: the compiled-in manifest with every component and
// package the invocation names declared as an on-demand guest, and the
// project's `[registries]` as the routing of the packages among them.
fn declare(manifest: Manifest, plan: &Plan) -> Manifest {
    let declared: BTreeSet<String> =
        manifest.guests.iter().map(|guest| guest.name.clone()).collect();
    plan.adapters
        .iter()
        .filter(|(adapter, _)| !declared.contains(&adapter.guest()))
        .filter_map(|(adapter, pin)| entry(adapter, pin.as_ref()))
        .fold(manifest, Manifest::guest)
        .registries(wasm_pkg(&plan.registries))
}

// The on-demand guest one adapter reference declares: a component from its
// path, a package by its exact reference, each under the pin its source
// carries. A bare name is a guest the runtime declares at boot or the loader
// refuses, so it declares nothing here.
fn entry(adapter: &AdapterRef, pin: Option<&Digest>) -> Option<GuestEntry> {
    let source = match adapter {
        AdapterRef::File(path) => SourceSpec::from(path.as_path()),
        AdapterRef::Package { .. } => SourceSpec::package(adapter.to_string()),
        AdapterRef::Static(_) => return None,
    };
    // The invocation names the path, the pin, and the registry routing alike,
    // so the pin proves nothing about who built the bytes: the only
    // pre-compiled artifact is the engine compiled into this binary, and an
    // adapter is always raw wasm.
    let entry = GuestEntry::new(adapter.guest(), source).on_demand().wasm_only();
    // A decoded `sha256:` digest always parses, and the engine holds the
    // attested digest to the pin regardless.
    Some(match pin.and_then(|pin| pin.as_str().parse().ok()) {
        Some(digest) => entry.digest(digest),
        None => entry,
    })
}

// The wasm-pkg configuration routing each package namespace to the registry
// the project's `[registries]` table names, `emery` to augentic.io unless a
// line re-routes it.
fn wasm_pkg(registries: &Registries) -> String {
    #[derive(Serialize)]
    struct Config<'a> {
        namespace_registries: BTreeMap<&'a str, &'a str>,
    }
    // A table of strings always serialises; an empty configuration routes no
    // package, which the loader refuses typed.
    toml::to_string(&Config {
        namespace_registries: registries.routes(),
    })
    .unwrap_or_default()
}

// The level the verbosity flags select from command mode's `info`: one rung
// per `-v` up and per `-q` down omnia's scale, clamped at its ends. No flag
// selects nothing, so the process `RUST_LOG` stands; both select nothing
// either, since the engine's grammar refuses the pair.
fn level(verbose: u8, quiet: u8) -> Option<LevelFilter> {
    const LADDER: [LevelFilter; 6] = [
        LevelFilter::OFF,
        LevelFilter::ERROR,
        LevelFilter::WARN,
        LevelFilter::INFO,
        LevelFilter::DEBUG,
        LevelFilter::TRACE,
    ];
    if (verbose == 0) == (quiet == 0) {
        return None;
    }
    let start = LADDER.iter().position(|&rung| rung == Mode::Command.level())?;
    let rung = (start + usize::from(verbose)).saturating_sub(usize::from(quiet));
    Some(LADDER[rung.min(LADDER.len() - 1)])
}

// Unit tests by placement: `main.rs` is the shipped runtime, which no root
// suite drives in-process, and what it declares is read off the manifest.
#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(reference: &str) -> AdapterRef {
        reference.parse().expect("a well-formed adapter reference")
    }

    // Every adapter an invocation names — a local component, a package —
    // is an on-demand guest that admits raw wasm alone, pinned when the
    // source pins it; a bare name declares nothing.
    #[test]
    fn declared_adapters_are_wasm_only() {
        let pin: Digest = format!("sha256:{}", "ab".repeat(32)).parse().expect("a digest");
        let plan = Plan {
            adapters: vec![
                (adapter("adapters/custom.wasm"), Some(pin.clone())),
                (adapter("emery:intent@1.2.3"), None),
                (adapter("engine"), None),
            ],
            ..Plan::default()
        };

        let manifest = declare(Manifest::new(), &plan);
        let names: Vec<&str> = manifest.guests.iter().map(|guest| guest.name.as_str()).collect();
        assert_eq!(names, ["custom", "emery:intent@1.2.3"]);
        for guest in &manifest.guests {
            assert!(guest.on_demand, "`{}` loads on demand", guest.name);
            assert!(guest.wasm_only, "`{}` admits raw wasm alone", guest.name);
        }
        assert_eq!(
            manifest.guests[0].digest.map(|digest| digest.to_string()),
            Some(pin.to_string())
        );
        assert_eq!(manifest.guests[1].digest, None);
    }
}
