//! Verifies `emery build` over scripted target, file, and version-control capabilities.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;

use emery_adapter::target::{
    Departure, DesignType, MergeRule, PartReach, Report, Requirement, Scenario, SectionChange,
    Slice, Standing, TypeChange, Verdict,
};
use emery_adapter::{Route, Trigger};
use emery_engine::vcs::{REVISION, SCOPE};
use emery_engine::{EntryKind, REVISION_KEY};
use omnia_sdk::model::{Error as ModelError, Reply};
use omnia_sdk::vcs::{Change, ChangeKind, Entry, Error, Merged, Rule, Strategy};
use omnia_sdk::{bad_gateway, bad_request};
use omnia_test::SeenFormat;
use omnia_test::guest::ScriptedModel;
use serde_json::{Value, json};
use support::provider::{Provider, Rendezvous, cli, digest};
use support::scratch::Scratch;
use support::store::{seed, seed_at, stage, stage_amendment, stage_fresh};
use support::vcs::HEAD;
use support::{cli_ok, fail};

const MERGE_MD: &str = include_str!("../crates/engine/prose/merge.md");
// The resolution of `src/lib.rs` between `authentication` and `orders`.
const RESOLVED_LIB: &str = r#"{"files": [{"path": "src/lib.rs", "content": "pub mod authentication;\npub mod orders;\n"}]}"#;

// The target every scenario names, and the guest it loads and dispatches as.
const BUILDER: &str = "acme:builder@2.1.0";
const BUILDER_ID: &str = "acme:builder";
// The config building `BUILDER` into the project checkout, `repository = "."`;
// a run naming it on the command line creates a repository of its own instead.
const PROJECT_CONFIG: &str = "tests/build/project.toml";

// The commit the project checkout sits on, where a scenario tells it from the head.
const BASE: &str = "1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d";
// The head an earlier run's label points at, where a scenario resumes.
const LABELLED: &str = "e1f2a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4";
// The head the integration working copy an earlier run left sits on, where a
// scenario resumes the slices that run merged before its wave was verified.
const STAGED: &str = "5a6b7c8d9e0f1a2b3c4d5e6f708192a3b4c5d6e7";
const INTEGRATION: &str = "./.emery/vcs/integration";
const WORKTREES: &str = "./.emery/vcs/worktrees";
const REMOTE_URL: &str = "https://example.com/acme/shop.git";
// The target's repository, under its name: `builder`, the adapter's, when
// the config gives none. A clone of `REMOTE_URL`, or the one the engine creates.
const TARGET_REPO: &str = "./.emery/vcs/repos/builder";
const USAGE_EXIT: u8 = 64;

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");
const WIDE_PLAN: &[u8] = include_bytes!("build/wide-plan.json");
// An amendment as `emery amend` stages one, read by a build for its kind alone.
const AMENDMENT: &[u8] = include_bytes!("specify/6-amendment.json");
const SPEC_001: &str = include_str!("build/spec-001.md");
const SPEC_002: &str = include_str!("build/spec-002.md");
const DESIGN_MD: &str = include_str!("build/design.md");

const PLAN_001: &str = "## Part: authentication\n\nID: PART-001\nRequirements: [REQ-001, \
                        REQ-002]\n\nDelivers sign-in and the session it issues; a session cannot \
                        be verified without a sign-in to issue it, so the two stems are one build.";
const PLAN_002: &str = "## Part: orders\n\nID: PART-002\nRequirements: [REQ-003, REQ-004]\nTypes: \
                        [orders.line, orders.order]\nDepends on: [PART-001]\n\nDelivers order \
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
    let id = seed(&provider.files, SPEC, DESIGN, PLAN);
    (provider, id)
}

// A provider over the wide revision: two slices ready at once, one after.
fn wide() -> (Provider, String) {
    let provider = Provider::idle();
    let id = seed(&provider.files, SPEC, DESIGN, WIDE_PLAN);
    (provider, id)
}

fn report(covered: &[&str], written: &[&str]) -> Report {
    Report {
        covered: covered.iter().map(ToString::to_string).collect(),
        written: written.iter().map(ToString::to_string).collect(),
    }
}

fn verdict(passed: bool, failures: &[&str]) -> Verdict {
    if passed && failures.is_empty() {
        Verdict::passed()
    } else if !passed {
        Verdict::failed(failures.iter().copied())
    } else {
        Verdict {
            passed,
            failures: failures.iter().map(ToString::to_string).collect(),
        }
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

// The exchange that reads the revision from `repo` at `commit`: a working
// copy cut there for the three documents, and removed.
fn read_revision(repo: &str, commit: &str) -> Vec<String> {
    vec![format!("add {repo} {REVISION} {commit}"), format!("remove {REVISION}")]
}

// The exchange that opens a build into the project checkout standing at
// `base`: the head, the revision read from it, then the checkout held clean.
fn project(base: &str) -> Vec<String> {
    let mut calls = vec!["head .".to_owned()];
    calls.extend(read_revision(".", base));
    calls.push("pending .".to_owned());
    calls
}

// What the run read through `Files` beyond the revision's own documents.
fn reads(provider: &Provider) -> Vec<(String, String)> {
    provider.files.reads().into_iter().filter(|(at, _)| at != REVISION).collect()
}

// What the run listed through `Files` beyond the revision's own tree.
fn listed(provider: &Provider) -> Vec<(String, String)> {
    provider.files.listed().into_iter().filter(|(at, _)| at != REVISION).collect()
}

// A requirement as the fixture lowers it: one scenario with no `given` and no `and`.
fn requirement(
    id: &str, subject: &str, statement: &str, (name, when, then): (&str, &str, &str),
) -> Requirement {
    Requirement {
        id: id.to_owned(),
        subject: subject.to_owned(),
        statement: vec![statement.to_owned()],
        scenarios: vec![Scenario {
            name: name.to_owned(),
            given: Vec::new(),
            when: when.to_owned(),
            then: then.to_owned(),
            and: Vec::new(),
        }],
        trigger: None,
    }
}

// The requirements, the owned types, and the brief the two-slice plan's
// slices carry typed beside their renderings, each whole as lowered from
// the fixtures.
fn typed(first: &Slice, second: &Slice) {
    assert_eq!(
        first.requirements,
        [
            requirement(
                "REQ-001",
                "auth.login",
                "Users sign in with a credential.",
                ("Login", "a valid credential is presented", "the caller is signed in"),
            ),
            requirement(
                "REQ-002",
                "session.timeout",
                "Sessions expire after an hour of inactivity.",
                ("Timeout", "a session is idle for an hour", "it times out"),
            ),
        ],
        "the requirements are lowered from the specification, in id order"
    );
    assert_eq!(first.types, [], "authentication owns no type");
    assert_eq!(
        first.brief,
        ["Delivers sign-in and the session it issues; a session cannot be verified without a \
          sign-in to issue it, so the two stems are one build."]
    );
    assert_eq!(
        second.requirements,
        [
            Requirement {
                trigger: Some(Trigger::Http(Route {
                    method: "POST".to_owned(),
                    path: "/orders".to_owned(),
                })),
                ..requirement(
                    "REQ-003",
                    "orders.create",
                    "A signed-in user places an order.",
                    ("Create", "a signed-in caller places an order", "the order is created"),
                )
            },
            Requirement {
                trigger: Some(Trigger::Message("orders.cancel-requested".to_owned())),
                ..requirement(
                    "REQ-004",
                    "orders.cancel",
                    "A signed-in user cancels an open order.",
                    ("Cancel", "a signed-in caller cancels an order", "the order is cancelled"),
                )
            },
        ],
        "a requirement's trigger is lowered typed beside its rendering"
    );
    assert_eq!(
        second.types,
        [
            DesignType {
                key: "orders.line".to_owned(),
                signature: "interface Line { sku: string }".to_owned(),
            },
            DesignType {
                key: "orders.order".to_owned(),
                signature: "interface Order { id: string }".to_owned(),
            },
        ],
        "each owned key with the design's signature, in the plan's order"
    );
    assert_eq!(
        second.brief,
        ["Delivers order creation and cancellation for a signed-in caller; verified by placing \
          and cancelling an order under a session from `authentication`."]
    );
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

// Scripts what the adapter's initialisation adds to the integration copy,
// nothing where its layout adds nothing to the base.
fn initialised(provider: &Provider, paths: &[&str]) {
    provider.vcs.pending.script(
        INTEGRATION,
        Ok(paths.iter().map(|path| change(path, ChangeKind::Added)).collect()),
    );
}

// The message a build seals a slice under in its working copy and as its merge.
fn message(
    id: &str, (slice, name): (&str, &str), requirements: &str, covered: &str, base: &str,
    wave: usize,
) -> String {
    let part = slice.replace("SLICE-", "PART-");
    format!(
        "{slice} {name}\n\nSlice: {slice}\nPart: {part}\nRevision: {id}\nBudget: 12\nRequirements: \
         {requirements}\nCovered: {covered}\nAdapter: {BUILDER}\nBase: {base}\nWave: {wave}"
    )
}

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

// The `verified` entries of an envelope: each wave with the slices merged
// in it, every one verified at the unscripted head.
fn verified(waves: &[(usize, &[&str])]) -> Value {
    Value::Array(
        waves
            .iter()
            .map(|(wave, slices)| json!({"wave": wave, "slices": slices, "head": HEAD}))
            .collect(),
    )
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

// The exchange that opens a fresh build over `repo` from `base`: the label
// looked for, the integration working copy cut, and what the adapter's
// initialisation left in it read.
fn opened(repo: &str, base: &str, id: &str) -> Vec<String> {
    vec![
        format!("labelled {repo} emery/{id}"),
        format!("add {repo} {INTEGRATION} {base}"),
        format!("pending {INTEGRATION}"),
    ]
}

// The exchange a fresh two-slice build makes over `repo` from `base`: the
// build opened, one slice per wave, the integration copy removed.
fn integrated(repo: &str, base: &str, id: &str) -> Vec<String> {
    let mut calls = opened(repo, base, id);
    calls.extend(built(repo, base, id));
    calls
}

// The two waves of a two-slice build over `repo`, one slice each, then the
// integration copy removed once every slice is merged.
fn built(repo: &str, base: &str, id: &str) -> Vec<String> {
    let mut calls = wave(repo, id, base, &[AUTHENTICATION]);
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

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    // the text render
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 2 parts in 2 waves; budget 12 (default): 2 slices in 2 \
             waves, widest 1\n  base {BASE}\n  wave 1: 1 slice verified at 9f8e7d6c\n    \
             SLICE-001 authentication: covered 2/2, written 1 file, merged m:SLICE-001\n  wave 2: \
             1 slice verified at 9f8e7d6c\n    SLICE-002 orders: covered 1/2 (uncovered REQ-004), \
             written 3 files, merged m:SLICE-002\n  labelled emery/{id} at {HEAD}\n"
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

    // each slice carries its own documents, typed beside the renderings, and
    // the head it builds over
    assert_eq!(first.id, "SLICE-001");
    assert_eq!(first.part, "PART-001");
    assert_eq!(first.name, "authentication");
    assert_eq!((first.index, first.count), (1, 1));
    assert_eq!(first.spec, SPEC_001, "the specification is cut to the slice");
    assert_eq!(first.design, DESIGN_MD, "the design is whole");
    assert_eq!(first.plan, PLAN_001, "the plan entry is the slice's own");
    assert_eq!(first.base, BASE, "the first wave builds over the base");
    assert_eq!(second.id, "SLICE-002");
    assert_eq!(second.part, "PART-002");
    assert_eq!(second.name, "orders");
    assert_eq!((second.index, second.count), (1, 1));
    assert_eq!(second.spec, SPEC_002);
    assert_eq!(second.design, DESIGN_MD);
    assert_eq!(second.plan, PLAN_002);
    assert_eq!(second.base, HEAD, "the second wave builds over the verified head");
    typed(first, second);
    assert_eq!(provider.target.verifies(), [INTEGRATION, INTEGRATION], "one verify per wave");
    assert_eq!(provider.target.inits(), [INTEGRATION], "one init, before any slice");

    // the sealed base, one commit and merge per slice, the label after each wave
    let mut expected = project(BASE);
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
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["parts"], json!([["PART-001"], ["PART-002"]]), "{envelope}");
    assert_eq!(envelope["waves"], json!([["SLICE-001"], ["SLICE-002"]]), "{envelope}");
    assert_eq!(envelope["base"], BASE, "{envelope}");
    assert!(envelope.get("resumed").is_none(), "a fresh build resumes nothing: {envelope}");
    assert!(envelope.get("initialised").is_none(), "an init scaffolding nothing seals nothing");
    assert_eq!(
        envelope["slices"],
        json!([
            {"id": "SLICE-001", "part": "PART-001", "name": "authentication", "index": 1, "count": 1, "wave": 1, "covered": ["REQ-001", "REQ-002"], "uncovered": [], "written": ["src/authentication.rs"], "commit": "m:SLICE-001"},
            {"id": "SLICE-002", "part": "PART-002", "name": "orders", "index": 1, "count": 1, "wave": 2, "covered": ["REQ-003"], "uncovered": ["REQ-004"], "written": ["Cargo.toml", "src/lib.rs", "src/orders.rs"], "commit": "m:SLICE-002"},
        ]),
        "`written` is the copy's change set, sorted: {envelope}"
    );
    assert_eq!(
        envelope["verified"],
        verified(&[(1, &["SLICE-001"]), (2, &["SLICE-002"])]),
        "{envelope}"
    );
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

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["parts"], json!([["PART-001", "PART-003"], ["PART-002"]]), "{envelope}");
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
    let mut expected = project(BASE);
    expected.extend(opened(".", BASE, &id));
    expected.extend(wave(".", &id, BASE, &[AUTHENTICATION, WIDE_ORDERS]));
    expected.extend(wave(".", &id, HEAD, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected);
    assert_eq!(provider.target.verifies(), [INTEGRATION, INTEGRATION]);
    assert_eq!(
        envelope["slices"],
        json!([
            {"id": "SLICE-001", "part": "PART-001", "name": "authentication", "index": 1, "count": 1, "wave": 1, "covered": ["REQ-001"], "uncovered": [], "written": ["src/authentication.rs"], "commit": "m:SLICE-001"},
            {"id": "SLICE-003", "part": "PART-003", "name": "orders", "index": 1, "count": 1, "wave": 1, "covered": ["REQ-003", "REQ-004"], "uncovered": [], "written": ["src/orders.rs"], "commit": "m:SLICE-003"},
            {"id": "SLICE-002", "part": "PART-002", "name": "sessions", "index": 1, "count": 1, "wave": 2, "covered": ["REQ-002"], "uncovered": [], "written": ["src/sessions.rs"], "commit": "m:SLICE-002"},
        ]),
        "{envelope}"
    );
    assert_eq!(
        envelope["verified"],
        verified(&[(1, &["SLICE-001", "SLICE-003"]), (2, &["SLICE-002"])]),
        "{envelope}"
    );

    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    for slice in [AUTHENTICATION, WIDE_ORDERS, WIDE_SESSIONS] {
        wrote(&provider, slice);
    }
    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 3 parts in 2 waves; budget 12 (default): 3 slices in 2 \
             waves, widest 2\n  base {BASE}\n  wave 1: 2 slices verified at 9f8e7d6c\n    \
             SLICE-001 authentication: covered 1/1, written 1 file, merged m:SLICE-001\n    \
             SLICE-003 orders: covered 2/2, written 1 file, merged m:SLICE-003\n  wave 2: 1 slice \
             verified at 9f8e7d6c\n    SLICE-002 sessions: covered 1/1, written 1 file, merged \
             m:SLICE-002\n  labelled emery/{id} at {HEAD}\n"
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

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG, "--jobs", "1"]).await;

    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-003".to_owned(), BASE.to_owned(), worktree("SLICE-003")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ]
    );
    let mut expected = project(BASE);
    expected.extend(opened(".", BASE, &id));
    expected.extend(wave(".", &id, BASE, &[AUTHENTICATION, WIDE_ORDERS]));
    expected.extend(wave(".", &id, HEAD, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected, "the same history as a wide run");

    let response = cli(&provider, &["emery", "build", "--config", PROJECT_CONFIG, "-j", "0"]).await;
    assert_eq!(response.exit, USAGE_EXIT, "{}", String::from_utf8_lossy(&response.stderr));
    assert_eq!(provider.target.calls().len(), 3, "a usage error dispatches nothing");
}

// The merge rules the target declares ride every merge, in the order declared.
#[tokio::test]
async fn build_merge_rules() {
    let (mut provider, _) = planned();
    provider.target.rules = vec![MergeRule::union("src/*/mod.rs"), MergeRule::ours("Cargo.lock")];

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

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

// The paths a slice's merge leaves conflicted under the adapter's rules are
// put to the model as one question: the prompt, both sides of each path
// read from the two working copies, and the plan entries of the slice and
// of the wave's slices that wrote them, steered to the conflicted paths; a
// resolution the gate refuses comes back for correction.
#[tokio::test]
async fn conflict_two_slices() {
    let (provider, _) = conflicted_at_lib(&[
        r#"{"files": [{"path": "src/lib.rs", "content": "<<<<<<< HEAD\npub mod authentication;\n=======\npub mod orders;\n>>>>>>> SLICE-003\n"}]}"#,
        RESOLVED_LIB,
    ]);
    let copy = worktree("SLICE-003");
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    wrote(&provider, WIDE_SESSIONS);

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let seen = provider.model.seen();
    assert_eq!(seen.len(), 2, "one question, two candidates");
    let turn = &seen[0];
    assert_eq!(turn.system.as_deref(), Some(MERGE_MD), "the merge prompt leads the turn");
    let SeenFormat::Schema { name, schema } = &turn.format else {
        panic!("the resolution is steered by schema");
    };
    assert_eq!(name, "merge-SLICE-003");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["files"]["minItems"], 1);
    assert_eq!(schema["properties"]["files"]["maxItems"], 1);
    assert_eq!(schema["$defs"]["Resolution"]["properties"]["path"]["enum"], json!(["src/lib.rs"]));
    let brief = &turn.messages[0];
    assert_eq!(
        brief,
        "# Slice being merged\n\n## Part: orders\n\nID: PART-003\nRequirements: [REQ-003, \
         REQ-004]\nTypes: [orders.line, orders.order]\n\nDelivers order creation and \
         cancellation; it waits on nothing.\n\n# Merged before it in the wave\n\n## Part: \
         authentication\n\nID: PART-001\nRequirements: [REQ-001]\n\nDelivers sign-in.\n\n# \
         Conflicts\n\n## `src/lib.rs`\n\n### Integrated tree\n\n```\npub mod \
         authentication;\n```\n\n### Slice\n\n```\npub mod orders;\n```\n\n"
    );
    let exchanges = provider.model.exchanges();
    assert_eq!(exchanges.len(), 2);
    assert!(
        exchanges[0].outcome.as_ref().is_err_and(|c| c.contains("still carries conflict markers")),
        "{exchanges:?}"
    );
    assert!(exchanges[1].outcome.is_ok(), "{exchanges:?}");
    assert_eq!(
        listed(&provider),
        [
            (INTEGRATION.to_owned(), ".".to_owned()),
            (INTEGRATION.to_owned(), "src".to_owned()),
            (copy.clone(), ".".to_owned()),
            (copy.clone(), "src".to_owned()),
        ],
        "each side is walked for what stands at the path"
    );
    assert_eq!(
        reads(&provider).iter().filter(|read| read.0 == copy).count(),
        3,
        "the slice's side is read for the gate, the brief, and the gate again: {:?}",
        reads(&provider)
    );
    provider.model.assert_exhausted();
    provider.vcs.assert_exhausted();
}

// A merge conflict on a slice resumed in the run's first wave but the plan's
// second names no slice from the plan's first wave in the brief: the wave
// counter restarted at one, not the plan wave being merged.
#[tokio::test]
async fn conflict_resumed_wave() {
    let provider = Provider::answering([RESOLVED_LIB]);
    let id = seed(&provider.files, SPEC, DESIGN, WIDE_PLAN);
    let copy = worktree("SLICE-002");
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry(
                "w1",
                &format!(
                    "Wave 1 verified\n\nRevision: {id}\nAdapter: {BUILDER}\nSlices: SLICE-001, \
                     SLICE-003"
                ),
            ),
            entry(
                "m:SLICE-003",
                &message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
            ),
            entry("m:SLICE-001", &message(&id, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
        ]),
    );
    provider.vcs.merges.script(INTEGRATION, Ok(conflicted(&["src/lib.rs"])));
    changed(&provider, "SLICE-002", &[("src/lib.rs", ChangeKind::Modified)]);
    changed(&provider, "SLICE-002", &[("src/lib.rs", ChangeKind::Modified)]);
    wrote(&provider, WIDE_SESSIONS);
    provider.vcs.commits.script(&copy, Ok(Some("SLICE-002-commit".to_owned())));
    provider.vcs.commits.script(&copy, Ok(Some("SLICE-002-resolved".to_owned())));
    for at in [INTEGRATION, copy.as_str()] {
        provider.files.lists(at, ".", &[("src", EntryKind::Directory)]);
        provider.files.lists(at, "src", &[("lib.rs", EntryKind::File)]);
    }
    provider.files.holds(INTEGRATION, "src/lib.rs", "pub mod authentication;\npub mod orders;\n");
    provider.files.holds(&copy, "src/lib.rs", "pub mod sessions;\n");

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let brief = &provider.model.seen()[0].messages[0];
    assert!(
        !brief.contains("# Merged before it in the wave"),
        "plan wave two has no merged siblings yet: {brief}"
    );
    provider.model.assert_exhausted();
}

