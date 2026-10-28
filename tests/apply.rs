//! Verifies `emery apply` and `emery specify --apply` over scripted storage and version control.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::sync::Arc;

use emery_engine::amend::{SliceChange, SliceClass};
use emery_engine::vcs::{REVISION, RevisionMessage, Summary};
use emery_engine::{EntryKind, REVISION_KEY, STAGED_CONTAINER, STAGED_KEY};
use omnia_sdk::vcs::{Change, ChangeKind, Error};
use serde_json::{Value, json};
use support::provider::{Provider, cli};
use support::scratch::Scratch;
use support::store::{carry, document, revision, seed, stage, stage_amendment, staged, written};
use support::vcs::HEAD;
use support::{cli_ok, fail};

// The one-source reference `specify` names; the scripted loader lands any
// exact package reference.
const SOURCE: &str = "acme:source@1.0.0";
// The target every scenario seals through, into the project checkout.
const PROJECT_CONFIG: &str = "tests/build/project.toml";
// The target adapter reference the created-repository scenarios name.
const BUILDER: &str = "acme:builder@2.1.0";
const TARGET_REPO: &str = "./.emery/vcs/repos/builder";

const SPEC_ANSWER: &str = include_str!("specify/spec-draft.json");
const DESIGN_ANSWER: &str = include_str!("specify/design-draft.json");

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");
const WIDE_PLAN: &[u8] = include_bytes!("build/wide-plan.json");
// `SPEC` with `orders.cancel` held in conflict between `docs` and `wiki`.
const CONFLICTED_SPEC: &[u8] = include_bytes!("amend/conflicted-spec.json");

// A staged candidate becomes the current revision: the repository the
// target names is read for the revision it holds, the three documents are
// written under `spec/` in a working copy cut at its head, sealed as one
// commit under a `Revision` message, and labelled; the pointer names the
// revision, the staged slot is cleared and its documents pruned, and the
// output names the sealed revision alone, since a first seal displaces
// nothing.
#[tokio::test]
async fn apply_seals() {
    let provider = Provider::idle();
    let id = stage(&provider.storage, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!("sealed revision {id} as emery/{id} at Revision\n")
    );
    assert_eq!(current(&provider), id, "the pointer names the sealed revision");
    assert_eq!(
        provider.vcs.calls(),
        [
            "head .".to_owned(),
            format!("add . {REVISION} {HEAD}"),
            format!("remove {REVISION}"),
            format!("add . {REVISION} {HEAD}"),
            format!("pending {REVISION}"),
            format!("commit {REVISION} Revision {id}"),
            format!("label . emery/{id} Revision-commit"),
            format!("remove {REVISION}"),
        ],
        "read, then written, sealed, and labelled in a working copy cut at the head"
    );
    assert_eq!(
        provider.files.writes(),
        [
            (REVISION.to_owned(), document("spec"), SPEC.to_vec()),
            (REVISION.to_owned(), document("design"), DESIGN.to_vec()),
            (REVISION.to_owned(), document("plan"), PLAN.to_vec()),
        ],
        "the three documents, byte for byte"
    );
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(
        message,
        RevisionMessage {
            revision: id.clone(),
            parent: None,
            base: Some(HEAD.to_owned()),
            summary: None,
        }
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "the candidate is pruned");

    // the sealed revision is what show and a second apply see
    cli_ok(&provider, &["emery", "show", "plan", "--config", PROJECT_CONFIG]).await;
    fail(&provider, &["emery", "show", "plan", "--staged"], 2, "nothing-staged").await;
    fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 2, "nothing-staged").await;
}

