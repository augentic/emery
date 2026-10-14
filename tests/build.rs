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

use emery_adapter::target::{Base, MergeRule, MergeStrategy, Report, Verdict};
use emery_engine::EntryKind;
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
// The target's repository, under its name: `builder`, the adapter's, when
// the config gives none. A clone of `REMOTE_URL`, or the one the engine lays.
const TARGET_REPO: &str = "./.emery/vcs/repos/builder";
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

// Scripts what the next build of `slice` leaves changed in its working copy.
fn changed(provider: &Provider, slice: &str, paths: &[(&str, ChangeKind)]) {
    provider.vcs.pending.script(
        &worktree(slice),
        Ok(paths.iter().map(|(path, kind)| change(path, *kind)).collect()),
    );
}

// Scripts one added file, `src/<name>.rs`, as what the next build of a slice leaves.
fn wrote(provider: &Provider, (slice, name): (&str, &str)) {
    changed(provider, slice, &[(&format!("src/{name}.rs"), ChangeKind::Added)]);
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
        vec![format!("labelled {repo} emery/{id}"), format!("add {repo} {INTEGRATION} {base}")];
    calls.extend(wave(repo, id, base, &[AUTHENTICATION]));
    calls.extend(wave(repo, id, HEAD, &[ORDERS]));
    calls.push(format!("remove {INTEGRATION}"));
    calls
}

// One wave's exchange over `repo`: every slice's working copy cut at `head`,
// then each found where it was cut, its changes read, sealed, merged, and
// removed in id order, then the integrated head verified and labelled.
fn wave(repo: &str, id: &str, head: &str, slices: &[(&str, &str)]) -> Vec<String> {
    let mut calls: Vec<String> =
        slices.iter().map(|(slice, _)| format!("add {repo} {} {head}", worktree(slice))).collect();
    for (slice, name) in slices {
        calls.push(format!("head {}", worktree(slice)));
        calls.push(format!("pending {}", worktree(slice)));
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
// head; what each changed in its copy, as the copy holds it rather than as
// the report lists it, is sealed as its commit and merged into the
// integration working copy, each wave verified and labelled, and the
// engine's own state left as it was.
#[tokio::test]
async fn build_plan() {
    let (mut provider, id) = planned();
    provider.target.reports.insert(
        "SLICE-002".to_string(),
        Ok(report(&["REQ-003"], &["src/orders.rs", "Cargo.toml"])),
    );
    let orders_changed = [
        ("src/orders.rs", ChangeKind::Added),
        ("Cargo.toml", ChangeKind::Modified),
        ("src/lib.rs", ChangeKind::Modified),
    ];
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    wrote(&provider, AUTHENTICATION);
    changed(&provider, "SLICE-002", &orders_changed);
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
             orders: covered 1/2 (uncovered REQ-004), written 3 files, merged m:SLICE-002\n  \
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
    wrote(&provider, AUTHENTICATION);
    changed(&provider, "SLICE-002", &orders_changed);
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
            {"id": "SLICE-002", "name": "orders", "wave": 2, "covered": ["REQ-003"], "uncovered": ["REQ-004"], "written": ["Cargo.toml", "src/lib.rs", "src/orders.rs"], "commit": "m:SLICE-002"},
        ]),
        "`written` is the copy's change set, sorted: {envelope}"
    );
    assert_eq!(envelope["verified"], json!([HEAD, HEAD]), "{envelope}");
    assert!(envelope.get("repaired").is_none(), "no verify changed the tree: {envelope}");
    assert_eq!(envelope["head"], HEAD, "{envelope}");
    assert_eq!(envelope["label"], format!("emery/{id}"), "{envelope}");
    assert_eq!(envelope["repository"], ".", "the project's own: {envelope}");

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
    for slice in [AUTHENTICATION, WIDE_ORDERS, WIDE_SESSIONS] {
        wrote(&provider, slice);
    }

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
        format!("labelled . emery/{id}"),
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
    for slice in [AUTHENTICATION, WIDE_ORDERS, WIDE_SESSIONS] {
        wrote(&provider, slice);
    }
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
        format!("labelled . emery/{id}"),
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
    for slice in [AUTHENTICATION, WIDE_ORDERS, WIDE_SESSIONS, WIDE_ORDERS] {
        wrote(&provider, slice);
    }

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
        format!("labelled . emery/{id}"),
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
    for slice in [AUTHENTICATION, WIDE_ORDERS, WIDE_SESSIONS, WIDE_ORDERS] {
        wrote(&provider, slice);
    }
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

