//! Verifies the operator journey from a committed plan through `build`.
//!
//! Each scenario seeds a revision, drives the real command façade over a
//! scripted target, and asserts the slices dispatched, the envelope, the exit
//! code, and that the engine wrote nothing of its own.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use emery_adapter::target::Report;
use emery_engine::{ADAPTERS, CONTAINER, REVISION_KEY};
use omnia_sdk::plugins::Location;
use omnia_sdk::{bad_gateway, bad_request};
use omnia_test::guest::Memory;
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use support::{Provider, cli_ok, digest, fail};

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");
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

// Two roots, as the guest's two preopens: a config file is project-relative,
// a component is relative to the adapters root the runtime mounts apart from
// the project, so each write answers with the path the CLI is handed.
struct Scratch {
    project: tempfile::TempDir,
    adapters: tempfile::TempDir,
}

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn adapters_root() -> PathBuf {
    Path::new(PROJECT_ROOT).join(ADAPTERS)
}

impl Scratch {
    fn new() -> Self {
        let adapters = adapters_root();
        fs::create_dir_all(&adapters).expect("adapters root");
        Self {
            project: tempfile::TempDir::new_in(PROJECT_ROOT).expect("project tempdir"),
            adapters: tempfile::TempDir::new_in(adapters).expect("adapters tempdir"),
        }
    }

    fn write_under(root: &Path, base: &Path, name: &str, body: &str) -> String {
        let path = root.join(name);
        fs::write(&path, body).unwrap_or_else(|err| panic!("write {name}: {err}"));
        path.strip_prefix(base)
            .expect("path under its root")
            .to_str()
            .expect("utf-8 path")
            .to_string()
    }

    // The loader is scripted, so the component only has to exist as a `.wasm` file.
    fn component(&self) -> String {
        Self::write_under(self.adapters.path(), &adapters_root(), "builder.wasm", "\0asm-stub")
    }

    fn config(&self, body: &str) -> String {
        Self::write_under(self.project.path(), Path::new(PROJECT_ROOT), "emery.toml", body)
    }
}

// The engine's content id: SHA-256 over the length-prefixed bodies, spec,
// design, then plan.
fn revision(spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for body in [spec, design, plan] {
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}

fn seed(storage: &Memory, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let id = revision(spec, design, plan);
    storage.insert_object(CONTAINER, &format!("{id}/spec.json"), spec);
    storage.insert_object(CONTAINER, &format!("{id}/design.json"), design);
    storage.insert_object(CONTAINER, &format!("{id}/plan.json"), plan);
    storage.insert_state(REVISION_KEY, id.as_bytes());
    id
}

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
    let scratch = Scratch::new();
    let component = scratch.component();
    let (mut provider, id) = planned();
    provider.target.reports.insert(
        "SLICE-002".to_string(),
        Ok(report(&["REQ-003"], &["src/orders.rs", "Cargo.toml"])),
    );
    let before = provider.storage.snapshot();

    let resp = cli_ok(&provider, &["emery", "build", &component]).await;

    // the text render
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  SLICE-001 authentication: covered 2/2, written 1 file\n  \
             SLICE-002 orders: covered 1/2 (uncovered REQ-004), written 2 files\n"
        )
    );

    // one load, one gate, two builds in order
    let loads = provider.plugins.loads();
    let [(Location::Path(path), None)] = loads.as_slice() else {
        panic!("a local component is one unpinned load by path: {loads:?}");
    };
    assert_eq!(*path, format!("{ADAPTERS}/{component}"));
    assert_eq!(*provider.target.metadata.lock().expect("metadata"), ["builder"]);
    let calls = provider.target.calls.lock().expect("calls").clone();
    let [(first_id, first, first_root), (second_id, second, second_root)] = calls.as_slice() else {
        panic!("two slices are two dispatches: {calls:?}");
    };
    assert_eq!((first_id.as_str(), second_id.as_str()), ("builder", "builder"));
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
    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", &component]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
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

// The `[target]` table names the adapter and its pin, whether the file is
// named or discovered; the `[[source]]` entries beside it stay out.
#[tokio::test]
async fn build_from_config() {
    let scratch = Scratch::new();
    let component = scratch.component();
    let pin = digest("cd");
    let config = scratch.config(&format!(
        "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n\n\
         [target]\nadapter = \"{component}\"\ndigest = \"{pin}\"\n"
    ));
    let (mut provider, _) = planned();
    provider.plugins = provider.plugins.clone().digest("builder", pin.clone());

    cli_ok(&provider, &["emery", "build", "--config", &config]).await;

    assert_eq!(
        provider.plugins.loads(),
        [(Location::Path(format!("{ADAPTERS}/{component}")), Some(pin))],
        "the pin rides the load"
    );
    assert!(
        provider.source.metadata.lock().expect("metadata").is_empty(),
        "no source adapter loads for a build"
    );
    assert_eq!(provider.target.calls.lock().expect("calls").len(), 2);
}