// Sealing over a current revision displaces it: the new commit lands over
// the one holding the outgoing revision and names it as its parent, the
// output carries the diff against it, and the envelope names the parent,
// the commit, the label, and the kind of candidate sealed.
#[tokio::test]
async fn apply_displaces() {
    let provider = Provider::idle();
    let outgoing = seed(&provider.files, SPEC, DESIGN, PLAN);
    let candidate = stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    assert_ne!(outgoing, candidate);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "apply", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], candidate, "{envelope}");
    assert_eq!(envelope["parent"], outgoing, "{envelope}");
    assert_eq!(envelope["commit"], "Revision-commit", "{envelope}");
    assert_eq!(envelope["label"], format!("emery/{candidate}"), "{envelope}");
    assert_eq!(envelope["repository"], ".", "{envelope}");
    assert_eq!(envelope["kind"], "revision", "{envelope}");
    assert_eq!(envelope["diff"]["from"], outgoing, "{envelope}");
    assert_eq!(
        envelope["diff"]["plan"]["added"],
        serde_json::json!([{"id": "SLICE-003", "name": "orders"}]),
        "the diff is against the revision displaced: {envelope}"
    );
    assert_eq!(current(&provider), candidate);
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(message.parent.as_deref(), Some(outgoing.as_str()), "the parent is the outgoing");
    assert_eq!(message.base.as_deref(), Some(HEAD), "sealed over the commit holding it");
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");

    // the text render carries the same diff on one line
    let provider = Provider::idle();
    seed(&provider.files, SPEC, DESIGN, PLAN);
    stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "sealed revision {candidate} as emery/{candidate} at Revision\n  diff vs {outgoing}: \
             spec +0 -0 ~0, design +0 -0 ~0, plan +1 -0 ~2 preamble\n"
        )
    );
}

// Sealing a candidate identical to the current revision writes the same
// bytes, so the working copy has nothing pending and no commit is made: the
// label is set at the commit already holding the revision, and the pointer
// keeps the id.
#[tokio::test]
async fn apply_same_revision() {
    let provider = Provider::idle();
    let id = seed(&provider.files, SPEC, DESIGN, PLAN);
    assert_eq!(stage(&provider.storage, SPEC, DESIGN, PLAN), id);
    provider.vcs.commits.script(REVISION, Ok(None));

    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "sealed revision {id} as emery/{id} at 9f8e7d6c\n  diff vs {id}: none (byte-stable)\n"
        )
    );
    assert_eq!(current(&provider), id);
    assert!(
        provider.vcs.calls().contains(&format!("label . emery/{id} {HEAD}")),
        "labelled at the commit already holding it: {:?}",
        provider.vcs.calls()
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    provider.vcs.assert_exhausted();
}

