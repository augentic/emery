//! Verifies `emery amend` over scripted sources, storage, and version control.

#![cfg(not(target_arch = "wasm32"))]

mod support;
#[path = "support/verbs.rs"]
mod verbs;

use emery_engine::vcs::REVISION;
use emery_engine::{STAGED_CONTAINER, STAGED_KEY};
use serde_json::Value;
use support::{
    HEAD, Provider, cli, cli_ok, evidence, fail, requirement, seed, stage, stage_amendment,
};
use verbs::verbs;

// The target every scenario reads the current revision through: the project checkout.
const PROJECT_CONFIG: &str = "tests/build/project.toml";
// The change sources, documentation adapters the scripted loader lands at any version.
const DOCS: &str = "acme:docs@1.0.0";
const WIKI: &str = "acme:wiki@1.0.0";

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");
const CLASSES: &str = include_str!("amend/classes.json");
const AMENDMENT: &[u8] = include_bytes!("specify/6-amendment.json");

// The grouping over the four requirements of `SPEC` and the five change
// claims `bind` scripts: indices 0..=3 are `docs`, 4 is `wiki`, and 5..=8 the
// standing requirements in id order.
const CLASSES_GROUPING: &str = r#"{"groups": [
    {"claims": [0, 4, 5], "classes": [[0], [4], [5]]},
    {"claims": [1, 6], "classes": [[1], [6]]},
    {"claims": [2, 7], "classes": [[2, 7]]},
    {"claims": [3], "classes": [[3]]},
    {"claims": [8], "classes": [[8]]}
]}"#;

// The vcs calls a read of the current revision makes, and the only ones an amend makes.
fn read_calls() -> [String; 3] {
    ["head .".to_owned(), format!("add . {REVISION} {HEAD}"), format!("remove {REVISION}")]
}

// Two documentation sources: `docs` revises the timeout, restates the order,
// adds a refund, and disagrees with `wiki` over the login.
fn bind(provider: &mut Provider) {
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("auth.login", "Users sign in with a magic link."),
            requirement("session.timeout", "Sessions expire after 30 minutes of inactivity."),
            requirement("orders.create", "A signed-in user places an order."),
            requirement(
                "orders.refund",
                "A signed-in user requests a refund of a delivered order.",
            ),
        ])),
    );
    provider.source.evidence.insert(
        "wiki".to_owned(),
        Ok(evidence(vec![requirement("auth.login", "Users sign in with a passkey.")])),
    );
}

fn staged_amendment(provider: &Provider) -> (String, Value) {
    let raw = provider.storage.state(STAGED_KEY).expect("staged record");
    let record: Value = serde_json::from_slice(&raw).expect("staged record JSON");
    assert_eq!(record["kind"], "amendment", "{record}");
    let id = record["id"].as_str().expect("staged record id").to_owned();
    let body = provider
        .storage
        .object(STAGED_CONTAINER, &format!("{id}/amendment.json"))
        .expect("the staged amendment document");
    (id, serde_json::from_slice(&body).expect("the amendment is JSON"))
}

