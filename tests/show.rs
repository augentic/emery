//! Verifies `emery show amendment`, what a staged amendment refuses, and a hand-merged `spec/`, over scripted storage.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use emery_engine::{STAGED_CONTAINER, STAGED_KEY};
use serde_json::Value;
use support::provider::Provider;
use support::store::{seed, stage, stage_amendment, stage_fresh};
use support::vcs::HEAD;
use support::{cli_ok, fail};

// The one-source reference `specify` names; the scripted loader lands any
// exact package reference.
const SOURCE: &str = "acme:source@1.0.0";
// The target every revision verb reads through, into the project checkout.
const PROJECT_CONFIG: &str = "tests/build/project.toml";

const AMENDMENT: &[u8] = include_bytes!("specify/6-amendment.json");
const AMENDMENT_RENDERED: &str = include_str!("specify/6-amendment.md");

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");

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
    plan["slices"][1]["requirements"] = serde_json::json!(["REQ-003", "REQ-005"]);
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
             (`REQ-005` is placed by `SLICE-002` and is no requirement of the specification; \
             `REQ-004` is placed by no slice), so they were edited or merged by hand; a seal \
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