// A candidate that amends the current revision, its specification's parent
// naming it, is diffed within one history: requirements pair by id alone, so
// an added one anchoring where a withdrawn one did is an addition and not
// the withdrawn one under another id, and what the candidate leaves out is
// retired with the statement withdrawn rather than removed. The same two
// specifications drafted apart, the parent unset, pair by anchor as before.
#[tokio::test]
async fn apply_amends() {
    let spec = std::str::from_utf8(SPEC).expect("spec.json is UTF-8");
    let base = spec.replace(
        "\"claim\": \"orders.cancel\",\n          \"path\": null",
        "\"claim\": \"orders.cancel\",\n          \"path\": \"docs/orders.md#L20-L30\"",
    );
    let provider = Provider::idle();
    let outgoing = seed(&provider.files, base.as_bytes(), DESIGN, PLAN);

    // the timeout revised, the cancellation withdrawn, a refund added at the
    // cancellation's anchor, under the current revision as parent
    let apart = base
        .replace(
            "Sessions expire after an hour of inactivity.",
            "Sessions expire after 30 minutes of inactivity.",
        )
        .replace("\"id\": \"REQ-004\"", "\"id\": \"REQ-005\"")
        .replace("\"subject\": \"orders.cancel\"", "\"subject\": \"orders.refund\"")
        .replace("A signed-in user cancels an open order.", "A signed-in user requests a refund.");
    let amended = apart.replace("\"parent\": null", &format!("\"parent\": \"{outgoing}\""));
    let candidate = stage(&provider.storage, amended.as_bytes(), DESIGN, PLAN);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "apply", "--config", PROJECT_CONFIG])
            .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], candidate, "{envelope}");
    assert_eq!(envelope["parent"], outgoing, "{envelope}");
    let spec = &envelope["diff"]["spec"];
    assert_eq!(
        spec["added"],
        serde_json::json!([{"id": "REQ-005", "subject": "orders.refund"}]),
        "the refund is added, not the cancellation under another id: {envelope}"
    );
    assert_eq!(spec["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(
        spec["retired"],
        serde_json::json!([{
            "id": "REQ-004",
            "subject": "orders.cancel",
            "statement": "A signed-in user cancels an open order.",
        }]),
        "{envelope}"
    );
    assert_eq!(
        spec["changed"],
        serde_json::json!([{"id": "REQ-002", "subject": "session.timeout", "fields": ["body"]}]),
        "{envelope}"
    );

    // the text render counts the retired beside the rest
    let provider = Provider::idle();
    seed(&provider.files, base.as_bytes(), DESIGN, PLAN);
    stage(&provider.storage, amended.as_bytes(), DESIGN, PLAN);
    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "sealed revision {candidate} as emery/{candidate} at Revision\n  diff vs {outgoing}: \
             spec +1 -0 ~1 retired 1, design +0 -0 ~0, plan +0 -0 ~0\n"
        )
    );

    // the same candidate drafted apart pairs by anchor: the refund is the
    // cancellation under another id, and nothing is retired
    let provider = Provider::idle();
    seed(&provider.files, base.as_bytes(), DESIGN, PLAN);
    stage(&provider.storage, apart.as_bytes(), DESIGN, PLAN);
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "apply", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let spec = &envelope["diff"]["spec"];
    assert_eq!(spec["added"], serde_json::json!([]), "{envelope}");
    assert_eq!(spec["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(spec["retired"], serde_json::json!([]), "{envelope}");
    assert_eq!(
        spec["changed"],
        serde_json::json!([
            {"id": "REQ-002", "subject": "session.timeout", "fields": ["body"]},
            {"id": "REQ-005", "subject": "orders.refund", "was": "REQ-004", "fields": ["id", "subject", "body"]},
        ]),
        "{envelope}"
    );
}

// An amendment prepared against `base` at `commit`, as `emery amend` stages
// one over `SPEC`, `DESIGN`, and `PLAN`: the timeout revised under its id,
// the cancellation retired and leaving its slice, a refund added under the
// next id into the slice of its stem, the rest as the base holds it.
fn amendment_over(base: &str, commit: &str) -> Vec<u8> {
    let mut spec: Value = serde_json::from_slice(SPEC).expect("spec.json is JSON");
    let mut design: Value = serde_json::from_slice(DESIGN).expect("design.json is JSON");
    let mut plan: Value = serde_json::from_slice(PLAN).expect("plan.json is JSON");
    let before = "Sessions expire after an hour of inactivity.";
    let after = "Sessions expire after 30 minutes of inactivity.";
    let refund = "A signed-in user requests a refund of a delivered order.";

    spec["parent"] = json!(base);
    let requirements = spec["requirements"].as_array_mut().expect("requirements");
    requirements[1]["body"] = json!([after]);
    let mut added = requirements.remove(3);
    added["id"] = json!("REQ-005");
    added["subject"] = json!("orders.refund");
    added["body"] = json!([refund]);
    requirements.push(added);
    design["parent"] = json!(base);
    plan["parent"] = json!(base);
    plan["slices"][1]["requirements"] = json!(["REQ-003", "REQ-005"]);

    let change = |id: &str, subject: &str, class: &str, before: Value, after: Value| {
        json!({
            "id": id, "subject": subject, "class": class, "before": before, "after": after,
            "sources": [], "scope": null,
        })
    };
    let amendment = json!({
        "emery": 8,
        "base": {"revision": base, "commit": commit},
        "changes": [
            change("REQ-001", "auth.login", "unchanged",
                json!("Users sign in with a credential."), json!("Users sign in with a credential.")),
            change("REQ-002", "session.timeout", "changed", json!(before), json!(after)),
            change("REQ-003", "orders.create", "unchanged",
                json!("A signed-in user places an order."), json!("A signed-in user places an order.")),
            change("REQ-004", "orders.cancel", "retired",
                json!("A signed-in user cancels an open order."), Value::Null),
            change("REQ-005", "orders.refund", "added", Value::Null, json!(refund)),
        ],
        "spec": spec,
        "design": design,
        "plan": plan,
        "sections": {"preamble": false, "added": [], "removed": [], "changed": []},
        "slices": [{"id": "SLICE-002", "name": "orders", "class": "changed"}],
    });
    serde_json::to_vec_pretty(&amendment).expect("the amendment serialises")
}

// A staged amendment seals as a revision over its base: the repository is
// read for the revision it holds, the amendment's three documents are
// written under `spec/` in a working copy cut at the commit the base was
// read at, sealed as one commit whose message names the base as parent and
// carries the counts and the slices changed, and labelled; the pointer
// names the sealed revision, the slot is cleared and the amendment pruned,
// the base's label stands, and the output names the sealed revision, the
// base it amends, and the diff within one history, the withdrawn
// cancellation retired rather than removed.
#[tokio::test]
async fn apply_amendment() {
    let provider = Provider::idle();
    let outgoing = seed(&provider.files, SPEC, DESIGN, PLAN);
    let amendment = stage_amendment(&provider.storage, &amendment_over(&outgoing, HEAD));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "apply", "--config", PROJECT_CONFIG])
            .await;

    let sealed = revision(
        &written(&provider.files, "spec"),
        &written(&provider.files, "design"),
        &written(&provider.files, "plan"),
    );
    assert_ne!(sealed, outgoing);
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], sealed, "{envelope}");
    assert_eq!(envelope["parent"], outgoing, "{envelope}");
    assert_eq!(envelope["commit"], "Revision-commit", "{envelope}");
    assert_eq!(envelope["label"], format!("emery/{sealed}"), "{envelope}");
    assert_eq!(envelope["kind"], "amendment", "{envelope}");
    assert_eq!(envelope["amendment"], amendment, "{envelope}");
    assert_eq!(envelope["diff"]["from"], outgoing, "{envelope}");
    assert_eq!(
        envelope["diff"]["spec"]["retired"],
        json!([{
            "id": "REQ-004",
            "subject": "orders.cancel",
            "statement": "A signed-in user cancels an open order.",
        }]),
        "one history, so what the amendment leaves out is retired: {envelope}"
    );
    assert_eq!(
        envelope["diff"]["spec"]["added"],
        json!([{"id": "REQ-005", "subject": "orders.refund"}]),
        "{envelope}"
    );
    assert_eq!(current(&provider), sealed, "the pointer names the sealed revision");
    assert_eq!(
        provider.vcs.calls(),
        [
            "head .".to_owned(),
            format!("add . {REVISION} {HEAD}"),
            format!("remove {REVISION}"),
            format!("add . {REVISION} {HEAD}"),
            format!("pending {REVISION}"),
            format!("commit {REVISION} Revision {sealed}"),
            format!("label . emery/{sealed} Revision-commit"),
            format!("remove {REVISION}"),
        ],
        "read, then sealed and labelled in a working copy cut at the base's commit"
    );
    let spec: Value = serde_json::from_slice(&written(&provider.files, "spec")).expect("JSON");
    assert_eq!(spec["parent"], outgoing, "the amendment's documents are what is sealed");
    assert_eq!(spec["requirements"][3]["id"], "REQ-005", "{spec}");
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(
        message,
        RevisionMessage {
            revision: sealed.clone(),
            parent: Some(outgoing.clone()),
            base: Some(HEAD.to_owned()),
            summary: Some(Summary {
                added: 1,
                changed: 1,
                retired: 1,
                slices: vec![SliceChange {
                    id: "SLICE-002".parse().expect("a slice id"),
                    name: "orders".to_owned(),
                    class: SliceClass::Changed,
                }],
            }),
        }
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "the amendment is pruned");
    assert!(
        !provider.vcs.calls().iter().any(|call| call.contains(&format!("emery/{outgoing}"))),
        "the base's label is left standing: {:?}",
        provider.vcs.calls()
    );

    // the sealed revision is what show sees, and nothing is staged now
    fail(&provider, &["emery", "show", "amendment"], 2, "nothing-staged").await;
    fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 2, "nothing-staged").await;

    // the text render names the base the revision amends
    let provider = Provider::idle();
    seed(&provider.files, SPEC, DESIGN, PLAN);
    stage_amendment(&provider.storage, &amendment_over(&outgoing, HEAD));
    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "sealed revision {sealed} as emery/{sealed} at Revision\n  amends {outgoing}\n  diff \
             vs {outgoing}: spec +1 -0 ~1 retired 1, design +0 -0 ~0, plan +0 -0 ~1\n"
        )
    );
}