// A slice that conflicts on its third build ends the run as
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
        Ok(conflicted(&["src/orders.rs"])),
    ] {
        provider.vcs.merges.script(INTEGRATION, answer);
    }

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "slice-conflict").await;

    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-003` (orders) failed in wave 3; SLICE-001, SLICE-002 merged before it \
             stay committed in `{INTEGRATION}`: its merge conflicts at src/orders.rs on its build 3"
        ),
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("merge rule")),
        "{envelope}"
    );
    assert_eq!(
        dispatched(&provider).iter().filter(|(slice, ..)| slice == "SLICE-003").count(),
        3,
        "the slice is built once per wave until its last attempt"
    );
    let calls = provider.vcs.calls();
    let labels: Vec<&String> = calls.iter().filter(|call| call.starts_with("label ")).collect();
    assert_eq!(
        labels,
        [&format!("label . emery/{id} {HEAD}"), &format!("label . emery/{id} {HEAD}")],
        "labelled after the two waves that merged a slice"
    );
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");
    assert_eq!(
        calls.iter().filter(|call| **call == format!("remove {}", worktree("SLICE-003"))).count(),
        3,
        "the slice's working copy goes after each conflict: {calls:?}"
    );
    assert_eq!(
        provider.target.verifies(),
        [INTEGRATION, INTEGRATION],
        "the third wave is never verified"
    );
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

// What a verify's repairs and checks leave in the integrated tree is sealed
// as the wave's own commit before the head is labelled, so the label points
// at a sealed tree; the seal is reported with its paths, and the wave as
// repaired when one of them lies outside the adapter's `ours` rules, since
// a lockfile a check regenerated under one is no repair.
#[tokio::test]
async fn build_verify_repairs() {
    let (mut provider, id) = planned();
    provider.target.rules = vec![MergeRule {
        paths: Cow::Borrowed("Cargo.lock"),
        strategy: MergeStrategy::Ours,
    }];
    let repaired = || {
        Ok(vec![
            change("src/orders.rs", ChangeKind::Modified),
            change("Cargo.lock", ChangeKind::Modified),
        ])
    };
    provider.vcs.pending.script(INTEGRATION, repaired());

    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        stdout.contains(
            "  wave 1: 1 slice verified at 9f8e7d6c, repaired\n    SLICE-001 authentication: \
             covered 2/2, written 0 files, merged m:SLICE-001\n  wave 2: 1 slice verified at \
             9f8e7d6c\n"
        ),
        "{stdout}"
    );
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

    provider.vcs.pending.script(INTEGRATION, repaired());
    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["repaired"], json!([1]), "{envelope}");
    assert_eq!(
        envelope["sealed"],
        json!([{"wave": 1, "commit": "Wave-commit", "paths": ["Cargo.lock", "src/orders.rs"]}]),
        "{envelope}"
    );
    assert_eq!(envelope["verified"], json!([HEAD, HEAD]), "{envelope}");

    // a seal of regenerated paths alone is reported, and is no repair
    provider.vcs.pending.script(
        INTEGRATION,
        Ok(vec![
            change("Cargo.lock", ChangeKind::Modified),
            change("crates/orders/Cargo.lock", ChangeKind::Added),
        ]),
    );
    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("  wave 1: 1 slice verified at 9f8e7d6c, sealed\n"), "{stdout}");
    provider.vcs.pending.script(INTEGRATION, Ok(vec![change("Cargo.lock", ChangeKind::Modified)]));
    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("repaired").is_none(), "{envelope}");
    assert_eq!(envelope["sealed"][0]["paths"], json!(["Cargo.lock"]), "{envelope}");
    assert_eq!(
        provider.vcs.messages().len(),
        3 * 4,
        "the two slices' commits and the wave's, each run"
    );
    provider.vcs.assert_exhausted();
}

// A verify that fails upstream is put once more over the same integration
// copy, since nothing of it is sealed until the verdict; the run lands.
#[tokio::test]
async fn build_verify_retries_bad_gateway() {
    let (provider, id) = planned();
    provider.target.verdict(Err(bad_gateway!("the model timed out")));

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["label"], json!(format!("emery/{id}")), "{envelope}");
    assert_eq!(envelope["verified"], json!([HEAD, HEAD]), "{envelope}");
    assert_eq!(
        provider.target.verifies(),
        [INTEGRATION, INTEGRATION, INTEGRATION],
        "the first wave verified twice, the second once"
    );
    let calls = provider.vcs.calls();
    assert_eq!(calls.iter().filter(|call| call.starts_with("label ")).count(), 2, "{calls:?}");
    assert_eq!(
        calls.iter().filter(|call| call.starts_with(&format!("add . {INTEGRATION}"))).count(),
        1,
        "the copy is cut once: {calls:?}"
    );
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

    // the second upstream failure is the run's
    provider.target.verdict(Err(bad_gateway!("the model timed out")));
    provider.target.verdict(Err(bad_gateway!("the model timed out again")));
    let envelope = fail(&provider, &["emery", "build", BUILDER], 4, "bad_gateway").await;
    assert_message(&envelope, "wave 1 failed; SLICE-001 merged in it stays committed in");
    assert_message(&envelope, ": the model timed out again");
    assert_eq!(provider.target.verifies().len(), 3, "one for the first run, two for the second");
}

// A label the repository holds for the revision is resumed once it is read
// as a label and descends from the base: the slices its history records are
// not built again, and the rest build over the labelled head. A commit a
// build did not seal, or one sealed for another revision, records nothing.
#[tokio::test]
async fn build_resumed() {
    let (provider, id) = wide();
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
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
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
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

    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
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
    wrote(&provider, WIDE_SESSIONS);
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
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
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
            format!("labelled . emery/{id}"),
            format!("descends . {HEAD} {LABELLED}"),
            format!("log . {LABELLED} {HEAD}"),
            format!("add . {INTEGRATION} {LABELLED}"),
            format!("remove {INTEGRATION}"),
        ]
    );
    provider.vcs.assert_exhausted();
}

// A label that does not descend from the base is not this base's history:
// its chain is never read, and the build starts fresh from the base, every
// slice built and the label moved onto the new head.
#[tokio::test]
async fn build_resumed_off_base() {
    let (provider, id) = planned();
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.descends.script(".", Ok(false));

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("resumed").is_none_or(|resumed| resumed == &json!([])), "{envelope}");
    assert_eq!(envelope["base"], HEAD, "{envelope}");
    assert_eq!(envelope["slices"].as_array().map(Vec::len), Some(2), "{envelope}");
    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), HEAD.to_owned(), worktree("SLICE-001")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ],
        "every slice builds, the first over the base"
    );
    let mut expected = vec![
        "pending .".to_owned(),
        "head .".to_owned(),
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
        format!("add . {INTEGRATION} {HEAD}"),
    ];
    expected.extend(wave(".", &id, HEAD, &[AUTHENTICATION]));
    expected.extend(wave(".", &id, HEAD, &[ORDERS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected, "no log over a label off the base");
    provider.vcs.assert_exhausted();
}

// A label over the base whose chain records no slice of this revision is
// still where the build resumes from: every slice builds over the labelled
// head, and nothing is reported resumed.
#[tokio::test]
async fn build_resumed_empty() {
    let (provider, id) = planned();
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry("c0ffee", "Merge branch 'feature'"),
            entry(
                "m:SLICE-001",
                &message("other-revision", AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1),
            ),
        ]),
    );

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("resumed").is_none_or(|resumed| resumed == &json!([])), "{envelope}");
    assert_eq!(envelope["slices"].as_array().map(Vec::len), Some(2), "{envelope}");
    assert_eq!(
        dispatched(&provider)[0],
        ("SLICE-001".to_owned(), LABELLED.to_owned(), worktree("SLICE-001")),
        "the first wave builds over the labelled head"
    );
    let mut expected = vec![
        "pending .".to_owned(),
        "head .".to_owned(),
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
        format!("log . {LABELLED} {HEAD}"),
        format!("add . {INTEGRATION} {LABELLED}"),
    ];
    expected.extend(wave(".", &id, LABELLED, &[AUTHENTICATION]));
    expected.extend(wave(".", &id, HEAD, &[ORDERS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected);
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

// A `[target] repository` builds into a clone of it under the target's
// name, from its default branch as the origin holds it: cloned when the run
// has none, fetched when it has one, the label set in the clone and the
// clone named beside the base.
#[tokio::test]
async fn build_repository() {
    let scratch = Scratch::new();
    let config = scratch
        .config(&format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"));
    let (provider, id) = planned();

    let resp = cli_ok(&provider, &["emery", "build", "--config", &config]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        stdout.contains(&format!("\n  base origin/HEAD-commit\n  repository {TARGET_REPO}\n")),
        "{stdout}"
    );
    assert!(stdout.ends_with(&format!("  labelled emery/{id} at {HEAD}\n")), "{stdout}");
    let mut expected = vec![
        format!("clone {REMOTE_URL} {TARGET_REPO}"),
        format!("resolve {TARGET_REPO} origin/HEAD"),
    ];
    expected.extend(integrated(TARGET_REPO, "origin/HEAD-commit", &id));
    assert_eq!(provider.vcs.calls(), expected, "cloned, then built from the default branch");
    let roots: Vec<String> = provider.target.calls().into_iter().map(|(_, _, root)| root).collect();
    assert_eq!(roots, [worktree("SLICE-001"), worktree("SLICE-002")]);

    // the next run finds the clone and fetches
    provider.vcs.clones.script(TARGET_REPO, Err(Error::Exists(TARGET_REPO.to_owned())));
    cli_ok(&provider, &["emery", "build", "--config", &config]).await;
    let calls = provider.vcs.calls();
    assert_eq!(calls[expected.len()], format!("clone {REMOTE_URL} {TARGET_REPO}"));
    assert_eq!(calls[expected.len() + 1], format!("fetch {TARGET_REPO} origin"));
    assert_eq!(calls[expected.len() + 2], format!("resolve {TARGET_REPO} origin/HEAD"));
    assert!(!calls.iter().any(|call| call.starts_with("push ")), "nothing is pushed: {calls:?}");
    provider.vcs.assert_exhausted();
}

// A `[target] name` is the directory the repository is kept under, cloned
// or laid; one that is not kebab-case is refused before anything is read.
#[tokio::test]
async fn build_named() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[target]\nname = \"shop\"\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"
    ));
    let (mut provider, id) = planned();

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", &config]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["repository"], "./.emery/vcs/repos/shop", "{envelope}");
    assert_eq!(provider.vcs.calls()[0], format!("clone {REMOTE_URL} ./.emery/vcs/repos/shop"));
    assert!(
        provider.vcs.calls().contains(&format!("label ./.emery/vcs/repos/shop emery/{id} {HEAD}"))
    );

    // the engine's own repository is kept under the same name
    provider.target.base = Base::Fresh;
    let config = scratch.config(&format!("[target]\nname = \"shop\"\nadapter = \"{BUILDER}\"\n"));
    cli_ok(&provider, &["emery", "build", "--config", &config]).await;
    assert!(provider.vcs.calls().contains(&"init ./.emery/vcs/repos/shop".to_owned()));

    let config = scratch.config(&format!("[target]\nname = \"Shop\"\nadapter = \"{BUILDER}\"\n"));
    let before = provider.vcs.calls().len();
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "bad_request").await;
    assert_message(&envelope, "target `Shop` is not a kebab-case name");
    assert_eq!(provider.vcs.calls().len(), before, "nothing is read for a refused name");
    provider.vcs.assert_exhausted();
}

// The grammar names the target's repository alone: a `branch` or a `remote`
// in `[target]` is no field of it, refused at the file before anything loads.
#[tokio::test]
async fn build_target_grammar() {
    let scratch = Scratch::new();
    let (provider, _) = planned();

    for (key, value) in [("branch", "main"), ("remote", "origin")] {
        let config = scratch.config(&format!(
            "[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n{key} = \
             \"{value}\"\n"
        ));
        let envelope =
            fail(&provider, &["emery", "build", "--config", &config], 1, "bad_request").await;
        assert_message(&envelope, &format!("unknown field `{key}`"));
    }
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
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
    wrote(&provider, ORDERS);

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

// An adapter that builds into a fresh tree, named with no repository, builds
// into one the engine lays under the target's name: initialised, given a
// `.gitignore` of the engine's own directory, sealed as `Initialise`, and
// built from that commit, the project directory never read. The envelope
// names the repository beside the base.
#[tokio::test]
async fn build_fresh_laid() {
    let (mut provider, id) = planned();
    provider.target.base = Base::Fresh;

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["base"], "Initialise-commit", "{envelope}");
    assert_eq!(envelope["repository"], TARGET_REPO, "{envelope}");
    assert_eq!(
        provider.files.writes(),
        [(TARGET_REPO.to_owned(), ".gitignore".to_owned(), b".emery/\n".to_vec())],
        "the one file the root commit holds"
    );
    assert_eq!(provider.vcs.messages()[0], "Initialise");
    let mut expected =
        vec![format!("init {TARGET_REPO}"), format!("commit {TARGET_REPO} Initialise")];
    expected.extend(integrated(TARGET_REPO, "Initialise-commit", &id));
    assert_eq!(provider.vcs.calls(), expected, "laid, sealed, then built as any repository");
    assert_eq!(
        provider.files.listed(),
        [(INTEGRATION.to_owned(), ".".to_owned())],
        "held to the fresh base as a named one is"
    );
    assert_eq!(provider.target.calls().len(), 2, "every slice builds");
    provider.vcs.assert_exhausted();
}

// The repository the engine laid is kept, so the next run starts from the
// same root commit, writing and sealing nothing; one initialised but never
// sealed, a run that failed between the two, is sealed now.
#[tokio::test]
async fn build_fresh_reused() {
    let (mut provider, id) = planned();
    provider.target.base = Base::Fresh;
    provider.vcs.inits.script(TARGET_REPO, Err(Error::Exists(TARGET_REPO.to_owned())));
    provider.vcs.heads.script(TARGET_REPO, Ok(BASE.to_owned()));

    let resp = cli_ok(&provider, &["emery", "build", BUILDER]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains(&format!("\n  base {BASE}\n  repository {TARGET_REPO}\n")), "{stdout}");
    assert!(provider.files.writes().is_empty(), "nothing laid again");
    let mut expected = vec![format!("init {TARGET_REPO}"), format!("head {TARGET_REPO}")];
    expected.extend(integrated(TARGET_REPO, BASE, &id));
    assert_eq!(provider.vcs.calls(), expected);
    assert!(!provider.vcs.calls().contains(&"pending .".to_owned()), "the project is never read");

    provider.vcs.inits.script(TARGET_REPO, Err(Error::Exists(TARGET_REPO.to_owned())));
    provider.vcs.heads.script(TARGET_REPO, Err(Error::NotFound("HEAD".to_owned())));
    cli_ok(&provider, &["emery", "build", BUILDER]).await;
    let calls = provider.vcs.calls();
    assert_eq!(
        calls[expected.len()..expected.len() + 3],
        [
            format!("init {TARGET_REPO}"),
            format!("head {TARGET_REPO}"),
            format!("commit {TARGET_REPO} Initialise")
        ],
        "an unborn repository is sealed"
    );
    assert_eq!(provider.files.writes().len(), 1);
    provider.vcs.assert_exhausted();
}

// An adapter that builds into a fresh tree is refused a named repository
// whose default branch's root holds a directory: the integration copy is
// cut and read, the directories named outside the engine's own, and no
// slice dispatched.
#[tokio::test]
async fn build_fresh_base_held() {
    let scratch = Scratch::new();
    let config = scratch
        .config(&format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"));
    let (mut provider, id) = planned();
    provider.target.base = Base::Fresh;
    provider.vcs.resolves.script(TARGET_REPO, Ok(BASE.to_owned()));
    provider.files.lists(
        INTEGRATION,
        ".",
        &[
            ("src", EntryKind::Directory),
            ("README.md", EntryKind::File),
            (".git", EntryKind::Directory),
            ("adapters", EntryKind::Directory),
            (".emery", EntryKind::Directory),
            ("Cargo.toml", EntryKind::File),
        ],
    );

    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "base-not-fresh").await;

    assert_message(
        &envelope,
        &format!(
            "`{BUILDER_ID}` builds into a fresh tree, and the base `{BASE}` holds directories: \
             adapters, src"
        ),
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("drop `[target] repository`")),
        "{envelope}"
    );
    assert_eq!(provider.files.listed(), [(INTEGRATION.to_owned(), ".".to_owned())]);
    assert_eq!(
        provider.vcs.calls(),
        [
            format!("clone {REMOTE_URL} {TARGET_REPO}"),
            format!("resolve {TARGET_REPO} origin/HEAD"),
            format!("labelled {TARGET_REPO} emery/{id}"),
            format!("add {TARGET_REPO} {INTEGRATION} {BASE}"),
        ],
        "the copy is cut and left, nothing built"
    );
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
    assert!(provider.target.verifies().is_empty(), "no wave is verified");
    provider.vcs.assert_exhausted();
}

// A named repository whose root holds files alone, or links, beside the
// engine's own directories is fresh enough: the build runs as any other. An
// adapter building over any base never reads the root.
#[tokio::test]
async fn build_fresh_base_files_only() {
    let scratch = Scratch::new();
    let config = scratch
        .config(&format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"));
    let (mut provider, _) = planned();
    provider.target.base = Base::Fresh;
    provider.files.lists(
        INTEGRATION,
        ".",
        &[
            ("README.md", EntryKind::File),
            (".gitignore", EntryKind::File),
            ("LICENSE", EntryKind::Link),
            (".git", EntryKind::Directory),
            (".emery", EntryKind::Directory),
        ],
    );

    cli_ok(&provider, &["emery", "build", "--config", &config]).await;

    assert_eq!(provider.files.listed(), [(INTEGRATION.to_owned(), ".".to_owned())]);
    assert_eq!(provider.target.calls().len(), 2, "every slice builds");

    provider.target.base = Base::Any;
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    provider.files.lists(INTEGRATION, ".", &[("src", EntryKind::Directory)]);
    cli_ok(&provider, &["emery", "build", BUILDER]).await;
    assert_eq!(provider.files.listed().len(), 1, "an adapter over any base reads no root");
    assert_eq!(provider.target.calls().len(), 4);
    provider.vcs.assert_exhausted();
}

// A build resumed from a label over the base was admitted when the label
// was set, so the root is not read again, whatever the integrated tree now
// holds; the label stands in the repository the engine laid.
#[tokio::test]
async fn build_fresh_resumed() {
    let (mut provider, id) = planned();
    provider.target.base = Base::Fresh;
    provider.vcs.inits.script(TARGET_REPO, Err(Error::Exists(TARGET_REPO.to_owned())));
    provider.vcs.heads.script(TARGET_REPO, Ok(BASE.to_owned()));
    provider.vcs.labelleds.script(TARGET_REPO, Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        TARGET_REPO,
        Ok(vec![entry(
            "m:SLICE-001",
            &message(&id, AUTHENTICATION, "REQ-001, REQ-002", "REQ-001, REQ-002", HEAD, 1),
        )]),
    );
    provider.files.lists(INTEGRATION, ".", &[("src", EntryKind::Directory)]);

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["resumed"], json!(["SLICE-001"]), "{envelope}");
    assert!(provider.files.listed().is_empty(), "a resumed build reads no root");
    assert_eq!(
        dispatched(&provider),
        [("SLICE-002".to_owned(), LABELLED.to_owned(), worktree("SLICE-002"))]
    );
    assert!(
        provider.vcs.calls().contains(&format!("descends {TARGET_REPO} {BASE} {LABELLED}")),
        "the label is read in the laid repository"
    );
    provider.vcs.assert_exhausted();
}

// A repository the URL does not name, or one with no default branch, is
// `revision-not-found`; a remote that cannot be reached is the gateway's.
#[tokio::test]
async fn build_repository_missing() {
    let scratch = Scratch::new();
    let config = scratch
        .config(&format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"));
    let (provider, _) = planned();

    provider.vcs.resolves.script(TARGET_REPO, Err(Error::NotFound("origin/HEAD".to_owned())));
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 2, "revision-not-found").await;
    assert_message(&envelope, &format!("repository `{REMOTE_URL}` has no `origin/HEAD`"));
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains(".emery/vcs/repos/<name>")),
        "{envelope}"
    );
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");

    provider.vcs.clones.script(
        TARGET_REPO,
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

// A working copy that no longer sits on the head it was cut at is a build
// that sealed or moved commits of its own, through the shell the model has:
// the slice is refused before anything of it is sealed or merged, both
// commits named, and its working copy stays for inspection.
#[tokio::test]
async fn build_worktree_head_moved() {
    let (provider, _) = planned();
    let moved = "feedface0123456789abcdef0123456789abcdef";
    provider.vcs.heads.script(&worktree("SLICE-001"), Ok(moved.to_owned()));

    let envelope = fail(&provider, &["emery", "build", BUILDER], 3, "server_error").await;

    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-001` (authentication) failed in wave 1: its working copy sits on \
             `{moved}`, not the `{HEAD}` it was cut at: the build sealed or moved commits of its own"
        ),
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    assert!(provider.vcs.merged().is_empty(), "nothing is merged");
    let calls = provider.vcs.calls();
    assert_eq!(calls.last().cloned(), Some(format!("head {}", worktree("SLICE-001"))), "{calls:?}");
    assert!(!calls.contains(&format!("remove {}", worktree("SLICE-001"))), "left for inspection");
    assert_eq!(provider.target.calls().len(), 1, "the run stops at the first");
    assert!(provider.target.verifies().is_empty(), "a wave that did not integrate is not verified");
    provider.vcs.assert_exhausted();
}