// A higher-id slice already merged in the integration copy is named in the
// merge brief when a lower-id sibling of the same plan wave conflicts with
// it, even though id order would place it after the slice being merged.
#[tokio::test]
async fn conflict_staged_higher() {
    let provider = Provider::answering([RESOLVED_LIB]);
    let id = seed(&provider.files, SPEC, DESIGN, WIDE_PLAN);
    let copy = worktree("SLICE-001");
    provider.target.fails("SLICE-001", bad_gateway!("the model timed out"));
    provider.target.fails("SLICE-001", bad_gateway!("the model timed out again"));
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    wrote(&provider, WIDE_ORDERS);

    fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 4, "bad_gateway").await;

    staged(
        &provider,
        vec![entry(
            "m:SLICE-003",
            &message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
        )],
    );
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(vec![]));
    provider.vcs.merges.script(INTEGRATION, Ok(conflicted(&["src/lib.rs"])));
    changed(
        &provider,
        "SLICE-001",
        &[("src/authentication.rs", ChangeKind::Added), ("src/lib.rs", ChangeKind::Modified)],
    );
    changed(&provider, "SLICE-001", &[("src/lib.rs", ChangeKind::Modified)]);
    provider.vcs.commits.script(&copy, Ok(Some("SLICE-001-commit".to_owned())));
    provider.vcs.commits.script(&copy, Ok(Some("SLICE-001-resolved".to_owned())));
    for at in [INTEGRATION, copy.as_str()] {
        provider.files.lists(at, ".", &[("src", EntryKind::Directory)]);
        provider.files.lists(at, "src", &[("lib.rs", EntryKind::File)]);
    }
    provider.files.holds(INTEGRATION, "src/lib.rs", "pub mod orders;\n");
    provider.files.holds(&copy, "src/lib.rs", "pub mod authentication;\n");

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let brief = &provider.model.seen()[0].messages[0];
    assert!(
        brief.contains("# Merged before it in the wave\n\n## Part: orders\n\nID: PART-003"),
        "the staged higher-id sibling is named in the brief: {brief}"
    );
    provider.model.assert_exhausted();
}

// The accepted resolution is written into the slice's working copy, held
// to the secret gate, sealed over the slice's commit, and merged again with
// each resolved path taken whole ahead of the adapter's rules, so the slice
// lands as one merge commit in its wave and the paths are reported.
#[tokio::test]
async fn conflict_prior_merged() {
    let (mut provider, id) = conflicted_at_lib(&[RESOLVED_LIB]);
    provider.target.rules = vec![MergeRule::ours("Cargo.lock")];
    let copy = worktree("SLICE-003");
    provider.vcs.commits.script(&copy, Ok(Some("SLICE-003-commit".to_owned())));
    provider.vcs.commits.script(&copy, Ok(Some("SLICE-003-resolved".to_owned())));
    // what the build changed, then what the resolution left pending
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    wrote(&provider, WIDE_SESSIONS);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    assert_eq!(
        provider.files.writes(),
        [(
            copy.clone(),
            "src/lib.rs".to_owned(),
            b"pub mod authentication;\npub mod orders;\n".to_vec()
        )],
        "the resolution is written into the slice's working copy alone"
    );
    let mut expected = project(BASE);
    expected.extend(opened(".", BASE, &id));
    expected.extend([
        format!("add . {} {BASE}", worktree("SLICE-001")),
        format!("add . {copy} {BASE}"),
    ]);
    expected.extend(sealed(AUTHENTICATION));
    expected.extend([
        format!("head {copy}"),
        format!("pending {copy}"),
        format!("commit {copy} SLICE-003 orders"),
        format!("merge {INTEGRATION} SLICE-003-commit SLICE-003 orders"),
        format!("pending {copy}"),
        format!("commit {copy} SLICE-003 orders resolved at src/lib.rs"),
        format!("merge {INTEGRATION} SLICE-003-resolved SLICE-003 orders"),
        format!("remove {copy}"),
        format!("pending {INTEGRATION}"),
        format!("head {INTEGRATION}"),
        format!("label . emery/{id} {HEAD}"),
    ]);
    expected.extend(wave(".", &id, HEAD, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected);
    let orders = message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", BASE, 1);
    assert_eq!(
        provider.vcs.messages(),
        [
            message(&id, AUTHENTICATION, "REQ-001", "REQ-001", BASE, 1),
            orders.clone(),
            "SLICE-003 orders resolved at src/lib.rs".to_owned(),
            message(&id, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2),
        ],
        "the resolution is sealed over the slice's commit"
    );
    let ours = Rule {
        paths: "Cargo.lock".to_owned(),
        strategy: Strategy::Ours,
    };
    let theirs = Rule {
        paths: "/src/lib.rs".to_owned(),
        strategy: Strategy::Theirs,
    };
    assert_eq!(
        provider.vcs.merged()[1..3],
        [(orders.clone(), vec![ours.clone()]), (orders, vec![theirs, ours])],
        "the second merge carries the slice's message and takes each resolution whole"
    );

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["slices"],
        json!([
            {"id": "SLICE-001", "part": "PART-001", "name": "authentication", "index": 1, "count": 1, "wave": 1, "covered": ["REQ-001"], "uncovered": [], "written": ["src/authentication.rs", "src/lib.rs"], "commit": "m:SLICE-001"},
            {"id": "SLICE-003", "part": "PART-003", "name": "orders", "index": 1, "count": 1, "wave": 1, "covered": ["REQ-003", "REQ-004"], "uncovered": [], "written": ["src/lib.rs"], "resolved": ["src/lib.rs"], "commit": "m:SLICE-003"},
            {"id": "SLICE-002", "part": "PART-002", "name": "sessions", "index": 1, "count": 1, "wave": 2, "covered": ["REQ-002"], "uncovered": [], "written": ["src/sessions.rs"], "commit": "m:SLICE-002"},
        ]),
        "{envelope}"
    );
    assert_eq!(
        envelope["verified"],
        verified(&[(1, &["SLICE-001", "SLICE-003"]), (2, &["SLICE-002"])]),
        "{envelope}"
    );
    provider.model.assert_exhausted();
    provider.vcs.assert_exhausted();
}

// A resolution that keeps the slice's own side changes nothing in its
// working copy, so nothing is sealed over the slice's commit and that commit
// is merged again with the path taken whole; the text names the paths
// resolved beside the merge.
#[tokio::test]
async fn conflict_as_written() {
    let (provider, id) = conflicted_at_lib(&[
        r#"{"files": [{"path": "src/lib.rs", "content": "pub mod orders;\n"}]}"#,
    ]);
    let copy = worktree("SLICE-003");
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    wrote(&provider, WIDE_SESSIONS);
    provider.vcs.commits.script(&copy, Ok(Some("SLICE-003-commit".to_owned())));
    provider.vcs.commits.script(&copy, Ok(None));

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let calls = provider.vcs.calls();
    let merges: Vec<&String> = calls
        .iter()
        .filter(|call| call.starts_with(&format!("merge {INTEGRATION} SLICE-003")))
        .collect();
    assert_eq!(
        merges,
        [
            &format!("merge {INTEGRATION} SLICE-003-commit SLICE-003 orders"),
            &format!("merge {INTEGRATION} SLICE-003-commit SLICE-003 orders"),
        ],
        "the slice's own commit is merged again: {calls:?}"
    );
    assert_eq!(
        provider.vcs.merged()[2].1,
        [Rule {
            paths: "/src/lib.rs".to_owned(),
            strategy: Strategy::Theirs,
        }],
        "with the path taken whole"
    );
    assert_eq!(
        provider.vcs.messages(),
        [
            message(&id, AUTHENTICATION, "REQ-001", "REQ-001", BASE, 1),
            message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", BASE, 1),
            "SLICE-003 orders resolved at src/lib.rs".to_owned(),
            message(&id, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2),
        ],
        "sealing the resolution makes no commit"
    );
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "built revision {id}\n  plan: 3 parts in 2 waves; budget 12 (default): 3 slices in 2 \
             waves, widest 2\n  base {BASE}\n  wave 1: 2 slices verified at 9f8e7d6c\n    \
             SLICE-001 authentication: covered 1/1, written 2 files, merged m:SLICE-001\n    \
             SLICE-003 orders: covered 2/2, written 1 file, merged m:SLICE-003, resolved \
             (src/lib.rs)\n  wave 2: 1 slice verified at 9f8e7d6c\n    SLICE-002 sessions: \
             covered 1/1, written 1 file, merged m:SLICE-002\n  labelled emery/{id} at {HEAD}\n"
        )
    );
    provider.model.assert_exhausted();
    provider.vcs.assert_exhausted();
}