// An amendment whose change sources disagree over a requirement seals as a
// specify's conflict would: the written specification holds the
// requirement in conflict with the competing statements among its notes,
// the message counts it under neither added nor changed, and nothing
// refuses the seal, since the conflict is the operator's to reconcile in
// the next change.
#[tokio::test]
async fn apply_amendment_conflict() {
    let provider = Provider::idle();
    let outgoing = seed(&provider.files, SPEC, DESIGN, PLAN);
    let mut amendment: Value =
        serde_json::from_slice(&amendment_over(&outgoing, HEAD)).expect("the amendment is JSON");
    let contested: Value = serde_json::from_slice(CONFLICTED_SPEC).expect("the spec is JSON");
    let cancel = &contested["requirements"][3];
    let open = "A signed-in user cancels an open order.";
    let within = "A signed-in user cancels an order within an hour of placing it.";
    amendment["changes"][3] = json!({
        "id": "REQ-004", "subject": "orders.cancel", "class": "conflict",
        "before": open, "after": Value::Null,
        "sources": cancel["sources"], "scope": Value::Null,
    });
    amendment["spec"]["requirements"]
        .as_array_mut()
        .expect("requirements")
        .insert(3, cancel.clone());
    amendment["plan"]["slices"][1]["requirements"] = json!(["REQ-003", "REQ-004", "REQ-005"]);
    let body = serde_json::to_vec_pretty(&amendment).expect("the amendment serialises");
    stage_amendment(&provider.storage, &body);

    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;

    let sealed = revision(
        &written(&provider.files, "spec"),
        &written(&provider.files, "design"),
        &written(&provider.files, "plan"),
    );
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "sealed revision {sealed} as emery/{sealed} at Revision\n  amends {outgoing}\n  diff \
             vs {outgoing}: spec +1 -0 ~2, design +0 -0 ~0, plan +0 -0 ~1\n"
        )
    );
    let spec: Value = serde_json::from_slice(&written(&provider.files, "spec")).expect("JSON");
    assert_eq!(
        spec["requirements"][3], *cancel,
        "the conflict is sealed as the amendment holds it"
    );
    assert_eq!(spec["requirements"][3]["status"], "conflict", "{spec}");
    assert_eq!(spec["requirements"][3]["losers"][0]["statement"], open, "{spec}");
    assert_eq!(spec["requirements"][3]["losers"][1]["statement"], within, "{spec}");
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(
        message.summary,
        Some(Summary {
            added: 1,
            changed: 1,
            retired: 0,
            slices: vec![SliceChange {
                id: "SLICE-002".parse().expect("a slice id"),
                name: "orders".to_owned(),
                class: SliceClass::Changed,
            }],
        }),
        "a conflict is counted under no trailer: {message:?}"
    );
    assert_eq!(current(&provider), sealed, "the pointer names the sealed revision");
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
}

