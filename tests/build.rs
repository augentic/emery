//! Verifies the operator journey from a committed plan through `build`.
//!
//! Each scenario seeds a revision, drives the real command façade over a
//! scripted target, and asserts the slices dispatched, the envelope, the exit
//! code, and that the engine wrote nothing of its own.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;

use emery_adapter::target::Report;
use omnia_sdk::{bad_gateway, bad_request};
use serde_json::Value;
use support::{Provider, Scratch, cli_ok, digest, fail, seed};

// The target every scenario names, and the guest it loads and dispatches as.
const BUILDER: &str = "acme:builder@2.1.0";
const BUILDER_ID: &str = "acme:builder";

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");
const WIDE_PLAN: &[u8] = include_bytes!("build/wide-plan.json");
const SPEC_001: &str = include_str!("build/spec-001.md");
const SPEC_002: &str = include_str!("build/spec-002.md");
const DESIGN_MD: &str = include_str!("build/design.md");

const PLAN_001: &str = "## Slice: authentication\n\nID: SLICE-001\nRequirements: [REQ-001, \
                        REQ-002]\n\nDelivers sign-in and the session it issues; a session cannot \
                        be verified without a sign-in to issue it, so the two stems are one build.";
const PLAN_002: &str = "## Slice: orders\n\nID: SLICE-002\nRequirements: [REQ-003, REQ-004]\nTypes: \
                        [orders.line, orders.order]\nDepends on: [SLICE-001]\n\nDelivers order \
                        creation and cancellation for a signed-in caller; verified by placing and \
                        cancelling an order under a session from `authentication`.";

// A provider over the two-slice revision, its model never dispatched.
fn planned() -> (Provider, String) {
    let provider = Provider::idle();
    let id = seed(&provider.storage, SPEC, DESIGN, PLAN);
    (provider, id)
}

fn report(covered: &[&str], written: &[&str]) -> Report {
    Report {
        covered: covered.iter().map(ToString::to_string).collect(),
        written: written.iter().map(ToString::to_string).collect(),
    }
}

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

// --- journey ---

// Every slice of the plan is dispatched in dependency order, each with its
// plan entry, its cut of the specification, and the whole design, into the
// project root; the engine's own state is left as it was.
#[tokio::test]
async fn build_plan() {
    let (mut provider, id) = planned();
    provider.target.reports.insert(
        "SLICE-002".to_string(),
        Ok(report(&["REQ-003"], &["src/orders.rs", "Cargo.toml"])),
    );
    let before = provider.storage.snapshot();

    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;

    // the text render
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 2 slices in 2 waves, widest 1\n  SLICE-001 \
             authentication: covered 2/2, written 1 file\n  SLICE-002 orders: covered 1/2 \
             (uncovered REQ-004), written 2 files\n"
        )
    );

    // one gate, two builds in order, each to the guest the reference names
    assert_eq!(*provider.target.metadata.lock().expect("metadata"), [BUILDER_ID]);
    let calls = provider.target.calls();
    let [(first_id, first, first_root), (second_id, second, second_root)] = calls.as_slice() else {
        panic!("two slices are two dispatches: {calls:?}");
    };
    assert_eq!((first_id.as_str(), second_id.as_str()), (BUILDER_ID, BUILDER_ID));
    assert_eq!((first_root.as_str(), second_root.as_str()), (".", "."), "the project root");

    // each slice carries its own documents
    assert_eq!(first.id, "SLICE-001");
    assert_eq!(first.name, "authentication");
    assert_eq!(first.requirements, ["REQ-001", "REQ-002"]);
    assert_eq!(first.spec, SPEC_001, "the specification is cut to the slice");
    assert_eq!(first.design, DESIGN_MD, "the design is whole");
    assert_eq!(first.plan, PLAN_001, "the plan entry is the slice's own");
    assert_eq!(second.id, "SLICE-002");
    assert_eq!(second.name, "orders");
    assert_eq!(second.requirements, ["REQ-003", "REQ-004"]);
    assert_eq!(second.spec, SPEC_002);
    assert_eq!(second.design, DESIGN_MD);
    assert_eq!(second.plan, PLAN_002);

    // the JSON envelope
    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["waves"], serde_json::json!([["SLICE-001"], ["SLICE-002"]]), "{envelope}");
    assert_eq!(
        envelope["slices"],
        serde_json::json!([
            {"id": "SLICE-001", "name": "authentication", "covered": ["REQ-001", "REQ-002"], "uncovered": [], "written": ["src/authentication.rs"]},
            {"id": "SLICE-002", "name": "orders", "covered": ["REQ-003"], "uncovered": ["REQ-004"], "written": ["src/orders.rs", "Cargo.toml"]},
        ]),
        "{envelope}"
    );

    assert_eq!(provider.storage.snapshot(), before, "a build writes no engine state");
    provider.model.assert_exhausted();
}