// A resolution refused within the available rounds ends the run at the
// slice as `merge-unresolved`, naming the paths and the findings, with
// nothing written, both working copies left for inspection, and the wave
// neither verified nor labelled.
#[tokio::test]
async fn conflict_unmerged() {
    let (provider, _) = conflicted_at_lib(&[r#"{"files": []}"#, r#"{"files": []}"#]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "merge-unresolved")
            .await;

    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-003` (orders) failed in wave 1; SLICE-001 merged before it stays \
             committed in `{INTEGRATION}`: its merge conflicts at src/lib.rs, and the conflict \
             cannot be resolved: the model's resolution was refused within the available rounds: \
             - `src/lib.rs` is not answered"
        ),
    );
    assert!(
        envelope["message"].as_str().is_some_and(|m| !m.contains("## Previous answer")),
        "the rejected candidate is not spelled back: {envelope}"
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("merge rule")),
        "{envelope}"
    );
    assert!(provider.files.writes().is_empty(), "nothing is written on a refusal");
    left_for_inspection(&provider);
    provider.model.assert_exhausted();
    provider.vcs.assert_exhausted();
}

// A conflicted path the slice removed is nothing the model can combine, so
// the slice is refused as `merge-unresolved` before any turn is spent.
#[tokio::test]
async fn conflict_deleted_slice() {
    let (provider, _) = conflicted_at_lib(&[]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Deleted)]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "merge-unresolved")
            .await;

    assert_message(
        &envelope,
        "its merge conflicts at src/lib.rs, and the conflict cannot be resolved: the slice \
         removed `src/lib.rs`",
    );
    left_for_inspection(&provider);
    provider.vcs.assert_exhausted();
}

// A conflicted path the integrated tree no longer holds as a regular file,
// one a slice merged before it removed, is refused the same way.
#[tokio::test]
async fn conflict_deleted_base() {
    let (provider, _) = conflicted_at_lib(&[]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    provider.files.lists(INTEGRATION, "src", &[("orders.rs", EntryKind::File)]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "merge-unresolved")
            .await;

    assert_message(
        &envelope,
        "its merge conflicts at src/lib.rs, and the conflict cannot be resolved: the integrated \
         tree does not hold `src/lib.rs`",
    );
    left_for_inspection(&provider);
    provider.vcs.assert_exhausted();
}

// A symbolic link at a conflicted path, on either side, is refused the same
// way, since the model is handed text and a commit stores the link's target.
#[tokio::test]
async fn build_conflict_symlink() {
    let (provider, _) = conflicted_at_lib(&[]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    provider.files.lists(&worktree("SLICE-003"), "src", &[("lib.rs", EntryKind::Link)]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "merge-unresolved")
            .await;

    assert_message(
        &envelope,
        "its merge conflicts at src/lib.rs, and the conflict cannot be resolved: a symbolic link \
         stands at `src/lib.rs` in the slice",
    );
    left_for_inspection(&provider);
    provider.vcs.assert_exhausted();
}

// A conflicted path that is not text on one side is refused the same way.
#[tokio::test]
async fn build_conflict_binary() {
    let (provider, _) = conflicted_at_lib(&[]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    provider.files.holds_bytes(INTEGRATION, "src/lib.rs", &[0xff, 0xfe, 0x00, 0x01]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "merge-unresolved")
            .await;

    assert_message(
        &envelope,
        "its merge conflicts at src/lib.rs, and the conflict cannot be resolved: `src/lib.rs` is \
         not text in the integrated tree",
    );
    assert!(
        reads(&provider).contains(&(INTEGRATION.to_owned(), "src/lib.rs".to_owned())),
        "{:?}",
        reads(&provider)
    );
    left_for_inspection(&provider);
    provider.vcs.assert_exhausted();
}

// A resolution turn that fails upstream is put once more before the failure
// is the run's, as a slice's build is.
#[tokio::test]
async fn conflict_retry() {
    let (mut provider, _) = conflicted_at_lib(&[]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    wrote(&provider, WIDE_SESSIONS);
    provider.model = ScriptedModel::new([
        Err(ModelError::Backend("the bridge went away".to_owned())),
        Ok(Reply {
            answer: RESOLVED_LIB.to_owned(),
            usage: None,
        }),
    ]);

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    assert_eq!(provider.model.seen().len(), 2, "the question is put twice");
    assert_eq!(provider.files.writes().len(), 1, "and the second answer lands");
    provider.model.assert_exhausted();
    provider.vcs.assert_exhausted();
}

// A resolution that carries a credential is refused before it is sealed,
// as what a build or a verification writes is.
#[tokio::test]
async fn build_conflict_secret() {
    let (provider, _) = conflicted_at_lib(&[
        r#"{"files": [{"path": "src/lib.rs", "content": "pub mod authentication;\npub mod orders;\npub const KEY: &str = \"AKIAIOSFODNN7EXAMPLE\";\n"}]}"#,
    ]);
    let copy = worktree("SLICE-003");
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "secret-detected")
            .await;

    assert_message(
        &envelope,
        "slice `SLICE-003` (orders) failed in wave 1; SLICE-001 merged before it stays committed \
         in `./.emery/vcs/integration`: what the merge's resolution wrote carries a credential, so \
         none of it is sealed:\n- src/lib.rs: an AWS access key id",
    );
    assert_eq!(provider.files.writes().len(), 1, "the resolution was written, and read back");
    let calls = provider.vcs.calls();
    assert!(
        !calls.contains(&format!("commit {copy} SLICE-003 orders resolved at src/lib.rs")),
        "nothing of it is sealed: {calls:?}"
    );
    left_for_inspection(&provider);
    provider.model.assert_exhausted();
    provider.vcs.assert_exhausted();
}

// A wide build whose `orders` slice conflicts with `authentication` at
// `src/lib.rs`, both copies holding it as a text file: `authentication`
// changed it beside its own module, and the model answers `answers`. What
// `orders` left changed, and what `sessions` writes in the wave after, is
// the scenario's to script.
fn conflicted_at_lib(answers: &[&str]) -> (Provider, String) {
    let provider = Provider::answering(answers.iter().copied());
    let id = seed(&provider.files, SPEC, DESIGN, WIDE_PLAN);
    let copy = worktree("SLICE-003");
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    provider.vcs.merges.script(
        INTEGRATION,
        Ok(Merged {
            commit: Some("m:SLICE-001".to_owned()),
            conflicts: Vec::new(),
        }),
    );
    provider.vcs.merges.script(INTEGRATION, Ok(conflicted(&["src/lib.rs"])));
    changed(
        &provider,
        "SLICE-001",
        &[("src/authentication.rs", ChangeKind::Added), ("src/lib.rs", ChangeKind::Modified)],
    );
    for at in [INTEGRATION, copy.as_str()] {
        provider.files.lists(at, ".", &[("src", EntryKind::Directory)]);
        provider.files.lists(at, "src", &[("lib.rs", EntryKind::File)]);
    }
    provider.files.holds(INTEGRATION, "src/lib.rs", "pub mod authentication;\n");
    provider.files.holds(&copy, "src/lib.rs", "pub mod orders;\n");
    (provider, id)
}

// The calls one slice's integration makes once its copy is cut: found where
// it was cut, its changes read, sealed, merged, and removed.
fn sealed((slice, name): (&str, &str)) -> Vec<String> {
    vec![
        format!("head {}", worktree(slice)),
        format!("pending {}", worktree(slice)),
        format!("commit {} {slice} {name}", worktree(slice)),
        format!("merge {INTEGRATION} {slice}-commit {slice} {name}"),
        format!("remove {}", worktree(slice)),
    ]
}

// A failed `orders` leaves its copy and the integration copy, with the wave
// neither verified nor labelled.
fn left_for_inspection(provider: &Provider) {
    let calls = provider.vcs.calls();
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");
    assert!(
        !calls.contains(&format!("remove {}", worktree("SLICE-003"))),
        "the slice's working copy stays for inspection: {calls:?}"
    );
    assert!(!calls.iter().any(|call| call.starts_with("label ")), "no label: {calls:?}");
    assert!(provider.target.verifies().is_empty(), "a wave that did not integrate is not verified");
}

// A wave the adapter does not verify ends the run as `verify-failed`,
// carrying each failing check; the label is not set after a first-wave
// failure and stays at the last verified head after a later one, and the
// integration working copy stays for inspection.
#[tokio::test]
async fn verify_verdict_fail() {
    let (provider, id) = planned();
    provider.target.verdict(Ok(verdict(false, &["cargo test: 1 failed", "clippy: 2 warnings"])));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "verify-failed").await;

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
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "verify-failed").await;
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
async fn verify_repair_turn() {
    let (mut provider, id) = planned();
    provider.target.rules = vec![MergeRule::ours("Cargo.lock")];
    let repaired = || {
        Ok(vec![
            change("src/orders.rs", ChangeKind::Modified),
            change("Cargo.lock", ChangeKind::Modified),
        ])
    };
    initialised(&provider, &[]);
    provider.vcs.pending.script(INTEGRATION, repaired());

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

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
    let sealed = calls
        .iter()
        .position(|call| call == &format!("commit {INTEGRATION} Wave 1 verified"))
        .expect("a seal");
    assert_eq!(
        calls[sealed - 1..sealed + 3],
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

    initialised(&provider, &[]);
    provider.vcs.pending.script(INTEGRATION, repaired());
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["repaired"], json!([1]), "{envelope}");
    assert_eq!(
        envelope["sealed"],
        json!([{"wave": 1, "commit": "Wave-commit", "paths": ["Cargo.lock", "src/orders.rs"]}]),
        "{envelope}"
    );
    assert_eq!(
        envelope["verified"],
        verified(&[(1, &["SLICE-001"]), (2, &["SLICE-002"])]),
        "{envelope}"
    );

    // a seal of regenerated paths alone is reported, and is no repair
    initialised(&provider, &[]);
    provider.vcs.pending.script(
        INTEGRATION,
        Ok(vec![
            change("Cargo.lock", ChangeKind::Modified),
            change("crates/orders/Cargo.lock", ChangeKind::Added),
        ]),
    );
    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("  wave 1: 1 slice verified at 9f8e7d6c, sealed\n"), "{stdout}");
    initialised(&provider, &[]);
    provider.vcs.pending.script(INTEGRATION, Ok(vec![change("Cargo.lock", ChangeKind::Modified)]));
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
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
async fn verify_bad_gateway() {
    let (provider, id) = planned();
    provider.target.verdict(Err(bad_gateway!("the model timed out")));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["label"], json!(format!("emery/{id}")), "{envelope}");
    assert_eq!(
        envelope["verified"],
        verified(&[(1, &["SLICE-001"]), (2, &["SLICE-002"])]),
        "{envelope}"
    );
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

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 3, "server_error").await;
    assert_message(&envelope, "`acme:builder` returned an invalid verdict");
    assert_message(&envelope, "- `passed` is true, yet 1 failure is listed");
    assert!(!provider.vcs.calls().iter().any(|call| call.starts_with("label ")));

    // the second upstream failure is the run's
    provider.target.verdict(Err(bad_gateway!("the model timed out")));
    provider.target.verdict(Err(bad_gateway!("the model timed out again")));
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 4, "bad_gateway").await;
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

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    assert_eq!(
        dispatched(&provider),
        [("SLICE-002".to_owned(), LABELLED.to_owned(), worktree("SLICE-002"))],
        "the labelled slices are not built again; the rest build over the labelled head"
    );
    let mut expected = project(HEAD);
    expected.extend([
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
        format!("log . {LABELLED} {HEAD}"),
        format!("add . {INTEGRATION} {LABELLED}"),
    ]);
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
    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 3 parts in 2 waves; budget 12 (default): 3 slices in 2 \
             waves, widest 2\n  base {HEAD}\n  resumed: SLICE-001, SLICE-003\n  wave 1: 1 slice \
             verified at 9f8e7d6c\n    SLICE-002 sessions: covered 1/1, written 1 file, merged \
             m:SLICE-002\n  labelled emery/{id} at {HEAD}\n"
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

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 2 parts in 2 waves; budget 12 (default): 2 slices in 2 \
             waves, widest 1\n  base {HEAD}\n  resumed: SLICE-001, SLICE-002\n  labelled \
             emery/{id} at {LABELLED}\n"
        )
    );
    assert!(provider.target.calls().is_empty(), "nothing to build");
    assert!(provider.target.verifies().is_empty(), "nothing to verify");
    let mut expected = project(HEAD);
    expected.extend([
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
        format!("log . {LABELLED} {HEAD}"),
        format!("add . {INTEGRATION} {LABELLED}"),
        format!("remove {INTEGRATION}"),
    ]);
    assert_eq!(provider.vcs.calls(), expected);
    provider.vcs.assert_exhausted();
}

// A build reads the current revision and never the candidate `specify`
// staged beside it: the candidate's plan is not built, the output names it
// as staged and not sealed, and both slots stand as they were.
#[tokio::test]
async fn build_candidate_staged() {
    let (provider, id) = planned();
    let candidate = stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    assert_ne!(candidate, id);
    // two runs, text then JSON, each resumed complete
    for _ in 0..2 {
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
    }
    let before = provider.storage.snapshot();

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 2 parts in 2 waves; budget 12 (default): 2 slices in 2 \
             waves, widest 1\n  candidate {candidate} is staged and not sealed; `emery apply` \
             seals it\n  base {HEAD}\n  resumed: SLICE-001, SLICE-002\n  labelled emery/{id} at \
             {LABELLED}\n"
        ),
        "the two-slice revision is built, not the three-slice candidate"
    );
    assert_eq!(provider.storage.snapshot(), before, "a build writes nothing to either slot");

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["candidate"], json!({"id": candidate, "kind": "revision"}), "{envelope}");
    assert!(provider.target.calls().is_empty(), "nothing to build");
    provider.vcs.assert_exhausted();
}

// A staged amendment is no candidate `emery apply` seals: a build over one
// names it as an amendment, for `emery show amendment` to review, and the
// envelope carries its kind.
#[tokio::test]
async fn build_amendment_staged() {
    let (provider, id) = planned();
    let amendment = stage_amendment(&provider.storage, AMENDMENT);
    for _ in 0..2 {
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
    }
    let before = provider.storage.snapshot();

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "built revision {id}\n  plan: 2 parts in 2 waves; budget 12 (default): 2 slices in 2 \
             waves, widest 1\n  amendment {amendment} is staged; `emery show amendment` reviews \
             it\n  base {HEAD}\n  resumed: SLICE-001, SLICE-002\n  labelled emery/{id} at \
             {LABELLED}\n"
        )
    );
    assert_eq!(provider.storage.snapshot(), before, "a build writes nothing to either slot");

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["candidate"], json!({"id": amendment, "kind": "amendment"}), "{envelope}");
    assert!(provider.target.calls().is_empty(), "nothing to build");
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

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

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
    let mut expected = project(HEAD);
    expected.extend([
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
        format!("add . {INTEGRATION} {HEAD}"),
        format!("pending {INTEGRATION}"),
    ]);
    expected.extend(built(".", HEAD, &id));
    assert_eq!(provider.vcs.calls(), expected, "no log over a label off the base");
    assert_eq!(provider.target.inits(), [INTEGRATION], "a fresh build initialises the tree");
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

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("resumed").is_none_or(|resumed| resumed == &json!([])), "{envelope}");
    assert_eq!(envelope["slices"].as_array().map(Vec::len), Some(2), "{envelope}");
    assert_eq!(
        dispatched(&provider)[0],
        ("SLICE-001".to_owned(), LABELLED.to_owned(), worktree("SLICE-001")),
        "the first wave builds over the labelled head"
    );
    let mut expected = project(HEAD);
    expected.extend([
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
        format!("log . {LABELLED} {HEAD}"),
        format!("add . {INTEGRATION} {LABELLED}"),
        format!("pending {INTEGRATION}"),
    ]);
    expected.extend(wave(".", &id, LABELLED, &[AUTHENTICATION]));
    expected.extend(wave(".", &id, HEAD, &[ORDERS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected);
    assert_eq!(provider.target.inits(), [INTEGRATION], "a label naming no slice is initialised");
    provider.vcs.assert_exhausted();
}

// Scripts the integration working copy an earlier run left sitting on
// `STAGED`, its first-parent chain back to the head holding `entries`.
fn staged(provider: &Provider, entries: Vec<Entry>) {
    provider.vcs.adds.script(INTEGRATION, Err(Error::Exists(INTEGRATION.to_owned())));
    provider.vcs.heads.script(INTEGRATION, Ok(STAGED.to_owned()));
    provider.vcs.logs.script(".", Ok(entries));
}

// The exchange that reads the integration copy an earlier run left and cuts
// it again at `at`: its head, whether it descends from the head, its chain,
// and the cut.
fn restaged(id: &str, at: &str) -> Vec<String> {
    vec![
        format!("labelled . emery/{id}"),
        format!("add . {INTEGRATION} {HEAD}"),
        format!("head {INTEGRATION}"),
        format!("descends . {HEAD} {STAGED}"),
        format!("log . {STAGED} {HEAD}"),
        format!("remove {INTEGRATION}"),
        format!("add . {INTEGRATION} {at}"),
    ]
}

// The integration working copy a run left after its wave failed to verify
// holds the wave's slices merged over the head: they are not built again,
// the copy is cut again at its head so what the verify left uncommitted
// goes and the merges stay, the wave is verified from there with no turn
// spent on a build, and the rest of the plan builds over it.
#[tokio::test]
async fn build_resumed_staged() {
    let (provider, id) = wide();
    let chain = || {
        vec![
            entry(
                "m:SLICE-003",
                &message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
            ),
            entry("m:SLICE-001", &message(&id, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
        ]
    };
    staged(&provider, chain());
    wrote(&provider, WIDE_SESSIONS);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    assert_eq!(
        dispatched(&provider),
        [("SLICE-002".to_owned(), STAGED.to_owned(), worktree("SLICE-002"))],
        "the staged slices are not built again; the rest build over the staged head"
    );
    assert_eq!(
        provider.target.verifies(),
        [INTEGRATION, INTEGRATION],
        "the staged wave is verified"
    );
    let mut expected = project(HEAD);
    expected.extend(restaged(&id, STAGED));
    expected.extend([
        format!("pending {INTEGRATION}"),
        format!("head {INTEGRATION}"),
        format!("label . emery/{id} {STAGED}"),
    ]);
    expected.extend(wave(".", &id, STAGED, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected);
    assert_eq!(
        provider.vcs.messages(),
        [message(&id, WIDE_SESSIONS, "REQ-002", "REQ-002", STAGED, 2)],
        "the one slice built is sealed over the staged head"
    );
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["base"], HEAD, "{envelope}");
    assert_eq!(envelope["resumed"], json!(["SLICE-001", "SLICE-003"]), "{envelope}");
    assert_eq!(
        envelope["slices"],
        json!([{"id": "SLICE-002", "part": "PART-002", "name": "sessions", "index": 1, "count": 1, "wave": 2, "covered": ["REQ-002"], "uncovered": [], "written": ["src/sessions.rs"], "commit": "m:SLICE-002"}]),
        "{envelope}"
    );
    assert_eq!(
        envelope["verified"],
        json!([
            {"wave": 1, "slices": ["SLICE-001", "SLICE-003"], "head": STAGED},
            {"wave": 2, "slices": ["SLICE-002"], "head": HEAD},
        ]),
        "the staged wave is verified at the staged head: {envelope}"
    );
    assert_eq!(envelope["head"], HEAD, "{envelope}");

    staged(&provider, chain());
    wrote(&provider, WIDE_SESSIONS);
    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 3 parts in 2 waves; budget 12 (default): 3 slices in 2 \
             waves, widest 2\n  base {HEAD}\n  resumed: SLICE-001, SLICE-003\n  wave 1: 2 slices \
             verified at 5a6b7c8d\n    SLICE-001: staged by an earlier run, not built again\n    \
             SLICE-003: staged by an earlier run, not built again\n  wave 2: 1 slice verified at \
             9f8e7d6c\n    SLICE-002 sessions: covered 1/1, written 1 file, merged m:SLICE-002\n  \
             labelled emery/{id} at {HEAD}\n"
        )
    );
    provider.vcs.assert_exhausted();
}

// A slice whose build fails for good ends the run only once the slices
// built beside it are merged, so they stay committed in the integration
// working copy and are named as such; the next run finds them staged there,
// builds the failed slice alone over the staged head, and verifies the wave
// whole.
#[tokio::test]
async fn resume_staged_sibling() {
    let (provider, id) = wide();
    provider.target.fails("SLICE-003", bad_gateway!("the model timed out"));
    provider.target.fails("SLICE-003", bad_gateway!("the model timed out again"));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 4, "bad_gateway").await;

    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-003` (orders) failed in wave 1; SLICE-001 merged before it stays \
             committed in `{INTEGRATION}`: the model timed out again"
        ),
    );
    assert_eq!(
        provider.vcs.messages(),
        [message(&id, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)],
        "the sibling is sealed before the failure ends the run"
    );
    let merged: Vec<String> =
        provider.vcs.merged().into_iter().map(|(message, _)| message).collect();
    assert_eq!(merged, provider.vcs.messages(), "and merged");
    assert!(provider.target.verifies().is_empty(), "a wave left unbuilt is not verified");
    let calls = provider.vcs.calls();
    assert!(!calls.iter().any(|call| call.starts_with("label ")), "no label: {calls:?}");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left to resume: {calls:?}");

    // the next run builds the failed slice alone, over the staged sibling
    staged(
        &provider,
        vec![entry("m:SLICE-001", &message(&id, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1))],
    );
    wrote(&provider, WIDE_ORDERS);
    wrote(&provider, WIDE_SESSIONS);
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        dispatched(&provider)[3..],
        [
            ("SLICE-003".to_owned(), STAGED.to_owned(), worktree("SLICE-003")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ],
        "the failed slice builds over the staged head, the next wave over the verified one"
    );
    let mut expected = project(HEAD);
    expected.extend(restaged(&id, STAGED));
    expected.extend(wave(".", &id, STAGED, &[WIDE_ORDERS]));
    expected.extend(wave(".", &id, HEAD, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(calls_since(&provider, calls.len()), expected);
    assert_eq!(
        provider.vcs.messages()[1],
        message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", STAGED, 1),
        "the failed slice is sealed over the staged head, in the wave it was staged for"
    );
    assert_eq!(envelope["resumed"], json!(["SLICE-001"]), "{envelope}");
    assert_eq!(
        envelope["verified"],
        verified(&[(1, &["SLICE-001", "SLICE-003"]), (2, &["SLICE-002"])]),
        "the wave is verified whole: {envelope}"
    );
    assert_eq!(envelope["slices"][0]["id"], "SLICE-003", "{envelope}");
    assert_eq!(envelope["slices"][0]["wave"], 1, "{envelope}");
    provider.vcs.assert_exhausted();
}

// An integration working copy whose head does not descend from the head
// holds another base's work: its chain is never read, and it is cut again
// at the head with nothing resumed.
#[tokio::test]
async fn stale_off_head() {
    let (provider, id) = planned();
    provider.vcs.adds.script(INTEGRATION, Err(Error::Exists(INTEGRATION.to_owned())));
    provider.vcs.heads.script(INTEGRATION, Ok(STAGED.to_owned()));
    provider.vcs.descends.script(".", Ok(false));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("resumed").is_none(), "{envelope}");
    assert_eq!(envelope["slices"].as_array().map(Vec::len), Some(2), "{envelope}");
    let calls = provider.vcs.calls();
    assert_eq!(
        calls[4..10],
        [
            format!("labelled . emery/{id}"),
            format!("add . {INTEGRATION} {HEAD}"),
            format!("head {INTEGRATION}"),
            format!("descends . {HEAD} {STAGED}"),
            format!("remove {INTEGRATION}"),
            format!("add . {INTEGRATION} {HEAD}"),
        ],
        "{calls:?}"
    );
    assert_eq!(
        dispatched(&provider)[0],
        ("SLICE-001".to_owned(), HEAD.to_owned(), worktree("SLICE-001")),
        "every slice builds over the head"
    );
    provider.vcs.assert_exhausted();
}

// An integration working copy holding a slice of another revision over the
// head is another plan's work, and one whose chain names no slice holds
// nothing to resume: each is cut again at the head with nothing resumed.
#[tokio::test]
async fn stale_other_revision() {
    let (provider, id) = planned();
    staged(
        &provider,
        vec![
            entry("m:SLICE-002", &message(&id, ORDERS, "REQ-003, REQ-004", "REQ-003", HEAD, 2)),
            entry(
                "m:SLICE-001",
                &message("other-revision", AUTHENTICATION, "REQ-001, REQ-002", "REQ-001", HEAD, 1),
            ),
        ],
    );

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("resumed").is_none(), "{envelope}");
    assert_eq!(envelope["slices"].as_array().map(Vec::len), Some(2), "{envelope}");
    let calls = provider.vcs.calls();
    assert_eq!(calls[4..11], restaged(&id, HEAD), "{calls:?}");
    assert_eq!(
        dispatched(&provider)[0],
        ("SLICE-001".to_owned(), HEAD.to_owned(), worktree("SLICE-001")),
        "every slice builds over the head"
    );

    // a chain that seals no slice
    staged(
        &provider,
        vec![entry(
            "w1",
            &format!("Wave 1 verified\n\nRevision: {id}\nAdapter: {BUILDER}\nSlices: SLICE-001"),
        )],
    );
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("resumed").is_none(), "{envelope}");
    assert_eq!(calls_since(&provider, calls.len())[4..11], restaged(&id, HEAD));
    assert_eq!(dispatched(&provider).len(), 4, "every slice builds, each run");
    provider.vcs.assert_exhausted();
}