// An amendment prepared against a revision, or a commit, other than the one
// the repository now holds as current is stale: the seal is refused before
// anything is written, naming both, and the amendment stays staged for
// `emery amend` to prepare it again.
#[tokio::test]
async fn apply_amendment_stale() {
    let other = "0000000000000000000000000000000000000000000000000000000000000000";

    // the base revision is not the current one
    let provider = Provider::idle();
    let current = seed(&provider.files, SPEC, DESIGN, PLAN);
    let amendment = stage_amendment(&provider.storage, &amendment_over(other, HEAD));
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "amendment-stale")
            .await;
    let message = envelope["message"].as_str().expect("a message");
    assert!(
        message.contains(&format!("against revision `{other}` at commit `{HEAD}`")),
        "{envelope}"
    );
    assert!(
        message.contains(&format!("holds revision `{current}` at commit `{HEAD}`")),
        "{envelope}"
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery amend")),
        "{envelope}"
    );
    assert_eq!(
        provider.vcs.calls(),
        ["head .".to_owned(), format!("add . {REVISION} {HEAD}"), format!("remove {REVISION}")],
        "read, and nothing sealed"
    );
    assert!(provider.files.writes().is_empty(), "nothing is written");
    assert!(provider.storage.state(REVISION_KEY).is_none(), "the pointer does not move");
    assert!(
        provider.storage.object(STAGED_CONTAINER, &format!("{amendment}/amendment.json")).is_some(),
        "the amendment stays staged"
    );

    // the base commit is not the one the revision now stands at
    let provider = Provider::idle();
    let current = seed(&provider.files, SPEC, DESIGN, PLAN);
    stage_amendment(&provider.storage, &amendment_over(&current, "1234567890abcdef"));
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "amendment-stale")
            .await;
    let message = envelope["message"].as_str().expect("a message");
    assert!(message.contains("at commit `1234567890abcdef`"), "{envelope}");
    assert!(
        message.contains(&format!("holds revision `{current}` at commit `{HEAD}`")),
        "{envelope}"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");

    // no revision stands where the base was read
    let provider = Provider::idle();
    stage_amendment(&provider.storage, &amendment_over(other, HEAD));
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "amendment-stale")
            .await;
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.ends_with("holds no revision")),
        "{envelope}"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
}