// One change over a sealed revision: the standing requirements are put to the
// grouping after the claims, each requirement lands in one class decided by
// code, a revised or restated requirement keeps its id, an added one takes
// the next free id, and the amendment is staged for `show amendment` with
// the repository read and never written.
#[tokio::test]
async fn amend_classes() {
    let mut provider = Provider::answering([CLASSES_GROUPING, CLASSES_GROUPING]);
    bind(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  1 added, 1 changed, 0 retired, 1 \
             conflict, 2 unchanged\n  conflicts: auth.login\n  stems: auth, orders, session\n  \
             nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    let expected: Value = serde_json::from_str(&CLASSES.replace("<revision>", &revision))
        .expect("the fixture is JSON");
    assert_eq!(amendment, expected);

    // the grouping lists the standing requirements after the claims, indexed on
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(request.contains("- 4 `wiki` `auth.login`"), "{request}");
    assert!(request.contains("## Standing requirements"), "{request}");
    assert!(request.contains("- 5 `auth.login` — Users sign in with a credential."), "{request}");
    assert!(request.contains("- 8 `orders.cancel`"), "{request}");
    assert!(
        request.contains("claims 0, 4, 5 share the id `auth.login`"),
        "a claim at a standing id is pre-merged with it: {request}"
    );

    // the repository is read for the revision and nothing more
    assert_eq!(provider.vcs.calls(), read_calls());
    assert!(provider.files.writes().is_empty(), "an amend writes no document");

    // the envelope carries the amendment, its base, and the counts
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["amendment"], id);
    assert_eq!(envelope["base"]["revision"], revision);
    assert_eq!(envelope["base"]["commit"], HEAD);
    assert_eq!(
        envelope["tally"],
        serde_json::json!({"added": 1, "changed": 1, "retired": 0, "conflict": 1, "unchanged": 2})
    );
    assert_eq!(envelope["conflicts"], serde_json::json!(["auth.login"]));
    assert_eq!(envelope["stems"], serde_json::json!(["auth", "orders", "session"]));

    // what was staged is what show renders
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(shown.contains("### Requirement: session.timeout\n\nID: REQ-002"), "{shown}");
    assert!(shown.contains("After: Sessions expire after 30 minutes of inactivity."), "{shown}");
    assert!(shown.contains("### Requirement: orders.refund\n\nID: REQ-005"), "{shown}");
}

// A grouping that merges two standing requirements is refused as a
// correction: the specification already holds them apart.
#[tokio::test]
async fn amend_merged_standing() {
    // the order claims merged with both standing order requirements
    let merged = r#"{"groups": [
        {"claims": [0, 4, 5], "classes": [[0], [4], [5]]},
        {"claims": [1, 6], "classes": [[1], [6]]},
        {"claims": [2, 3, 7, 8], "classes": [[2, 7], [3], [8]]}
    ]}"#;
    let mut provider = Provider::answering([merged, merged, merged]);
    bind(&mut provider);
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let envelope = fail(
        &provider,
        &["emery", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG],
        1,
        "bad_request",
    )
    .await;
    assert!(
        envelope["message"].as_str().is_some_and(|text| {
            text.contains("merges the standing requirements `orders.cancel`, `orders.create`")
        }),
        "{envelope}"
    );
    provider.model.assert_exhausted();
}

// A change that restates every requirement it touches leaves the
// specification as it stands: the grouping is still the model's, nothing is
// staged, and the output says so.
#[tokio::test]
async fn amend_no_change() {
    let grouping = r#"{"groups": [
        {"claims": [0, 3], "classes": [[0, 3]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let mut provider = Provider::answering([grouping, grouping]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![requirement("orders.create", "A signed-in user places an order.")])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "nothing staged: no change against the current revision\n  base {revision} at {}\n  \
             0 added, 0 changed, 0 retired, 0 conflict, 4 unchanged\n",
            &HEAD[..8]
        )
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "no document is stored");
    assert_eq!(provider.vcs.calls(), read_calls());

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "amend", DOCS, "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["amendment"], Value::Null, "{envelope}");
    assert_eq!(envelope["stems"], Value::Null, "an empty list is elided: {envelope}");
    assert_eq!(envelope["tally"]["unchanged"], 4, "{envelope}");
    provider.model.assert_exhausted();
}

// An amend replaces an amendment staged before it, pruning the displaced
// document, and never a revision candidate: over one it is
// `amendment-pending` before any source is extracted, the candidate staying
// where it is.
#[tokio::test]
async fn amend_over_staged() {
    let mut provider = Provider::answering([CLASSES_GROUPING]);
    bind(&mut provider);
    seed(&provider.files, SPEC, DESIGN, PLAN);
    let displaced = stage_amendment(&provider.storage, AMENDMENT);

    cli_ok(&provider, &["emery", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG]).await;

    let (id, _) = staged_amendment(&provider);
    assert_ne!(id, displaced);
    assert!(
        provider.storage.object(STAGED_CONTAINER, &format!("{displaced}/amendment.json")).is_none(),
        "the displaced amendment is pruned"
    );

    let mut provider = Provider::idle();
    bind(&mut provider);
    seed(&provider.files, SPEC, DESIGN, PLAN);
    let candidate = stage(&provider.storage, SPEC, DESIGN, PLAN);

    let envelope = fail(
        &provider,
        &["emery", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG],
        1,
        "amendment-pending",
    )
    .await;
    assert!(
        envelope["message"].as_str().is_some_and(|text| text.contains(&candidate)),
        "{envelope}"
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery apply")),
        "{envelope}"
    );
    assert!(provider.source.calls().is_empty(), "no source is extracted over a staged candidate");
    assert!(
        provider.source.metadata.lock().expect("metadata").is_empty(),
        "no adapter is loaded over a staged candidate"
    );
}

