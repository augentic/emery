//! Verifies Emery's command-line contract independently of engine behaviour.
//!
//! The scenarios cover available commands, grammar errors, exit-code mapping,
//! stream selection, and text and JSON output.
//!
//! Every scenario finishes before the engine touches a model or a source, so
//! the scripted provider remains idle. End-to-end product behaviour is covered
//! by `specify.rs`.

#![cfg(not(target_arch = "wasm32"))]

mod support;
#[path = "support/verbs.rs"]
mod verbs;

use omnia_sdk::api::command::USAGE_EXIT;
use serde_json::Value;
use support::{Provider, cli, cli_ok, fail};
use verbs::verbs;

struct Case {
    name: &'static str,
    argv: &'static [&'static str],
    exit: u8,
    stdout: &'static str,
    stderr: &'static str,
    json_channels: bool,
}

const CASES: [Case; 5] = [
    Case {
        name: "help",
        argv: &["emery", "--help"],
        exit: 0,
        stdout: "Usage: emery [OPTIONS] <COMMAND>",
        stderr: "",
        json_channels: false,
    },
    Case {
        name: "version",
        argv: &["emery", "--version"],
        exit: 0,
        stdout: concat!("emery ", env!("CARGO_PKG_VERSION")),
        stderr: "",
        json_channels: false,
    },
    Case {
        name: "completions",
        argv: &["emery", "completions", "zsh"],
        exit: 0,
        stdout: "_emery",
        stderr: "",
        json_channels: false,
    },
    Case {
        name: "specify source required",
        argv: &["emery", "specify"],
        exit: 1,
        stdout: "",
        stderr: "specify-source-required",
        json_channels: false,
    },
    Case {
        name: "show not generated",
        argv: &["emery", "--format", "json", "show", "spec"],
        exit: 2,
        stdout: "",
        stderr: "spec-not-generated",
        json_channels: true,
    },
];

// Deleted verbs are deleted from the grammar, not hidden. A usage error
// exits `USAGE_EXIT` (64), so exit 2 always means a `NotFound` envelope.
#[tokio::test]
async fn route_budget() {
    let provider = Provider::idle();

    for removed in [
        &["emery", "init"][..],
        &["emery", "plan", "status"][..],
        &["emery", "plan", "author"][..],
        &["emery", "plan", "refine"][..],
        &["emery", "plan", "execute"][..],
        &["emery", "plan", "archive"][..],
        &["emery", "slice", "list"][..],
        &["emery", "slice", "validate"][..],
        &["emery", "source", "survey"][..],
        &["emery", "source", "extract"][..],
        &["emery", "source", "resolve"][..],
        &["emery", "target", "resolve"][..],
        &["emery", "system", "survey"][..],
        &["emery", "system", "plan"][..],
        &["emery", "system", "review"][..],
        &["emery", "system", "status"][..],
        &["emery", "adapter", "add"][..],
        &["emery", "adapter", "upgrade"][..],
        &["emery", "archive", "prune"][..],
        &["emery", "journal", "show"][..],
        &["emery", "debt"][..],
    ] {
        assert_eq!(cli(&provider, removed).await.exit, USAGE_EXIT, "{removed:?}");
    }

    let help = cli(&provider, &["emery", "--help"]).await;
    assert_eq!(help.exit, 0);
    let help = String::from_utf8_lossy(&help.stdout);
    assert_eq!(verbs(&help), ["completions", "show", "specify"]);
    for gone in ["init", "plan", "slice", "system", "journal", "debt", "adapter"] {
        assert!(
            !help.lines().any(|line| line.trim_start().starts_with(gone)),
            "help must not list `{gone}`: {help}"
        );
    }
}

// A run naming no sources discovers the project-root `emery.toml`; with no
// file to discover it fails with a typed error and writes nothing. The CWD
// move is safe under nextest's process-per-test isolation.
#[tokio::test]
async fn no_sources() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::env::set_current_dir(dir.path()).expect("enter empty project");
    let provider = Provider::idle();

    let response = cli(&provider, &["emery", "specify"]).await;
    assert_eq!(response.exit, 1);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("no sources"), "{stderr}");

    fail(&provider, &["emery", "specify"], 1, "specify-source-required").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// Naming the file carrier without a value explicitly selects the