// The `Vcs` calls a run made after the first `skip`.
fn calls_since(provider: &Provider, skip: usize) -> Vec<String> {
    provider.vcs.calls().into_iter().skip(skip).collect()
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
    expected.extend(read_revision(TARGET_REPO, "origin/HEAD-commit"));
    expected.extend(integrated(TARGET_REPO, "origin/HEAD-commit", &id));
    assert_eq!(
        provider.vcs.calls(),
        expected,
        "cloned, the revision read from the default branch, then built from it"
    );
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
// or created; one that is not kebab-case is refused before anything is read.
#[tokio::test]
async fn build_named() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[target]\nname = \"shop\"\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"
    ));
    let (provider, id) = planned();

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", &config]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["repository"], "./.emery/vcs/repos/shop", "{envelope}");
    assert_eq!(provider.vcs.calls()[0], format!("clone {REMOTE_URL} ./.emery/vcs/repos/shop"));
    assert!(
        provider.vcs.calls().contains(&format!("label ./.emery/vcs/repos/shop emery/{id} {HEAD}"))
    );

    // the repository `apply` created is read under the same name, never created here
    let config = scratch.config(&format!("[target]\nname = \"shop\"\nadapter = \"{BUILDER}\"\n"));
    cli_ok(&provider, &["emery", "build", "--config", &config]).await;
    assert!(provider.vcs.calls().contains(&"head ./.emery/vcs/repos/shop".to_owned()));
    assert!(!provider.vcs.calls().iter().any(|call| call.starts_with("init ")), "nothing created");

    let config = scratch.config(&format!("[target]\nname = \"Shop\"\nadapter = \"{BUILDER}\"\n"));
    let before = provider.vcs.calls().len();
    let envelope =
        fail(&provider, &["emery", "build", "--config", &config], 1, "bad_request").await;
    assert_message(&envelope, "target `Shop` is not a kebab-case name");
    assert_eq!(provider.vcs.calls().len(), before, "nothing is read for a refused name");
    provider.vcs.assert_exhausted();
}

// A command-line adapter layers over the `[target]` table: it takes the
// place of the file's `adapter` under the file's `name` and `repository`,
// the `digest` that pinned the file's release going with the adapter it
// pinned.
#[tokio::test]
async fn layered_target() {
    let scratch = Scratch::new();
    let pin = digest("cd");
    let config = scratch.config(&format!(
        "[target]\nname = \"shop\"\nadapter = \"acme:other@1.0.0\"\ndigest = \"{pin}\"\n\
         repository = \"{REMOTE_URL}\"\n"
    ));
    let (provider, id) = planned();

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER, "--config", &config])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["repository"], "./.emery/vcs/repos/shop", "{envelope}");
    assert_eq!(
        *provider.target.metadata.lock().expect("metadata"),
        [BUILDER_ID],
        "the command-line adapter is the one gated"
    );
    assert_eq!(provider.vcs.calls()[0], format!("clone {REMOTE_URL} ./.emery/vcs/repos/shop"));
    assert!(
        provider.vcs.calls().contains(&format!("label ./.emery/vcs/repos/shop emery/{id} {HEAD}"))
    );
    provider.vcs.assert_exhausted();
}

// `--repository` names where the build lands: the project checkout for a run
// naming its adapter on the command line, which would otherwise create a
// repository of its own, and the clone in place of the one the file names.
#[tokio::test]
async fn repository_flag() {
    let (provider, id) = planned();
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    let resp = cli_ok(&provider, &["emery", "build", BUILDER, "--repository", "."]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains(&format!("\n  base {BASE}\n  wave 1:")), "{stdout}");
    let mut expected = project(BASE);
    expected.extend(integrated(".", BASE, &id));
    assert_eq!(provider.vcs.calls(), expected, "built into the project checkout, nothing created");

    // over a file, the flag's URL is cloned in place of the file's
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[target]\nadapter = \"{BUILDER}\"\nrepository = \"https://example.com/acme/old.git\"\n"
    ));
    let before = provider.vcs.calls().len();
    cli_ok(&provider, &["emery", "build", "--config", &config, "--repository", REMOTE_URL]).await;
    assert_eq!(provider.vcs.calls()[before], format!("clone {REMOTE_URL} {TARGET_REPO}"));
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

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

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
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["slices"][0]["commit"], Value::Null, "{envelope}");
    assert_eq!(envelope["slices"][1]["commit"], "m:SLICE-002", "{envelope}");
    provider.vcs.assert_exhausted();
}

// The working copy a failed run left is removed and cut again, so a build
// never resumes from what an earlier one wrote outside the label: an
// integration copy still sitting on the head holds nothing to resume, and
// its chain is never read; a slice's copy is never read at all.
#[tokio::test]
async fn build_stale_integration() {
    let (provider, _) = planned();
    provider.vcs.adds.script(INTEGRATION, Err(Error::Exists(INTEGRATION.to_owned())));
    provider.vcs.adds.script(&worktree("SLICE-001"), Err(Error::Exists(worktree("SLICE-001"))));

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let calls = provider.vcs.calls();
    assert_eq!(
        calls[5..13],
        [
            format!("add . {INTEGRATION} {HEAD}"),
            format!("head {INTEGRATION}"),
            format!("remove {INTEGRATION}"),
            format!("add . {INTEGRATION} {HEAD}"),
            format!("pending {INTEGRATION}"),
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
async fn malformed_reference() {
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

// The revision is read from the repository before the adapter loads, so a
// repository holding none refuses typed and nothing is asked of the loader:
// the head's tree is read in a working copy cut for it and removed, and
// nothing else is touched.
#[tokio::test]
async fn build_no_revision() {
    let provider = Provider::idle();

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 2, "spec-not-generated")
            .await;
    assert_message(&envelope, "no specification revision is sealed in the repository");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery specify")),
        "the hint names the way out: {envelope}"
    );
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
    assert_eq!(
        provider.vcs.calls(),
        project(HEAD)[..3],
        "the head's tree is read and nothing else"
    );
    assert_eq!(
        provider.files.listed(),
        [(REVISION.to_owned(), ".".to_owned())],
        "a tree with no `spec/` is read no further"
    );

    // a candidate staged and not sealed is no revision: the refusal names it
    let candidate = stage(&provider.storage, SPEC, DESIGN, PLAN);
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 2, "spec-not-generated")
            .await;
    assert_message(&envelope, &format!("candidate `{candidate}` is staged and not sealed"));
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery apply")),
        "the hint names the seal: {envelope}"
    );
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");

    // a repository `apply` has not created yet holds none either, and the
    // build creates nothing
    provider.vcs.heads.script(TARGET_REPO, Err(Error::NotARepository));
    let envelope = fail(&provider, &["emery", "build", BUILDER], 2, "spec-not-generated").await;
    assert_message(&envelope, "no specification revision is sealed in the repository");
    assert!(!provider.vcs.calls().iter().any(|call| call.starts_with("init ")), "nothing created");
    provider.vcs.assert_exhausted();
}

// The pointer the engine keeps names a label; one that no longer descends
// from the head, the branch having moved on without it, is no revision, and
// the refusal names the label so the operator merges or rebases it.
#[tokio::test]
async fn build_label_behind() {
    let (provider, id) = planned();
    provider.storage.insert_state(REVISION_KEY, id.as_bytes());
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.descends.script(".", Ok(false));
    provider.files.lists(REVISION, ".", &[]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 2, "spec-not-generated")
            .await;

    assert_message(&envelope, &format!("label `emery/{id}` does not stand over its head"));
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("merge or rebase")),
        "{envelope}"
    );
    let mut expected = vec![
        "head .".to_owned(),
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
    ];
    expected.extend(read_revision(".", HEAD));
    assert_eq!(provider.vcs.calls(), expected, "the label passed over, the head's tree read");
    provider.vcs.assert_exhausted();
}

// The pointer names a label standing over the head: the revision is read at
// the label's commit, which is the build's resumed head, and never from the
// head's own tree.
#[tokio::test]
async fn build_labelled_revision() {
    let (provider, id) = planned();
    provider.storage.insert_state(REVISION_KEY, id.as_bytes());
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    let before = provider.storage.snapshot();

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["base"], HEAD, "the base is the head, not the label");
    assert!(envelope.get("resumed").is_none(), "the label's chain names no slice: {envelope}");
    let mut expected = vec![
        "head .".to_owned(),
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
        format!("add . {REVISION} {LABELLED}"),
        format!("remove {REVISION}"),
        "pending .".to_owned(),
        format!("labelled . emery/{id}"),
        format!("descends . {HEAD} {LABELLED}"),
        format!("log . {LABELLED} {HEAD}"),
        format!("add . {INTEGRATION} {LABELLED}"),
        format!("pending {INTEGRATION}"),
    ];
    expected.extend(wave(".", &id, LABELLED, &[AUTHENTICATION]));
    expected.extend(wave(".", &id, HEAD, &[ORDERS]));
    expected.push(format!("remove {INTEGRATION}"));
    assert_eq!(provider.vcs.calls(), expected, "read at the label, built over it");
    assert_eq!(provider.target.inits(), [INTEGRATION], "a label naming no slice is initialised");
    assert_eq!(provider.storage.snapshot(), before, "the pointer stands");
    provider.vcs.assert_exhausted();
}

// A `spec/` the label's tree holds that is not the revision the label
// names, a document edited by hand, is refused rather than served; so is a
// `spec/` of the operator's own in a repository holding no label.
#[tokio::test]
async fn build_spec_mismatch() {
    let (provider, id) = planned();
    provider.storage.insert_state(REVISION_KEY, id.as_bytes());
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.files.holds(REVISION, "spec/plan.json", "{}\n");

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "spec-mismatch").await;
    assert_message(
        &envelope,
        &format!(
            "`spec/` at commit `{LABELLED}`, labelled `emery/{id}`, does not match revision `{id}`"
        ),
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("git checkout emery/")),
        "{envelope}"
    );
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
    assert_eq!(provider.vcs.calls().last(), Some(&format!("remove {REVISION}")), "the copy goes");

    // the operator's own `spec/`, in a repository with no label
    let provider = Provider::idle();
    provider.files.lists(REVISION, ".", &[("spec", EntryKind::Directory)]);
    provider.files.lists(REVISION, "spec", &[("openapi.yaml", EntryKind::File)]);
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "spec-mismatch").await;
    assert_message(
        &envelope,
        &format!("`spec/` at commit `{HEAD}` holds `openapi.yaml`, where the three documents"),
    );
    provider.vcs.assert_exhausted();
}

#[tokio::test]
async fn build_outdated() {
    let provider = Provider::idle();
    seed(
        &provider.files,
        br#"{"emery": 1, "requirements": []}"#,
        br#"{"emery": 1, "sections": []}"#,
        br#"{"emery": 1, "slices": []}"#,
    );

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "spec-outdated").await;
    assert_message(&envelope, "grammar 1");
    assert!(provider.target.metadata.lock().expect("metadata").is_empty(), "nothing loads");
}

// The target's `emery-version` pin is gated as a source adapter's is, before
// any slice is dispatched and before the base is settled: the revision is
// read, and nothing else is asked of version control.
#[tokio::test]
async fn unsupported_version() {
    let (mut provider, _) = planned();
    provider.target.versions.insert(BUILDER_ID.to_string(), "99.0.0".to_string());

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "unsupported-version")
            .await;
    assert_message(&envelope, "adapter `acme:builder` requires emery 99.0.0 or newer");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
    assert_eq!(provider.vcs.calls(), project(HEAD)[..3], "the base is not settled");
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

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "base-not-sealed")
            .await;
    assert_message(&envelope, "the project checkout holds pending changes: src/main.rs, notes.md");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("`.gitignore`")),
        "{envelope}"
    );
    assert_eq!(provider.vcs.calls(), project(HEAD), "nothing past the check");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");

    // pending changes beneath `.emery/` alone leave the base sealed
    provider
        .vcs
        .pending
        .script(".", Ok(vec![change(".emery/storage/revision.json", ChangeKind::Added)]));
    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;
    assert_eq!(provider.target.calls().len(), 2);

    // a repository with no commit yet holds no revision either
    provider.vcs.heads.script(".", Err(Error::NotFound("HEAD".to_owned())));
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 2, "spec-not-generated")
            .await;
    assert_message(
        &envelope,
        "no specification revision is sealed in the repository, which has no commit",
    );
    assert_eq!(provider.vcs.calls().last(), Some(&"head .".to_owned()), "nothing is cut");
    provider.vcs.assert_exhausted();
}

// A build lands as a commit, so a project directory that is no repository
// is refused before any slice when the target names it, with the ways out:
// a repository URL, or no `repository` at all.
#[tokio::test]
async fn build_not_repository() {
    let (provider, _) = planned();
    provider.vcs.heads.script(".", Err(Error::NotARepository));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "repository-required")
            .await;
    assert_message(&envelope, "the project directory is not a repository");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| {
            hint.contains("`[target] repository`") && hint.contains("drop the key")
        }),
        "{envelope}"
    );
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
}

