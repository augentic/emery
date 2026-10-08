//! Verifies the operator journey from a committed plan through `build`.
//!
//! Each scenario seeds a revision, drives the real command façade over a
//! scripted target and scripted version control, and asserts the slices
//! dispatched, the working copies cut, the commits sealed and merged, the
//! waves verified and labelled, the envelope, the exit code, and that the
//! engine wrote nothing of its own.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::borrow::Cow;
use std::fs;

use emery_adapter::target::{MergeRule, MergeStrategy, Report, Verdict};
use omnia_sdk::vcs::{Change, ChangeKind, Entry, Error, Merged, Rule, Strategy};
use omnia_sdk::{bad_gateway, bad_request};
use serde_json::{Value, json};
use support::{HEAD, Provider, Rendezvous, Scratch, cli, cli_ok, digest, fail, seed};

// The target every scenario names, and the guest it loads and dispatches as.
const BUILDER: &str = "acme:builder@2.1.0";
const BUILDER_ID: &str = "acme:builder";

// The commit the project checkout sits on, where a scenario tells it from the head.
const BASE: &str = "1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d";
// The head an earlier run's label points at, where a scenario resumes.
const LABELLED: &str = "e1f2a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4";
const INTEGRATION: &str = "./.emery/vcs/integration";
const WORKTREES: &str = "./.emery/vcs/worktrees";
const REMOTE_URL: &str = "https://example.com/acme/shop.git";
// `REMOTE_URL` normalised and hashed, the clone every run of it shares.
const REMOTE_CLONE: &str = "./.emery/vcs/repos/f4c82940a714c3c5";
const USAGE_EXIT: u8 = 64;

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

// The two-slice plan's slices, and the wide plan's.
const AUTHENTICATION: (&str, &str) = ("SLICE-001", "authentication");
const ORDERS: (&str, &str) = ("SLICE-002", "orders");
const WIDE_SESSIONS: (&str, &str) = ("SLICE-002", "sessions");
const WIDE_ORDERS: (&str, &str) = ("SLICE-003", "orders");

// A provider over the two-slice revision, its model never dispatched.
fn planned() -> (Provider, String) {
    let provider = Provider::idle();
    let id = seed(&provider.storage, SPEC, DESIGN, PLAN);
    (provider, id)
}

