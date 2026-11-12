//! Verifies `emery show` over amendments, schedules, what a staged amendment refuses, and a hand-merged `spec/`, over scripted storage.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use emery_engine::{STAGED_CONTAINER, STAGED_KEY};
use omnia_sdk::api::command::USAGE_EXIT;
use omnia_sdk::vcs::Entry;
use serde_json::{Value, json};
use support::provider::{Provider, cli};
use support::scratch::Scratch;
use support::store::{seed, seed_at, stage, stage_amendment, stage_fresh};
use support::vcs::HEAD;
use support::{cli_ok, fail};

// The one-source reference `specify` names; the scripted loader lands any
// exact package reference.
const SOURCE: &str = "acme:source@1.0.0";
// The target every revision verb reads through, into the project checkout.
const PROJECT_CONFIG: &str = "tests/build/project.toml";
const BUILDER: &str = "acme:builder@2.1.0";
const LABELLED: &str = "fedcba9876543210fedcba9876543210fedcba98";

const AMENDMENT: &[u8] = include_bytes!("specify/6-amendment.json");
const AMENDMENT_RENDERED: &str = include_str!("specify/6-amendment.md");

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");
const WIDE_PLAN: &[u8] = include_bytes!("build/wide-plan.json");

// A staged amendment renders as `amendment.md`: the front matter names its
// id, the counts lead, and each class with entries follows in the fixed
// order. The read touches no repository, and `--staged` changes nothing.
#[tokio::test]
async fn show_amendment() {
    let provider = Provider::idle();
    let id = stage_amendment(&provider.storage, AMENDMENT);

    let resp = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        AMENDMENT_RENDERED.replace("<revision>", &id)
    );
    assert!(provider.vcs.calls().is_empty(), "an amendment is read from the slot alone");

    let staged = cli_ok(&provider, &["emery", "show", "amendment", "--staged"]).await;
    assert_eq!(staged.stdout, resp.stdout, "`--staged` reads the same slot");

    let resp = cli_ok(&provider, &["emery", "--format", "json", "show", "amendment"]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let document: Value = serde_json::from_slice(AMENDMENT).expect("the fixture is JSON");
    assert_eq!(envelope["revision"], id, "the envelope names the amendment");
    assert_eq!(envelope["document"], document, "the envelope carries the typed amendment");
    assert_eq!(
        envelope["body"],
        AMENDMENT_RENDERED.replace("<revision>", &id),
        "the envelope carries the projection"
    );
}

// `show amendment` reads the slot for an amendment alone: nothing staged, or
// a revision candidate staged, is `nothing-staged`, hinted to `emery amend`.
#[tokio::test]
async fn show_amendment_none() {
    let provider = Provider::idle();
    let envelope = fail(&provider, &["emery", "show", "amendment"], 2, "nothing-staged").await;
    assert_eq!(envelope["message"], "no amendment is staged", "{envelope}");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery amend")),
        "{envelope}"
    );

    stage(&provider.storage, SPEC, DESIGN, PLAN);
    let envelope = fail(&provider, &["emery", "show", "amendment"], 2, "nothing-staged").await;
    assert_eq!(envelope["message"], "no amendment is staged", "{envelope}");
}

// A staged amendment holds the slot: `specify` is refused before any
// adapter loads, and a revision document asked of the slot names it, each
// `amendment-pending` naming the amendment and leaving it staged; `apply`
// seals it, which `tests/apply.rs` asserts.
#[tokio::test]
async fn amendment_pending() {
    let provider = Provider::idle();
    let id = stage_amendment(&provider.storage, AMENDMENT);

    let envelope = fail(&provider, &["emery", "specify", SOURCE], 1, "amendment-pending").await;
    assert!(envelope["message"].as_str().is_some_and(|text| text.contains(&id)), "{envelope}");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery show amendment")),
        "{envelope}"
    );
    assert!(provider.source.calls().is_empty(), "no source is extracted over a staged amendment");

    let envelope =
        fail(&provider, &["emery", "show", "spec", "--staged"], 1, "amendment-pending").await;
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery show amendment")),
        "{envelope}"
    );

    let envelope = fail(
        &provider,
        &["emery", "show", "spec", "--config", PROJECT_CONFIG],
        2,
        "spec-not-generated",
    )
    .await;
    assert!(
        envelope["message"]
            .as_str()
            .is_some_and(|text| text.contains(&format!("amendment `{id}` is staged"))),
        "{envelope}"
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery show amendment")),
        "{envelope}"
    );

    assert!(
        provider.storage.object(STAGED_CONTAINER, &format!("{id}/amendment.json")).is_some(),
        "the amendment stays staged"
    );
    provider.model.assert_exhausted();
}