// A build naming no repository, as one naming its adapter on the command
// line does, builds into the one `apply` created under the target's name
// and sealed the revision in: read from its head, the project directory
// never read, nothing created. The adapter initialises the tree over that
// commit, given a `.gitignore` of the engine's own directory where the
// adapter scaffolded none, since the repository is the engine's, and the
// envelope names the repository beside the base.
#[tokio::test]
async fn build_laid() {
    let (provider, id) = planned();
    provider.vcs.heads.script(TARGET_REPO, Ok(BASE.to_owned()));
    initialised(&provider, &["Cargo.toml"]);

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["base"], BASE, "{envelope}");
    assert_eq!(envelope["repository"], TARGET_REPO, "{envelope}");
    assert_eq!(
        envelope["initialised"],
        json!({"commit": "Initialise-commit", "paths": [".gitignore", "Cargo.toml"]}),
        "the layout and the engine's ignore file are sealed together: {envelope}"
    );
    assert_eq!(
        provider.files.writes(),
        [(INTEGRATION.to_owned(), ".gitignore".to_owned(), b".emery/\n".to_vec())],
        "the one file the engine adds to the initialisation"
    );
    assert_eq!(provider.target.inits(), [INTEGRATION], "the adapter scaffolds its layout");
    assert_eq!(provider.vcs.messages()[0], "Initialise");
    let mut expected = vec![format!("head {TARGET_REPO}")];
    expected.extend(read_revision(TARGET_REPO, BASE));
    expected.extend([
        format!("labelled {TARGET_REPO} emery/{id}"),
        format!("add {TARGET_REPO} {INTEGRATION} {BASE}"),
        format!("pending {INTEGRATION}"),
        format!("commit {INTEGRATION} Initialise"),
    ]);
    expected.extend(built(TARGET_REPO, "Initialise-commit", &id));
    assert_eq!(provider.vcs.calls(), expected, "read, initialised, then built as any repository");
    assert!(!provider.vcs.calls().iter().any(|call| call.starts_with("init ")), "nothing created");
    assert!(!provider.vcs.calls().contains(&"pending .".to_owned()), "the project is never read");
    assert_eq!(provider.target.calls().len(), 2, "every slice builds");

    // an adapter whose layout holds a `.gitignore` is given none
    provider.vcs.heads.script(TARGET_REPO, Ok(BASE.to_owned()));
    initialised(&provider, &[".gitignore", "Cargo.toml"]);
    cli_ok(&provider, &["emery", "build", BUILDER]).await;
    assert_eq!(provider.files.writes().len(), 1, "the adapter's `.gitignore` stands");
    assert_eq!(provider.vcs.messages()[3], "Initialise");

    // so is one whose tree already holds one
    provider.vcs.heads.script(TARGET_REPO, Ok(BASE.to_owned()));
    provider.files.lists(INTEGRATION, ".", &[(".gitignore", EntryKind::File)]);
    initialised(&provider, &["Cargo.toml"]);
    cli_ok(&provider, &["emery", "build", BUILDER]).await;
    assert_eq!(provider.files.writes().len(), 1, "the held `.gitignore` stands");
    provider.vcs.assert_exhausted();
}

// A fresh build over a base the engine did not create has the adapter
// initialise the integration copy before any slice: what the layout adds is
// sealed as `Initialise`, the commit every slice builds over, and reported
// with its paths. The base is never read for what it holds, since the
// adapter scaffolds what the tree lacks.
#[tokio::test]
async fn build_initialised() {
    let scratch = Scratch::new();
    let config = scratch
        .config(&format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"));
    let (provider, id) = planned();
    provider.vcs.resolves.script(TARGET_REPO, Ok(BASE.to_owned()));
    initialised(&provider, &["src/lib.rs", "Cargo.toml"]);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", &config]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["base"], BASE, "{envelope}");
    assert_eq!(
        envelope["initialised"],
        json!({"commit": "Initialise-commit", "paths": ["Cargo.toml", "src/lib.rs"]}),
        "{envelope}"
    );
    assert_eq!(provider.target.inits(), [INTEGRATION]);
    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), "Initialise-commit".to_owned(), worktree("SLICE-001")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ],
        "the first wave builds over the initialised tree"
    );
    let mut expected = vec![
        format!("clone {REMOTE_URL} {TARGET_REPO}"),
        format!("resolve {TARGET_REPO} origin/HEAD"),
    ];
    expected.extend(read_revision(TARGET_REPO, BASE));
    expected.extend([
        format!("labelled {TARGET_REPO} emery/{id}"),
        format!("add {TARGET_REPO} {INTEGRATION} {BASE}"),
        format!("pending {INTEGRATION}"),
        format!("commit {INTEGRATION} Initialise"),
    ]);
    expected.extend(built(TARGET_REPO, "Initialise-commit", &id));
    assert_eq!(provider.vcs.calls(), expected, "initialised, sealed, then built");
    assert!(listed(&provider).is_empty(), "the base is read for the revision alone");
    assert_eq!(
        reads(&provider),
        [
            (INTEGRATION.to_owned(), "src/lib.rs".to_owned()),
            (INTEGRATION.to_owned(), "Cargo.toml".to_owned()),
        ],
        "what the layout scaffolded is held to the secret gate"
    );
    assert_eq!(
        provider.vcs.messages()[..2],
        [
            "Initialise".to_owned(),
            message(
                &id,
                AUTHENTICATION,
                "REQ-001, REQ-002",
                "REQ-001, REQ-002",
                "Initialise-commit",
                1
            ),
        ]
    );
    assert!(
        provider.files.writes().is_empty(),
        "the engine writes nothing in a repository it holds"
    );

    provider.vcs.clones.script(TARGET_REPO, Err(Error::Exists(TARGET_REPO.to_owned())));
    provider.vcs.resolves.script(TARGET_REPO, Ok(BASE.to_owned()));
    initialised(&provider, &["src/lib.rs", "Cargo.toml"]);
    let resp = cli_ok(&provider, &["emery", "build", "--config", &config]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        stdout.contains(&format!(
            "\n  base {BASE}\n  repository {TARGET_REPO}\n  initialised Initiali, 2 files\n  wave 1: "
        )),
        "{stdout}"
    );
    provider.vcs.assert_exhausted();
}

// A credential the adapter's initialisation scaffolds refuses the build before
// anything is sealed or any slice dispatched, naming the initialisation.
#[tokio::test]
async fn build_init_secret() {
    let (provider, _) = planned();
    initialised(&provider, &[".npmrc", "package.json"]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "secret-detected")
            .await;

    assert_message(
        &envelope,
        "what the initialisation wrote carries a credential, so none of it is sealed:\n- .npmrc: a \
         file that carries a credential by name",
    );
    assert_eq!(provider.target.inits(), [INTEGRATION]);
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    let calls = provider.vcs.calls();
    assert_eq!(calls.last().cloned(), Some(format!("pending {INTEGRATION}")), "{calls:?}");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");
    provider.vcs.assert_exhausted();
}

// A build resumed from a label in the repository the engine created reads the
// label there and starts from it, the slices its chain names not built
// again and nothing initialised again.
#[tokio::test]
async fn build_laid_resumed() {
    let (provider, id) = planned();
    provider.vcs.heads.script(TARGET_REPO, Ok(BASE.to_owned()));
    provider.vcs.labelleds.script(TARGET_REPO, Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        TARGET_REPO,
        Ok(vec![entry(
            "m:SLICE-001",
            &message(&id, AUTHENTICATION, "REQ-001, REQ-002", "REQ-001, REQ-002", HEAD, 1),
        )]),
    );

    let resp = cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["resumed"], json!(["SLICE-001"]), "{envelope}");
    assert!(provider.target.inits().is_empty(), "nothing initialised again");
    assert_eq!(
        dispatched(&provider),
        [("SLICE-002".to_owned(), LABELLED.to_owned(), worktree("SLICE-002"))]
    );
    assert!(
        provider.vcs.calls().contains(&format!("descends {TARGET_REPO} {BASE} {LABELLED}")),
        "the label is read in the created repository"
    );
    assert!(provider.files.writes().is_empty(), "nothing written to a resumed tree");
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
    plan["parts"][0]["depends-on"] = json!(["PART-002"]);
    let provider = Provider::idle();
    seed(&provider.files, SPEC, DESIGN, &serde_json::to_vec(&plan).expect("a plan"));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 3, "server_error").await;

    assert_message(&envelope, "the slices left to build wait on one another: SLICE-001, SLICE-002");
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
}

// A target's refusal of a slice keeps its class and code, named for the
// slice and its wave; the slices after it are never dispatched, no label is
// set, and the working copies stay for inspection.
#[tokio::test]
async fn target_error() {
    let (mut provider, _) = planned();
    provider
        .target
        .reports
        .insert("SLICE-001".to_string(), Err(bad_request!("the tree already holds `src/`")));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "bad_request").await;
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

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 3, "server_error").await;
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
async fn worktree_head_moved() {
    let (provider, _) = planned();
    let moved = "feedface0123456789abcdef0123456789abcdef";
    provider.vcs.heads.script(&worktree("SLICE-001"), Ok(moved.to_owned()));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 3, "server_error").await;

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
async fn slice_bad_gateway() {
    let (provider, id) = planned();
    provider.target.fails("SLICE-001", bad_gateway!("the model timed out"));
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ],
        "the failed build is put again over the same head"
    );
    let mut expected = project(BASE);
    expected.extend(opened(".", BASE, &id));
    expected.extend([
        format!("add . {} {BASE}", worktree("SLICE-001")),
        format!("remove {}", worktree("SLICE-001")),
    ]);
    expected.extend(built(".", BASE, &id));
    assert_eq!(provider.vcs.calls(), expected, "the first copy goes before the second is cut");
    assert_eq!(provider.vcs.messages().len(), 2, "one commit per slice");
    provider.vcs.assert_exhausted();
}

// How the model backend reports a turn its time budget ended: a refusal,
// not an upstream failure, yet one the engine puts once more.
const TIMED_OUT: &str = "adapter `acme:builder`: budget exhausted: cursor run timed out after 600s \
                         (absolute cap exceeded while still active)";
// A refusal of the same class the engine never puts again.
const SPENT: &str = "adapter `acme:builder`: budget exhausted: the correction rounds are spent";

// A build the backend's time budget ended is put once more in a fresh copy,
// as one that failed upstream is, and the run lands; a refusal of the same
// class that is no timeout is the run's at once, and the hint names the
// resume for the second timeout alone.
#[tokio::test]
async fn slice_timeout() {
    let (provider, id) = planned();
    provider.target.fails("SLICE-001", bad_request!("{TIMED_OUT}"));
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        ],
        "the timed-out build is put again over the same head"
    );
    let calls = provider.vcs.calls();
    assert_eq!(
        calls[7..9],
        [
            format!("add . {} {BASE}", worktree("SLICE-001")),
            format!("remove {}", worktree("SLICE-001"))
        ],
        "the first copy goes before the second is cut: {calls:?}"
    );
    assert_eq!(provider.vcs.messages().len(), 2, "one commit per slice");

    // a refusal that is no timeout is not put again
    provider.target.fails("SLICE-001", bad_request!("{SPENT}"));
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "bad_request").await;
    assert_message(
        &envelope,
        &format!("slice `SLICE-001` (authentication) failed in wave 1: {SPENT}"),
    );
    assert!(envelope.get("hint").is_none(), "{envelope}");
    assert_eq!(provider.target.calls().len(), 4, "put once");

    // the second timeout is the run's, hinted
    provider.target.fails("SLICE-001", bad_request!("{TIMED_OUT}"));
    provider.target.fails("SLICE-001", bad_request!("{TIMED_OUT}"));
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "bad_request").await;
    assert_message(
        &envelope,
        &format!("slice `SLICE-001` (authentication) failed in wave 1: {TIMED_OUT}"),
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("re-run `emery build`")),
        "the hint names the resume: {envelope}"
    );
    assert_eq!(provider.target.calls().len(), 6, "put twice, no more");
    assert_eq!(provider.vcs.messages().len(), 2, "nothing more is sealed");
    assert_eq!(
        calls_since(&provider, calls.len())
            .iter()
            .filter(|call| call.starts_with("label "))
            .count(),
        0,
        "no label after either refusal"
    );
    let label = format!("label . emery/{id} {HEAD}");
    assert_eq!(calls.iter().filter(|call| **call == label).count(), 2, "the first run's two");
    provider.vcs.assert_exhausted();
}

// A verify the backend's time budget ended is put once more over the same
// integration copy, as one that failed upstream is, so what the first
// attempt built under `target/` and repaired stands; a refusal of the same
// class that is no timeout is the wave's at once.
#[tokio::test]
async fn verify_timeout() {
    let (provider, id) = planned();
    provider.target.verdict(Err(bad_request!("{TIMED_OUT}")));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["label"], json!(format!("emery/{id}")), "{envelope}");
    assert_eq!(
        envelope["verified"],
        verified(&[(1, &["SLICE-001"]), (2, &["SLICE-002"])]),
        "{envelope}"
    );
    assert_eq!(
        provider.target.verifies(),
        [INTEGRATION, INTEGRATION, INTEGRATION],
        "the first wave verified twice, the second once"
    );
    let calls = provider.vcs.calls();
    assert_eq!(
        calls.iter().filter(|call| call.starts_with(&format!("add . {INTEGRATION}"))).count(),
        1,
        "the copy is cut once: {calls:?}"
    );

    // a refusal that is no timeout is the wave's at once
    provider.target.verdict(Err(bad_request!("{SPENT}")));
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "bad_request").await;
    assert_message(
        &envelope,
        &format!(
            "wave 1 failed; SLICE-001 merged in it stays committed in `{INTEGRATION}`; \
             `emery/{id}` is not set: {SPENT}"
        ),
    );
    assert_eq!(provider.target.verifies().len(), 4, "put once");

    // the second timeout is the wave's
    provider.target.verdict(Err(bad_request!("{TIMED_OUT}")));
    provider.target.verdict(Err(bad_request!("{TIMED_OUT}")));
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "bad_request").await;
    assert_message(
        &envelope,
        &format!("wave 1 failed; SLICE-001 merged in it stays committed in `{INTEGRATION}`"),
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("re-run `emery build`")),
        "{envelope}"
    );
    assert_eq!(provider.target.verifies().len(), 6, "put twice, no more");
    provider.vcs.assert_exhausted();
}