// A slice waiting on nothing is built in the first wave whatever its id, so
// the serial order is the waves flattened and a build at a width of one
// reproduces it.
#[tokio::test]
async fn build_waves() {
    let provider = Provider::idle();
    let id = seed(&provider.storage, SPEC, DESIGN, WIDE_PLAN);

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["waves"],
        serde_json::json!([["SLICE-001", "SLICE-003"], ["SLICE-002"]]),
        "{envelope}"
    );
    let dispatched: Vec<String> =
        provider.target.calls().into_iter().map(|(_, slice, _)| slice.id).collect();
    assert_eq!(dispatched, ["SLICE-001", "SLICE-003", "SLICE-002"]);
    let built: Vec<&str> = envelope["slices"]
        .as_array()
        .expect("slices")
        .iter()
        .filter_map(|slice| slice["id"].as_str())
        .collect();
    assert_eq!(built, dispatched, "the report follows the dispatch order");

    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        stdout
            .starts_with(&format!("built revision {id}\n  plan: 3 slices in 2 waves, widest 2\n")),
        "{stdout}"
    );
}

// The `[target]` table names the adapter and its pin, whether the file is
// named or discovered; the `[[source]]` entries beside it stay out.
#[tokio::test]
async fn build_from_config() {
    let scratch = Scratch::new();
    let pin = digest("cd");
    let config = scratch.config(&format!(
        "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
         [target]\nadapter = \"{BUILDER}\"\ndigest = \"{pin}\"\n"
    ));
    let (provider, _) = planned();

    cli_ok(&provider, &["emery", "build", "--config", &config]).await;

    assert!(
        provider.source.metadata.lock().expect("metadata").is_empty(),
        "no source adapter loads for a build"
    );
    assert_eq!(*provider.target.metadata.lock().expect("metadata"), [BUILDER_ID]);
    assert_eq!(provider.target.calls().len(), 2);
}

// A run naming no adapter reads the project-root file. The CWD move is
// hermetic under nextest's process-per-test isolation.
#[tokio::test]
async fn build_discovered() {
    let project = tempfile::TempDir::new().expect("project dir");
    fs::write(project.path().join("emery.toml"), format!("[target]\nadapter = \"{BUILDER}\"\n"))
        .expect("write emery.toml");
    std::env::set_current_dir(project.path()).expect("enter project");
    let (provider, _) = planned();

    cli_ok(&provider, &["emery", "build"]).await;

    assert_eq!(*provider.target.metadata.lock().expect("metadata"), [BUILDER_ID]);
    assert_eq!(provider.target.calls().len(), 2);
}

// --- refusals ---

// A file without a `[target]` table names no adapter, as an empty project does.
#[tokio::test]
async fn build_no_target() {
    let scratch = Scratch::new();
    let config =
        scratch.config("[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n");
    let (provider, _) = planned();

    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "build-target-required").await;
    assert_message(&envelope, "has no `[target]` table");

    // a malformed table is refused at the field
    let config =
        scratch.config(&format!("[target]\nadapter = \"{BUILDER}\"\nplatform = \"rust\"\n"));
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "bad_request").await;
    assert_message(&envelope, "unknown field `platform`");

    // an adapter that is not an exact package reference is refused at the field
    let config = scratch.config("[target]\nadapter = \"builder\"\n");
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "bad_request").await;
    assert_message(&envelope, "adapter `builder` names no version");
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
}