// A `spec/` at a head no label names whose documents read as no revision —
// a hand merge of two branches that each sealed one — is refused rather than
// served: `show` fails `spec-mismatch` naming the commit and what failed to
// read, hinted to keep one side's three files whole, and `apply` over a
// fresh candidate is refused the same before anything is written, since the
// tree is the operator's to restore first.
#[tokio::test]
async fn show_hand_merged() {
    let hand_merged = b"<<<<<<< a\n{\"emery\": 8}\n=======\n{\"emery\": 8}\n>>>>>>> b\n";
    let provider = Provider::idle();
    seed(&provider.files, hand_merged, DESIGN, PLAN);

    let envelope =
        fail(&provider, &["emery", "show", "spec", "--config", PROJECT_CONFIG], 1, "spec-mismatch")
            .await;

    let message = envelope["message"].as_str().expect("a message");
    assert!(
        message.starts_with(&format!("`spec/` at commit `{HEAD}` holds documents")),
        "{envelope}"
    );
    assert!(message.contains("edited or merged by hand"), "{envelope}");
    assert!(
        envelope["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("git checkout <branch> -- spec/")),
        "{envelope}"
    );

    stage_fresh(&provider.storage, SPEC, DESIGN, PLAN);
    fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "spec-mismatch").await;
    assert!(provider.files.writes().is_empty(), "nothing is written");
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
}

// A hand merge that kept one side's `spec.json` beside the other side's
// `design.json` and `plan.json` leaves three documents that each read, yet
// read as no one revision's: the plan places a requirement the
// specification does not state and leaves one it states unplaced. It is
// refused the same way, the description naming the breaks, so the hint
// says to keep one side whole.
#[tokio::test]
async fn show_hand_mixed() {
    let mut plan: Value = serde_json::from_slice(PLAN).expect("the plan fixture");
    plan["parts"][1]["requirements"] = serde_json::json!(["REQ-003", "REQ-005"]);
    let provider = Provider::idle();
    seed(&provider.files, SPEC, DESIGN, plan.to_string().as_bytes());

    let envelope =
        fail(&provider, &["emery", "show", "spec", "--config", PROJECT_CONFIG], 1, "spec-mismatch")
            .await;

    let message = envelope["message"].as_str().expect("a message");
    assert_eq!(
        message,
        format!(
            "`spec/` at commit `{HEAD}` holds documents that read as no one revision's \
             (`REQ-005` is placed by `PART-002` and is no requirement of the specification; \
             `REQ-004` is placed by no part), so they were edited or merged by hand; a seal \
             writes them whole"
        ),
        "{envelope}"
    );
    assert!(
        envelope["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("git checkout <branch> -- spec/")),
        "{envelope}"
    );

    stage_fresh(&provider.storage, SPEC, DESIGN, PLAN);
    fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "spec-mismatch").await;
    assert!(provider.files.writes().is_empty(), "nothing is written");
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
}