// A target naming no repository seals into one the engine creates under the
// target's name: created now, the revision its root commit, the envelope
// naming the repository. A later seal finds it and lands over its head.
#[tokio::test]
async fn apply_creates_repository() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!("[target]\nadapter = \"{BUILDER}\"\n"));
    let provider = Provider::idle();
    let id = stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.vcs.heads.script(TARGET_REPO, Err(Error::NotARepository));

    let resp = cli_ok(&provider, &["emery", "apply", "--config", &config]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!("sealed revision {id} as emery/{id} at Revision\n  repository {TARGET_REPO}\n")
    );
    assert_eq!(
        provider.vcs.calls(),
        [
            format!("head {TARGET_REPO}"),
            format!("init {TARGET_REPO}"),
            format!("pending {TARGET_REPO}"),
            format!("commit {TARGET_REPO} Revision {id}"),
            format!("label {TARGET_REPO} emery/{id} Revision-commit"),
        ],
        "created, then the revision sealed as its root commit"
    );
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(message.base, None, "a root commit has no base");
    assert_eq!(
        provider
            .files
            .writes()
            .iter()
            .map(|(at, path, _)| (at.as_str(), path.as_str()))
            .collect::<Vec<_>>(),
        [
            (TARGET_REPO, document("spec").as_str()),
            (TARGET_REPO, document("design").as_str()),
            (TARGET_REPO, document("plan").as_str()),
        ],
        "written into the repository's own checkout"
    );
    assert_eq!(current(&provider), id);

    // the repository is kept: the next seal lands over its head
    let candidate = stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    provider.vcs.heads.script(TARGET_REPO, Ok("root-commit".to_owned()));
    let before = provider.vcs.calls().len();
    cli_ok(&provider, &["emery", "apply", "--config", &config]).await;
    let calls = provider.vcs.calls();
    assert_eq!(
        calls[before..before + 5],
        [
            format!("head {TARGET_REPO}"),
            format!("labelled {TARGET_REPO} emery/{id}"),
            format!("add {TARGET_REPO} {REVISION} root-commit"),
            format!("remove {REVISION}"),
            format!("add {TARGET_REPO} {REVISION} root-commit"),
        ],
        "read, then sealed over the head: {calls:?}"
    );
    assert!(!calls[before..].iter().any(|call| call.starts_with("init ")), "created once");
    assert_eq!(current(&provider), candidate);
    provider.vcs.assert_exhausted();
}