// A build that fails upstream is put once more in a fresh working copy,
// the first removed; the second build is the one sealed, under the same
// head, and nothing of the first reaches the history.
#[tokio::test]
async fn build_retries_bad_gateway() {
    let (provider, id) = planned();
    provider.target.fails("SLICE-001", bad_gateway!("the model timed out"));
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    cli_ok(&provider, &["emery", "build", BUILDER]).await;

    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ],
        "the failed build is put again over the same head"
    );
    let mut expected = vec![
        "pending .".to_owned(),
        "head .".to_owned(),
        format!("labelled . emery/{id}"),
        format!("add . {INTEGRATION} {BASE}"),
        format!("add . {} {BASE}", worktree("SLICE-001")),
        format!("remove {}", worktree("SLICE-001")),
    ];
    expected.extend(wave(".", &id, BASE, &[AUTHENTICATION]));
    expected.extend(wave(".", &id, HEAD, &[ORDERS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected, "the first copy goes before the second is cut");
    assert_eq!(provider.vcs.messages().len(), 2, "one commit per slice");
    provider.vcs.assert_exhausted();
}

// A failure after a wave was verified names what stays merged, and where;
// the label stays where that wave left it. A second upstream failure of
// one slice is the run's.
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
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("re-run `emery build`")),
        "the hint names the resume: {envelope}"
    );
    assert_eq!(provider.target.calls().len(), 3, "the failed slice is put twice, no more");
    assert_eq!(provider.vcs.messages().len(), 1, "the first slice's commit stands");
    let calls = provider.vcs.calls();
    assert!(calls.contains(&format!("label . emery/{id} {HEAD}")), "the first wave's label stands");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection");
    assert_eq!(
        calls.iter().filter(|call| **call == format!("remove {}", worktree("SLICE-002"))).count(),
        1,
        "the first copy goes, the second stays for inspection: {calls:?}"
    );
}