// An amendment whose stored bytes no longer hash to the id its record names
// is corruption, so `show amendment` fails closed rather than rendering it.
#[tokio::test]
async fn corrupt_amendment() {
    let provider = Provider::idle();
    let id = stage_amendment(&provider.storage, AMENDMENT);
    let tampered = String::from_utf8_lossy(AMENDMENT).replace("'hello'", "'hi'");
    provider.storage.insert_object(
        STAGED_CONTAINER,
        &format!("{id}/amendment.json"),
        tampered.as_bytes(),
    );

    fail(&provider, &["emery", "show", "amendment"], 3, "server_error").await;

    // a record naming an amendment the container lacks is corruption too
    let provider = Provider::idle();
    let record = serde_json::json!({
        "id": "0123456789abcdef",
        "kind": "amendment",
        "base": {"revision": "r", "commit": "c"},
    })
    .to_string();
    provider.storage.insert_state(STAGED_KEY, record.as_bytes());
    fail(&provider, &["emery", "show", "amendment"], 3, "server_error").await;
}

// `show schedule` cuts the sealed plan under the target adapter's metadata
// budget when neither `--budget` nor `[target] budget` sets one, spending no
// model turn; `[target] budget` overrides the adapter's default without
// loading the adapter.
#[tokio::test]
async fn show_schedule() {
    let mut provider = Provider::idle();
    provider.target.budget = Some(4);
    let id = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "show", "schedule", "--config", PROJECT_CONFIG]).await;
    let expected = format!(
        "---\nemery: 10\nrevision: {id}\n---\n\n# Schedule\n\nBudget: 4 (adapter)\nWaves: \
         [SLICE-001], [SLICE-002]\n\n## Slice: authentication\n\nID: SLICE-001\nPart: PART-001 \
         (1/1)\nWave: 1\nRequirements: [REQ-001, REQ-002]\n\n## Slice: orders\n\nID: \
         SLICE-002\nPart: PART-002 (1/1)\nWave: 2\nRequirements: [REQ-003, REQ-004]\nTypes: \
         [orders.line, orders.order]\n"
    );
    assert_eq!(String::from_utf8_lossy(&resp.stdout), expected);
    assert_eq!(
        *provider.target.metadata.lock().expect("metadata"),
        ["acme:builder"],
        "the target adapter's metadata supplies the budget when neither flag nor config sets one"
    );
    provider.model.assert_exhausted();

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "show", "schedule", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["body"], expected, "{envelope}");
    assert_eq!(
        envelope["document"],
        json!({
            "budget": 4,
            "source": "adapter",
            "slices": [
                {
                    "id": "SLICE-001",
                    "part": "PART-001",
                    "name": "authentication",
                    "index": 1,
                    "count": 1,
                    "requirements": ["REQ-001", "REQ-002"],
                    "over-budget": false,
                },
                {
                    "id": "SLICE-002",
                    "part": "PART-002",
                    "name": "orders",
                    "index": 1,
                    "count": 1,
                    "requirements": ["REQ-003", "REQ-004"],
                    "types": ["orders.line", "orders.order"],
                    "over-budget": false,
                },
            ],
            "waves": [["SLICE-001"], ["SLICE-002"]],
            "over-budget": [],
        }),
        "{envelope}"
    );

    // `[target] budget` overrides the adapter's metadata without loading the adapter
    let scratch = Scratch::new();
    let config = scratch
        .config("[target]\nadapter = \"acme:builder@2.1.0\"\nrepository = \".\"\nbudget = 6\n");
    let mut provider = Provider::idle();
    provider.target.budget = Some(4);
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "show", "schedule", "--config", &config])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["document"]["budget"], 6, "{envelope}");
    assert_eq!(envelope["document"]["source"], "config", "{envelope}");
    assert!(
        provider.target.metadata.lock().expect("metadata").is_empty(),
        "a configured budget needs no adapter metadata load"
    );
}