// The seal is refused, and nothing sealed, where the specification cannot
// live in the repository: a `spec/` of the operator's own at the head, a
// project checkout that is no repository or has no commit, and a credential
// among what the seal would commit.
#[tokio::test]
async fn apply_refused() {
    // the operator's own `spec/`
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.files.lists(REVISION, ".", &[("spec", EntryKind::Directory)]);
    provider.files.lists(REVISION, "spec", &[("openapi.yaml", EntryKind::File)]);
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "spec-mismatch").await;
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.contains(
            "`spec/` at commit `9f8e7d6c5b4a39281706f5e4d3c2b1a0f9e8d7c6` holds `openapi.yaml`"
        )),
        "{envelope}"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    assert!(provider.files.writes().is_empty(), "nothing is written");

    // a project directory that is no repository
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.vcs.heads.script(".", Err(Error::NotARepository));
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "repository-required")
            .await;
    assert_eq!(envelope["message"], "the project directory is not a repository", "{envelope}");

    // a project repository with no commit
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.vcs.heads.script(".", Err(Error::NotFound("HEAD".to_owned())));
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "base-not-sealed")
            .await;
    assert_eq!(
        envelope["message"], "the project repository has no commit to hold the specification",
        "{envelope}"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");

    // a credential the seal would commit
    let provider = Provider::idle();
    stage(&provider.storage, SPEC, DESIGN, PLAN);
    provider.vcs.pending.script(
        REVISION,
        Ok(vec![Change {
            path: ".npmrc".to_owned(),
            kind: ChangeKind::Added,
        }]),
    );
    let envelope =
        fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 1, "secret-detected")
            .await;
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.starts_with(
            "what the seal wrote carries a credential, so none of it is sealed:\n- .npmrc"
        )),
        "{envelope}"
    );
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
    assert_eq!(provider.vcs.calls().last(), Some(&format!("remove {REVISION}")), "the copy goes");
    provider.vcs.assert_exhausted();
}

// The verbs that read or seal the current revision need the repository it
// lives in: `apply`, `show`, and `specify --apply` are refused typed where
// no `[target]` names one, before anything is read, while `show --staged`
// and a bare `specify` need none.
#[tokio::test]
async fn apply_target_required() {
    let scratch = Scratch::new();
    let config = scratch.config("[[source]]\nname = \"source\"\nadapter = \"acme:source@1.0.0\"\n");
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    stage(&provider.storage, SPEC, DESIGN, PLAN);

    for argv in [
        vec!["emery", "apply", "--config", config.as_str()],
        vec!["emery", "apply"],
        vec!["emery", "show", "spec", "--config", config.as_str()],
        vec!["emery", "show", "spec"],
        vec!["emery", "specify", "--apply", "--config", config.as_str()],
        vec!["emery", "specify", SOURCE, "--apply"],
    ] {
        let envelope = fail(&provider, &argv, 1, "build-target-required").await;
        assert!(
            envelope["message"]
                .as_str()
                .is_some_and(|message| message.contains("has no repository to live in")),
            "{argv:?}: {envelope}"
        );
        assert!(
            envelope["hint"].as_str().is_some_and(|hint| hint.contains("[target]")),
            "{argv:?}: {envelope}"
        );
    }
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
    assert!(provider.source.metadata.lock().expect("metadata").is_empty(), "nothing loads");

    cli_ok(&provider, &["emery", "show", "spec", "--staged"]).await;
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    assert!(provider.vcs.calls().is_empty(), "a run with no target reads no repository");
    provider.model.assert_exhausted();
}