// project-relative `emery.toml`; a missing explicit file is a read
// error, never a discovery miss.
#[tokio::test]
async fn default_config() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::env::set_current_dir(dir.path()).expect("enter empty project");
    let provider = Provider::idle();

    let response = cli(&provider, &["emery", "specify", "--config"]).await;
    assert_eq!(response.exit, 3);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("emery.toml"), "{stderr}");
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// `--config` carries the whole source list; mixing it with argv sources is
// refused with a typed error.
#[tokio::test]
async fn mixed_sources() {
    let provider = Provider::idle();

    for argv in [
        &["emery", "specify", "docs", "--config", "emery.toml"][..],
        &["emery", "specify", "--description", "intent=text", "--config", "emery.toml"][..],
    ] {
        fail(&provider, argv, 1, "bad_request").await;
    }
}

// Each source binds once; a repeated key is refused with a typed error
// whichever carrier repeats it.
#[tokio::test]
async fn duplicate() {
    let provider = Provider::idle();
    for argv in [
        &["emery", "specify", "docs", "docs"][..],
        &["emery", "specify", "docs", "--description", "docs=inline text"][..],
    ] {
        fail(&provider, argv, 1, "bad_request").await;
    }
}

// `--description` needs the `<adapter>=<text>` shape.
#[tokio::test]
async fn bad_description() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--description", "no-equals"], 1, "bad_request").await;
}

// Superseded spellings are gone from the grammar, not aliased: clap refuses
// them as unknown arguments.
#[tokio::test]
async fn old_flags() {
    let provider = Provider::idle();
    for argv in [
        &["emery", "specify", "--sources", "emery.toml"][..],
        &["emery", "specify", "--value", "intent=text"][..],
    ] {
        assert_eq!(cli(&provider, argv).await.exit, USAGE_EXIT, "{argv:?}");
    }
}

// The verbosity flags are the runtime's: it reads them from argv through the
// plan and sets the run's level before the guest runs, so the grammar
// declares them — before or after the verb, repeated — and the run is
// otherwise the bare run. `-v` beside `-q` is the grammar's own usage error,
// and the plan of a usage error selects nothing.
#[tokio::test]
async fn verbosity_flags() {
    let provider = Provider::idle();
    for (argv, verbose, quiet) in [
        (&["emery", "-v", "show", "spec"][..], 1, 0),
        (&["emery", "-vv", "show", "spec"][..], 2, 0),
        (&["emery", "--verbose", "show", "spec"][..], 1, 0),
        (&["emery", "show", "spec", "-v"][..], 1, 0),
        (&["emery", "-q", "show", "spec"][..], 0, 1),
        (&["emery", "-qq", "show", "spec"][..], 0, 2),
        (&["emery", "show", "spec", "--quiet"][..], 0, 1),
    ] {
        fail(&provider, argv, 2, "spec-not-generated").await;
        let plan = emery_cli::plan(argv.iter().copied());
        assert_eq!((plan.verbose, plan.quiet), (verbose, quiet), "{argv:?}");
        assert!(plan.adapters.is_empty(), "`show` names no adapter: {argv:?}");
    }

    assert_eq!(cli(&provider, &["emery", "-v", "-q", "show", "spec"]).await.exit, USAGE_EXIT);
    let plan = emery_cli::plan(["emery", "-v", "-q", "show", "spec"]);
    assert_eq!((plan.verbose, plan.quiet), (0, 0), "a usage error plans the bare run");

    let help = cli_ok(&provider, &["emery", "--help"]).await;
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("-v, --verbose"), "{help}");
    assert!(help.contains("-q, --quiet"), "{help}");

    // The flattened flags bring none of their own docs into the about text.
    let short = cli_ok(&provider, &["emery", "-h"]).await;
    let short = String::from_utf8_lossy(&short.stdout);
    assert_eq!(help.lines().next(), short.lines().next(), "{help}");
}