// `--budget` cuts a multi-sub-stem part into serial slices, packs untriggered
// sub-stems before triggered ones, keeps a shared-trigger group whole as
// `over-budget`, and skips loading the target adapter.
#[tokio::test]
async fn schedule_budget_flag() {
    let mut spec: Value = serde_json::from_slice(SPEC).expect("spec fixture");
    spec["requirements"][1]["subject"] = json!("orders.validate.schema");
    spec["requirements"][2]["subject"] = json!("orders.create.route");
    spec["requirements"][3]["subject"] = json!("orders.create.persist");
    spec["requirements"][3]["trigger"] = json!({"http": {"method": "POST", "path": "/orders"}});

    let mut plan: Value = serde_json::from_slice(PLAN).expect("plan fixture");
    plan["parts"][0]["requirements"] = json!(["REQ-001"]);
    plan["parts"][1]["requirements"] = json!(["REQ-002", "REQ-003", "REQ-004"]);

    let provider = Provider::idle();
    let id =
        seed(&provider.files, spec.to_string().as_bytes(), DESIGN, plan.to_string().as_bytes());

    let resp = cli_ok(
        &provider,
        &[
            "emery",
            "--format",
            "json",
            "show",
            "schedule",
            "--config",
            PROJECT_CONFIG,
            "--budget",
            "1",
        ],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["document"]["budget"], 1, "{envelope}");
    assert_eq!(envelope["document"]["source"], "flag", "{envelope}");
    assert_eq!(
        envelope["document"]["waves"],
        json!([["SLICE-001"], ["SLICE-002"], ["SLICE-003"]]),
        "{envelope}"
    );
    assert_eq!(envelope["document"]["over-budget"], json!(["SLICE-003"]), "{envelope}");
    assert_eq!(
        envelope["document"]["slices"],
        json!([
            {
                "id": "SLICE-001",
                "part": "PART-001",
                "name": "authentication",
                "index": 1,
                "count": 1,
                "requirements": ["REQ-001"],
                "over-budget": false,
            },
            {
                "id": "SLICE-002",
                "part": "PART-002",
                "name": "orders",
                "index": 1,
                "count": 2,
                "requirements": ["REQ-002"],
                "types": ["orders.line", "orders.order"],
                "over-budget": false,
            },
            {
                "id": "SLICE-003",
                "part": "PART-002",
                "name": "orders",
                "index": 2,
                "count": 2,
                "requirements": ["REQ-003", "REQ-004"],
                "over-budget": true,
            },
        ]),
        "{envelope}"
    );
    assert!(
        provider.target.metadata.lock().expect("metadata").is_empty(),
        "`--budget` resolves the budget without loading the target adapter"
    );

    let text =
        cli_ok(&provider, &["emery", "show", "schedule", "--config", PROJECT_CONFIG, "-b", "1"])
            .await;
    let body = String::from_utf8_lossy(&text.stdout);
    assert_eq!(envelope["body"], body.as_ref());
    assert!(body.contains("Budget: 1 (flag)\nWaves: [SLICE-001], [SLICE-002], [SLICE-003]\nOver budget: [SLICE-003]\n"), "{body}");
    assert!(body.contains("## Slice: orders (1/2)\n\nID: SLICE-002\nPart: PART-002 (1/2)\nWave: 2\nRequirements: [REQ-002]\nTypes: [orders.line, orders.order]\n"), "{body}");
    assert!(body.contains("## Slice: orders (2/2)\n\nID: SLICE-003\nPart: PART-002 (2/2)\nWave: 3\nRequirements: [REQ-003, REQ-004]\nOver budget: true\n"), "{body}");

    let zero =
        cli(&provider, &["emery", "show", "schedule", "--config", PROJECT_CONFIG, "--budget", "0"])
            .await;
    assert_eq!(zero.exit, USAGE_EXIT);
}

// `show schedule --staged` is refused: a schedule cuts the sealed plan
// `emery build` reads, never the staged candidate.
#[tokio::test]
async fn schedule_staged_flag() {
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);

    let envelope =
        fail(&provider, &["emery", "show", "schedule", "--staged"], 1, "bad_request").await;
    assert!(
        envelope["message"].as_str().is_some_and(|text| text.contains("emery show plan --staged")),
        "{envelope}"
    );
    assert!(provider.vcs.calls().is_empty(), "refused before touching the repository");
    assert!(provider.target.metadata.lock().expect("metadata").is_empty());
}

