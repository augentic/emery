//! Verifies `emery build` over scripted target, file, and version-control capabilities.

#![cfg(not(target_arch = "wasm32"))]
#![expect(dead_code, reason = "helpers and constants resume in pending remainder")]

mod support;

use emery_adapter::target::{DesignType, MergeRule, Report, Requirement, Scenario, Slice, Verdict};
use emery_adapter::{Route, Trigger};
use emery_engine::EntryKind;
use emery_engine::vcs::REVISION;
use omnia_sdk::bad_gateway;
use omnia_test::SeenFormat;
use omnia_wasi_vcs::{Change, ChangeKind, Entry, Error, Merged, Rule, Strategy};
use serde_json::{Value, json};
use support::provider::{Provider, Rendezvous, builder_digest, cli};
use support::store::{seed, stage, stage_amendment};
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

fn script_heads(provider: &Provider, repo: &str, head: &str) {
    provider.vcs.heads.clear(repo);
    for _ in 0..8 {
        provider.vcs.heads.script(repo, Ok(head.to_owned()));
    }
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
    let at = worktree(slice);
    provider
        .vcs
        .pending
        .script(&at, Ok(paths.iter().map(|(path, kind)| change(path, *kind)).collect()));
    for (path, kind) in paths {
        match kind {
            ChangeKind::Added | ChangeKind::Modified => {
                let content = if path.ends_with("Cargo.toml") {
                    "[package]\nname = \"shop\"\n".to_owned()
                } else {
                    format!("// {path}\n")
                };
                provider.files.holds(&at, path, &content);
            }
            ChangeKind::Deleted => provider.files.remove(&at, path),
        }
    }
}

// Scripts one added file, `src/<name>.rs`, as what the next build of a slice leaves.
fn wrote(provider: &Provider, (slice, name): (&str, &str)) {
    changed(provider, slice, &[(&format!("src/{name}.rs"), ChangeKind::Added)]);
}

// Scripts what the adapter's initialisation adds to the integration copy,
// nothing where its layout adds nothing to the base.
fn script_verify_repairs(provider: &Provider, pending: impl Fn() -> Result<Vec<Change>, Error>) {
    for _ in 0..2 {
        provider.vcs.pending.script(INTEGRATION, Ok(Vec::new()));
        provider.files.holds(INTEGRATION, "src/orders.rs", "// orders\n");
        provider.files.holds(INTEGRATION, "Cargo.lock", "");
        provider.vcs.pending.script(INTEGRATION, pending());
    }
}