// --- the secret gate ---

const PEM: &str = "const key = `\n-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA\n-----END RSA \
                   PRIVATE KEY-----\n`;\n";

// A file a slice's build left changed that carries a credential refuses the
// slice before anything of it is sealed: each path is named with what it
// carries, the copy stays as it was written for inspection, nothing is
// merged or labelled, and every changed file was read from the copy.
#[tokio::test]
async fn build_secret_in_slice() {
    let (provider, _) = planned();
    let copy = worktree("SLICE-001");
    changed(
        &provider,
        "SLICE-001",
        &[
            ("src/auth.rs", ChangeKind::Added),
            ("src/session.rs", ChangeKind::Added),
            ("config/deploy.yaml", ChangeKind::Modified),
        ],
    );
    provider.files.holds(&copy, "src/auth.rs", PEM);
    provider.files.holds(&copy, "src/session.rs", "pub fn issue() {}\n");
    provider.files.holds(
        &copy,
        "config/deploy.yaml",
        "aws:\n  access_key_id: AKIAIOSFODNN7EXAMPLE\n  token: ${AWS_TOKEN}\n",
    );

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "secret-detected").await;

    assert_message(
        &envelope,
        "slice `SLICE-001` (authentication) failed in wave 1: what the build wrote carries a \
         credential, so none of it is sealed:\n- src/auth.rs: a PEM private key block\n- \
         config/deploy.yaml: an AWS access key id",
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains(".emery/vcs/worktrees/<slice>")),
        "{envelope}"
    );
    assert_eq!(
        provider.files.reads(),
        [
            (copy.clone(), "src/auth.rs".to_owned()),
            (copy.clone(), "src/session.rs".to_owned()),
            (copy.clone(), "config/deploy.yaml".to_owned()),
        ],
        "every changed file is read, in the order the copy lists them"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    assert!(provider.vcs.merged().is_empty(), "nothing is merged");
    let calls = provider.vcs.calls();
    assert_eq!(calls.last().cloned(), Some(format!("pending {copy}")), "{calls:?}");
    assert!(!calls.contains(&format!("remove {copy}")), "left for inspection: {calls:?}");
    assert!(!calls.iter().any(|call| call.starts_with("label ")), "no label: {calls:?}");
    assert!(provider.target.verifies().is_empty(), "a wave that did not integrate is not verified");
    assert_eq!(provider.target.calls().len(), 1, "the run stops at the first");
    provider.vcs.assert_exhausted();
}