// `specify --apply` stages the candidate and seals it in one run: the
// output carries both lines, the envelope says the candidate is no longer
// staged and names what was sealed, and the slot and the repository stand
// as after `apply`.
#[tokio::test]
async fn specify_apply() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    let resp =
        cli_ok(&provider, &["emery", "specify", SOURCE, "--apply", "--config", PROJECT_CONFIG])
            .await;

    let id = current(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged candidate {id}\n  plan: 1 slice in 1 wave, widest 1\n  sealed revision {id} \
             as emery/{id} at Revision\n"
        )
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "the candidate is pruned");
    assert_eq!(provider.vcs.messages().len(), 1, "one revision commit");
    assert!(provider.vcs.calls().contains(&format!("label . emery/{id} Revision-commit")));
    provider.model.assert_exhausted();

    // the envelope under a re-run that seals the same bytes over the head
    // now holding them
    let sealed = Provider::over(Arc::clone(&provider.storage), [SPEC_ANSWER, DESIGN_ANSWER]);
    assert_eq!(carry(&provider.files, &sealed.files), id, "the written bytes are the id");
    let provider = sealed;
    provider.vcs.commits.script(REVISION, Ok(None));
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "specify", SOURCE, "--apply", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["staged"], false, "{envelope}");
    assert_eq!(envelope["applied"]["revision"], id, "{envelope}");
    assert_eq!(envelope["applied"]["parent"], id, "{envelope}");
    assert_eq!(envelope["applied"]["commit"], HEAD, "nothing to seal: {envelope}");
    assert_eq!(envelope["applied"]["kind"], "revision", "{envelope}");
    assert_eq!(envelope["diff"]["from"], id, "the diff is against the revision displaced");
    assert_eq!(current(&provider), id);
    provider.model.assert_exhausted();
}

// The stage lands before the seal is attempted: a seal the repository
// refuses fails the run after the candidate is staged, so `emery apply` can
// seal it once the repository answers, and nothing of the current revision
// moves.
#[tokio::test]
async fn specify_apply_seal_refused() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    let outgoing = seed(&provider.files, SPEC, DESIGN, PLAN);
    provider.vcs.commits.script(REVISION, Err(Error::Access("index.lock held".to_owned())));

    let resp = cli(
        &provider,
        &["emery", "--format", "json", "specify", SOURCE, "--apply", "--config", PROJECT_CONFIG],
    )
    .await;

    assert_eq!(resp.exit, 4, "{}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], "bad_gateway", "{envelope}");
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.contains("index.lock held")),
        "{envelope}"
    );
    let candidate = staged(&provider.storage);
    assert_ne!(candidate, outgoing);
    assert!(
        provider.storage.object(STAGED_CONTAINER, &format!("{candidate}/spec.json")).is_some(),
        "the candidate stands staged"
    );
    assert!(provider.storage.state(REVISION_KEY).is_none(), "the pointer does not move");
    assert!(
        !provider.vcs.calls().iter().any(|call| call.starts_with("label ")),
        "nothing is labelled"
    );
    provider.model.assert_exhausted();

    // once the repository answers, apply seals what the run left staged
    let provider = Provider::over(Arc::clone(&provider.storage), Vec::<String>::new());
    seed(&provider.files, SPEC, DESIGN, PLAN);
    cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
    assert_eq!(current(&provider), candidate);
    provider.vcs.assert_exhausted();
}

// `--apply` with no source is refused as a bare `specify` is, before any
// load, and nothing is staged or sealed.
#[tokio::test]
async fn specify_apply_no_source() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::env::set_current_dir(dir.path()).expect("enter empty project");
    let provider = Provider::idle();

    fail(&provider, &["emery", "specify", "--apply"], 1, "specify-source-required").await;
    assert!(provider.source.metadata.lock().expect("metadata").is_empty(), "nothing loads");
    assert!(provider.storage.state(STAGED_KEY).is_none());
    assert!(provider.storage.state(REVISION_KEY).is_none());
    assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
}

fn current(provider: &Provider) -> String {
    String::from_utf8(provider.storage.state(REVISION_KEY).expect("current"))
        .expect("utf-8 revision id")
}