fn slice_entry(
    commit: &str, revision: &str, (slice, part, name): (&str, &str, &str), reqs: &str, wave: usize,
) -> Entry {
    Entry {
        id: commit.to_owned(),
        message: format!(
            "{slice} {name}\n\nSlice: {slice}\nPart: {part}\nRevision: {revision}\nBudget: \
             12\nRequirements: {reqs}\nCovered: {reqs}\nAdapter: {BUILDER}\nBase: {HEAD}\nWave: \
             {wave}"
        ),
    }
}

// Seeds a built three-part parent (`WIDE_PLAN`) and a child that leaves
// `PART-001` (`authentication`) carried, changes `PART-002` (`sessions`), and
// removes `PART-003` (`orders`).
fn change_schedule_lineage() -> (Provider, String, String, Vec<Entry>) {
    let provider = Provider::idle();
    let parent = seed_at(&provider.files, LABELLED, SPEC, DESIGN, WIDE_PLAN);

    let mut child_spec: Value = serde_json::from_slice(SPEC).expect("spec");
    child_spec["parent"] = json!(parent);
    child_spec["requirements"].as_array_mut().expect("requirements").truncate(2);
    child_spec["requirements"][1]["body"] = json!(["Sessions expire after 15 minutes."]);

    let mut child_design: Value = serde_json::from_slice(DESIGN).expect("design");
    child_design["parent"] = json!(parent);
    child_design["sections"].as_array_mut().expect("sections").truncate(1);

    let mut child_plan: Value = serde_json::from_slice(WIDE_PLAN).expect("wide-plan");
    child_plan["parent"] = json!(parent);
    child_plan["parts"].as_array_mut().expect("parts").truncate(2);

    let child = seed(
        &provider.files,
        &serde_json::to_vec(&child_spec).expect("spec"),
        &serde_json::to_vec(&child_design).expect("design"),
        &serde_json::to_vec(&child_plan).expect("plan"),
    );

    let parent_log = vec![
        slice_entry(
            "m:SLICE-003",
            &parent,
            ("SLICE-003", "PART-003", "orders"),
            "REQ-003, REQ-004",
            1,
        ),
        slice_entry("m:SLICE-002", &parent, ("SLICE-002", "PART-002", "sessions"), "REQ-002", 2),
        slice_entry(
            "m:SLICE-001",
            &parent,
            ("SLICE-001", "PART-001", "authentication"),
            "REQ-001",
            1,
        ),
    ];
    (provider, parent, child, parent_log)
}