// A key pasted into a string literal keeps its header on one line with the
// escaped newline and the rest of the key after it, and is refused as the
// block form is; a certificate spelled the same way beside it passes.
#[tokio::test]
async fn build_secret_in_literal() {
    let (provider, _) = planned();
    let copy = worktree("SLICE-001");
    changed(
        &provider,
        "SLICE-001",
        &[("src/key.ts", ChangeKind::Added), ("src/cert.ts", ChangeKind::Added)],
    );
    provider.files.holds(
        &copy,
        "src/key.ts",
        "const key = \"-----BEGIN RSA PRIVATE KEY-----\\nMIIEowIBAAKCAQEA\\n-----END RSA PRIVATE \
         KEY-----\\n\";\n",
    );
    provider.files.holds(
        &copy,
        "src/cert.ts",
        "const cert = \"-----BEGIN CERTIFICATE-----\\nMIICljCCAX4C\\n-----END \
         CERTIFICATE-----\\n\";\n",
    );

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "secret-detected").await;

    assert_message(&envelope, "so none of it is sealed:\n- src/key.ts: a PEM private key block");
    assert!(!envelope["message"].as_str().unwrap_or("").contains("src/cert.ts"), "{envelope}");
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    provider.vcs.assert_exhausted();
}

// A file that carries a credential by name is refused unread, whatever it
// holds; a public key beside it is read and passes.
#[tokio::test]
async fn build_secret_file() {
    let (provider, _) = planned();
    let copy = worktree("SLICE-001");
    changed(
        &provider,
        "SLICE-001",
        &[
            ("deploy/.npmrc", ChangeKind::Added),
            ("deploy/id_rsa.pub", ChangeKind::Added),
            ("ops/.aws/credentials", ChangeKind::Added),
        ],
    );
    provider.files.holds(&copy, "deploy/.npmrc", "registry=https://registry.npmjs.org/\n");
    provider.files.holds(&copy, "deploy/id_rsa.pub", "ssh-rsa AAAAB3NzaC1yc2E deploy@example\n");

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "secret-detected").await;

    assert_message(
        &envelope,
        "- deploy/.npmrc: a file that carries a credential by name\n- ops/.aws/credentials: a file \
         that carries a credential by name",
    );
    assert!(!envelope["message"].as_str().unwrap_or("").contains("id_rsa.pub"), "{envelope}");
    assert_eq!(
        provider.files.reads(),
        [(copy.clone(), "deploy/id_rsa.pub".to_owned())],
        "a file refused by name is never read"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    assert!(!provider.vcs.calls().contains(&format!("remove {copy}")), "left for inspection");
    provider.vcs.assert_exhausted();
}

