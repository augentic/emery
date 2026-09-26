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

// A usage error exits `USAGE_EXIT`, so exit 2 always means a `NotFound` envelope.
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

// The CWD move is hermetic under nextest's process-per-test isolation.
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

// A bare `--config` names the project-relative `emery.toml` explicitly, so a
// missing file is a read error, never a discovery miss.
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

// A repeated key is refused whichever carrier repeats it.
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

#[tokio::test]
async fn bad_description() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--description", "no-equals"], 1, "bad_request").await;
}

// Superseded spellings are gone from the grammar, not aliased.
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

// The runtime reads the verbosity flags from argv before the guest runs, so the
// grammar declares them and the run is otherwise the bare run.
#[tokio::test]
async fn verbosity_flags() {
    let provider = Provider::idle();
    for argv in [
        &["emery", "-v", "show", "spec"][..],
        &["emery", "-vv", "show", "spec"][..],
        &["emery", "--verbose", "show", "spec"][..],
        &["emery", "show", "spec", "-v"][..],
        &["emery", "-q", "show", "spec"][..],
        &["emery", "-qq", "show", "spec"][..],
        &["emery", "show", "spec", "--quiet"][..],
    ] {
        fail(&provider, argv, 2, "spec-not-generated").await;
    }

    assert_eq!(cli(&provider, &["emery", "-v", "-q", "show", "spec"]).await.exit, USAGE_EXIT);

    let help = cli_ok(&provider, &["emery", "--help"]).await;
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("-v, --verbose"), "{help}");
    assert!(help.contains("-q, --quiet"), "{help}");

    // the flattened flags bring none of their own docs into the about text
    let short = cli_ok(&provider, &["emery", "-h"]).await;
    let short = String::from_utf8_lossy(&short.stdout);
    assert_eq!(help.lines().next(), short.lines().next(), "{help}");
}

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

#[tokio::test]
async fn response_contract() {
    for case in CASES {
        // a fresh store keeps `specify` sourceless and `show` without a revision
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
