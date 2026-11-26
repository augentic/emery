//! Verifies `emery build` over scripted target, file, and version-control capabilities.

#![cfg(not(target_arch = "wasm32"))]
#![expect(dead_code, reason = "helpers and constants resume in pending remainder")]

mod support;

use emery_adapter::target::{DesignType, MergeRule, Report, Requirement, Scenario, Slice, Verdict};
use emery_adapter::{Route, Trigger};
use emery_engine::{EntryKind, vcs::REVISION};
use omnia_sdk::bad_gateway;
use omnia_test::SeenFormat;
use omnia_wasi_vcs::{Change, ChangeKind, Entry, Error, Merged, Rule, Strategy};
use serde_json::{Value, json};
use support::provider::{Provider, Rendezvous, cli};
use support::store::seed;
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
    for _ in 0..24 {
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
    provider.vcs.pending.script(
        &at,
        Ok(paths.iter().map(|(path, kind)| change(path, *kind)).collect()),
    );
    for (path, kind) in paths {
        match kind {
            ChangeKind::Added | ChangeKind::Modified => {
                let content = if path.ends_with("Cargo.toml") {
                    "[package]\nname = \"shop\"\n".to_owned()
                } else if path.ends_with("lib.rs") {
                    format!("// {path}\n")
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