// The runtime declares a `specify` run's adapters before the engine runs, by
// reading the same carriers the run decodes: once per guest, a repeated
// reference pinned by the first digest among them, and carriers that do not
// decode declaring nothing — the run reports the refusal itself.
#[tokio::test]
async fn deployment_plan() {
    let scratch = tempfile::TempDir::new_in(env!("CARGO_MANIFEST_DIR")).expect("project tempdir");
    let component = scratch.path().join("custom.wasm");
    std::fs::write(&component, b"\0asm-stub").expect("write component");
    let component = component
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .expect("path under project")
        .to_str()
        .expect("utf-8 path")
        .to_string();
    let pin = support::digest("cd");
    let config = scratch.path().join("emery.toml");
    std::fs::write(
        &config,
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"./custom.wasm\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"./custom.wasm\"\ndigest = \"{pin}\"\n\n\
             [[source]]\nname = \"demo\"\nadapter = \"demo@1.2.0\"\n\n\
             [[source]]\nname = \"intent\"\nadapter = \"intent\"\n"
        ),
    )
    .expect("write emery.toml");
    let config = config.strip_prefix(env!("CARGO_MANIFEST_DIR")).expect("path under project");

    let plan = emery_cli::plan(["emery", "specify", "--config", config.to_str().expect("utf-8")]);
    let declared: Vec<(String, Option<String>)> = plan
        .adapters
        .iter()
        .map(|(adapter, pin)| (adapter.guest(), pin.as_ref().map(ToString::to_string)))
        .collect();
    assert_eq!(
        declared,
        [
            ("custom".to_string(), Some(pin.to_string())),
            ("emery:demo@1.2.0".to_string(), None),
            ("intent".to_string(), None),
        ],
        "one entry per guest, the shared component pinned by the one digest given"
    );

    let plan = emery_cli::plan(["emery", "specify", &component, "--description", "intent=brief"]);
    let declared: Vec<String> = plan.adapters.iter().map(|(adapter, _)| adapter.guest()).collect();
    assert_eq!(declared, ["custom", "intent"], "argv and `--description` carriers plan alike");

    let plan = emery_cli::plan(["emery", "specify", &component, "--config", "emery.toml"]);
    assert!(plan.adapters.is_empty(), "carriers the run refuses to combine declare nothing");
}

// `show` fails with a typed `spec-not-generated` error before any revision
// is committed.
#[tokio::test]
async fn no_revision() {
    let provider = Provider::idle();

    let response = cli(&provider, &["emery", "show", "spec"]).await;
    assert_eq!(response.exit, 2);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("spec-not-generated"), "{stderr}");

    fail(&provider, &["emery", "show", "design"], 2, "spec-not-generated").await;
}

#[tokio::test]
async fn completions() {
    let provider = Provider::idle();

    let completions = cli_ok(&provider, &["emery", "completions", "zsh"]).await;
    assert!(!completions.stdout.is_empty());
    let help = cli_ok(&provider, &["emery", "completions", "--help"]).await;
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("Pipe into your shell's completion directory"));
    assert!(help.contains("emery completions zsh > ~/.zsh/_emery"));
}

// Adapters version independently, so the binary reports its own SemVer.
#[tokio::test]
async fn host_semver() {
    let provider = Provider::idle();
    let response = cli_ok(&provider, &["emery", "--version"]).await;
    let stdout = String::from_utf8_lossy(&response.stdout);
    let expected = format!("emery {}", env!("CARGO_PKG_VERSION"));
    assert!(stdout.trim_end().ends_with(&expected), "{stdout}");
}

// Omnia forwards raw argv; a routed-id argv[0] renders as `emery`.
#[tokio::test]
async fn argv_zero_replaced() {
    let provider = Provider::idle();
    let expected = cli(&provider, &["emery", "specify", "--no-such-flag"]).await;
    let forwarded = cli(&provider, &["emery:engine@0.1.0", "specify", "--no-such-flag"]).await;

    assert_eq!(expected.exit, USAGE_EXIT);
    assert_eq!(forwarded.exit, expected.exit);
    assert_eq!(forwarded.stderr, expected.stderr);
    let stderr = String::from_utf8_lossy(&forwarded.stderr);
    assert!(stderr.contains("Usage: emery specify"), "{stderr}");
    assert!(!stderr.contains("emery:engine@0.1.0"));
}

// The stdout/stderr channel contract, table-driven across the surface.
#[tokio::test]
async fn response_contract() {
    for case in CASES {
        // A fresh store keeps `specify` sourceless and `show` without a revision.
        let response = cli(&Provider::idle(), case.argv).await;
        let stdout = String::from_utf8(response.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(response.stderr).expect("stderr is UTF-8");

        assert_eq!(response.exit, case.exit, "{} exit", case.name);
        assert!(stdout.contains(case.stdout), "{} stdout: {stdout}", case.name);
        assert!(stderr.contains(case.stderr), "{} stderr: {stderr}", case.name);
        if case.json_channels {
            if !stdout.is_empty() {
                serde_json::from_str::<Value>(&stdout)
                    .unwrap_or_else(|error| panic!("{} stdout JSON: {error}", case.name));
            }
            if !stderr.is_empty() {
                serde_json::from_str::<Value>(&stderr)
                    .unwrap_or_else(|error| panic!("{} stderr JSON: {error}", case.name));
            }
        }
    }
}