// A provider over the wide revision: two slices ready at once, one after.
fn wide() -> (Provider, String) {
    let provider = Provider::idle();
    let id = seed(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    (provider, id)
}

fn report(covered: &[&str], written: &[&str]) -> Report {
    Report {
        covered: covered.iter().map(ToString::to_string).collect(),
        written: written.iter().map(ToString::to_string).collect(),
    }
}

fn verdict(passed: bool, failures: &[&str]) -> Verdict {
    Verdict {
        passed,
        failures: failures.iter().map(ToString::to_string).collect(),
    }
}

fn change(path: &str, kind: ChangeKind) -> Change {
    Change {
        path: path.to_owned(),
        kind,
    }
}

fn conflicted(paths: &[&str]) -> Merged {
    Merged {
        commit: None,
        conflicts: paths.iter().map(ToString::to_string).collect(),
    }
}

fn entry(id: &str, message: &str) -> Entry {
    Entry {
        id: id.to_owned(),
        message: message.to_owned(),
    }
}

fn worktree(slice: &str) -> String {
    format!("{WORKTREES}/{slice}")
}

// The message a build seals a slice under in its working copy and as its merge.
fn message(
    id: &str, (slice, name): (&str, &str), requirements: &str, covered: &str, base: &str,
    wave: usize,
) -> String {
    format!(
        "{slice} {name}\n\nSlice: {slice}\nRevision: {id}\nRequirements: {requirements}\nCovered: \
         {covered}\nAdapter: {BUILDER}\nBase: {base}\nWave: {wave}"
    )
}

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

// What the target was asked to build, in dispatch order: the slice, the head
// it builds over, and the working copy it is lent.
fn dispatched(provider: &Provider) -> Vec<(String, String, String)> {
    provider
        .target
        .calls()
        .into_iter()
        .map(|(_, slice, root)| (slice.id, slice.base, root))
        .collect()
}

// The exchange a fresh two-slice build makes over `repo` from `base`: the
// label looked for, the integration working copy cut, one slice per wave.
fn integrated(repo: &str, base: &str, id: &str) -> Vec<String> {
    let mut calls =
        vec![format!("resolve {repo} emery/{id}"), format!("add {repo} {INTEGRATION} {base}")];
    calls.extend(wave(repo, id, base, &[AUTHENTICATION]));
    calls.extend(wave(repo, id, HEAD, &[ORDERS]));
    calls.push(format!("remove {INTEGRATION}"));
    calls
}

// One wave's exchange over `repo`: every slice's working copy cut at `head`,
// then each sealed, merged, and removed in id order, then the integrated
// head verified and labelled.
fn wave(repo: &str, id: &str, head: &str, slices: &[(&str, &str)]) -> Vec<String> {
    let mut calls: Vec<String> =
        slices.iter().map(|(slice, _)| format!("add {repo} {} {head}", worktree(slice))).collect();
    for (slice, name) in slices {
        calls.push(format!("commit {} {slice} {name}", worktree(slice)));
        calls.push(format!("merge {INTEGRATION} {slice}-commit {slice} {name}"));
        calls.push(format!("remove {}", worktree(slice)));
    }
    calls.extend([
        format!("pending {INTEGRATION}"),
        format!("head {INTEGRATION}"),
        format!("label {repo} emery/{id} {HEAD}"),
    ]);
    calls
}

// --- journey ---

// Every slice of the plan is dispatched after the slices it depends on, each
// with its plan entry, its cut of the specification, the whole design, and
// the head it builds over, into a working copy of its own cut from that
// head; what each wrote is sealed as its commit and merged into the
// integration working copy, each wave verified and labelled, and the
// engine's own state left as it was.
#[tokio::test]
async fn build_plan() {
    let (mut provider, id) = planned();
    provider.target.reports.insert(
        "SLICE-002".to_string(),
        Ok(report(&["REQ-003"], &["src/orders.rs", "Cargo.toml"])),
    );
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    let before = provider.storage.snapshot();

    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;

    // the text render
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 2 slices in 2 waves, widest 1\n  base {BASE}\n  wave 1: \
             1 slice verified at 9f8e7d6c\n    SLICE-001 authentication: covered 2/2, written 1 \
             file, merged m:SLICE-001\n  wave 2: 1 slice verified at 9f8e7d6c\n    SLICE-002 \
             orders: covered 1/2 (uncovered REQ-004), written 2 files, merged m:SLICE-002\n  \
             labelled emery/{id} at {HEAD}\n"
        )
    );

    // one gate, two builds in order, each to the guest the reference names
    assert_eq!(*provider.target.metadata.lock().expect("metadata"), [BUILDER_ID]);
    let calls = provider.target.calls();
    let [(first_id, first, first_root), (second_id, second, second_root)] = calls.as_slice() else {
        panic!("two slices are two dispatches: {calls:?}");
    };
    assert_eq!((first_id.as_str(), second_id.as_str()), (BUILDER_ID, BUILDER_ID));
    assert_eq!(
        (first_root.as_str(), second_root.as_str()),
        (worktree("SLICE-001").as_str(), worktree("SLICE-002").as_str()),
        "a working copy per slice, never the checkout"
    );

    // each slice carries its own documents and the head it builds over
    assert_eq!(first.id, "SLICE-001");
    assert_eq!(first.name, "authentication");
    assert_eq!(first.requirements, ["REQ-001", "REQ-002"]);
    assert_eq!(first.spec, SPEC_001, "the specification is cut to the slice");
    assert_eq!(first.design, DESIGN_MD, "the design is whole");
    assert_eq!(first.plan, PLAN_001, "the plan entry is the slice's own");
    assert_eq!(first.base, BASE, "the first wave builds over the base");
    assert_eq!(second.id, "SLICE-002");
    assert_eq!(second.name, "orders");
    assert_eq!(second.requirements, ["REQ-003", "REQ-004"]);
    assert_eq!(second.spec, SPEC_002);
    assert_eq!(second.design, DESIGN_MD);
    assert_eq!(second.plan, PLAN_002);
    assert_eq!(second.base, HEAD, "the second wave builds over the verified head");
    assert_eq!(provider.target.verifies(), [INTEGRATION, INTEGRATION], "one verify per wave");

    // the sealed base, one commit and merge per slice, the label after each wave
    let mut expected = vec!["pending .".to_owned(), "head .".to_owned()];
    expected.extend(integrated(".", BASE, &id));
    assert_eq!(provider.vcs.calls(), expected);
    let messages = [
        message(&id, AUTHENTICATION, "REQ-001, REQ-002", "REQ-001, REQ-002", BASE, 1),
        message(&id, ORDERS, "REQ-003, REQ-004", "REQ-003", HEAD, 2),
    ];
    assert_eq!(provider.vcs.messages(), messages);
    let merged: Vec<String> =
        provider.vcs.merged().into_iter().map(|(message, _)| message).collect();
    assert_eq!(merged, messages, "a merge carries the slice's message");

    // the JSON envelope
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["waves"], json!([["SLICE-001"], ["SLICE-002"]]), "{envelope}");
    assert_eq!(envelope["base"], BASE, "{envelope}");
    assert!(envelope.get("resumed").is_none(), "a fresh build resumes nothing: {envelope}");
    assert_eq!(
        envelope["slices"],
        json!([
            {"id": "SLICE-001", "name": "authentication", "wave": 1, "covered": ["REQ-001", "REQ-002"], "uncovered": [], "written": ["src/authentication.rs"], "commit": "m:SLICE-001"},
            {"id": "SLICE-002", "name": "orders", "wave": 2, "covered": ["REQ-003"], "uncovered": ["REQ-004"], "written": ["src/orders.rs", "Cargo.toml"], "commit": "m:SLICE-002"},
        ]),
        "{envelope}"
    );
    assert_eq!(envelope["verified"], json!([HEAD, HEAD]), "{envelope}");
    assert_eq!(envelope["head"], HEAD, "{envelope}");
    assert_eq!(envelope["label"], format!("emery/{id}"), "{envelope}");
    assert!(envelope.get("pushed").is_none(), "no remote, no push: {envelope}");

    assert_eq!(provider.storage.snapshot(), before, "a build writes no engine state");
    provider.model.assert_exhausted();
    provider.vcs.assert_exhausted();
}