// A credential a verification's repairs leave in the integrated tree refuses
// the wave before its changes are sealed: the slices merged in it stay
// committed, the wave's commit is never made, and the label stands where the
// last verified wave left it.
#[tokio::test]
async fn build_secret_in_repair() {
    let (provider, id) = planned();
    provider.vcs.pending.script(
        INTEGRATION,
        Ok(vec![
            change("src/orders.rs", ChangeKind::Modified),
            change(".github/publish.sh", ChangeKind::Added),
        ]),
    );
    provider.files.holds(INTEGRATION, "src/orders.rs", "pub fn place() {}\n");
    provider.files.holds(
        INTEGRATION,
        ".github/publish.sh",
        "npm config set //registry.npmjs.org/:_authToken=npm_a1B2c3D4e5F6g7H8i9J0k1L2m3N4o5P6q7R8\n",
    );

    let envelope = fail(&provider, &["emery", "build", BUILDER], 1, "secret-detected").await;

    assert_message(
        &envelope,
        &format!(
            "wave 1 failed; SLICE-001 merged in it stays committed in `{INTEGRATION}`; \
             `emery/{id}` is not set: what the verification wrote carries a credential, so none \
             of it is sealed:\n- .github/publish.sh: an npm token"
        ),
    );
    assert_eq!(
        provider.files.reads(),
        [
            (INTEGRATION.to_owned(), "src/orders.rs".to_owned()),
            (INTEGRATION.to_owned(), ".github/publish.sh".to_owned()),
        ]
    );
    assert_eq!(provider.vcs.messages().len(), 1, "the slice's commit alone; no wave commit");
    let calls = provider.vcs.calls();
    assert_eq!(calls.last().cloned(), Some(format!("pending {INTEGRATION}")), "{calls:?}");
    assert!(!calls.iter().any(|call| call.starts_with("label ")), "no label: {calls:?}");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");
    assert_eq!(provider.target.calls().len(), 1, "the second wave is never built");
    provider.vcs.assert_exhausted();
}