// Over a built ancestor, `show schedule` settles the change basis, asks
// `Task::Scope` when a target adapter is configured (or uses the minimum
// scope and says so when none is), and marks `[carried]` and `[retiring]`
// slices in the projection.
#[tokio::test]
async fn show_schedule_change() {
    let (provider, parent, child, parent_log) = change_schedule_lineage();
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(parent_log.clone()));

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "show", "schedule", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], child, "{envelope}");
    assert_eq!(envelope["document"]["scoped"], json!(true), "{envelope}");
    assert_eq!(
        envelope["document"]["change"],
        json!({
            "from": parent,
            "commit": LABELLED,
            "rebuilt": [{"id": "PART-002", "name": "sessions", "reasons": ["requirements"]}],
            "carried": ["PART-001"],
            "retiring": ["SLICE-003"],
        }),
        "{envelope}"
    );
    let expected = format!(
        "---\nemery: 10\nrevision: {child}\n---\n\n# Schedule\n\nBudget: 12 (default)\nWaves: \
         [SLICE-001, SLICE-003], [SLICE-002]\nChange: {parent} at {LABELLED}\n\n## Slice: \
         authentication [carried]\n\nID: SLICE-001\nPart: PART-001 (1/1)\nWave: 1\nRequirements: \
         [REQ-001]\n\n## Slice: sessions\n\nID: SLICE-002\nPart: PART-002 (1/1)\nWave: \
         2\nRequirements: [REQ-002]\n\n## Slice: orders [retiring]\n\nID: SLICE-003\nPart: \
         PART-003 (1/1)\nWave: 1\nRequirements: []\nDeparted: [REQ-003, REQ-004]\n"
    );
    assert_eq!(envelope["body"], expected, "{envelope}");
    assert_eq!(provider.target.scopes().len(), 1, "`Task::Scope` asked once");
    provider.vcs.assert_exhausted();

    // when the target adapter widens the scope, the widened part is unmarked
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(parent_log.clone()));
    provider.target.scope(Ok(vec!["PART-001".to_owned(), "PART-002".to_owned()]));
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "show", "schedule", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["document"]["change"]["rebuilt"],
        json!([
            {"id": "PART-001", "name": "authentication", "reasons": ["adapter"]},
            {"id": "PART-002", "name": "sessions", "reasons": ["requirements"]},
        ]),
        "{envelope}"
    );
    assert_eq!(envelope["document"]["change"]["carried"], json!([]), "{envelope}");
    assert!(
        envelope["body"].as_str().is_some_and(
            |body| body.contains("## Slice: authentication\n") && !body.contains("[carried]")
        ),
        "{envelope}"
    );
    provider.vcs.assert_exhausted();

    // with no target adapter configured, `show schedule` uses the minimum scope and says so
    let scratch = Scratch::new();
    let repo_only = scratch.config("[target]\nrepository = \".\"\n");
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(".", Ok(parent_log));
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "show", "schedule", "--config", &repo_only],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["document"]["scoped"], json!(false), "{envelope}");
    assert_eq!(envelope["document"]["change"]["carried"], json!(["PART-001"]), "{envelope}");
    assert!(
        envelope["body"].as_str().is_some_and(|body| {
            body.contains(&format!(
                "Change: {parent} at {LABELLED} (minimum scope; no target adapter)\n"
            )) && body.contains("## Slice: authentication [carried]\n")
                && body.contains("## Slice: orders [retiring]\n")
        }),
        "{envelope}"
    );
    assert_eq!(provider.target.scopes().len(), 2, "no scope call without an adapter");
    provider.vcs.assert_exhausted();
}

// `show schedule --whole`, or a revision whose parent build is incomplete,
// projects a whole schedule with no `change` field, no `Change:` header, and
// no `[carried]` or `[retiring]` slice marks.
#[tokio::test]
async fn show_schedule_whole() {
    let (provider, parent, child, _) = change_schedule_lineage();

    // `--whole` skips lineage inspection and `Task::Scope`
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "show", "schedule", "--whole", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], child, "{envelope}");
    assert!(envelope["document"].get("change").is_none(), "{envelope}");
    assert!(envelope["document"].get("scoped").is_none(), "{envelope}");
    let expected = format!(
        "---\nemery: 10\nrevision: {child}\n---\n\n# Schedule\n\nBudget: 12 (default)\nWaves: \
         [SLICE-001], [SLICE-002]\n\n## Slice: authentication\n\nID: SLICE-001\nPart: PART-001 \
         (1/1)\nWave: 1\nRequirements: [REQ-001]\n\n## Slice: sessions\n\nID: SLICE-002\nPart: \
         PART-002 (1/1)\nWave: 2\nRequirements: [REQ-002]\n"
    );
    assert_eq!(envelope["body"], expected, "{envelope}");
    assert!(provider.target.scopes().is_empty(), "`--whole` never asks `scope`");
    provider.vcs.assert_exhausted();

    // an incomplete parent build falls back to the same whole schedule
    provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
    provider.vcs.logs.script(
        ".",
        Ok(vec![slice_entry(
            "m:SLICE-001",
            &parent,
            ("SLICE-001", "PART-001", "authentication"),
            "REQ-001",
            1,
        )]),
    );
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "show", "schedule", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert!(envelope["document"].get("change").is_none(), "{envelope}");
    assert!(envelope["document"].get("scoped").is_none(), "{envelope}");
    assert_eq!(envelope["body"], expected, "{envelope}");
    assert!(provider.target.scopes().is_empty(), "incomplete parent does not ask `scope`");
    provider.vcs.assert_exhausted();
}
