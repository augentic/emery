//! Verifies `emery show amendment` and what a staged amendment refuses, over scripted storage.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use emery_engine::{STAGED_CONTAINER, STAGED_KEY};
use serde_json::Value;
use support::provider::Provider;
use support::store::{stage, stage_amendment};
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
// adapter loads, `apply` does not seal it, and a revision document asked of
// the slot names it, each `amendment-pending` naming the amendment and
// leaving it staged.
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
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "amendment-pending")
            .await;
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery show amendment")),
        "{envelope}"
    );

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