// The slices ready at once are built at once, each over the wave's head in
// a working copy of its own, and merged in id order whatever order they
// finished in; the next wave builds over the verified head. The rendezvous
// holds each build until the other is dispatched, so an engine building one
// slice at a time fails the scenario.
#[tokio::test]
async fn build_waves() {
    let (mut provider, id) = wide();
    provider.target.rendezvous = Some(Rendezvous::from_iter(["SLICE-001", "SLICE-003"]));
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["waves"], json!([["SLICE-001", "SLICE-003"], ["SLICE-002"]]), "{envelope}");
    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-003".to_owned(), BASE.to_owned(), worktree("SLICE-003")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ],
        "the first wave over the base, the second over its verified head"
    );
    let mut expected = vec![
        "pending .".to_owned(),
        "head .".to_owned(),
        format!("resolve . emery/{id}"),
        format!("add . {INTEGRATION} {BASE}"),
    ];
    expected.extend(wave(".", &id, BASE, &[AUTHENTICATION, WIDE_ORDERS]));
    expected.extend(wave(".", &id, HEAD, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected);
    assert_eq!(provider.target.verifies(), [INTEGRATION, INTEGRATION]);
    assert_eq!(
        envelope["slices"],
        json!([
            {"id": "SLICE-001", "name": "authentication", "wave": 1, "covered": ["REQ-001"], "uncovered": [], "written": ["src/authentication.rs"], "commit": "m:SLICE-001"},
            {"id": "SLICE-003", "name": "orders", "wave": 1, "covered": ["REQ-003", "REQ-004"], "uncovered": [], "written": ["src/orders.rs"], "commit": "m:SLICE-003"},
            {"id": "SLICE-002", "name": "sessions", "wave": 2, "covered": ["REQ-002"], "uncovered": [], "written": ["src/sessions.rs"], "commit": "m:SLICE-002"},
        ]),
        "{envelope}"
    );
    assert_eq!(envelope["verified"], json!([HEAD, HEAD]), "{envelope}");

    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 3 slices in 2 waves, widest 2\n  base {BASE}\n  wave 1: \
             2 slices verified at 9f8e7d6c\n    SLICE-001 authentication: covered 1/1, written 1 \
             file, merged m:SLICE-001\n    SLICE-003 orders: covered 2/2, written 1 file, merged \
             m:SLICE-003\n  wave 2: 1 slice verified at 9f8e7d6c\n    SLICE-002 sessions: covered \
             1/1, written 1 file, merged m:SLICE-002\n  labelled emery/{id} at {HEAD}\n"
        )
    );
    provider.vcs.assert_exhausted();
}

// `--jobs` caps how many slices build at once and nothing else: a cap of
// one dispatches the wave's slices one after another over the same head,
// and the exchange and the history are the wide run's. A cap of zero is a
// usage error.
#[tokio::test]
async fn build_jobs_one() {
    let (provider, id) = wide();
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    cli_ok(&provider, &["emery", "build", BUILDER, "--jobs", "1"]).await;

    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-003".to_owned(), BASE.to_owned(), worktree("SLICE-003")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ]
    );
    let mut expected = vec![
        "pending .".to_owned(),
        "head .".to_owned(),
        format!("resolve . emery/{id}"),
        format!("add . {INTEGRATION} {BASE}"),
    ];
    expected.extend(wave(".", &id, BASE, &[AUTHENTICATION, WIDE_ORDERS]));
    expected.extend(wave(".", &id, HEAD, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected, "the same history as a wide run");

    let response = cli(&provider, &["emery", "build", BUILDER, "-j", "0"]).await;
    assert_eq!(response.exit, USAGE_EXIT, "{}", String::from_utf8_lossy(&response.stderr));
    assert_eq!(provider.target.calls().len(), 3, "a usage error dispatches nothing");
}

// The merge rules the target declares ride every merge, in the order declared.
#[tokio::test]
async fn build_merge_rules() {
    let (mut provider, _) = planned();
    provider.target.rules = vec![
        MergeRule {
            paths: Cow::Borrowed("src/*/mod.rs"),
            strategy: MergeStrategy::Union,
        },
        MergeRule {
            paths: Cow::Borrowed("Cargo.lock"),
            strategy: MergeStrategy::Ours,
        },
    ];

    cli_ok(&provider, &["emery", "build", BUILDER]).await;

    let policies: Vec<Vec<Rule>> =
        provider.vcs.merged().into_iter().map(|(_, policy)| policy).collect();
    let expected = vec![
        Rule {
            paths: "src/*/mod.rs".to_owned(),
            strategy: Strategy::Union,
        },
        Rule {
            paths: "Cargo.lock".to_owned(),
            strategy: Strategy::Ours,
        },
    ];
    assert_eq!(policies, [expected.clone(), expected]);
}