// `baseline` names the current revision's own requirements in the grouping,
// so a source of that name is refused before any load.
#[tokio::test]
async fn amend_baseline_refused() {
    let provider = Provider::idle();
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let envelope = fail(
        &provider,
        &[
            "emery",
            "amend",
            "--description",
            "acme:baseline@1.0.0=Sessions never expire.",
            "--config",
            PROJECT_CONFIG,
        ],
        1,
        "bad_request",
    )
    .await;
    assert!(
        envelope["message"].as_str().is_some_and(|text| text.contains("`baseline`")),
        "{envelope}"
    );
    assert!(provider.vcs.calls().is_empty(), "refused before the repository is read");
    assert!(provider.source.metadata.lock().expect("metadata").is_empty(), "and before any load");
}

// An amendment is prepared against a sealed revision: with none it is
// `spec-not-generated`, hinted to seal one, and a staged candidate is named
// as what to seal.
#[tokio::test]
async fn amend_no_revision() {
    let provider = Provider::idle();

    let envelope = fail(
        &provider,
        &["emery", "amend", DOCS, "--config", PROJECT_CONFIG],
        2,
        "spec-not-generated",
    )
    .await;
    assert!(
        envelope["hint"]
            .as_str()
            .is_some_and(|hint| { hint.contains("emery specify") && hint.contains("emery amend") }),
        "{envelope}"
    );
    assert!(provider.source.calls().is_empty(), "no source is extracted with no revision");

    let candidate = stage(&provider.storage, SPEC, DESIGN, PLAN);
    let envelope = fail(
        &provider,
        &["emery", "amend", DOCS, "--config", PROJECT_CONFIG],
        2,
        "spec-not-generated",
    )
    .await;
    assert!(
        envelope["message"].as_str().is_some_and(|text| text.contains(&candidate)),
        "{envelope}"
    );
    assert!(
        envelope["hint"]
            .as_str()
            .is_some_and(|hint| { hint.contains("emery apply") && hint.contains("emery amend") }),
        "{envelope}"
    );
}

// An amend reads the current revision from the target's repository, so a run
// naming no `[target]` is refused typed before anything is read, and one
// naming no source is refused as a specify is.
#[tokio::test]
async fn amend_target_required() {
    let provider = Provider::idle();
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let envelope = fail(&provider, &["emery", "amend", DOCS], 1, "build-target-required").await;
    assert!(envelope["hint"].as_str().is_some_and(|hint| hint.contains("[target]")), "{envelope}");
    assert!(provider.vcs.calls().is_empty(), "nothing is read without a target");

    let envelope = fail(
        &provider,
        &["emery", "amend", "--config", PROJECT_CONFIG],
        1,
        "specify-source-required",
    )
    .await;
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery amend")),
        "{envelope}"
    );
}

// The verb is hidden while nothing seals what it stages: it runs and has
// help of its own, and the top-level help and the completions leave it out.
#[tokio::test]
async fn amend_hidden() {
    let provider = Provider::idle();

    let help = cli_ok(&provider, &["emery", "amend", "--help"]).await;
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("--description"), "{help}");
    assert!(help.contains("--config"), "{help}");

    let top = cli(&provider, &["emery", "--help"]).await;
    let top = String::from_utf8_lossy(&top.stdout);
    assert!(!verbs(&top).contains(&"amend"), "{top}");

    let completions = cli_ok(&provider, &["emery", "completions", "zsh"]).await;
    let completions = String::from_utf8_lossy(&completions.stdout);
    assert!(completions.contains("'apply:"), "the shown verbs are completed: {completions}");
    assert!(!completions.contains("'amend:"), "{completions}");
    assert!(!completions.contains("(amend)"), "{completions}");
    assert!(completions.contains("amendment"), "the artifact the verb stages is: {completions}");
}