// A run naming no adapter reads the project-root file; a package adapter
// routes through its `[registries]` table. The CWD move is hermetic under
// nextest's process-per-test isolation.
#[tokio::test]
async fn build_discovered() {
    let project = tempfile::TempDir::new().expect("project dir");
    fs::write(
        project.path().join("emery.toml"),
        "[target]\nadapter = \"acme:builder@2.1.0\"\n\n[registries]\nacme = \"registry.acme.io\"\n",
    )
    .expect("write emery.toml");
    std::env::set_current_dir(project.path()).expect("enter project");
    let (provider, _) = planned();

    cli_ok(&provider, &["emery", "build"]).await;

    assert_eq!(
        provider.plugins.loads(),
        [(
            Location::Registry {
                package: "acme:builder@2.1.0".to_string(),
                endpoint: Some("registry.acme.io".to_string()),
            },
            None
        )]
    );
    assert_eq!(provider.target.calls.lock().expect("calls").len(), 2);

    // the table routes an argv adapter all the same
    let (provider, _) = planned();
    cli_ok(&provider, &["emery", "build", "acme:builder@2.1.0"]).await;
    assert_eq!(provider.loaded(), ["acme:builder"]);
}

// --- refusals ---

// A file without a `[target]` table names no adapter, as an empty project does.
#[tokio::test]
async fn build_no_target() {
    let scratch = Scratch::new();
    let config = scratch.config("[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n");
    let (provider, _) = planned();

    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "build-target-required").await;
    assert_message(&envelope, "has no `[target]` table");
    assert!(provider.plugins.loads().is_empty(), "nothing loads for a run naming no target");

    // a malformed table is refused at the field
    let config = scratch.config("[target]\nadapter = \"builder\"\nplatform = \"rust\"\n");
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "bad_request").await;
    assert_message(&envelope, "unknown field `platform`");
}

// The revision is read before the adapter loads, so a project with none
// refuses typed and nothing is asked of the loader.
#[tokio::test]
async fn build_no_revision() {
    let scratch = Scratch::new();
    let component = scratch.component();
    let provider = Provider::idle();

    let envelope = fail(&provider, &["emery", "build", &component], 2, "spec-not-generated").await;
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("build")),
        "the hint names the way out: {envelope}"
    );
    assert!(provider.plugins.loads().is_empty(), "the revision is read before any load");
    assert!(provider.target.calls.lock().expect("calls").is_empty(), "no slice is dispatched");
}

#[tokio::test]
async fn build_outdated() {
    let scratch = Scratch::new();
    let component = scratch.component();
    let provider = Provider::idle();
    seed(
        &provider.storage,
        br#"{"emery": 1, "requirements": []}"#,
        br#"{"emery": 1, "sections": []}"#,
        br#"{"emery": 1, "slices": []}"#,
    );

    let envelope = fail(&provider, &["emery", "build", &component], 1, "spec-outdated").await;
    assert_message(&envelope, "grammar 1");
    assert!(provider.plugins.loads().is_empty(), "an outdated revision loads nothing");
}

// The target's `emery-version` pin is gated as a source adapter's is, before
// any slice is dispatched.
#[tokio::test]
async fn build_unsupported_version() {
    let scratch = Scratch::new();
    let component = scratch.component();
    let (mut provider, _) = planned();
    provider.target.versions.insert("builder".to_string(), "99.0.0".to_string());

    let envelope = fail(&provider, &["emery", "build", &component], 1, "unsupported-version").await;
    assert_message(&envelope, "adapter `builder` requires emery 99.0.0 or newer");
    assert!(provider.target.calls.lock().expect("calls").is_empty(), "no slice is dispatched");
}

// A target's refusal of a slice keeps its class and code, named for the
// slice; the slices after it are never dispatched.
#[tokio::test]
async fn build_refused() {
    let scratch = Scratch::new();
    let component = scratch.component();
    let (mut provider, _) = planned();
    provider
        .target
        .reports
        .insert("SLICE-001".to_string(), Err(bad_request!("the tree already holds `src/`")));

    let envelope = fail(&provider, &["emery", "build", &component], 1, "bad_request").await;
    assert_message(
        &envelope,
        "slice `SLICE-001` (authentication) failed: the tree already holds `src/`",
    );
    assert!(!envelope["message"].as_str().unwrap_or("").contains("stays written"));
    assert_eq!(provider.target.calls.lock().expect("calls").len(), 1, "the run stops at the first");
}

// A report the gate refuses is the adapter's defect: every rule it breaks is
// named, and the run stops there.
#[tokio::test]
async fn build_bad_report() {
    let scratch = Scratch::new();
    let component = scratch.component();
    let (mut provider, _) = planned();
    provider.target.reports.insert(
        "SLICE-001".to_string(),
        Ok(report(&["REQ-001", "REQ-009"], &["src/auth.rs", "../escape.rs", "src/auth.rs"])),
    );

    let envelope = fail(&provider, &["emery", "build", &component], 3, "server_error").await;
    assert_message(&envelope, "`builder` returned an invalid report");
    for finding in [
        "- covered `REQ-009` is not a requirement of slice `SLICE-001`; its requirements are \
         REQ-001, REQ-002",
        "- written `../escape.rs` escapes the workspace root",
        "- written `src/auth.rs` is listed twice",
    ] {
        assert_message(&envelope, finding);
    }
    assert_eq!(provider.target.calls.lock().expect("calls").len(), 1);
}

// A failure after a slice was built names what stays written.
#[tokio::test]
async fn build_stops_at_failure() {
    let scratch = Scratch::new();
    let component = scratch.component();
    let (mut provider, _) = planned();
    provider
        .target
        .reports
        .insert("SLICE-002".to_string(), Err(bad_gateway!("the model timed out")));

    let envelope = fail(&provider, &["emery", "build", &component], 4, "bad_gateway").await;
    assert_message(
        &envelope,
        "slice `SLICE-002` (orders) failed; SLICE-001 built before it stays written: the model \
         timed out",
    );
    assert_eq!(provider.target.calls.lock().expect("calls").len(), 2);
}