// A credential file the build removed carries nothing: a deleted path is
// passed over unread, the rest is read, and the slice seals as any other.
#[tokio::test]
async fn build_secret_deleted() {
    let (provider, _) = planned();
    let copy = worktree("SLICE-001");
    changed(
        &provider,
        "SLICE-001",
        &[(".env", ChangeKind::Deleted), ("src/auth.rs", ChangeKind::Added)],
    );
    provider.files.holds(&copy, "src/auth.rs", "pub fn sign_in() {}\n");
    wrote(&provider, ORDERS);
    provider.files.holds(&worktree("SLICE-002"), "src/orders.rs", "pub fn place() {}\n");

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["slices"][0]["written"], json!([".env", "src/auth.rs"]), "{envelope}");
    assert_eq!(
        provider.files.reads(),
        [(copy, "src/auth.rs".to_owned()), (worktree("SLICE-002"), "src/orders.rs".to_owned()),],
        "the deleted path is never read"
    );
    assert_eq!(provider.vcs.messages().len(), 2, "one commit per slice");
    provider.vcs.assert_exhausted();
}

// A changed path the gate cannot read is never taken as clean: the slice
// fails as the read did, nothing of it is sealed, and the copy stays for
// inspection.
#[tokio::test]
async fn build_change_unreadable() {
    let (provider, _) = planned();
    let copy = worktree("SLICE-001");
    changed(
        &provider,
        "SLICE-001",
        &[("src/auth.rs", ChangeKind::Added), ("src/hook", ChangeKind::Added)],
    );
    provider.files.holds(&copy, "src/auth.rs", "pub fn sign_in() {}\n");
    provider.files.unreadable(&copy, "src/hook");

    let envelope = fail(&provider, &["emery", "build", BUILDER], 3, "server_error").await;

    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-001` (authentication) failed in wave 1: reading `src/hook` under \
             `{copy}`"
        ),
    );
    assert_eq!(
        provider.files.reads(),
        [(copy.clone(), "src/auth.rs".to_owned()), (copy.clone(), "src/hook".to_owned())],
        "the gate stops at the read it cannot make"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    let calls = provider.vcs.calls();
    assert_eq!(calls.last().cloned(), Some(format!("pending {copy}")), "{calls:?}");
    assert!(!calls.contains(&format!("remove {copy}")), "left for inspection: {calls:?}");
    assert!(!calls.iter().any(|call| call.starts_with("label ")), "no label: {calls:?}");
    provider.vcs.assert_exhausted();
}