// A slice whose merge conflicts is left unmerged, its working copy removed,
// and built again in the next wave over the verified head, beside whatever
// else is ready then; its merge then lands and the paths it conflicted at
// are recorded.
#[tokio::test]
async fn build_conflict_rebuilt() {
    let (provider, id) = wide();
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    provider.vcs.merges.script(
        INTEGRATION,
        Ok(Merged {
            commit: Some("m:SLICE-001".to_owned()),
            conflicts: Vec::new(),
        }),
    );
    provider.vcs.merges.script(INTEGRATION, Ok(conflicted(&["src/orders.rs", "src/lib.rs"])));

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-003".to_owned(), BASE.to_owned(), worktree("SLICE-003")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
            ("SLICE-003".to_owned(), HEAD.to_owned(), worktree("SLICE-003")),
        ],
        "the conflicted slice is built again over the verified head"
    );
    let mut expected = vec![
        "pending .".to_owned(),
        "head .".to_owned(),
        format!("resolve . emery/{id}"),
        format!("add . {INTEGRATION} {BASE}"),
    ];
    expected.extend(wave(".", &id, BASE, &[AUTHENTICATION, WIDE_ORDERS]));
    expected.extend(wave(".", &id, HEAD, &[WIDE_SESSIONS, WIDE_ORDERS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected);
    assert_eq!(
        provider.vcs.messages(),
        [
            message(&id, AUTHENTICATION, "REQ-001", "REQ-001", BASE, 1),
            message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", BASE, 1),
            message(&id, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2),
            message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 2),
        ],
        "the second build is sealed under the wave it merged in"
    );

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["slices"],
        json!([
            {"id": "SLICE-001", "name": "authentication", "wave": 1, "covered": ["REQ-001"], "uncovered": [], "written": ["src/authentication.rs"], "commit": "m:SLICE-001"},
            {"id": "SLICE-002", "name": "sessions", "wave": 2, "covered": ["REQ-002"], "uncovered": [], "written": ["src/sessions.rs"], "commit": "m:SLICE-002"},
            {"id": "SLICE-003", "name": "orders", "wave": 2, "covered": ["REQ-003", "REQ-004"], "uncovered": [], "written": ["src/orders.rs"], "commit": "m:SLICE-003", "conflicts": ["src/orders.rs", "src/lib.rs"]},
        ]),
        "{envelope}"
    );

    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    provider.vcs.merges.script(
        INTEGRATION,
        Ok(Merged {
            commit: Some("m:SLICE-001".to_owned()),
            conflicts: Vec::new(),
        }),
    );
    provider.vcs.merges.script(INTEGRATION, Ok(conflicted(&["src/orders.rs", "src/lib.rs"])));
    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 3 slices in 2 waves, widest 2\n  base {BASE}\n  wave 1: \
             1 slice verified at 9f8e7d6c\n    SLICE-001 authentication: covered 1/1, written 1 \
             file, merged m:SLICE-001\n  wave 2: 2 slices verified at 9f8e7d6c\n    SLICE-002 \
             sessions: covered 1/1, written 1 file, merged m:SLICE-002\n    SLICE-003 orders: \
             covered 2/2, written 1 file, merged m:SLICE-003, conflicted (src/orders.rs, \
             src/lib.rs)\n  labelled emery/{id} at {HEAD}\n"
        )
    );
    provider.vcs.assert_exhausted();
}