// A reference the grammar refuses never reaches the loader: no version, no
// namespace, a path, a bare name.
#[tokio::test]
async fn build_malformed_reference() {
    let (provider, _) = planned();

    for (reference, fragment) in [
        ("acme:builder", "adapter `acme:builder` names no version"),
        ("builder@2.1.0", "adapter `builder@2.1.0` names no namespace"),
        ("./builder.wasm", "adapter `./builder.wasm` names no version"),
        ("builder", "adapter `builder` names no version"),
        ("acme:builder@latest", "adapter `acme:builder@latest` has an invalid version `latest`"),
    ] {
        let envelope =
            fail(&provider, &["emery", "build", reference], 1, "adapter-reference").await;
        assert_message(&envelope, fragment);
        assert_eq!(
            envelope["hint"], "an adapter is an exact package reference, `namespace:name@version`",
            "{envelope}"
        );
    }
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
}

// The revision is read before the adapter loads, so a project with none
// refuses typed and nothing is asked of the loader.
#[tokio::test]
async fn build_no_revision() {
    let provider = Provider::idle();

    let envelope = fail(&provider, &["emery", "build", BUILDER], 2, "spec-not-generated").await;
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("build")),
        "the hint names the way out: {envelope}"
    );
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
}

#[tokio::test]
async fn build_outdated() {
    let provider = Provider::idle();
    seed(
        &provider.storage,
        br#"{"emery": 1, "requirements": []}"#,
        br#"{"emery": 1, "sections": []}"#,
        br#"{"emery": 1, "slices": []}"#,
    );

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "spec-outdated").await;
    assert_message(&envelope, "grammar 1");
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
}

// The target's `emery-version` pin is gated as a source adapter's is, before
// any slice is dispatched.
#[tokio::test]
async fn build_unsupported_version() {
    let (mut provider, _) = planned();
    provider.target.versions.insert(BUILDER_ID.to_string(), "99.0.0".to_string());

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "unsupported-version").await;
    assert_message(&envelope, "adapter `acme:builder` requires emery 99.0.0 or newer");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
}

// A target's refusal of a slice keeps its class and code, named for the
// slice; the slices after it are never dispatched.
#[tokio::test]
async fn build_refused() {
    let (mut provider, _) = planned();
    provider
        .target
        .reports
        .insert("SLICE-001".to_string(), Err(bad_request!("the tree already holds `src/`")));

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "bad_request").await;
    assert_message(
        &envelope,
        "slice `SLICE-001` (authentication) failed: the tree already holds `src/`",
    );
    assert!(!envelope["message"].as_str().unwrap_or("").contains("stays written"));
    assert_eq!(provider.target.calls().len(), 1, "the run stops at the first");
}

// A report the gate refuses is the adapter's defect: every rule it breaks is
// named, and the run stops there.
#[tokio::test]
async fn build_bad_report() {
    let (mut provider, _) = planned();
    provider.target.reports.insert(
        "SLICE-001".to_string(),
        Ok(report(&["REQ-001", "REQ-009"], &["src/auth.rs", "../escape.rs", "src/auth.rs"])),
    );

    let envelope = fail(&provider, &["emery", "build", BUILDER], 3, "server_error").await;
    assert_message(&envelope, "`acme:builder` returned an invalid report");
    for finding in [
        "- covered `REQ-009` is not a requirement of slice `SLICE-001`; its requirements are \
         REQ-001, REQ-002",
        "- written `../escape.rs` escapes the root",
        "- written `src/auth.rs` is listed twice",
    ] {
        assert_message(&envelope, finding);
    }
    assert_eq!(provider.target.calls().len(), 1);
}

// A failure after a slice was built names what stays written.
#[tokio::test]
async fn build_stops_at_failure() {
    let (mut provider, _) = planned();
    provider
        .target
        .reports
        .insert("SLICE-002".to_string(), Err(bad_gateway!("the model timed out")));

    let envelope = fail(&provider, &["emery", "build", BUILDER], 4, "bad_gateway").await;
    assert_message(
        &envelope,
        "slice `SLICE-002` (orders) failed; SLICE-001 built before it stays written: the model \
         timed out",
    );
    assert_eq!(provider.target.calls().len(), 2);
}