fn initialised(provider: &Provider, paths: &[&str]) {
    provider.vcs.pending.script(
        INTEGRATION,
        Ok(paths.iter().map(|path| change(path, ChangeKind::Added)).collect()),
    );
    for path in paths {
        provider.files.holds(INTEGRATION, path, "");
    }
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
    script_heads(&provider, ".", BASE);
    wrote(&provider, AUTHENTICATION);
    changed(&provider, "SLICE-002", &orders_changed);
    let before = provider.storage.snapshot().await;

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    // the text render
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 2 parts in 2 waves; budget 12 (adapter): 2 slices in 2 \
             waves, widest 1\n  base {BASE}\n  wave 1: 1 slice verified at 9f8e7d6c\n    \
             SLICE-001 authentication: covered 2/2, written 1 file, merged m:SLICE-001\n  wave 2: \
             1 slice verified at 9f8e7d6c\n    SLICE-002 orders: covered 1/2 (uncovered REQ-004), \
             written 3 files, merged m:SLICE-002\n  labelled emery/{id} at {HEAD}\n"
        )
    );

    // one gate, two builds in order, each to the guest the reference names
    let calls = provider.target.calls();
    assert!(
        calls.iter().all(|(id, _, _)| id == BUILDER_ID),
        "every dispatch uses the configured target: {calls:?}"
    );
    let [(first_id, first, first_root), (second_id, second, second_root)] = calls.as_slice() else {
        panic!("two slices are two dispatches: {calls:?}");
    };
    assert_eq!((first_id.as_str(), second_id.as_str()), (BUILDER_ID, BUILDER_ID));
    assert_eq!(
        (first_root.as_str(), second_root.as_str()),
        (worktree("SLICE-001").as_str(), worktree("SLICE-002").as_str()),
        "a working copy per slice, never the checkout"
    );

    // each slice is dispatched once; full lowering is recorded when slice wire is enabled (A26).
    assert_eq!(first.id, "SLICE-001");
    assert_eq!(second.id, "SLICE-002");
    assert_eq!(provider.target.verifies(), [INTEGRATION, INTEGRATION], "one verify per wave");
    assert_eq!(provider.target.inits(), [INTEGRATION], "one init, before any slice");

    // the sealed base, one commit and merge per slice, the label after each wave
    let mut expected = project(BASE);
    expected.extend(integrated(".", BASE, &id));
    provider.vcs.assert_subsequence(&expected, "the sealed base through both waves");
    let messages = [
        message(&id, AUTHENTICATION, "REQ-001, REQ-002", "REQ-001, REQ-002", BASE, 1),
        message(&id, ORDERS, "REQ-003, REQ-004", "REQ-003", HEAD, 2),
    ];
    assert_eq!(provider.vcs.messages(), messages);
    let merged: Vec<String> =
        provider.vcs.merged().into_iter().map(|(message, _)| message).collect();
    assert_eq!(merged, messages, "a merge carries the slice's message");

    assert_eq!(provider.storage.snapshot().await, before, "a build writes no engine state");
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
    script_heads(&provider, ".", BASE);
    for slice in [AUTHENTICATION, WIDE_ORDERS, WIDE_SESSIONS] {
        wrote(&provider, slice);
    }

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["parts"], json!([["PART-001", "PART-003"], ["PART-002"]]), "{envelope}");
    assert_eq!(envelope["waves"], json!([["SLICE-001", "SLICE-003"], ["SLICE-002"]]), "{envelope}");
    let mut wave_one = dispatched(&provider);
    let wave_two = wave_one.pop().expect("wave two");
    assert_eq!(
        wave_two,
        ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
        "wave two builds over the verified head"
    );
    wave_one.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        wave_one,
        [
            ("SLICE-001".to_owned(), BASE.to_owned(), worktree("SLICE-001")),
            ("SLICE-003".to_owned(), BASE.to_owned(), worktree("SLICE-003")),
        ],
        "the first wave over the base, in id order"
    );
    let mut expected = project(BASE);
    expected.extend(opened(".", BASE, &id));
    expected.extend(wave(".", &id, BASE, &[AUTHENTICATION, WIDE_ORDERS]));
    expected.extend(wave(".", &id, HEAD, &[WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    provider.vcs.assert_subsequence(&expected, "wide build waves");
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

    script_heads(&provider, ".", BASE);
    for slice in [AUTHENTICATION, WIDE_ORDERS, WIDE_SESSIONS] {
        wrote(&provider, slice);
    }
    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 3 parts in 2 waves; budget 12 (adapter): 3 slices in 2 \
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
    script_heads(&provider, ".", BASE);

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
    provider.vcs.assert_subsequence(&expected, "the same history as a wide run");

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
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    provider.files.holds(&worktree("SLICE-003"), "src/lib.rs", "pub mod orders;\n");
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

// Scripts the integration working copy an earlier run left sitting on
// `STAGED`, its first-parent chain back to the head holding `entries`.
fn staged(provider: &Provider, entries: Vec<Entry>) {
    provider.vcs.adds.script(INTEGRATION, Err(Error::Exists(INTEGRATION.to_owned())));
    for _ in 0..8 {
        provider.vcs.heads.script(INTEGRATION, Ok(STAGED.to_owned()));
    }
    provider.vcs.descends.script(".", Ok(true));
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

// The `Vcs` calls a run made after the first `skip`.
fn calls_since(provider: &Provider, skip: usize) -> Vec<String> {
    provider.vcs.calls().into_iter().skip(skip).collect()
}

// A higher-id slice already merged in the integration copy is named in the
// merge brief when a lower-id sibling of the same plan wave conflicts with
// it, even though id order would place it after the slice being merged.
#[tokio::test]
#[ignore = "component rung: resuming integration clears files the merge turn reads"]
async fn conflict_staged_higher() {
    let provider = Provider::answering([RESOLVED_LIB]);
    let id = seed(&provider.files, SPEC, DESIGN, WIDE_PLAN);
    let copy = worktree("SLICE-001");
    provider.target.fails("SLICE-001", bad_gateway!("the model timed out"));
    provider.target.fails("SLICE-001", bad_gateway!("the model timed out again"));
    script_heads(&provider, ".", BASE);
    wrote(&provider, WIDE_ORDERS);

    fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 4, "bad_gateway").await;
    provider.target.failures.lock().expect("failures").remove("SLICE-001");

    staged(
        &provider,
        vec![entry(
            "m:SLICE-003",
            &message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
        )],
    );
    script_heads(&provider, ".", BASE);
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
    changed(
        &provider,
        "SLICE-001",
        &[("src/authentication.rs", ChangeKind::Added), ("src/lib.rs", ChangeKind::Modified)],
    );
    provider.files.holds(&copy, "src/authentication.rs", "// authentication\n");

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

    assert!(
        provider
            .vcs
            .messages()
            .iter()
            .any(|message| message.contains("SLICE-003 orders resolved at src/lib.rs")),
        "the resolution is sealed over the slice's commit"
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
    provider.vcs.assert_subsequence(&expected, "merge conflict with prior slice");
    let orders = message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", BASE, 1);
    assert_eq!(
        provider.vcs.messages(),
        [
            message(&id, AUTHENTICATION, "REQ-001", "REQ-001", BASE, 1),
            orders,
            "SLICE-003 orders resolved at src/lib.rs".to_owned(),
            message(&id, WIDE_SESSIONS, "REQ-002", "REQ-002", HEAD, 2),
        ],
        "the resolution is sealed over the slice's commit"
    );
    assert!(
        provider.vcs.merged()[2]
            .1
            .iter()
            .any(|rule| rule.paths == "/src/lib.rs" && rule.strategy == Strategy::Theirs),
        "the second merge takes the resolution whole: {:?}",
        provider.vcs.merged()
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
    assert!(
        provider.vcs.merged()[2]
            .1
            .iter()
            .any(|rule| rule.paths == "/src/lib.rs" && rule.strategy == Strategy::Theirs),
        "with the path taken whole: {:?}",
        provider.vcs.merged()[2].1
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
            "built revision {id}\n  plan: 3 parts in 2 waves; budget 12 (adapter): 3 slices in 2 \
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
    assert!(provider.files.worktree_writes().is_empty(), "nothing is written on a refusal");
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
    provider.files.remove(INTEGRATION, "src/lib.rs");
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
    left_for_inspection(&provider);
    provider.vcs.assert_exhausted();
}

// A resolution turn that fails upstream is put once more before the failure
// is the run's, as a slice's build is.
#[tokio::test]
async fn conflict_retry() {
    let (provider, _) = conflicted_at_lib_on(
        Provider::answering([RESOLVED_LIB]).failing_model(1, "the bridge went away"),
    );
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    changed(&provider, "SLICE-003", &[("src/lib.rs", ChangeKind::Modified)]);
    wrote(&provider, WIDE_SESSIONS);

    cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    assert!(!provider.model.exchanges().is_empty(), "the merge turn is put at least once");
    assert!(
        provider.vcs.messages().iter().any(|message| message.contains("resolved at src/lib.rs")),
        "the resolution is sealed"
    );
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
    assert_eq!(
        provider.files.worktree_writes().len(),
        1,
        "the resolution was written, and read back"
    );
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
    conflicted_at_lib_on(Provider::answering(answers.iter().copied()))
}

fn conflicted_at_lib_on(provider: Provider) -> (Provider, String) {
    let id = seed(&provider.files, SPEC, DESIGN, WIDE_PLAN);
    let copy = worktree("SLICE-003");
    script_heads(&provider, ".", BASE);
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
#[ignore = "component rung: a follow-up build sees guest-written lockfiles under integration after the first run"]
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
    script_verify_repairs(&provider, repaired);

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        stdout.contains(
            "  wave 1: 1 slice verified at 9f8e7d6c, repaired\n    SLICE-001 authentication: \
             covered 2/2, written 0 files, merged m:SLICE-001\n"
        ),
        "{stdout}"
    );
    assert!(stdout.contains("  wave 2: 1 slice verified at 9f8e7d6c"), "{stdout}");
    assert!(
        provider
            .vcs
            .calls()
            .iter()
            .any(|call| call == &format!("commit {INTEGRATION} Wave 1 verified")),
        "a seal"
    );
    provider.vcs.assert_subsequence(
        &[
            format!("pending {INTEGRATION}"),
            format!("commit {INTEGRATION} Wave 1 verified"),
            format!("head {INTEGRATION}"),
            format!("label . emery/{id} {HEAD}"),
        ],
        "wave-one verify seal",
    );
    assert!(
        provider.vcs.messages().iter().any(|message| message
            == &format!(
                "Wave 1 verified\n\nRevision: {id}\nAdapter: {BUILDER}\nSlices: SLICE-001"
            )),
        "{:?}",
        provider.vcs.messages()
    );
    assert_eq!(provider.vcs.messages().len(), 4, "the two slices' commits and the two wave seals");

    provider.files.purge(INTEGRATION);
    provider.files.purge(WORKTREES);
    provider.vcs.pending.script(".", Ok(vec![]));
    provider.vcs.pending.script(INTEGRATION, Ok(vec![]));
    for path in ["Cargo.lock", "crates/orders/Cargo.lock"] {
        provider.files.remove(INTEGRATION, path);
    }
    script_verify_repairs(&provider, repaired);
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["repaired"], json!([1, 2]), "{envelope}");
    assert_eq!(
        envelope["sealed"],
        json!([
            {"wave": 1, "commit": "Wave-commit", "paths": ["Cargo.lock", "src/orders.rs"]},
            {"wave": 2, "commit": "Wave-commit", "paths": ["Cargo.lock", "src/orders.rs"]}
        ]),
        "{envelope}"
    );
    assert_eq!(
        envelope["verified"],
        verified(&[(1, &["SLICE-001"]), (2, &["SLICE-002"])]),
        "{envelope}"
    );

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
        12,
        "the two slices' commits and the wave seal, each run"
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
    provider.vcs.assert_subsequence(&expected, "resume from label chain");
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
            "built revision {id}\n  plan: 3 parts in 2 waves; budget 12 (adapter): 3 slices in 2 \
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
            "built revision {id}\n  plan: 2 parts in 2 waves; budget 12 (adapter): 2 slices in 2 \
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
    provider.vcs.assert_subsequence(&expected, "resume complete");
    provider.vcs.assert_exhausted();
}

// A build reads the current revision and never the candidate `specify`
// staged beside it: the candidate's plan is not built, the output names it
// as staged and not sealed, and both slots stand as they were.
#[tokio::test]
async fn build_candidate_staged() {
    let (provider, id) = planned();
    let candidate = stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN).await;
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
    let before = provider.storage.snapshot().await;

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "built revision {id}\n  plan: 2 parts in 2 waves; budget 12 (adapter): 2 slices in 2 \
             waves, widest 1\n  candidate {candidate} is staged and not sealed; `emery apply` \
             seals it\n  base {HEAD}\n  resumed: SLICE-001, SLICE-002\n  labelled emery/{id} at \
             {LABELLED}\n"
        ),
        "the two-slice revision is built, not the three-slice candidate"
    );
    assert_eq!(provider.storage.snapshot().await, before, "a build writes nothing to either slot");

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
    let amendment = stage_amendment(&provider.storage, AMENDMENT).await;
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
    let before = provider.storage.snapshot().await;

    let resp = cli_ok(&provider, &["emery", "build", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "built revision {id}\n  plan: 2 parts in 2 waves; budget 12 (adapter): 2 slices in 2 \
             waves, widest 1\n  amendment {amendment} is staged; `emery show amendment` reviews \
             it\n  base {HEAD}\n  resumed: SLICE-001, SLICE-002\n  labelled emery/{id} at \
             {LABELLED}\n"
        )
    );
    assert_eq!(provider.storage.snapshot().await, before, "a build writes nothing to either slot");

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
    provider.vcs.assert_subsequence(&expected, "no log over a label off the base");
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
    provider.vcs.assert_subsequence(&expected, "resume empty label chain");
    assert_eq!(provider.target.inits(), [INTEGRATION], "a label naming no slice is initialised");
    provider.vcs.assert_exhausted();
}

// The integration working copy a run left after its wave failed to verify
// holds the wave's slices merged over the head: they are not built again,
// the copy is cut again at its head so what the verify left uncommitted
// goes and the merges stay, the wave is verified from there with no turn
// spent on a build, and the rest of the plan builds over it.
#[tokio::test]
async fn build_resumed_staged() {
    let (provider, id) = wide();
    script_heads(&provider, ".", HEAD);
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
    provider.vcs.assert_subsequence(&expected, "resume staged integration");
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
            "built revision {id}\n  plan: 3 parts in 2 waves; budget 12 (adapter): 3 slices in 2 \
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
#[ignore = "component rung: multi-cli staging needs failures sync and dispatch-order assertions still settling"]
async fn resume_staged_sibling() {
    let (provider, id) = wide();
    script_heads(&provider, ".", HEAD);
    provider.target.fails("SLICE-003", bad_gateway!("the model timed out"));
    provider.target.fails("SLICE-003", bad_gateway!("the model timed out again"));

    let envelope =
        fail(&provider, &["emery", "build", "--config", PROJECT_CONFIG], 4, "bad_gateway").await;

    assert_message(
        &envelope,
        &format!(
            "slice `SLICE-003` (orders) failed in wave 1; SLICE-001 merged before it stays \
             committed in `{INTEGRATION}`: adapter `acme:builder`: the model timed out"
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
    provider.target.failures.lock().expect("failures").remove("SLICE-003");

    // the next run builds the failed slice alone, over the staged sibling
    script_heads(&provider, ".", HEAD);
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
            ("SLICE-002".to_owned(), HEAD.to_owned(), worktree("SLICE-002")),
            ("SLICE-003".to_owned(), HEAD.to_owned(), worktree("SLICE-003")),
        ],
        "the failed slice builds over the staged head, the next wave over the verified head"
    );
    let mut expected = project(HEAD);
    expected.extend(restaged(&id, STAGED));
    expected.extend(wave(".", &id, HEAD, &[WIDE_ORDERS, WIDE_SESSIONS]));
    expected.push(format!("remove {INTEGRATION}"));
    support::vcs::assert_subsequence(
        &calls_since(&provider, calls.len()),
        &expected,
        "resume failed sibling",
    );
    assert_eq!(
        provider.vcs.messages()[1],
        message(&id, WIDE_ORDERS, "REQ-003, REQ-004", "REQ-003, REQ-004", HEAD, 1),
        "the failed slice is sealed over the verified head, in the wave it was staged for"
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
    let (provider, _id) = planned();
    provider.vcs.adds.script(INTEGRATION, Err(Error::Exists(INTEGRATION.to_owned())));
    provider.vcs.heads.script(INTEGRATION, Ok(STAGED.to_owned()));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope.get("resumed").is_none(), "{envelope}");
    assert_eq!(envelope["slices"].as_array().map(Vec::len), Some(2), "{envelope}");
    provider.vcs.assert_subsequence(
        &[
            format!("add . {INTEGRATION} {HEAD}"),
            format!("head {INTEGRATION}"),
            format!("remove {INTEGRATION}"),
            format!("add . {INTEGRATION} {HEAD}"),
        ],
        "stale integration off the head is cut again",
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
    provider.vcs.assert_subsequence(&restaged(&id, HEAD), "stale other revision");
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
    support::vcs::assert_subsequence(
        &calls_since(&provider, calls.len()),
        &restaged(&id, HEAD),
        "stale wave-only chain",
    );
    assert_eq!(dispatched(&provider).len(), 4, "every slice builds, each run");
    provider.vcs.assert_exhausted();
}

// The `[target]` table names the adapter and its pin, whether the file is
// named or discovered; the `[[source]]` entries beside it stay out.
#[tokio::test]
async fn build_from_config() {
    let (provider, _) = planned();
    let pin = builder_digest();
    let config = provider.write_project(
        "tests/build/a26-from-config.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
             [target]\nadapter = \"{BUILDER}\"\nrepository = \".\"\ndigest = \"{pin}\"\n"
        ),
    );
    cli_ok(&provider, &["emery", "build", "--config", &config]).await;

    assert!(
        provider.source.metadata.lock().expect("metadata").is_empty(),
        "no source adapter loads for a build"
    );
    assert_eq!(provider.target.calls().len(), 2);
}

// A run naming no adapter reads the project-root file. The CWD move is
// hermetic under nextest's process-per-test isolation.
#[tokio::test]
async fn build_discovered() {
    let (provider, _) = planned();
    provider.write_project(
        "emery.toml",
        format!("[target]\nadapter = \"{BUILDER}\"\nrepository = \".\"\n"),
    );

    cli_ok(&provider, &["emery", "build"]).await;

    assert_eq!(provider.target.calls().len(), 2);
}

// A `[target] repository` builds into a clone of it under the target's
// name, from its default branch as the origin holds it: cloned when the run
// has none, fetched when it has one, the label set in the clone and the
// clone named beside the base.
#[tokio::test]
async fn build_repository() {
    let (provider, id) = planned();
    let config = provider.write_project(
        "tests/build/a26-repository.toml",
        format!(
            "[target]
adapter = \"{BUILDER}\"
repository = \"{REMOTE_URL}\"
"
        ),
    );

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
    provider.vcs.assert_subsequence(&expected, "clone then build from default branch");
    let roots: Vec<String> = provider.target.calls().into_iter().map(|(_, _, root)| root).collect();
    assert_eq!(roots, [worktree("SLICE-001"), worktree("SLICE-002")]);

    // the next run finds the clone and fetches
    provider.vcs.clones.script(TARGET_REPO, Err(Error::Exists(TARGET_REPO.to_owned())));
    cli_ok(&provider, &["emery", "build", "--config", &config]).await;
    let calls = provider.vcs.calls();
    support::vcs::assert_subsequence(
        &calls[expected.len()..],
        &[
            format!("clone {REMOTE_URL} {TARGET_REPO}"),
            format!("fetch {TARGET_REPO} origin"),
            format!("resolve {TARGET_REPO} origin/HEAD"),
        ],
        "finds the clone and fetches",
    );
    assert!(!calls.iter().any(|call| call.starts_with("push ")), "nothing is pushed: {calls:?}");
    provider.vcs.assert_exhausted();
}

// A `[target] name` is the directory the repository is kept under, cloned
// or created; one that is not kebab-case is refused before anything is read.
#[tokio::test]
async fn build_named() {
    let (provider, id) = planned();

    let config = provider.write_project(
        "tests/build/a26-2.toml",
        format!(
            "[target]\nname = \"shop\"\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n"
        ),
    );
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", "--config", &config]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["repository"], "./.emery/vcs/repos/shop", "{envelope}");
    assert_eq!(provider.vcs.calls()[0], format!("clone {REMOTE_URL} ./.emery/vcs/repos/shop"));
    assert!(
        provider.vcs.calls().contains(&format!("label ./.emery/vcs/repos/shop emery/{id} {HEAD}"))
    );

    // the repository `apply` created is read under the same name, never created here
    let config = provider.write_project(
        "tests/build/a26-named-local.toml",
        format!("[target]\nname = \"shop\"\nadapter = \"{BUILDER}\"\n"),
    );
    cli_ok(&provider, &["emery", "build", "--config", &config]).await;
    assert!(provider.vcs.calls().contains(&"head ./.emery/vcs/repos/shop".to_owned()));
    assert!(!provider.vcs.calls().iter().any(|call| call.starts_with("init ")), "nothing created");

    let config = provider.write_project(
        "tests/build/a26-named-bad.toml",
        format!("[target]\nname = \"Shop\"\nadapter = \"{BUILDER}\"\n"),
    );
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
    let (provider, id) = planned();
    let pin = builder_digest();
    let config = provider.write_project(
        "tests/build/a26-layered.toml",
        format!(
            "[target]\nname = \"shop\"\nadapter = \"acme:other@1.0.0\"\ndigest = \"{pin}\"\n\
             repository = \"{REMOTE_URL}\"\n"
        ),
    );
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER, "--config", &config])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["repository"], "./.emery/vcs/repos/shop", "{envelope}");
    assert_eq!(provider.target.calls().len(), 2, "the command-line adapter is the one gated");
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
    script_heads(&provider, ".", BASE);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "build", BUILDER, "--repository", "."])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["base"], BASE, "{envelope}");
    let mut expected = project(BASE);
    expected.extend(integrated(".", BASE, &id));
    provider.vcs.assert_subsequence(&expected, "built into the project checkout, nothing created");

    // over a file, the flag's URL is cloned in place of the file's
    let config = provider.write_project(
        "tests/build/a26-repo-flag.toml",
        format!(
            "[target]\nadapter = \"{BUILDER}\"\nrepository = \"https://example.com/acme/old.git\"\n"
        ),
    );
    let before = provider.vcs.calls().len();
    cli_ok(&provider, &["emery", "build", "--config", &config, "--repository", REMOTE_URL]).await;
    support::vcs::assert_subsequence(
        &provider.vcs.calls()[before..],
        &[format!("clone {REMOTE_URL} {TARGET_REPO}")],
        "the flag's URL is cloned in place of the file's",
    );
    provider.vcs.assert_exhausted();
}

// The grammar names the target's repository alone: a `branch` or a `remote`
// in `[target]` is no field of it, refused at the file before anything loads.
#[tokio::test]
async fn build_target_grammar() {
    let (provider, _) = planned();

    for (key, value) in [("branch", "main"), ("remote", "origin")] {
        let config = provider.write_project(
            &format!("tests/build/grammar-{key}.toml"),
            format!(
                "[target]\nadapter = \"{BUILDER}\"\nrepository = \"{REMOTE_URL}\"\n{key} = \
                 \"{value}\"\n"
            ),
        );
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

    provider.vcs.assert_subsequence(
        &[
            format!("add . {INTEGRATION} {HEAD}"),
            format!("head {INTEGRATION}"),
            format!("remove {INTEGRATION}"),
            format!("add . {INTEGRATION} {HEAD}"),
            format!("pending {INTEGRATION}"),
            format!("add . {} {HEAD}", worktree("SLICE-001")),
            format!("remove {}", worktree("SLICE-001")),
            format!("add . {} {HEAD}", worktree("SLICE-001")),
        ],
        "stale integration and slice copies are cut again",
    );
    assert_eq!(provider.target.calls().len(), 2);
    provider.vcs.assert_exhausted();
}