// A slice that conflicts on its second build ends the run as
// `slice-conflict`, naming the slice and the paths; the label stays where
// the last verified wave left it and the integration working copy stays
// for inspection.
#[tokio::test]
async fn build_conflict_persists() {
    let (provider, id) = wide();
    for answer in [
        Ok(Merged {
            commit: Some("m:SLICE-001".to_owned()),
            conflicts: Vec::new(),
        }),
        Ok(conflicted(&["src/orders.rs"])),
        Ok(Merged {
            commit: Some("m:SLICE-002".to_owned()),
            conflicts: Vec::new(),
        }),
        Ok(conflicted(&["src/orders.rs"])),
    ] {
        provider.vcs.merges.script(INTEGRATION, answer);
    }

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "slice-conflict").await;

    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-003` (orders) failed in wave 2; SLICE-001, SLICE-002 merged before it \
             stay committed in `{INTEGRATION}`: its merge conflicts at src/orders.rs on its build 2"
        ),
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("merge rule")),
        "{envelope}"
    );
    let calls = provider.vcs.calls();
    let labels: Vec<&String> = calls.iter().filter(|call| call.starts_with("label ")).collect();
    assert_eq!(
        labels,
        [&format!("label . emery/{id} {HEAD}")],
        "labelled after the first wave alone"
    );
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");
    assert_eq!(
        calls.iter().filter(|call| **call == format!("remove {}", worktree("SLICE-003"))).count(),
        2,
        "the slice's working copy goes after each conflict: {calls:?}"
    );
    assert_eq!(provider.target.verifies(), [INTEGRATION], "the second wave is never verified");
    provider.vcs.assert_exhausted();
}

// A wave the adapter does not verify ends the run as `verify-failed`,
// carrying each failing check; the label is not set after a first-wave
// failure and stays at the last verified head after a later one, and the
// integration working copy stays for inspection.
#[tokio::test]
async fn build_verify_failed() {
    let (provider, id) = planned();
    provider.target.verdict(Ok(verdict(false, &["cargo test: 1 failed", "clippy: 2 warnings"])));

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "verify-failed").await;

    assert_message(
        &envelope,
        &format!(
            "wave 1 failed; SLICE-001 merged in it stays committed in `{INTEGRATION}`; \
             `emery/{id}` is not set: verification failed:\n- cargo test: 1 failed\n- clippy: 2 \
             warnings"
        ),
    );
    assert!(envelope["hint"].as_str().is_some_and(|hint| hint.contains("bisect")), "{envelope}");
    assert_eq!(provider.target.calls().len(), 1, "the second wave is never built");
    assert_eq!(provider.target.verifies(), [INTEGRATION]);
    let calls = provider.vcs.calls();
    assert!(!calls.iter().any(|call| call.starts_with("label ")), "no label: {calls:?}");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");

    // a later wave's failure leaves the label at the last verified head
    provider.target.verdict(Ok(verdict(true, &[])));
    provider.target.verdict(Ok(verdict(false, &["cargo test: 3 failed"])));
    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "verify-failed").await;
    assert_message(
        &envelope,
        &format!(
            "wave 2 failed; SLICE-002 merged in it stays committed in `{INTEGRATION}`; \
             `emery/{id}` stays at `{HEAD}`: verification failed:\n- cargo test: 3 failed"
        ),
    );
    let calls = provider.vcs.calls();
    assert_eq!(calls.iter().filter(|call| call.starts_with("label ")).count(), 1, "{calls:?}");
}

// What a verify's checks leave behind in the integrated tree is sealed as
// the wave's own commit before the head is labelled, so the label points at
// a sealed tree.
#[tokio::test]
async fn build_verify_by_products() {
    let (provider, id) = planned();
    provider.vcs.pending.script(
        INTEGRATION,
        Ok(vec![
            change("target/report.txt", ChangeKind::Added),
            change("Cargo.lock", ChangeKind::Modified),
        ]),
    );

    cli_ok(&provider, &["emery", "build", BUILDER]).await;

    let calls = provider.vcs.calls();
    let pending =
        calls.iter().position(|call| call == &format!("pending {INTEGRATION}")).expect("a check");
    assert_eq!(
        calls[pending..pending + 4],
        [
            format!("pending {INTEGRATION}"),
            format!("commit {INTEGRATION} Wave 1 verified"),
            format!("head {INTEGRATION}"),
            format!("label . emery/{id} {HEAD}"),
        ],
        "{calls:?}"
    );
    assert_eq!(
        provider.vcs.messages()[1],
        format!("Wave 1 verified\n\nRevision: {id}\nAdapter: {BUILDER}\nSlices: SLICE-001")
    );
    assert_eq!(provider.vcs.messages().len(), 3, "the two slices' commits and the wave's");
    provider.vcs.assert_exhausted();
}

// A verdict the gate refuses is the adapter's defect, named; a verify the
// adapter fails keeps its class and code, named for the wave.
#[tokio::test]
async fn build_bad_verdict() {
    let (provider, _) = planned();
    provider.target.verdict(Ok(verdict(true, &["cargo test: 1 failed"])));

    let envelope = fail(&provider, &["emery", "build", BUILDER], 3, "server_error").await;
    assert_message(&envelope, "`acme:builder` returned an invalid verdict");
    assert_message(&envelope, "- `passed` is true, yet 1 failure is listed");
    assert!(!provider.vcs.calls().iter().any(|call| call.starts_with("label ")));

    provider.target.verdict(Err(bad_gateway!("the model timed out")));
    let envelope = fail(&provider, &["emery", "build", BUILDER], 4, "bad_gateway").await;
    assert_message(&envelope, "wave 1 failed; SLICE-001 merged in it stays committed in");
    assert_message(&envelope, ": the model timed out");
}

// A label the repository holds for the revision is resumed: the slices its
// history records are not built again, and the rest build over the labelled
// head. A commit a build did not seal, or one sealed for another revision,
// records nothing.
#[tokio::test]
async fn build_resumed() {
    let (provider, id) = wide();
    provider.vcs.resolves.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry("w1", &format!("Wave 1 verified\n\nRevision: {id}\nAdapter: {BUILDER}\nSlices: SLICE-001, SLICE-003")),
            entry("m:SLICE-003", &message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1)),
            entry("m:SLICE-001", &message(&id, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
            entry("m:SLICE-002", &message("other-revision", WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2)),
            entry("c0ffee", "Merge branch 'feature'"),
        ]),
    );

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    assert_eq!(
        dispatched(&provider),
        [("SLICE-002".to_owned(), LABELLED.to_owned(), worktree("SLICE-002"))],
        "the labelled slices are not built again; the rest build over the labelled head"
    );
    let mut expected = vec![
        "pending .".to_owned(),
        "head .".to_owned(),
        format!("resolve . emery/{id}"),
        format!("log . {LABELLED} {HEAD}"),
        format!("add . {INTEGRATION} {LABELLED}"),
    ];
    expected.extend(wave(".", &id, LABELLED, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected);
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["resumed"], json!(["SLICE-001", "SLICE-003"]), "{envelope}");
    assert_eq!(envelope["slices"].as_array().map(Vec::len), Some(1), "{envelope}");
    assert_eq!(envelope["slices"][0]["id"], "SLICE-002", "{envelope}");
    assert_eq!(envelope["slices"][0]["wave"], 1, "{envelope}");
    assert_eq!(envelope["head"], HEAD, "{envelope}");

    provider.vcs.resolves.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry(
                "m:SLICE-003",
                &message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
            ),
            entry("m:SLICE-001", &message(&id, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
        ]),
    );
    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 3 slices in 2 waves, widest 2\n  base {HEAD}\n  resumed: \
             SLICE-001, SLICE-003\n  wave 1: 1 slice verified at 9f8e7d6c\n    SLICE-002 sessions: \
             covered 1/1, written 1 file, merged m:SLICE-002\n  labelled emery/{id} at {HEAD}\n"
        )
    );
    provider.vcs.assert_exhausted();
}

// A label recording every slice leaves nothing to build: no slice is
// dispatched, no wave verified, the label untouched, and the run succeeds
// reporting the labelled head.
#[tokio::test]
async fn build_resumed_complete() {
    let (provider, id) = planned();
    provider.vcs.resolves.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry("m:SLICE-002", &message(&id, ORDERS, "REQ-003, REQ-004", "REQ-003", HEAD, 2)),
            entry(
                "m:SLICE-001",
                &message(&id, AUTHENTICATION, "REQ-001, REQ-002", "REQ-001, REQ-002", HEAD, 1),
            ),
        ]),
    );

    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 2 slices in 2 waves, widest 1\n  base {HEAD}\n  resumed: \
             SLICE-001, SLICE-002\n  labelled emery/{id} at {LABELLED}\n"
        )
    );
    assert!(provider.target.calls().is_empty(), "nothing to build");
    assert!(provider.target.verifies().is_empty(), "nothing to verify");
    assert_eq!(
        provider.vcs.calls(),
        [
            "pending .".to_owned(),
            "head .".to_owned(),
            format!("resolve . emery/{id}"),
            format!("log . {LABELLED} {HEAD}"),
            format!("add . {INTEGRATION} {LABELLED}"),
            format!("remove {INTEGRATION}"),
        ]
    );
    provider.vcs.assert_exhausted();
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

// A `[target] repository` builds into a clone of it, from its branch's
// commit: cloned when the run has none, fetched when it has one, the label
// set in the clone and pushed nowhere.
#[tokio::test]
async fn build_repository() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\nbranch = \"main\"\n"
    ));
    let (provider, id) = planned();

    let resp = cli_ok(&provider, &["emery", "build", "--config", &config]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("\n  base main-commit\n"), "{stdout}");
    assert!(stdout.ends_with(&format!("  labelled emery/{id} at {HEAD}\n")), "{stdout}");
    let mut expected =
        vec![format!("clone {REMOTE_URL} {REMOTE_CLONE}"), format!("resolve {REMOTE_CLONE} main")];
    expected.extend(integrated(REMOTE_CLONE, "main-commit", &id));
    assert_eq!(provider.vcs.calls(), expected, "cloned, then built from the branch");
    let roots: Vec<String> = provider.target.calls().into_iter().map(|(_, _, root)| root).collect();
    assert_eq!(roots, [worktree("SLICE-001"), worktree("SLICE-002")]);

    // the next run finds the clone and fetches
    provider.vcs.clones.script(REMOTE_CLONE, Err(Error::Exists(REMOTE_CLONE.to_owned())));
    cli_ok(&provider, &["emery", "build", "--config", &config]).await;
    let calls = provider.vcs.calls();
    assert_eq!(calls[expected.len()], format!("clone {REMOTE_URL} {REMOTE_CLONE}"));
    assert_eq!(calls[expected.len() + 1], format!("fetch {REMOTE_CLONE} origin"));
    assert_eq!(calls[expected.len() + 2], format!("resolve {REMOTE_CLONE} main"));
    provider.vcs.assert_exhausted();
}

// A `[target] remote` pushes the label once every wave is verified, to the
// repository's remote, whether the repository is the project's or a clone.
#[tokio::test]
async fn build_pushed() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\nbranch = \
         \"main\"\nremote = \"origin\"\n"
    ));
    let (provider, id) = planned();

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", &config]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["pushed"], "origin", "{envelope}");
    let calls = provider.vcs.calls();
    let label = calls.iter().rposition(|call| call.starts_with("label ")).expect("a label");
    assert_eq!(
        calls[label..],
        [
            format!("label {REMOTE_CLONE} emery/{id} {HEAD}"),
            format!("push {REMOTE_CLONE} origin emery/{id}"),
            format!("remove {INTEGRATION}"),
        ],
        "pushed once after the last wave's label, before the working copy goes"
    );
    assert_eq!(calls.iter().filter(|call| call.starts_with("push ")).count(), 1);

    let resp = cli_ok(&provider, &["emery", "build", "--config", &config]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        stdout.ends_with(&format!("  labelled emery/{id} at {HEAD}\n  pushed to origin\n")),
        "{stdout}"
    );
}

// A `remote` alone pushes the project repository's label: a greenfield
// project whose checkout has a remote and no `[target] repository`.
#[tokio::test]
async fn build_remote_greenfield() {
    let scratch = Scratch::new();
    let config =
        scratch.config(&format!("[target]\nadapter = \"{BUILDER}\"\nremote = \"origin\"\n"));
    let (provider, id) = planned();

    cli_ok(&provider, &["emery", "build", "--config", &config]).await;

    let calls = provider.vcs.calls();
    assert_eq!(calls[0], "pending .");
    assert!(calls.contains(&format!("push . origin emery/{id}")), "{calls:?}");
    assert!(!calls.iter().any(|call| call.starts_with("fetch ")), "no clone: {calls:?}");

    // a remote the repository lacks is the operator's to name
    provider.vcs.pushes.script(".", Err(Error::NotFound("origin".to_owned())));
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 2, "revision-not-found").await;
    assert_message(&envelope, "the project repository has no `origin`");
    assert!(envelope["hint"].as_str().is_some_and(|hint| hint.contains("`remote`")), "{envelope}");
}

// A slice that changed nothing seals no commit and merges nothing, and the
// build goes on; the wave is still verified and the label still lands.
#[tokio::test]
async fn build_nothing_changed() {
    let (mut provider, id) = planned();
    provider
        .target
        .reports
        .insert("SLICE-001".to_string(), Ok(report(&["REQ-001", "REQ-002"], &[])));
    provider.vcs.commits.script(&worktree("SLICE-001"), Ok(None));

    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        stdout.contains(
            "  wave 1: 1 slice verified at 9f8e7d6c\n    SLICE-001 authentication: covered 2/2, \
             written 0 files, nothing to merge\n  wave 2: 1 slice verified at 9f8e7d6c\n    \
             SLICE-002 orders: covered 2/2, written 1 file, merged m:SLICE-002\n"
        ),
        "{stdout}"
    );
    assert!(stdout.ends_with(&format!("  labelled emery/{id} at {HEAD}\n")), "{stdout}");
    assert_eq!(provider.vcs.merged().len(), 1, "nothing to merge for the first slice");

    provider.vcs.commits.script(&worktree("SLICE-001"), Ok(None));
    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["slices"][0]["commit"], Value::Null, "{envelope}");
    assert_eq!(envelope["slices"][1]["commit"], "m:SLICE-002", "{envelope}");
    provider.vcs.assert_exhausted();
}

// The working copy a failed run left is removed and cut again, so a build
// never resumes from what an earlier one wrote outside the label.
#[tokio::test]
async fn build_stale_integration() {
    let (provider, _) = planned();
    provider.vcs.adds.script(INTEGRATION, Err(Error::Exists(INTEGRATION.to_owned())));
    provider.vcs.adds.script(&worktree("SLICE-001"), Err(Error::Exists(worktree("SLICE-001"))));

    cli_ok(&provider, &["emery", "build", BUILDER]).await;

    let calls = provider.vcs.calls();
    assert_eq!(
        calls[3..9],
        [
            format!("add . {INTEGRATION} {HEAD}"),
            format!("remove {INTEGRATION}"),
            format!("add . {INTEGRATION} {HEAD}"),
            format!("add . {} {HEAD}", worktree("SLICE-001")),
            format!("remove {}", worktree("SLICE-001")),
            format!("add . {} {HEAD}", worktree("SLICE-001")),
        ],
        "{calls:?}"
    );
    assert_eq!(provider.target.calls().len(), 2);
    provider.vcs.assert_exhausted();
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
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
}

// A repository is built from a branch, so one without a `branch`, or a
// `branch` without a repository, is refused at the file.
#[tokio::test]
async fn build_config() {
    let scratch = Scratch::new();
    let (provider, _) = planned();

    let config = scratch
        .config(&format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"));
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "bad_request").await;
    assert_message(&envelope, "the target sets `repository` without `branch`");

    let config = scratch.config(&format!("[target]\nadapter = \"{BUILDER}\"\nbranch = \"main\"\n"));
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "bad_request").await;
    assert_message(&envelope, "the target sets `branch` without `repository`");

    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
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
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
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
// any slice is dispatched and before the base is settled.
#[tokio::test]
async fn build_unsupported_version() {
    let (mut provider, _) = planned();
    provider.target.versions.insert(BUILDER_ID.to_string(), "99.0.0".to_string());

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "unsupported-version").await;
    assert_message(&envelope, "adapter `acme:builder` requires emery 99.0.0 or newer");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
}

// A checkout with pending changes is no base: every path is named, those
// beneath the engine's own `.emery/` set aside, and no slice is dispatched.
#[tokio::test]
async fn build_base_not_sealed() {
    let (provider, _) = planned();
    provider.vcs.pending.script(
        ".",
        Ok(vec![
            change("src/main.rs", ChangeKind::Modified),
            change(".emery/storage/revision.json", ChangeKind::Added),
            change("notes.md", ChangeKind::Added),
            change(".git/index", ChangeKind::Modified),
        ]),
    );

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "base-not-sealed").await;
    assert_message(&envelope, "the project checkout holds pending changes: src/main.rs, notes.md");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("`.gitignore`")),
        "{envelope}"
    );
    assert_eq!(provider.vcs.calls(), ["pending ."], "nothing past the check");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");

    // pending changes beneath `.emery/` alone leave the base sealed
    provider
        .vcs
        .pending
        .script(".", Ok(vec![change(".emery/storage/revision.json", ChangeKind::Added)]));
    cli_ok(&provider, &["emery", "build", BUILDER]).await;
    assert_eq!(provider.target.calls().len(), 2);

    // a repository with no commit yet has no base either
    provider.vcs.heads.script(".", Err(Error::NotFound("HEAD".to_owned())));
    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "base-not-sealed").await;
    assert_message(&envelope, "the project repository has no commit to build from");
    provider.vcs.assert_exhausted();
}

// A build lands as a commit, so a project directory that is no repository
// is refused before any slice, with the way out.
#[tokio::test]
async fn build_not_repository() {
    let (provider, _) = planned();
    provider.vcs.pending.script(".", Err(Error::NotARepository));

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "repository-required").await;
    assert_message(&envelope, "the project directory is not a repository");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("[target] repository")),
        "{envelope}"
    );
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
}

// A branch the repository lacks, or a repository the URL does not name, is
// `revision-not-found`; a remote that cannot be reached is the gateway's.
#[tokio::test]
async fn build_repository_missing() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\nbranch = \"release\"\n"
    ));
    let (provider, _) = planned();

    provider.vcs.resolves.script(REMOTE_CLONE, Err(Error::NotFound("release".to_owned())));
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 2, "revision-not-found").await;
    assert_message(&envelope, &format!("repository `{REMOTE_URL}` has no `release`"));
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");

    provider.vcs.clones.script(
        REMOTE_CLONE,
        Err(Error::Access("could not read from remote repository".to_owned())),
    );
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 4, "bad_gateway").await;
    assert_message(&envelope, &format!("repository `{REMOTE_URL}`: could not read"));
    provider.vcs.assert_exhausted();
}

// A stored plan whose slices wait on one another can be built from no wave:
// the engine names them and writes nothing.
#[tokio::test]
async fn build_cycle() {
    let mut plan: Value = serde_json::from_slice(PLAN).expect("the plan fixture");
    plan["slices"][0]["depends-on"] = json!(["SLICE-002"]);
    let provider = Provider::idle();
    seed(&provider.storage, SPEC, DESIGN, &serde_json::to_vec(&plan).expect("a plan"));

    let envelope = fail(&provider, &["emery", "build", BUILDER], 3, "server_error").await;

    assert_message(&envelope, "the slices left to build wait on one another: SLICE-001, SLICE-002");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
}

// A target's refusal of a slice keeps its class and code, named for the
// slice and its wave; the slices after it are never dispatched, no label is
// set, and the working copies stay for inspection.
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
        "slice `SLICE-001` (authentication) failed in wave 1: the tree already holds `src/`",
    );
    assert!(!envelope["message"].as_str().unwrap_or("").contains("stay committed"));
    assert_eq!(provider.target.calls().len(), 1, "the run stops at the first");
    let calls = provider.vcs.calls();
    assert_eq!(
        calls.last().cloned(),
        Some(format!("add . {} {HEAD}", worktree("SLICE-001"))),
        "{calls:?}"
    );
    assert!(provider.target.verifies().is_empty(), "a wave that did not build is not verified");
}

// A report the gate refuses is the adapter's defect: every rule it breaks is
// named, the run stops there, and nothing of the slice is sealed.
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
    assert!(provider.vcs.messages().is_empty(), "no commit for a refused report");
}

// A failure after a wave was verified names what stays merged, and where;
// the label stays where that wave left it.
#[tokio::test]
async fn build_stops_at_failure() {
    let (mut provider, id) = planned();
    provider
        .target
        .reports
        .insert("SLICE-002".to_string(), Err(bad_gateway!("the model timed out")));

    let envelope = fail(&provider, &["emery", "build", BUILDER], 4, "bad_gateway").await;
    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-002` (orders) failed in wave 2; SLICE-001 merged before it stays \
             committed in `{INTEGRATION}`: the model timed out"
        ),
    );
    assert_eq!(provider.target.calls().len(), 2);
    assert_eq!(provider.vcs.messages().len(), 1, "the first slice's commit stands");
    let calls = provider.vcs.calls();
    assert!(calls.contains(&format!("label . emery/{id} {HEAD}")), "the first wave's label stands");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection");
}