// A failure after a wave was verified names what stays merged, and where;
// the label stays where that wave left it. A second upstream failure of
// one slice is the run's.
#[tokio::test]
async fn wave_slice_error() {
    let (mut provider, id) = planned();
    provider
        .target
        .reports
        .insert("SLICE-002".to_string(), Err(bad_gateway!("the model timed out")));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 4, "bad_gateway").await;
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

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "secret-detected")
            .await;

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
        reads(&provider),
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

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "secret-detected")
            .await;

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

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "secret-detected")
            .await;

    assert_message(
        &envelope,
        "- deploy/.npmrc: a file that carries a credential by name\n- ops/.aws/credentials: a file \
         that carries a credential by name",
    );
    assert!(!envelope["message"].as_str().unwrap_or("").contains("id_rsa.pub"), "{envelope}");
    assert_eq!(
        reads(&provider),
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
    initialised(&provider, &[]);
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

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "secret-detected")
            .await;

    assert_message(
        &envelope,
        &format!(
            "wave 1 failed; SLICE-001 merged in it stays committed in `{INTEGRATION}`; \
             `emery/{id}` is not set: what the verification wrote carries a credential, so none \
             of it is sealed:\n- .github/publish.sh: an npm token"
        ),
    );
    assert_eq!(
        reads(&provider),
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

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["slices"][0]["written"], json!([".env", "src/auth.rs"]), "{envelope}");
    assert_eq!(
        reads(&provider),
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

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 3, "server_error").await;

    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-001` (authentication) failed in wave 1: reading `src/hook` under \
             `{copy}`"
        ),
    );
    assert_eq!(
        reads(&provider),
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

// --- the specification guard ---

// A slice's build that left a path under `spec/` changed — written, or
// removed — is refused before anything of it is sealed: every such path is
// named, no file is read for a credential, nothing is merged or labelled,
// and the copy stays for inspection.
#[tokio::test]
async fn slice_rewrites_spec() {
    let (provider, _) = planned();
    let copy = worktree("SLICE-001");
    changed(
        &provider,
        "SLICE-001",
        &[
            ("src/auth.rs", ChangeKind::Added),
            ("spec/spec.json", ChangeKind::Modified),
            ("spec/plan.json", ChangeKind::Deleted),
        ],
    );
    provider.files.holds(&copy, "src/auth.rs", PEM);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "spec-rewritten").await;

    assert_message(
        &envelope,
        "slice `SLICE-001` (authentication) failed in wave 1: what the build wrote touches \
         `spec/`, where the sealed specification lives, so none of it is sealed:\n- \
         spec/spec.json\n- spec/plan.json",
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains(".emery/vcs/worktrees/<slice>")),
        "{envelope}"
    );
    assert!(reads(&provider).is_empty(), "refused before any path is read for a credential");
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

// A layout the adapter's initialisation scaffolds under `spec/` refuses the
// build before anything is sealed or any slice dispatched, naming the
// initialisation.
#[tokio::test]
async fn init_rewrites_spec() {
    let (provider, _) = planned();
    initialised(&provider, &["spec/design.json", "package.json"]);

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "spec-rewritten").await;

    assert_message(
        &envelope,
        "what the initialisation wrote touches `spec/`, where the sealed specification lives, so \
         none of it is sealed:\n- spec/design.json",
    );
    assert_eq!(provider.target.inits(), [INTEGRATION]);
    assert!(provider.target.calls().is_empty(), "no slice is dispatched");
    assert!(reads(&provider).is_empty(), "refused before any path is read for a credential");
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    let calls = provider.vcs.calls();
    assert_eq!(calls.last().cloned(), Some(format!("pending {INTEGRATION}")), "{calls:?}");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");
    provider.vcs.assert_exhausted();
}

// A merge's resolution that leaves a path under `spec/` changed in the
// slice's copy is refused before it is sealed, as a build's or a
// verification's change set is.
#[tokio::test]
async fn conflict_rewrites_spec() {
    let (provider, _) = conflicted_at_lib(&[RESOLVED_LIB]);
    let copy = worktree("SLICE-003");
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    changed(
        &provider,
        "SLICE-003",
        &[("src/lib.rs", ChangeKind::Modified), ("spec/spec.json", ChangeKind::Modified)],
    );

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "spec-rewritten").await;

    assert_message(
        &envelope,
        "slice `SLICE-003` (orders) failed in wave 1; SLICE-001 merged before it stays committed \
         in `./.emery/vcs/integration`: what the merge's resolution wrote touches `spec/`, where \
         the sealed specification lives, so none of it is sealed:\n- spec/spec.json",
    );
    assert_eq!(provider.files.writes().len(), 1, "the resolution was written");
    let calls = provider.vcs.calls();
    assert!(
        !calls.contains(&format!("commit {copy} SLICE-003 orders resolved at src/lib.rs")),
        "nothing of it is sealed: {calls:?}"
    );
    left_for_inspection(&provider);
    provider.model.assert_exhausted();
    provider.vcs.assert_exhausted();
}

// A path under `spec/` a verification's repairs leave changed refuses the
// wave before its changes are sealed: the slices merged in it stay
// committed, the wave's commit is never made, and the label stands where
// the last verified wave left it.
#[tokio::test]
async fn repair_rewrites_spec() {
    let (provider, id) = planned();
    initialised(&provider, &[]);
    provider.vcs.pending.script(
        INTEGRATION,
        Ok(vec![
            change("src/orders.rs", ChangeKind::Modified),
            change("spec/spec.json", ChangeKind::Modified),
        ]),
    );

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "spec-rewritten").await;

    assert_message(
        &envelope,
        &format!(
            "wave 1 failed; SLICE-001 merged in it stays committed in `{INTEGRATION}`; \
             `emery/{id}` is not set: what the verification wrote touches `spec/`, where the \
             sealed specification lives, so none of it is sealed:\n- spec/spec.json"
        ),
    );
    assert!(reads(&provider).is_empty(), "refused before any path is read for a credential");
    assert_eq!(provider.vcs.messages().len(), 1, "the slice's commit alone; no wave commit");
    let calls = provider.vcs.calls();
    assert_eq!(calls.last().cloned(), Some(format!("pending {INTEGRATION}")), "{calls:?}");
    assert!(!calls.iter().any(|call| call.starts_with("label ")), "no label: {calls:?}");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");
    assert_eq!(provider.target.calls().len(), 1, "the second wave is never built");
    provider.vcs.assert_exhausted();
}

// A merge the rules leave conflicted under `spec/` is nothing the model is
// handed: the slice is refused as `merge-unresolved` naming the reserved
// directory, before any side is read or any turn is spent.
#[tokio::test]
async fn build_conflict_at_spec() {
    let (provider, _) = planned();
    let copy = worktree("SLICE-001");
    wrote(&provider, AUTHENTICATION);
    provider.vcs.merges.script(INTEGRATION, Ok(conflicted(&["spec/spec.json"])));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 1, "merge-unresolved")
            .await;

    assert_message(
        &envelope,
        "slice `SLICE-001` (authentication) failed in wave 1: its merge conflicts at \
         spec/spec.json, and the conflict cannot be resolved: `spec/spec.json` is under `spec/`, \
         where the specification lives",
    );
    assert!(listed(&provider).is_empty(), "neither side is read");
    assert!(provider.files.writes().is_empty(), "nothing is written");
    let calls = provider.vcs.calls();
    assert!(!calls.contains(&format!("remove {copy}")), "left for inspection: {calls:?}");
    assert!(!calls.contains(&format!("remove {INTEGRATION}")), "left for inspection: {calls:?}");
    assert!(!calls.iter().any(|call| call.starts_with("label ")), "no label: {calls:?}");
    assert!(provider.target.verifies().is_empty(), "a wave that did not integrate is not verified");
    provider.vcs.assert_exhausted();
}

// --- the schedule and the budget ---

// Builds a revision whose `authentication` part holds one requirement and
// whose `orders` part holds `subs`, each `(sub, count, trigger)` under
// `orders.<sub>-*`, owning `orders.line` and `orders.order` and depending
// on `PART-001`.
fn budgeted_revision(subs: &[(&str, usize, Option<Trigger>)]) -> (Provider, String) {
    let mut spec: Value = serde_json::from_slice(SPEC).expect("spec fixture");
    let mut requirements = vec![spec["requirements"][0].clone()];
    let mut order_ids = Vec::new();
    let mut req_num = 1usize;
    for &(sub, count, ref trigger) in subs {
        for index in 1..=count {
            req_num += 1;
            let id = format!("REQ-{req_num:03}");
            order_ids.push(id.clone());
            let subject = if count == 1 {
                format!("orders.{sub}")
            } else {
                format!("orders.{sub}.item-{index}")
            };
            let trigger_val = serde_json::to_value(trigger).expect("trigger serialises");
            requirements.push(json!({
                "id": id,
                "subject": subject,
                "status": "unknown",
                "covered": false,
                "trigger": trigger_val,
                "sources": [{"source": "docs", "claim": subject, "path": null}],
                "body": [format!("Statement for {id}.")],
                "losers": [],
                "scenarios": [{
                    "name": format!("Scenario {req_num}"),
                    "given": [],
                    "when": "invoked",
                    "then": "succeeds",
                    "and": []
                }]
            }));
        }
    }
    spec["requirements"] = Value::Array(requirements);
    let mut plan: Value = serde_json::from_slice(WIDE_PLAN).expect("wide-plan fixture");
    plan["parts"] = json!([
        {
            "id": "PART-001",
            "name": "authentication",
            "requirements": ["REQ-001"],
            "types": [],
            "uses": [],
            "depends-on": [],
            "brief": ["Delivers sign-in."]
        },
        {
            "id": "PART-002",
            "name": "orders",
            "requirements": order_ids,
            "types": ["orders.line", "orders.order"],
            "uses": [],
            "depends-on": ["PART-001"],
            "brief": ["Delivers orders over sign-in."]
        }
    ]);
    let provider = Provider::idle();
    let id = seed(
        &provider.files,
        &serde_json::to_vec(&spec).expect("spec"),
        DESIGN,
        &serde_json::to_vec(&plan).expect("plan"),
    );
    (provider, id)
}

// A 20-requirement part at `--budget 8` cuts into 3 serial slices in
// waves 2, 3, and 4 after `authentication`; the part's owned data types
// travel with its first slice alone, and each slice carries its `index` of
// `count` and the part's name and brief.
#[tokio::test]
async fn build_budget_cut() {
    let post = Some(Trigger::Http(Route {
        method: "POST".to_owned(),
        path: "/orders".to_owned(),
    }));
    let get = Some(Trigger::Http(Route {
        method: "GET".to_owned(),
        path: "/orders".to_owned(),
    }));
    let cancel = Some(Trigger::Message("orders.cancel".to_owned()));
    let (provider, id) =
        budgeted_revision(&[("create", 8, post), ("list", 7, get), ("cancel", 5, cancel)]);
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG, "--budget", "8"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["budget"], 8, "{envelope}");
    assert_eq!(envelope["budget-source"], "flag", "{envelope}");
    assert_eq!(envelope["parts"], json!([["PART-001"], ["PART-002"]]), "{envelope}");
    assert_eq!(
        envelope["waves"],
        json!([["SLICE-001"], ["SLICE-002"], ["SLICE-003"], ["SLICE-004"]]),
        "the three slices of `orders` build in serial waves: {envelope}"
    );
    let calls = provider.target.calls();
    assert_eq!(calls.len(), 4);
    let [(_, auth, _), (_, first, _), (_, second, _), (_, third, _)] = calls.as_slice() else {
        panic!("four slices: {calls:?}");
    };
    assert_eq!(
        (&auth.id, &auth.part, auth.index, auth.count),
        (&"SLICE-001".to_owned(), &"PART-001".to_owned(), 1, 1)
    );
    assert_eq!(
        (&first.id, &first.part, &first.name, first.index, first.count),
        (&"SLICE-002".to_owned(), &"PART-002".to_owned(), &"orders".to_owned(), 1, 3)
    );
    assert_eq!(
        (&second.id, &second.part, &second.name, second.index, second.count),
        (&"SLICE-003".to_owned(), &"PART-002".to_owned(), &"orders".to_owned(), 2, 3)
    );
    assert_eq!(
        (&third.id, &third.part, &third.name, third.index, third.count),
        (&"SLICE-004".to_owned(), &"PART-002".to_owned(), &"orders".to_owned(), 3, 3)
    );
    assert_eq!(first.requirements.len(), 8);
    assert_eq!(second.requirements.len(), 7);
    assert_eq!(third.requirements.len(), 5);
    assert_eq!(first.types.len(), 2, "the first slice of the part carries its owned types");
    assert!(second.types.is_empty(), "later slices of the part carry no types");
    assert!(third.types.is_empty(), "later slices of the part carry no types");
    assert_eq!(second.brief, first.brief, "every slice carries the part's brief");

    // text output shows the plan summary, the budget and its source, and `(index/count)` per slice
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG, "-b", "8"]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        stdout.contains(
            "  plan: 2 parts in 2 waves; budget 8 (flag): 4 slices in 4 waves, widest 1\n"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("    SLICE-002 orders (1/3): covered 8/8"), "{stdout}");
    assert!(stdout.contains("    SLICE-003 orders (2/3): covered 7/7"), "{stdout}");
    assert!(stdout.contains("    SLICE-004 orders (3/3): covered 5/5"), "{stdout}");
    assert!(!stdout.contains("over budget:"), "{stdout}");
    assert_eq!(
        provider.vcs.messages()[1],
        format!(
            "SLICE-002 orders\n\nSlice: SLICE-002\nPart: PART-002\nRevision: {id}\nBudget: \
             8\nRequirements: REQ-002, REQ-003, REQ-004, REQ-005, REQ-006, REQ-007, REQ-008, \
             REQ-009\nCovered: REQ-002, REQ-003, REQ-004, REQ-005, REQ-006, REQ-007, REQ-008, \
             REQ-009\nAdapter: {BUILDER}\nBase: {HEAD}\nWave: 2"
        )
    );
}

// Requirements sharing one trigger stay in one slice even when they span
// sub-stems and exceed the budget; the slice is flagged `over-budget` and
// warned in text output rather than refused.
#[tokio::test]
async fn budget_trigger_group() {
    let route = Some(Trigger::Http(Route {
        method: "POST".to_owned(),
        path: "/orders".to_owned(),
    }));
    let other = Some(Trigger::Http(Route {
        method: "GET".to_owned(),
        path: "/orders".to_owned(),
    }));
    let (provider, _) = budgeted_revision(&[
        ("validate", 6, route.clone()),
        ("persist", 4, route),
        ("list", 3, other),
    ]);
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG, "--budget", "6"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["schedule"].as_array().map(Vec::len), Some(3), "{envelope}");
    assert_eq!(envelope["schedule"][1]["id"], "SLICE-002");
    assert_eq!(envelope["schedule"][1]["requirements"].as_array().map(Vec::len), Some(10));
    assert_eq!(envelope["schedule"][1]["over-budget"], true, "{envelope}");
    assert_eq!(envelope["schedule"][2]["id"], "SLICE-003");
    assert_eq!(envelope["schedule"][2]["requirements"].as_array().map(Vec::len), Some(3));
    assert_eq!(envelope["schedule"][2]["over-budget"], false, "{envelope}");

    provider.vcs.heads.script(".", Ok(BASE.to_owned()));
    let resp =
        cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG, "--budget", "6"]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("  over budget: SLICE-002 orders (10 requirements)\n"), "{stdout}");
}

// Untriggered sub-stems (pipeline stages and helpers) pack ahead of
// triggered sub-stems (routes and consumers), regardless of where their
// requirement ids sit in the part.
#[tokio::test]
async fn budget_first_slice() {
    let route = Some(Trigger::Http(Route {
        method: "POST".to_owned(),
        path: "/orders".to_owned(),
    }));
    let (provider, _) =
        budgeted_revision(&[("entry", 4, route), ("stage-one", 3, None), ("stage-two", 3, None)]);
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG, "--budget", "6"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["schedule"][1]["requirements"],
        json!(["REQ-006", "REQ-007", "REQ-008", "REQ-009", "REQ-010", "REQ-011"]),
        "the two untriggered stages pack into the first slice of the part: {envelope}"
    );
    assert_eq!(
        envelope["schedule"][2]["requirements"],
        json!(["REQ-002", "REQ-003", "REQ-004", "REQ-005"]),
        "the triggered entry follows in the second slice: {envelope}"
    );
}

// When neither `--budget` nor `[target] budget` is given, the target
// adapter's `metadata.budget` is used and reported as `adapter`.
#[tokio::test]
async fn budget_from_metadata() {
    let (mut provider, _) = budgeted_revision(&[("a", 3, None), ("b", 3, None)]);
    provider.target.budget = Some(4);
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["budget"], 4, "{envelope}");
    assert_eq!(envelope["budget-source"], "adapter", "{envelope}");
    assert_eq!(envelope["schedule"].as_array().map(Vec::len), Some(3), "{envelope}");
}

// `[target] budget` in `emery.toml` overrides the adapter's metadata and is
// reported as `config`.
#[tokio::test]
async fn build_budget_from_config() {
    let scratch = Scratch::new();
    let config = scratch
        .config(&format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \".\"\nbudget = 3\n"));
    let (mut provider, _) = budgeted_revision(&[("a", 3, None), ("b", 3, None)]);
    provider.target.budget = Some(10);
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", &config]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["budget"], 3, "{envelope}");
    assert_eq!(envelope["budget-source"], "config", "{envelope}");
    assert_eq!(envelope["schedule"].as_array().map(Vec::len), Some(3), "{envelope}");
}

// `--budget` overrides `[target] budget` and the adapter's metadata, and is
// reported as `flag`; `--budget 0` is refused at the CLI boundary.
#[tokio::test]
async fn budget_flag_precedence() {
    let scratch = Scratch::new();
    let config = scratch
        .config(&format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \".\"\nbudget = 10\n"));
    let (mut provider, _) = budgeted_revision(&[("a", 3, None), ("b", 3, None)]);
    provider.target.budget = Some(12);
    provider.vcs.heads.script(".", Ok(BASE.to_owned()));

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", &config, "--budget", "3"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["budget"], 3, "{envelope}");
    assert_eq!(envelope["budget-source"], "flag", "{envelope}");
    assert_eq!(envelope["schedule"].as_array().map(Vec::len), Some(3), "{envelope}");

    let zero = cli(&provider, &["emery", "build", "--config", &config, "--budget", "0"]).await;
    assert_eq!(zero.exit, USAGE_EXIT);
}

// Resuming a multi-slice part reads the committed `Requirements:` from the
// label chain: the slices whose requirements are on the label are reported
// in `resumed`, the remaining slice builds with `types` empty (since the
// first slice already laid them), and its `index`/`count` stay `3` of `3`.
#[tokio::test]
async fn resume_requirements() {
    let (provider, id) = budgeted_revision(&[("a", 3, None), ("b", 3, None), ("c", 3, None)]);
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry(
                "m:SLICE-003",
                &message(
                    &id,
                    ("SLICE-003", "orders"),
                    "REQ-005, REQ-006, REQ-007",
                    "REQ-005, REQ-006, REQ-007",
                    HEAD,
                    2,
                ),
            ),
            entry(
                "m:SLICE-002",
                &message(
                    &id,
                    ("SLICE-002", "orders"),
                    "REQ-002, REQ-003, REQ-004",
                    "REQ-002, REQ-003, REQ-004",
                    HEAD,
                    2,
                ),
            ),
            entry("m:SLICE-001", &message(&id, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
        ]),
    );

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG, "--budget", "3"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["resumed"], json!(["SLICE-001", "SLICE-002", "SLICE-003"]), "{envelope}");
    let calls = provider.target.calls();
    let [(_, slice, _)] = calls.as_slice() else {
        panic!("only the third slice of orders remains: {calls:?}");
    };
    assert_eq!(slice.id, "SLICE-004");
    assert_eq!(slice.part, "PART-002");
    assert_eq!((slice.index, slice.count), (3, 3));
    assert!(slice.types.is_empty(), "types were laid in the first slice: {slice:?}");
    let req_ids: Vec<&str> = slice.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(req_ids, ["REQ-008", "REQ-009", "REQ-010"]);
}

// Changing `--budget` mid-build (from 6 on the first run to 3 on the resume)
// keeps every requirement committed on the label chain as resumed and re-cuts
// only the remaining uncovered requirements under the new budget.
#[tokio::test]
async fn resume_budget_change() {
    let (provider, id) = budgeted_revision(&[
        ("a", 1, None),
        ("b", 1, None),
        ("c", 1, None),
        ("d", 1, None),
        ("e", 1, None),
        ("f", 1, None),
        ("g", 1, None),
        ("h", 1, None),
        ("i", 1, None),
        ("j", 1, None),
    ]);
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry(
                "m:SLICE-002",
                &message(
                    &id,
                    ("SLICE-002", "orders"),
                    "REQ-002, REQ-003, REQ-004, REQ-005, REQ-006",
                    "REQ-002, REQ-003, REQ-004, REQ-005, REQ-006",
                    HEAD,
                    2,
                ),
            ),
            entry("m:SLICE-001", &message(&id, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
        ]),
    );

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG, "--budget", "3"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["budget"], 3, "{envelope}");
    assert_eq!(envelope["resumed"], json!(["SLICE-001", "SLICE-002"]), "{envelope}");
    let calls = provider.target.calls();
    assert_eq!(
        calls.len(),
        2,
        "the remaining 5 requirements cut into slices of 3 and 2: {calls:?}"
    );
    assert_eq!(calls[0].1.id, "SLICE-003");
    assert_eq!((calls[0].1.index, calls[0].1.count), (2, 3));
    assert_eq!(calls[0].1.types, []);
    let first_reqs: Vec<&str> = calls[0].1.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(first_reqs, ["REQ-007", "REQ-008", "REQ-009"]);
    assert_eq!(calls[1].1.id, "SLICE-004");
    assert_eq!((calls[1].1.index, calls[1].1.count), (3, 3));
    let second_reqs: Vec<&str> = calls[1].1.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(second_reqs, ["REQ-010", "REQ-011"]);
}

// --- settling the basis and scoping a change build ---

const GRANDPARENT: &str = "0123456789abcdef0123456789abcdef01234567";

// Builds an amended revision over `parent` from `plan_bytes`, rewriting
// `REQ-003`'s statement when `order_statement` is given.
fn amended_documents(
    parent: &str, plan_bytes: &[u8], order_statement: Option<&str>,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut spec: Value = serde_json::from_slice(SPEC).expect("spec fixture");
    spec["parent"] = json!(parent);
    if let Some(statement) = order_statement {
        spec["requirements"][2]["body"] = json!([statement]);
    }
    let mut design: Value = serde_json::from_slice(DESIGN).expect("design fixture");
    design["parent"] = json!(parent);
    let mut plan: Value = serde_json::from_slice(plan_bytes).expect("plan fixture");
    plan["parent"] = json!(parent);
    (
        serde_json::to_vec(&spec).expect("spec"),
        serde_json::to_vec(&design).expect("design"),
        serde_json::to_vec(&plan).expect("plan"),
    )
}

// Seeds a three-part parent revision (`WIDE_PLAN`) as built at `LABELLED`
// and seeds a child revision over it whose `orders` part (`PART-003`) has a
// changed requirement.
fn wide_lineage() -> (Provider, String, String) {
    let provider = Provider::idle();
    let parent = seed_at(&provider.files, LABELLED, SPEC, DESIGN, WIDE_PLAN);
    let (spec, design, plan) = amended_documents(
        &parent,
        WIDE_PLAN,
        Some("A signed-in user places an order with a discount."),
    );
    let child = seed(&provider.files, &spec, &design, &plan);
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry(
                "m:SLICE-003",
                &message(&parent, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
            ),
            entry("m:SLICE-002", &message(&parent, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2)),
            entry("m:SLICE-001", &message(&parent, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
        ]),
    );
    (provider, parent, child)
}

// Over a complete built parent, `Basis::settle` computes the minimum scope
// and carried parts, puts `Task::Scope` once in a working copy cut at the
// built commit, removes the copy, and reports `change` in the envelope.
#[tokio::test]
async fn build_change_reported() {
    let (provider, parent, child) = wide_lineage();

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], child, "{envelope}");
    assert!(envelope.get("whole").is_none(), "change and whole are mutually exclusive: {envelope}");
    assert_eq!(
        envelope["change"],
        json!({
            "from": parent,
            "commit": LABELLED,
            "rebuilt": [
                {"id": "PART-003", "name": "orders", "reasons": ["requirements"]}
            ],
            "carried": ["PART-001", "PART-002"],
            "retiring": []
        }),
        "{envelope}"
    );

    let scopes = provider.target.scopes();
    let [(adapter, scoped_change, root)] = scopes.as_slice() else {
        panic!("one scope call expected: {scopes:?}");
    };
    assert_eq!(adapter, BUILDER_ID);
    assert_eq!(root, SCOPE);
    assert_eq!(scoped_change.built, parent);
    assert_eq!(scoped_change.commit, LABELLED);
    assert_eq!(
        scoped_change.parts,
        [
            PartReach {
                id: "PART-001".to_owned(),
                name: "authentication".to_owned(),
                requirements: vec!["REQ-001".to_owned()],
                reasons: Vec::new(),
            },
            PartReach {
                id: "PART-002".to_owned(),
                name: "sessions".to_owned(),
                requirements: vec!["REQ-002".to_owned()],
                reasons: Vec::new(),
            },
            PartReach {
                id: "PART-003".to_owned(),
                name: "orders".to_owned(),
                requirements: vec!["REQ-003".to_owned(), "REQ-004".to_owned()],
                reasons: vec!["requirements".to_owned()],
            },
        ]
    );
    let calls = provider.vcs.calls();
    assert!(calls.contains(&format!("add . {SCOPE} {LABELLED}")), "{calls:?}");
    assert!(calls.contains(&format!("pending {SCOPE}")), "{calls:?}");
    assert!(calls.contains(&format!("remove {SCOPE}")), "{calls:?}");
    provider.vcs.assert_exhausted();
}

// The adapter's `Scoped` answer unions with the minimum scope, marking
// adapter-widened parts with `adapter`; an id that is no plan part, or a
// pending change left in the scope working copy, is refused as `server_error`.
#[tokio::test]
async fn build_change_scope() {
    let (provider, parent, _) = wide_lineage();
    provider.target.scope(Ok(vec!["PART-002".to_owned(), "PART-003".to_owned()]));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["change"],
        json!({
            "from": parent,
            "commit": LABELLED,
            "rebuilt": [
                {"id": "PART-002", "name": "sessions", "reasons": ["adapter"]},
                {"id": "PART-003", "name": "orders", "reasons": ["requirements"]}
            ],
            "carried": ["PART-001"],
            "retiring": []
        }),
        "{envelope}"
    );
    provider.vcs.assert_exhausted();

    // an id outside the plan is `server_error`
    let (provider, _, _) = wide_lineage();
    provider.target.scope(Ok(vec!["PART-099".to_owned()]));
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 3, "server_error").await;
    assert_message(
        &envelope,
        "scoped `PART-099` is not a part of the plan; its parts are PART-001, PART-002, PART-003",
    );
    assert!(provider.vcs.calls().contains(&format!("remove {SCOPE}")), "scope copy removed");
    assert!(provider.target.calls().is_empty(), "no slice built");
    provider.vcs.assert_exhausted();

    // a pending change in the scope working copy violates W^X and is `server_error`
    let (provider, _, _) = wide_lineage();
    provider.vcs.pending.script(SCOPE, Ok(vec![change("src/orders.rs", ChangeKind::Modified)]));
    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 3, "server_error").await;
    assert_message(&envelope, "modified the scope working copy: src/orders.rs");
    assert!(provider.vcs.calls().contains(&format!("remove {SCOPE}")), "scope copy removed");
    assert!(provider.target.calls().is_empty(), "no slice built");
    provider.vcs.assert_exhausted();
}

// When the nearest ancestor with slice commits on its label has an incomplete
// build, `Basis::settle` falls back to a whole build with `parent-unbuilt`
// without asking `scope`.
#[tokio::test]
async fn build_change_unbuilt_parent() {
    let provider = Provider::idle();
    let parent = seed_at(&provider.files, LABELLED, SPEC, DESIGN, PLAN);
    let (spec, design, plan) = amended_documents(&parent, PLAN, Some("Updated order creation."));
    let child = seed(&provider.files, &spec, &design, &plan);
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![entry(
            "m:SLICE-001",
            &message(&parent, AUTHENTICATION, "REQ-001, REQ-002", "REQ-001, REQ-002", HEAD, 1),
        )]),
    );

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], child, "{envelope}");
    assert!(envelope.get("change").is_none(), "{envelope}");
    assert_eq!(envelope["whole"], json!({"reason": "parent-unbuilt"}), "{envelope}");
    assert!(provider.target.scopes().is_empty(), "scope is not asked for an unbuilt parent");
    assert_eq!(provider.target.calls().len(), 2, "every slice of the plan builds");
    provider.vcs.assert_exhausted();
}

// A parent revision that was sealed and never built (`merged_under` empty) is
// stepped past to its own parent when that grandparent has a complete build.
#[tokio::test]
async fn build_change_unbuilt_grandparent_skipped() {
    let provider = Provider::idle();
    let grandparent = seed_at(&provider.files, GRANDPARENT, SPEC, DESIGN, PLAN);
    let (parent_spec, parent_design, parent_plan) =
        amended_documents(&grandparent, PLAN, Some("Intermediate order wording."));
    let parent = seed_at(&provider.files, LABELLED, &parent_spec, &parent_design, &parent_plan);
    let (child_spec, child_design, child_plan) =
        amended_documents(&parent, PLAN, Some("Final order wording."));
    let child = seed(&provider.files, &child_spec, &child_design, &child_plan);

    let grandparent_log = vec![
        entry(
            "m:SLICE-002",
            &message(&grandparent, ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 2),
        ),
        entry(
            "m:SLICE-001",
            &message(&grandparent, AUTHENTICATION, "REQ-001, REQ-002", "REQ-001, REQ-002", HEAD, 1),
        ),
    ];

    // parent label holds no slice commits; grandparent label holds a complete build
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(Vec::new()));
    provider.vcs.labelleds.script(".", Ok(GRANDPARENT.to_owned()));
    provider.vcs.logs.script(".", Ok(grandparent_log.clone()));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], child, "{envelope}");
    assert!(envelope.get("whole").is_none(), "{envelope}");
    assert_eq!(
        envelope["change"],
        json!({
            "from": grandparent,
            "commit": GRANDPARENT,
            "rebuilt": [
                {"id": "PART-002", "name": "orders", "reasons": ["requirements"]}
            ],
            "carried": ["PART-001"],
            "retiring": []
        }),
        "the unbuilt parent is stepped past to the built grandparent: {envelope}"
    );
    let scopes = provider.target.scopes();
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].1.built, grandparent);
    assert_eq!(scopes[0].1.commit, GRANDPARENT);
    provider.vcs.assert_exhausted();

    // when the parent was itself a change build (only `orders` committed on its
    // label, `authentication` carried from the grandparent), its carried set is
    // recomputed from `grandparent -> parent` so `parent` is recognised as complete
    let provider = Provider::idle();
    seed_at(&provider.files, GRANDPARENT, SPEC, DESIGN, PLAN);
    seed_at(&provider.files, LABELLED, &parent_spec, &parent_design, &parent_plan);
    seed(&provider.files, &child_spec, &child_design, &child_plan);
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![entry(
            "m:SLICE-002",
            &message(&parent, ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
        )]),
    );
    provider.vcs.labelleds.script(".", Ok(GRANDPARENT.to_owned()));
    provider.vcs.logs.script(".", Ok(grandparent_log));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["change"]["from"], parent, "{envelope}");
    assert_eq!(envelope["change"]["commit"], LABELLED, "{envelope}");
    let scopes = provider.target.scopes();
    assert_eq!(
        scopes.len(),
        2,
        "scoped once for grandparent -> parent and once for parent -> child"
    );
    assert_eq!(scopes[0].1.built, grandparent);
    assert_eq!(scopes[1].1.built, parent);
    provider.vcs.assert_exhausted();
}

// A parentless revision (`no-parent`), a revision sealed from a `--fresh`
// candidate over a lineage (`no-parent`), and a `--whole` invocation over a
// built parent (`requested`) each report `whole` and never ask `scope`.
#[tokio::test]
async fn build_whole() {
    // parentless baseline (`no-parent`)
    let (provider, _) = planned();
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("change").is_none(), "{envelope}");
    assert_eq!(envelope["whole"], json!({"reason": "no-parent"}), "{envelope}");
    assert!(provider.target.scopes().is_empty(), "no scope for no-parent");
    provider.vcs.assert_exhausted();

    // `--whole` over a lineage with a built parent (`requested`)
    let provider = Provider::idle();
    let parent = seed(&provider.files, SPEC, DESIGN, WIDE_PLAN);
    let (spec, design, plan) = amended_documents(&parent, WIDE_PLAN, Some("Updated orders."));
    seed(&provider.files, &spec, &design, &plan);
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--whole", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("change").is_none(), "{envelope}");
    assert_eq!(envelope["whole"], json!({"reason": "requested"}), "{envelope}");
    assert!(provider.target.scopes().is_empty(), "`--whole` never asks `scope`");
    provider.vcs.assert_exhausted();

    // a candidate staged `--fresh` and sealed over a lineage has no parent (`no-parent`)
    let provider = Provider::idle();
    seed(&provider.files, &spec, &design, &plan);
    let fresh = stage_fresh(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], fresh, "{envelope}");
    assert!(envelope.get("change").is_none(), "{envelope}");
    assert_eq!(envelope["whole"], json!({"reason": "no-parent"}), "{envelope}");
    assert!(provider.target.scopes().is_empty(), "no scope for fresh revision");
    provider.vcs.assert_exhausted();
}

// Seeds a three-hop lineage (`grandparent -> mid -> child`) where `mid` is
// unbuilt and `child` changes `REQ-003`, adds `REQ-005`, and updates design
// sections.
fn multi_hop_change_lineage() -> (Provider, String, String) {
    let mut parent_spec: Value = serde_json::from_slice(SPEC).expect("spec fixture");
    parent_spec["requirements"][0]["sources"][0]["path"] = json!("src/auth.ts#L1-L10");
    parent_spec["requirements"][2]["sources"][0]["path"] = json!("src/orders.ts#L1-L20");
    let mut parent_design: Value = serde_json::from_slice(DESIGN).expect("design fixture");
    parent_design["sections"].as_array_mut().expect("sections").extend([
        json!({"kind": "apis", "blocks": [{"text": "First upstream."}, {"text": "Second upstream."}]}),
        json!({"kind": "observability", "blocks": [{"text": "Metrics."}]}),
    ]);

    let provider = Provider::idle();
    let grandparent = seed_at(
        &provider.files,
        GRANDPARENT,
        &serde_json::to_vec(&parent_spec).expect("spec"),
        &serde_json::to_vec(&parent_design).expect("design"),
        WIDE_PLAN,
    );
    let (mid_spec, mid_design, mid_plan) = amended_documents(&grandparent, WIDE_PLAN, None);
    let mid = seed_at(&provider.files, LABELLED, &mid_spec, &mid_design, &mid_plan);

    let mut child_spec = parent_spec;
    child_spec["parent"] = json!(mid);
    child_spec["requirements"][2]["body"] = json!(["A signed-in user places an order directly."]);
    child_spec["requirements"][2]["sources"][0]["path"] = json!("src/orders.ts#L50-L60");
    child_spec["requirements"].as_array_mut().expect("requirements").push(json!({
        "id": "REQ-005",
        "subject": "orders.export",
        "status": "unknown",
        "covered": false,
        "trigger": null,
        "sources": [{"source": "docs", "claim": "orders.export", "path": "src/orders.ts#L1-L20"}],
        "body": ["A signed-in user exports an order."],
        "losers": [],
        "scenarios": [{
            "name": "Export",
            "given": [],
            "when": "a signed-in caller exports an order",
            "then": "the order is exported",
            "and": []
        }]
    }));
    let mut child_design: Value = serde_json::from_slice(DESIGN).expect("design fixture");
    child_design["parent"] = json!(mid);
    child_design["sections"][0]["blocks"][0]["text"] = json!("Updated overview.");
    child_design["sections"].as_array_mut().expect("sections").extend([
        json!({"kind": "apis", "blocks": [{"text": "Updated upstream."}]}),
        json!({"kind": "technical-logic", "blocks": [{"text": "Validation rules."}]}),
    ]);
    let mut child_plan: Value = serde_json::from_slice(WIDE_PLAN).expect("wide-plan fixture");
    child_plan["parent"] = json!(mid);
    child_plan["parts"][2]["requirements"] = json!(["REQ-003", "REQ-004", "REQ-005"]);
    let child = seed(
        &provider.files,
        &serde_json::to_vec(&child_spec).expect("spec"),
        &serde_json::to_vec(&child_design).expect("design"),
        &serde_json::to_vec(&child_plan).expect("plan"),
    );

    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(Vec::new()));
    provider.vcs.labelleds.script(".", Ok(GRANDPARENT.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry(
                "m:SLICE-003",
                &message(
                    &grandparent,
                    WIDE_ORDERS,
                    "REQ-003, REQ-004",
                    "REQ-003, REQ-004",
                    HEAD,
                    1,
                ),
            ),
            entry(
                "m:SLICE-002",
                &message(&grandparent, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2),
            ),
            entry(
                "m:SLICE-001",
                &message(&grandparent, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1),
            ),
        ]),
    );
    (provider, grandparent, child)
}

// A change build dispatches only the slices of parts in scope, skips
// `Initialise` (carried requirements count as merged, and `resumed` stays
// empty on a fresh change build), seals `Change:` on each slice commit, and
// hands each slice its typed `SliceChange` (`standing`, `prior`,
// `built_before`, `sections`).
#[tokio::test]
async fn build_change() {
    let (provider, grandparent, child) = multi_hop_change_lineage();
    wrote(&provider, WIDE_ORDERS);
    wrote(&provider, ("SLICE-004", "orders"));

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG, "--budget", "2"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], child, "{envelope}");
    assert!(envelope.get("initialised").is_none(), "Initialise skipped: {envelope}");
    assert!(envelope.get("resumed").is_none(), "carried slices are not resumed: {envelope}");
    assert!(provider.target.inits().is_empty(), "Task::Init not called on a change build");
    assert_eq!(
        envelope["change"],
        json!({
            "from": grandparent,
            "commit": GRANDPARENT,
            "rebuilt": [{"id": "PART-003", "name": "orders", "reasons": ["requirements"]}],
            "carried": ["PART-001", "PART-002"],
            "retiring": []
        }),
        "{envelope}"
    );
    assert_eq!(
        dispatched(&provider),
        [
            ("SLICE-003".to_owned(), HEAD.to_owned(), worktree("SLICE-003")),
            ("SLICE-004".to_owned(), HEAD.to_owned(), worktree("SLICE-004")),
        ],
        "carried parts PART-001 and PART-002 are skipped; PART-003 builds in two serial slices"
    );

    let calls = provider.target.calls();
    let first_change = calls[0].1.change.as_ref().expect("first slice carries SliceChange");
    assert_eq!(first_change.built, grandparent);
    assert_eq!(first_change.commit, GRANDPARENT);
    assert_eq!(
        calls[0].1.requirements.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["REQ-003", "REQ-005"],
        "untriggered REQ-005 packs into the first slice ahead of triggered REQ-003 and REQ-004"
    );
    assert_eq!(first_change.standing, [Standing::Changed, Standing::Added]);
    assert_eq!(first_change.prior.len(), 1);
    assert_eq!(first_change.prior[0].id, "REQ-003");
    assert_eq!(first_change.prior[0].statement, ["A signed-in user places an order."]);
    assert_eq!(first_change.built_before, [] as [String; 0]);
    assert_eq!(
        first_change.sections,
        [
            SectionChange {
                kind: "overview".to_owned(),
                before: Some(
                    "Sign-in issues a session; orders are placed and cancelled under it."
                        .to_owned()
                ),
                after: Some("Updated overview.".to_owned()),
            },
            SectionChange {
                kind: "apis".to_owned(),
                before: Some("First upstream.\n\nSecond upstream.".to_owned()),
                after: Some("Updated upstream.".to_owned()),
            },
            SectionChange {
                kind: "technical-logic".to_owned(),
                before: None,
                after: Some("Validation rules.".to_owned()),
            },
            SectionChange {
                kind: "observability".to_owned(),
                before: Some("Metrics.".to_owned()),
                after: None,
            },
        ]
    );

    let second_change = calls[1].1.change.as_ref().expect("second slice carries SliceChange");
    assert_eq!(
        calls[1].1.requirements.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["REQ-004"]
    );
    assert_eq!(second_change.standing, [Standing::Unchanged]);
    assert_eq!(second_change.prior, [] as [Requirement; 0]);
    assert_eq!(second_change.built_before, ["REQ-003", "REQ-005"]);

    assert_eq!(
        provider.vcs.messages(),
        [
            format!(
                "SLICE-003 orders\n\nSlice: SLICE-003\nPart: PART-003\nRevision: {child}\nBudget: \
                 2\nRequirements: REQ-003, REQ-005\nCovered: REQ-003, REQ-005\nChange: \
                 {grandparent}\nAdapter: {BUILDER}\nBase: {HEAD}\nWave: 1"
            ),
            format!(
                "SLICE-004 orders\n\nSlice: SLICE-004\nPart: PART-003\nRevision: {child}\nBudget: \
                 2\nRequirements: REQ-004\nCovered: REQ-004\nChange: {grandparent}\nAdapter: \
                 {BUILDER}\nBase: {HEAD}\nWave: 2"
            ),
        ]
    );
    provider.vcs.assert_exhausted();
}

// Resuming an interrupted change build skips both the carried parts and the
// slice already committed on the new revision's label, reports only the
// label's slice in `resumed`, and populates `built_before` on the remaining
// slice of the part.
#[tokio::test]
async fn build_change_resume() {
    let (provider, parent, child) = wide_lineage();
    // child's label already holds wave 1 (`SLICE-003` with `REQ-003`) under `--budget 1`
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![entry(
            "m:SLICE-003",
            &format!(
                "SLICE-003 orders\n\nSlice: SLICE-003\nPart: PART-003\nRevision: {child}\nBudget: \
                 1\nRequirements: REQ-003\nCovered: REQ-003\nChange: {parent}\nAdapter: \
                 {BUILDER}\nBase: {HEAD}\nWave: 1"
            ),
        )]),
    );
    wrote(&provider, ("SLICE-004", "orders"));

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG, "--budget", "1"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["resumed"],
        json!(["SLICE-003"]),
        "only the label's slice is resumed: {envelope}"
    );
    assert_eq!(
        dispatched(&provider),
        [("SLICE-004".to_owned(), LABELLED.to_owned(), worktree("SLICE-004"))],
        "carried parts and the resumed first slice are skipped; only SLICE-004 builds"
    );
    let calls = provider.target.calls();
    assert!(calls[0].1.types.is_empty(), "types were laid on the part's first slice");
    let change = calls[0].1.change.as_ref().expect("SliceChange present");
    assert_eq!(change.standing, [Standing::Unchanged]);
    assert_eq!(change.built_before, ["REQ-003"]);
    provider.vcs.assert_exhausted();
}

// Seeds a parent with three parts (`WIDE_PLAN`) and a child that removes
// `PART-003` (`orders`), its requirements, and its domain-model types.
fn retiring_lineage() -> (Provider, String, String, Vec<Entry>) {
    let provider = Provider::idle();
    let parent = seed_at(&provider.files, LABELLED, SPEC, DESIGN, WIDE_PLAN);

    let (mut child_spec, mut child_design, mut child_plan): (Value, Value, Value) = (
        serde_json::from_slice(SPEC).expect("spec"),
        serde_json::from_slice(DESIGN).expect("design"),
        serde_json::from_slice(WIDE_PLAN).expect("wide-plan"),
    );
    child_spec["parent"] = json!(parent);
    child_spec["requirements"].as_array_mut().expect("requirements").truncate(2);
    child_design["parent"] = json!(parent);
    child_design["sections"].as_array_mut().expect("sections").truncate(1);
    child_plan["parent"] = json!(parent);
    child_plan["parts"].as_array_mut().expect("parts").truncate(2);
    let child = seed(
        &provider.files,
        &serde_json::to_vec(&child_spec).expect("spec"),
        &serde_json::to_vec(&child_design).expect("design"),
        &serde_json::to_vec(&child_plan).expect("plan"),
    );

    let parent_log = vec![
        entry(
            "m:SLICE-003",
            &message(&parent, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
        ),
        entry("m:SLICE-002", &message(&parent, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2)),
        entry("m:SLICE-001", &message(&parent, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
    ];
    (provider, parent, child, parent_log)
}

// Removing a part from the plan cuts a retiring slice in wave 1 with the
// removed part's id, name, and brief, `requirements` empty, and every
// retired requirement in `departed`, seals `Departed:` and `Change:` on its
// commit, lists the slice in `change.retiring`, and resumes from the
// `Departed:` trailer on a re-run.
#[tokio::test]
async fn build_change_retire() {
    let (provider, parent, child, parent_log) = retiring_lineage();
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(parent_log.clone()));
    changed(&provider, "SLICE-003", &[("src/orders.rs", ChangeKind::Deleted)]);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["change"],
        json!({
            "from": parent,
            "commit": LABELLED,
            "rebuilt": [],
            "carried": ["PART-001", "PART-002"],
            "retiring": ["SLICE-003"]
        }),
        "{envelope}"
    );
    assert_eq!(
        envelope["schedule"][2],
        json!({"id": "SLICE-003", "part": "PART-003", "name": "orders", "index": 1, "count": 1, "requirements": [], "departed": ["REQ-003", "REQ-004"], "over-budget": false}),
        "{envelope}"
    );
    assert_eq!(envelope["waves"], json!([["SLICE-001", "SLICE-003"], ["SLICE-002"]]), "{envelope}");
    assert_eq!(
        dispatched(&provider),
        [("SLICE-003".to_owned(), HEAD.to_owned(), worktree("SLICE-003"))],
        "only the retiring slice runs, in wave 1"
    );

    let calls = provider.target.calls();
    let retiring = &calls[0].1;
    assert_eq!(
        (retiring.id.as_str(), retiring.part.as_str(), retiring.name.as_str()),
        ("SLICE-003", "PART-003", "orders")
    );
    assert_eq!(retiring.requirements, [] as [Requirement; 0]);
    assert_eq!(retiring.brief, ["Delivers order creation and cancellation; it waits on nothing."]);
    let retiring_change = retiring.change.as_ref().expect("SliceChange on retiring slice");
    assert!(retiring_change.is_retiring(&retiring.requirements));
    assert_eq!(retiring_change.vanished, ["orders"]);
    assert_eq!(
        retiring_change
            .departed
            .iter()
            .map(|d| (d.requirement.id.as_str(), d.moved_to.as_deref()))
            .collect::<Vec<_>>(),
        [("REQ-003", None), ("REQ-004", None)]
    );

    let commit_msg = format!(
        "SLICE-003 orders\n\nSlice: SLICE-003\nPart: PART-003\nRevision: {child}\nBudget: \
         12\nRequirements: \nCovered: \nDeparted: REQ-003, REQ-004\nChange: {parent}\nAdapter: \
         {BUILDER}\nBase: {HEAD}\nWave: 1"
    );
    assert_eq!(provider.vcs.messages(), [commit_msg.as_str()]);
    provider.vcs.assert_exhausted();

    // resuming when the retiring commit is on the label skips the retiring slice too
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(parent_log));
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(vec![entry("m:SLICE-003", &commit_msg)]));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["resumed"], json!(["SLICE-003"]), "{envelope}");
    assert!(envelope["slices"].as_array().is_some_and(Vec::is_empty), "{envelope}");
    assert_eq!(provider.target.calls().len(), 1, "no slice dispatched on resume");
    provider.vcs.assert_exhausted();
}

// When a requirement leaves a kept part (moving to another part or being
// retired while its part stays in the plan), no retiring slice is cut; the
// departure rides on the kept part's own slice with `moved_to` set or `None`.
#[tokio::test]
async fn build_change_departed_in_kept_part() {
    let provider = Provider::idle();
    let parent = seed_at(&provider.files, LABELLED, SPEC, DESIGN, WIDE_PLAN);

    // REQ-003 moves from PART-003 (`orders`) to PART-002 (`sessions`), and REQ-004 is retired;
    // PART-003 stays in the plan with a new REQ-005
    let mut child_spec: Value = serde_json::from_slice(SPEC).expect("spec fixture");
    child_spec["parent"] = json!(parent);
    child_spec["requirements"].as_array_mut().expect("requirements").truncate(3);
    child_spec["requirements"].as_array_mut().expect("requirements").push(json!({
        "id": "REQ-005",
        "subject": "orders.archive",
        "status": "unknown",
        "covered": false,
        "trigger": null,
        "sources": [{"source": "docs", "claim": "orders.archive", "path": null}],
        "body": ["Archive closed orders."],
        "losers": [],
        "scenarios": [{
            "name": "Archive",
            "given": [],
            "when": "an order is closed",
            "then": "it is archived",
            "and": []
        }]
    }));
    let mut child_design: Value = serde_json::from_slice(DESIGN).expect("design fixture");
    child_design["parent"] = json!(parent);
    let mut child_plan: Value = serde_json::from_slice(WIDE_PLAN).expect("wide-plan fixture");
    child_plan["parent"] = json!(parent);
    child_plan["parts"][1]["requirements"] = json!(["REQ-002", "REQ-003"]);
    child_plan["parts"][2]["requirements"] = json!(["REQ-005"]);

    seed(
        &provider.files,
        &serde_json::to_vec(&child_spec).expect("spec"),
        &serde_json::to_vec(&child_design).expect("design"),
        &serde_json::to_vec(&child_plan).expect("plan"),
    );
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry(
                "m:SLICE-003",
                &message(&parent, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
            ),
            entry("m:SLICE-002", &message(&parent, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2)),
            entry("m:SLICE-001", &message(&parent, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
        ]),
    );

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["change"],
        json!({
            "from": parent,
            "commit": LABELLED,
            "rebuilt": [
                {"id": "PART-002", "name": "sessions", "reasons": ["plan"]},
                {"id": "PART-003", "name": "orders", "reasons": ["requirements", "departed"]}
            ],
            "carried": ["PART-001"],
            "retiring": []
        }),
        "{envelope}"
    );
    let calls = provider.target.calls();
    assert_eq!(calls.len(), 2);
    let orders_slice = &calls.iter().find(|(_, s, _)| s.part == "PART-003").expect("orders").1;
    let orders_change = orders_slice.change.as_ref().expect("SliceChange on orders");
    assert_eq!(
        orders_change
            .departed
            .iter()
            .map(|d| (d.requirement.id.as_str(), d.moved_to.as_deref()))
            .collect::<Vec<_>>(),
        [("REQ-003", Some("PART-002")), ("REQ-004", None)]
    );
    let scopes = provider.target.scopes();
    assert_eq!(
        scopes[0].1.retired,
        [Departure {
            requirement: orders_change.departed[1].requirement.clone(),
            moved_to: None,
        }]
    );
    provider.vcs.assert_exhausted();
}

// A design type signature change reaches every part that owns or uses the
// type (`reasons: ["types"]`), populates `SliceChange.types` with the before
// and after signatures, and lays the part's owned types on its first slice
// alone when the rebuilt part spans multiple slices.
#[tokio::test]
async fn build_change_types_on_first_slice() {
    let mut parent_plan: Value = serde_json::from_slice(WIDE_PLAN).expect("wide-plan fixture");
    parent_plan["parts"][1]["uses"] = json!(["orders.order"]);
    parent_plan["parts"][1]["depends-on"] = json!(["PART-001", "PART-003"]);
    let provider = Provider::idle();
    let parent = seed_at(
        &provider.files,
        LABELLED,
        SPEC,
        DESIGN,
        &serde_json::to_vec(&parent_plan).expect("plan"),
    );

    let (mut child_spec, mut child_design): (Value, Value) = (
        serde_json::from_slice(SPEC).expect("spec"),
        serde_json::from_slice(DESIGN).expect("design"),
    );
    child_spec["parent"] = json!(parent);
    child_design["parent"] = json!(parent);
    child_design["sections"][1]["blocks"] = json!([
        {"type": {"key": "orders.order", "signature": "interface Order { id: string; total: number }", "role": "data"}},
        {"type": {"key": "orders.item", "signature": "interface Item { sku: string }", "role": "data"}}
    ]);
    let mut child_plan = parent_plan;
    child_plan["parent"] = json!(parent);
    child_plan["parts"][2]["types"] = json!(["orders.item", "orders.order"]);
    seed(
        &provider.files,
        &serde_json::to_vec(&child_spec).expect("spec"),
        &serde_json::to_vec(&child_design).expect("design"),
        &serde_json::to_vec(&child_plan).expect("plan"),
    );
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![
            entry("m:SLICE-002", &message(&parent, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2)),
            entry(
                "m:SLICE-003",
                &message(&parent, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
            ),
            entry("m:SLICE-001", &message(&parent, AUTHENTICATION, "REQ-001", "REQ-001", HEAD, 1)),
        ]),
    );

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG, "--budget", "1"],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["change"],
        json!({
            "from": parent,
            "commit": LABELLED,
            "rebuilt": [
                {"id": "PART-002", "name": "sessions", "reasons": ["types"]},
                {"id": "PART-003", "name": "orders", "reasons": ["types"]}
            ],
            "carried": ["PART-001"],
            "retiring": []
        }),
        "{envelope}"
    );

    let calls = provider.target.calls();
    let [(_, orders_first, _), (_, orders_second, _), (_, sessions, _)] = calls.as_slice() else {
        panic!("two serial slices of orders, then sessions: {calls:?}");
    };
    let item_type = DesignType {
        key: "orders.item".to_owned(),
        signature: "interface Item { sku: string }".to_owned(),
    };
    let order_type = DesignType {
        key: "orders.order".to_owned(),
        signature: "interface Order { id: string; total: number }".to_owned(),
    };
    assert_eq!(orders_first.types, [item_type, order_type], "owned types on first slice");
    assert_eq!(orders_second.types, [] as [DesignType; 0], "later slices carry no owned types");
    let order_change = TypeChange {
        key: "orders.order".to_owned(),
        before: Some("interface Order { id: string }".to_owned()),
        after: Some("interface Order { id: string; total: number }".to_owned()),
    };
    assert_eq!(
        orders_first.change.as_ref().expect("change").types,
        [
            TypeChange {
                key: "orders.item".to_owned(),
                before: None,
                after: Some("interface Item { sku: string }".to_owned()),
            },
            TypeChange {
                key: "orders.line".to_owned(),
                before: Some("interface Line { sku: string }".to_owned()),
                after: None,
            },
            order_change.clone(),
        ]
    );
    assert_eq!(sessions.change.as_ref().expect("change").types, [order_change]);
    provider.vcs.assert_exhausted();
}
