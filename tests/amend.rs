//! Verifies `emery amend` over scripted sources, storage, and version control.

#![cfg(not(target_arch = "wasm32"))]

mod support;
#[path = "support/verbs.rs"]
mod verbs;

use std::collections::BTreeMap;
use std::sync::Arc;

use emery_adapter::source::{Claim, ClaimKind, SourceKind};
use emery_adapter::{Route, Trigger};
use emery_engine::amend::PartClass;
use emery_engine::vcs::{PartSummary, REVISION, RevisionMessage, Summary};
use emery_engine::{REVISION_KEY, SNAPSHOTS_CONTAINER, STAGED_CONTAINER, STAGED_KEY};
use omnia_sdk::vcs::Error;
use omnia_test::SeenFormat;
use serde_json::Value;
use support::provider::{Provider, claim, cli, evidence, requirement, retire};
use support::scratch::Scratch;
use support::store::{seed, seed_snapshot, sha256, snapshot, stage, stage_amendment, written};
use support::vcs::HEAD;
use support::{cli_ok, fail};
use verbs::verbs;

// The target every scenario reads the current revision through: the project checkout.
const PROJECT_CONFIG: &str = "tests/build/project.toml";
// The change sources, documentation adapters the scripted loader lands at any version.
const DOCS: &str = "acme:docs@1.0.0";
const WIKI: &str = "acme:wiki@1.0.0";
// A change source read as code: the scenario scripts its kind as behaviour.
const CODE: &str = "acme:code@1.0.0";

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("specify/5-sliced.json");
// `SPEC` with `orders.cancel` held in conflict between `docs` and `wiki`.
const CONFLICTED_SPEC: &[u8] = include_bytes!("amend/conflicted-spec.json");
const CHAIN_PLAN: &[u8] = include_bytes!("amend/chain-plan.json");
const CLASSES: &str = include_str!("amend/classes.json");
const RETIRED: &str = include_str!("amend/retired.json");
const SCOPED: &str = include_str!("amend/scoped.json");
const DESIGNED: &str = include_str!("amend/designed.json");
const AMENDMENT: &[u8] = include_bytes!("specify/6-amendment.json");

const WITHDRAWN: &str = "Cancelling an open order is withdrawn.";
// The one paragraph of `DESIGN`'s overview.
const OVERVIEW: &str = "Sign-in issues a session; orders are placed and cancelled under it.";
// `DESIGN`'s domain model, the two types `PLAN`'s `orders` part owns, for a
// base design a scenario spells with sections of its own. Every inline base
// design must carry it: `Pruned::own` drops from a part the type keys the
// design lacks, so a base design without these two blocks makes the amended
// plan show `orders` as changed with `types: []` and nothing else different.
// A sealed revision is never in that state, since the slicing gate refuses a
// key the design lacks, so the rule is the fixture's, not the product's.
const DOMAIN_MODEL: &str = r#"{"kind": "domain-model", "blocks": [
    {"type": {"key": "orders.order", "signature": "interface Order { id: string }", "role": "data"}},
    {"type": {"key": "orders.line", "signature": "interface Line { sku: string }", "role": "data"}}
]}"#;
// The directive a documentation change prescribes a technology with.
const VAULT: &str = "Secrets live in the team vault.";

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

// The spec draft over the three requirements `CLASSES_GROUPING` drafts anew:
// the contested login, the revised timeout, and the added refund.
const CLASSES_DRAFT: &str = r#"{"preamble": [], "requirements": [
    {"subject": "auth.login", "scenarios": [
        {"name": "Login", "when": "a caller signs in", "then": "[unknown]"}
    ]},
    {"subject": "session.timeout", "scenarios": [
        {"name": "Timeout", "when": "a session is idle for 30 minutes", "then": "it times out"}
    ]},
    {"subject": "orders.refund", "scenarios": [
        {"name": "Refund", "when": "a signed-in caller requests a refund of a delivered order", "then": "the refund is recorded"}
    ]}
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
// the next free id, the amended specification is drafted over the added,
// revised, and contested requirements alone under those ids with the rest
// the base's as it stands, the design is the base's under it as parent
// since nothing is retired, the added requirement joins the part of its
// stem by code with no slicing turn, and the amendment is staged for
// `show amendment` with the repository read and never written.
#[tokio::test]
async fn amend_classes() {
    let mut provider =
        Provider::answering([CLASSES_GROUPING, CLASSES_DRAFT, CLASSES_GROUPING, CLASSES_DRAFT]);
    bind(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  1 added, 1 changed, 0 retired, 1 \
             conflict, 2 unchanged\n  conflicts: auth.login\n  stems: auth, orders, session\n  \
             parts: orders (brief kept)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    let expected: Value = serde_json::from_str(&CLASSES.replace("<revision>", &revision))
        .expect("the fixture is JSON");
    assert_eq!(amendment, expected);

    // the grouping lists the standing requirements after the claims, indexed
    // on; a change of requirement claims alone spends no design turn
    assert_eq!(provider.model.seen().len(), 2, "the grouping and the draft are the turns");
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(request.contains("- 4 `wiki` `auth.login`"), "{request}");
    assert!(request.contains("## Standing requirements"), "{request}");
    assert!(request.contains("- 5 `auth.login` — Users sign in with a credential."), "{request}");
    assert!(request.contains("- 8 `orders.cancel`"), "{request}");
    assert!(
        request.contains("claims 0, 4, 5 share the id `auth.login`"),
        "a claim at a standing id is pre-merged with it: {request}"
    );

    // the draft lists the three requirements drafted anew under their ids,
    // the change sources alone behind each, and none that stands
    let draft = provider.model.seen()[1].messages.join("\n");
    assert!(draft.contains("amend the current specification"), "{draft}");
    assert!(
        draft.contains("- REQ-001 `auth.login` — Status: conflict — Sources: [docs:auth.login, wiki:auth.login]"),
        "{draft}"
    );
    assert!(
        draft.contains(
            "- REQ-002 `session.timeout` — Status: unknown — Sources: [docs:session.timeout]"
        ),
        "{draft}"
    );
    assert!(draft.contains("- REQ-005 `orders.refund`"), "{draft}");
    assert!(!draft.contains("- REQ-003"), "a standing requirement is not drafted: {draft}");
    assert!(!draft.contains("- REQ-004"), "a standing requirement is not drafted: {draft}");
    assert!(!draft.contains("baseline"), "the base is no source of the draft: {draft}");

    // the unchanged requirements are the base's byte for byte, the retired
    // none, and the preamble the base's under the base as parent
    let base: Value = serde_json::from_slice(SPEC).expect("the base is JSON");
    let spec = &amendment["spec"];
    assert_eq!(spec["parent"], revision);
    assert_eq!(spec["preamble"], base["preamble"]);
    assert_eq!(spec["requirements"][2], base["requirements"][2]);
    assert_eq!(spec["requirements"][3], base["requirements"][3]);
    assert_eq!(spec["requirements"].as_array().map(Vec::len), Some(5));

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
    let tally =
        r#"{"added": 1, "changed": 1, "retired": 0, "conflict": 1, "unchanged": 2, "by-scope": 0}"#;
    assert_eq!(envelope["tally"], serde_json::from_str::<Value>(tally).expect("JSON"));
    assert_eq!(envelope["conflicts"], serde_json::json!(["auth.login"]));
    assert_eq!(envelope["stems"], serde_json::json!(["auth", "orders", "session"]));
    assert_eq!(envelope["sections"], Value::Null, "an unchanged design is elided: {envelope}");
    let parts = r#"[{"id": "PART-002", "name": "orders", "class": "changed", "brief": "kept"}]"#;
    assert_eq!(envelope["parts"], serde_json::from_str::<Value>(parts).expect("JSON"));

    // what was staged is what show renders: a changed requirement's old and
    // new statement and its redrafted scenarios, an unchanged one's
    // statement alone, no design section, and the part the added
    // requirement joined with its types and brief as they stood, the brief
    // noted as the base's since no slicing turn was put
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(!shown.contains("## Design"), "{shown}");
    assert!(
        shown.ends_with(
            "## Plan\n\n### Part: orders [changed]\n\nID: PART-002\nRequirements: [REQ-003, \
             REQ-004, REQ-005]\nTypes: [orders.line, orders.order]\n\nDelivers order creation \
             and cancellation for a signed-in caller; verified by placing and cancelling an \
             order under a session from `authentication`.\n\nNote: the brief is the base's; no \
             slicing turn redrafted it.\n"
        ),
        "{shown}"
    );
    assert!(
        shown.contains(
            "### Requirement: session.timeout\n\nID: REQ-002\nSources: [docs:session.timeout]\n\n\
             Before: Sessions expire after an hour of inactivity.\n\nAfter: Sessions expire \
             after 30 minutes of inactivity.\n\n#### Scenario: Timeout\n\n- **WHEN** a session \
             is idle for 30 minutes\n- **THEN** it times out\n"
        ),
        "{shown}"
    );
    assert!(shown.contains("### Requirement: orders.refund\n\nID: REQ-005"), "{shown}");
    assert!(shown.contains("#### Scenario: Refund"), "{shown}");
    assert!(!shown.contains("#### Scenario: Create"), "a standing scenario is not shown: {shown}");
    provider.model.assert_exhausted();
}

// A `retire` beside a source's requirements retires the standing requirement
// stating the behaviour it withdraws: matched by id, where the grouping
// pre-merges the two, and else by the model, which places it with the
// requirement under the id the specification spells; either way the
// requirement keeps its id and subject, the withdrawal is its one source,
// `show amendment` lists it as retired with the statement withdrawn, and the
// amended specification leaves it out with no draft turn spent, since
// nothing is drafted anew. The requirement leaves its part, which shrinks
// and keeps the types it owns, so the design stands.
#[tokio::test]
async fn amend_retired() {
    // indices 0 and 1 are `docs`, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1, 5], "classes": [[1], [5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let mut provider = Provider::answering([grouping]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("orders.create", "A signed-in user places an order."),
            retire("orders.cancel", WITHDRAWN),
        ])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 1 retired, 0 \
             conflict, 3 unchanged\n  stems: orders\n  parts: orders (brief kept)\n  nothing \
             sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    let expected: Value = serde_json::from_str(&RETIRED.replace("<revision>", &revision))
        .expect("the fixture is JSON");
    assert_eq!(amendment, expected);
    assert_eq!(provider.model.seen().len(), 1, "the grouping is the one turn");

    // the retire is listed among the claims, marked, and pre-merged at its id
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(
        request.contains(&format!("- 1 `docs` `orders.cancel` (retire) — {WITHDRAWN}")),
        "{request}"
    );
    assert!(request.contains("claims 1, 5 share the id `orders.cancel`"), "{request}");
    assert!(request.contains("A claim marked `(retire)`"), "{request}");
    assert_eq!(provider.vcs.calls(), read_calls());

    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(shown.contains("0 added, 0 changed, 1 retired, 0 conflict, 3 unchanged."), "{shown}");
    assert!(
        shown.contains(
            "## Retired\n\n### Requirement: orders.cancel\n\nID: REQ-004\nSources: \
             [docs:orders.cancel]\n\nA signed-in user cancels an open order."
        ),
        "{shown}"
    );
    assert!(!shown.contains("## Design"), "the design stands: {shown}");
    assert!(
        shown.ends_with(
            "## Plan\n\n### Part: orders [changed]\n\nID: PART-002\nRequirements: \
             [REQ-003]\nTypes: [orders.line, orders.order]\n\nDelivers order creation and \
             cancellation for a signed-in caller; verified by placing and cancelling an order \
             under a session from `authentication`.\n\nNote: the brief is the base's; no \
             slicing turn redrafted it.\n"
        ),
        "{shown}"
    );

    // a retire under another id is matched by the model, and a class mixing
    // the withdrawal with the standing statement is sent back to separate
    let mixed = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0], [4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let mut provider = Provider::answering([mixed, grouping]);
    provider
        .source
        .evidence
        .insert("docs".to_owned(), Ok(evidence(vec![retire("orders.cancellation", WITHDRAWN)])));
    seed(&provider.files, SPEC, DESIGN, PLAN);

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let exchanges = provider.model.exchanges();
    assert!(
        exchanges[0].outcome.as_ref().is_err_and(|finding| {
            finding.contains("a `(retire)` claim withdraws a behaviour and cannot share a class")
        }),
        "{exchanges:?}"
    );
    let (_, amendment) = staged_amendment(&provider);
    let retired = &amendment["changes"][3];
    assert_eq!(retired["id"], "REQ-004", "{amendment}");
    assert_eq!(retired["subject"], "orders.cancel", "{amendment}");
    assert_eq!(retired["class"], "retired", "{amendment}");
    assert_eq!(retired["after"], Value::Null, "{amendment}");
    assert_eq!(retired["sources"][0]["claim"], "orders.cancellation", "{amendment}");
    provider.model.assert_exhausted();
}

// Retiring every requirement of a part removes the part, and the design
// types it owned leave the design with it, a section left with no block
// going too: the amendment lists the part removed and the section removed,
// the output names both, `show amendment` renders the two sections, and
// the grouping is the one turn spent.
#[tokio::test]
async fn amend_retired_part() {
    // indices 0 and 1 are `docs`, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0], [4]]},
        {"claims": [1, 5], "classes": [[1], [5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let mut provider = Provider::answering([grouping, grouping]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            retire("orders.create", "Placing an order is withdrawn."),
            retire("orders.cancel", WITHDRAWN),
        ])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 2 retired, 0 \
             conflict, 2 unchanged\n  stems: orders\n  design: domain-model\n  parts: orders \
             (removed)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 1, "the grouping is the one turn");

    // the plan keeps the part left standing and the preamble, the design
    // its overview alone, each under the base as parent
    let plan = &amendment["plan"];
    assert_eq!(plan["parent"], revision, "{amendment}");
    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    assert_eq!(plan["preamble"], base["preamble"], "{amendment}");
    assert_eq!(plan["parts"].as_array().map(Vec::len), Some(1), "{amendment}");
    assert_eq!(plan["parts"][0], base["parts"][0], "{amendment}");
    let design = &amendment["design"];
    assert_eq!(design["parent"], revision, "{amendment}");
    assert_eq!(design["sections"].as_array().map(Vec::len), Some(1), "{amendment}");
    assert_eq!(design["sections"][0]["kind"], "overview", "{amendment}");
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": ["domain-model"], "changed": []}),
        "{amendment}"
    );
    assert_eq!(
        amendment["parts"],
        serde_json::json!([
            {"id": "PART-002", "name": "orders", "class": "removed", "brief": "kept"}
        ]),
        "{amendment}"
    );

    // a removed part carries no brief to note
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.ends_with(
            "## Design\n\nRemoved: Domain model.\n\n## Plan\n\n### Part: orders [removed]\n\nID: \
             PART-002\n"
        ),
        "{shown}"
    );

    // the envelope carries what the design and the plan change
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "amend", DOCS, "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["sections"], amendment["sections"], "{envelope}");
    assert_eq!(envelope["parts"], amendment["parts"], "{envelope}");
    provider.model.assert_exhausted();
}

// Removing a part re-derives the dependencies of the parts that stood
// after it from the types the remaining parts own and use: the part after
// it is changed in its dependencies alone, and the design, which the
// removed part owned nothing of, stands.
#[tokio::test]
async fn amend_retired_chain() {
    // index 0 is `docs`, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 2], "classes": [[0], [2]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let mut provider = Provider::answering([grouping]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![retire("session.timeout", "Sessions no longer time out.")])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, CHAIN_PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 1 retired, 0 \
             conflict, 3 unchanged\n  stems: session\n  parts: sessions (removed), orders (brief \
             kept)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    let base: Value = serde_json::from_slice(CHAIN_PLAN).expect("the base plan is JSON");
    let parts = amendment["plan"]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 2, "{amendment}");
    assert_eq!(parts[0], base["parts"][0], "{amendment}");
    assert_eq!(parts[1]["id"], "PART-003", "{amendment}");
    assert_eq!(parts[1]["depends-on"], serde_json::json!([]), "{amendment}");
    assert_eq!(parts[1]["requirements"], base["parts"][2]["requirements"], "{amendment}");
    assert_eq!(parts[1]["types"], base["parts"][2]["types"], "{amendment}");
    assert_eq!(parts[1]["brief"], base["parts"][2]["brief"], "{amendment}");
    assert_eq!(
        amendment["parts"],
        serde_json::json!([
            {"id": "PART-002", "name": "sessions", "class": "removed", "brief": "kept"},
            {"id": "PART-003", "name": "orders", "class": "changed", "brief": "kept"}
        ]),
        "{amendment}"
    );
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": [], "changed": []}),
        "{amendment}"
    );
    let design: Value = serde_json::from_slice(DESIGN).expect("the base design is JSON");
    assert_eq!(amendment["design"]["sections"], design["sections"], "{amendment}");
    provider.model.assert_exhausted();
}

// Retiring the part that owns a named or used type removes the type from
// the design, prunes it from other parts' `uses`, and drops the derived
// edges onto the removed part with no design or slicing turn; when a
// slicing turn is opened, `## Current plan` renders each current part's
// `uses` and derived `depends on`, and a join's `uses` merges in design
// order and re-derives `depends-on` across current and new parts alike.
#[tokio::test]
async fn part_edges() {
    let base_design = format!(
        r#"{{"emery": 10, "parent": null, "preamble": [], "sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}]}},
            {{"kind": "domain-model", "blocks": [
                {{"type": {{"key": "session.session", "signature": "interface Session {{ token: string }}", "role": "data"}}}},
                {{"type": {{"key": "orders.order", "signature": "interface Order {{ id: string; session: Session }}", "role": "data"}}}},
                {{"type": {{"key": "orders.line", "signature": "interface Line {{ sku: string }}", "role": "data"}}}}
            ]}}
        ]}}"#
    );
    let base_plan = r#"{"emery": 10, "parent": null, "preamble": [], "parts": [
        {
            "id": "PART-001", "name": "authentication", "requirements": ["REQ-001"],
            "types": [], "uses": ["session.session"], "depends-on": ["PART-002"],
            "brief": ["Delivers sign-in."]
        },
        {
            "id": "PART-002", "name": "sessions", "requirements": ["REQ-002"],
            "types": ["session.session"], "uses": [], "depends-on": [],
            "brief": ["Delivers the session a sign-in issues."]
        },
        {
            "id": "PART-003", "name": "orders", "requirements": ["REQ-003", "REQ-004"],
            "types": ["orders.line", "orders.order"], "uses": [], "depends-on": ["PART-002"],
            "brief": ["Delivers order creation and cancellation."]
        }
    ]}"#;
    let grouping = r#"{"groups": [
        {"claims": [0, 2], "classes": [[0], [2]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let mut provider = Provider::answering([grouping]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![retire("session.timeout", "Sessions no longer time out.")])),
    );
    seed(&provider.files, SPEC, base_design.as_bytes(), base_plan.as_bytes());

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    assert_eq!(provider.model.seen().len(), 1, "the grouping is the one turn");
    let (_, amendment) = staged_amendment(&provider);
    let parts = amendment["plan"]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 2, "{amendment}");
    assert_eq!(parts[0]["id"], "PART-001", "{amendment}");
    assert_eq!(parts[0]["uses"], serde_json::json!([]), "{amendment}");
    assert_eq!(parts[0]["depends-on"], serde_json::json!([]), "{amendment}");
    assert_eq!(parts[1]["id"], "PART-003", "{amendment}");
    assert_eq!(parts[1]["depends-on"], serde_json::json!([]), "{amendment}");
    provider.model.assert_exhausted();

    // when a slicing turn opens, current `uses` are shown in `## Current
    // plan` and a join's `uses` merges in design order and re-derives
    // `depends-on`
    let slicing = r#"{"parts": [], "joins": [
        {"part": "PART-001", "requirements": ["REQ-005", "REQ-006"], "uses": ["orders.line"]}
    ], "briefs": [
        {"part": "PART-001", "brief": ["Delivers sign-in, its session, and the per-caller limit."]}
    ]}"#;
    let (provider, _) = limited([slicing]);
    let using_plan = String::from_utf8_lossy(PLAN).replace(
        "\"types\": [],\n      \"uses\": [],\n      \"depends-on\": []",
        "\"types\": [],\n      \"uses\": [\"orders.order\"],\n      \"depends-on\": []",
    );
    seed(&provider.files, SPEC, DESIGN, using_plan.as_bytes());

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let turn = &provider.model.seen()[2];
    let request = turn.messages.join("\n");
    assert!(
        request.contains(
            "## Current plan\n\n- `PART-001` `authentication` — REQ-001, REQ-002 — uses: \
             `orders.order` — depends on: `orders`\n- `PART-002` `orders` — REQ-003, REQ-004 — \
             types: `orders.line`, `orders.order`\n"
        ),
        "{request}"
    );
    let (_, amendment) = staged_amendment(&provider);
    let parts = amendment["plan"]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 2, "{amendment}");
    assert_eq!(parts[0]["uses"], serde_json::json!(["orders.line", "orders.order"]), "{amendment}");
    assert_eq!(parts[0]["depends-on"], serde_json::json!(["PART-002"]), "{amendment}");
    provider.model.assert_exhausted();
}

// The paragraphs of a base design's `## APIs and integrations` that describe
// the order requirements.
const CONSUMED: &str = "Cancellations arrive as a message, which REQ-004 consumes.";
const STORED: &str =
    "Orders are kept in the order store, which REQ-003 writes and REQ-004 updates.";

// A base design over `DESIGN`'s overview and domain model whose
// `## APIs and integrations` holds the paragraphs `apis`.
fn describing(apis: &[&str]) -> String {
    let blocks: Vec<String> = apis.iter().map(|text| format!(r#"{{"text": "{text}"}}"#)).collect();
    format!(
        r#"{{"emery": 10, "parent": null, "preamble": [], "sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}]}},
            {DOMAIN_MODEL},
            {{"kind": "apis", "blocks": [{}]}}
        ]}}"#,
        blocks.join(", ")
    )
}

// A retirement informs the sections that describe it: where a paragraph of
// the design cites a retired requirement, one design turn is put after the
// grouping with the section's paragraphs numbered, and a paragraph
// describing retired requirements alone is dropped unnamed or reworded
// named; the types the emptied part owned leave the design by code before
// the turn, so the domain model is not offered.
#[tokio::test]
async fn amend_retired_design() {
    let base = describing(&[CONSUMED, STORED]);
    // indices 0 and 1 are `docs`, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0], [4]]},
        {"claims": [1, 5], "classes": [[1], [5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Sign-in issues a session."}], "revised": [1]},
        {"kind": "apis", "blocks": [{"text": "The order store holds nothing now that orders are withdrawn."}], "revised": [2]}
    ]}"#;
    let mut provider = Provider::answering([grouping, design]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            retire("orders.create", "Placing an order is withdrawn."),
            retire("orders.cancel", WITHDRAWN),
        ])),
    );
    let revision = seed(&provider.files, SPEC, base.as_bytes(), PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 2 retired, 0 \
             conflict, 2 unchanged\n  stems: orders\n  design: overview, domain-model, apis\n  \
             parts: orders (removed)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 2, "the grouping and the design");
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": ["domain-model"], "changed": ["overview", "apis"]}),
        "{amendment}"
    );
    assert_eq!(
        amendment["design"]["sections"],
        serde_json::json!([
            {"kind": "overview", "blocks": [{"text": "Sign-in issues a session."}]},
            {"kind": "apis", "blocks": [{"text": "The order store holds nothing now that orders are withdrawn."}]}
        ]),
        "{amendment}"
    );

    // the turn lists the describing section's paragraphs numbered, the
    // retired requirements, and the rule for a paragraph describing them;
    // the pruned domain model is offered nowhere
    let request = provider.model.seen()[1].messages.join("\n");
    assert!(request.starts_with("Amend `design.md`"), "{request}");
    assert!(
        request.contains(&format!("### APIs and integrations\n\n[1] {CONSUMED}\n[2] {STORED}\n")),
        "{request}"
    );
    assert!(
        request.contains("A paragraph describing requirements the change retires"),
        "{request}"
    );
    assert!(
        request.contains("- REQ-003 `orders.create`\n- REQ-004 `orders.cancel`\n"),
        "{request}"
    );
    assert!(!request.contains("### Domain model"), "{request}");
    assert!(!request.contains("\n## Type blocks\n"), "{request}");
    provider.model.assert_exhausted();
}

// A retire of one requirement over a section describing it alone: the
// section is answered with no block and leaves the design, the overview is
// kept verbatim, and the part shrinks keeping its types.
#[tokio::test]
async fn amend_retired_section() {
    let base = describing(&[CONSUMED]);
    // indices 0 and 1 are `docs`, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1, 5], "classes": [[1], [5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}], "revised": []}},
            {{"kind": "apis", "blocks": [], "revised": []}}
        ]}}"#
    );
    let mut provider = Provider::answering([grouping, design.as_str()]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("orders.create", "A signed-in user places an order."),
            retire("orders.cancel", WITHDRAWN),
        ])),
    );
    let revision = seed(&provider.files, SPEC, base.as_bytes(), PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 1 retired, 0 \
             conflict, 3 unchanged\n  stems: orders\n  design: apis\n  parts: orders (brief \
             kept)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 2, "the grouping and the design");
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": ["apis"], "changed": []}),
        "{amendment}"
    );
    let sections = amendment["design"]["sections"].as_array().expect("sections");
    assert_eq!(sections.len(), 2, "{amendment}");
    assert_eq!(
        sections[0],
        serde_json::json!({"kind": "overview", "blocks": [{"text": OVERVIEW}]})
    );
    assert_eq!(sections[1]["kind"], "domain-model", "{amendment}");
    assert_eq!(
        amendment["plan"]["parts"][1]["types"],
        serde_json::json!(["orders.line", "orders.order"]),
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

// An overview whose paragraph cites a retired requirement opens the turn on
// its own, where no other section describes it: the overview alone is
// listed, revised and named, and the amendment stages for it.
#[tokio::test]
async fn amend_retired_overview() {
    let cited = "Sign-in issues a session; orders are placed (REQ-003) and cancelled (REQ-004) \
                 under it.";
    let base = format!(
        r#"{{"emery": 10, "parent": null, "preamble": [], "sections": [
            {{"kind": "overview", "blocks": [{{"text": "{cited}"}}]}},
            {DOMAIN_MODEL}
        ]}}"#
    );
    // indices 0 and 1 are `docs`, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1, 5], "classes": [[1], [5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let revised = "Sign-in issues a session; orders are placed (REQ-003) under it.";
    let design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{revised}"}}], "revised": [1]}}
        ]}}"#
    );
    let mut provider = Provider::answering([grouping, design.as_str()]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("orders.create", "A signed-in user places an order."),
            retire("orders.cancel", WITHDRAWN),
        ])),
    );
    seed(&provider.files, SPEC, base.as_bytes(), PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("\n  design: overview\n"), "{stdout}");
    assert_eq!(provider.model.seen().len(), 2, "the grouping and the design");
    let request = provider.model.seen()[1].messages.join("\n");
    assert!(request.contains(&format!("### Overview\n\n[1] {cited}\n")), "{request}");
    assert!(!request.contains("### APIs and integrations"), "{request}");
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": [], "changed": ["overview"]}),
        "{amendment}"
    );
    assert_eq!(amendment["design"]["sections"][0]["blocks"][0]["text"], revised, "{amendment}");
    provider.model.assert_exhausted();
}

// A paragraph named `revised` is reworded, never dropped, while it describes
// a requirement the change keeps: where a paragraph cites a retired and a
// kept requirement, in `## Overview` as in `## APIs and integrations`, an
// answer that names it and leaves the kept one described nowhere in the
// section is a finding naming the requirement, and the next answer rewording
// it around the kept one is accepted.
#[tokio::test]
async fn revised_dropped_block() {
    let cited = "Sign-in issues a session; orders are placed (REQ-003) and cancelled (REQ-004) \
                 under it.";
    let base = format!(
        r#"{{"emery": 10, "parent": null, "preamble": [], "sections": [
            {{"kind": "overview", "blocks": [{{"text": "{cited}"}}]}},
            {DOMAIN_MODEL},
            {{"kind": "apis", "blocks": [{{"text": "{STORED}"}}]}}
        ]}}"#
    );
    // indices 0 and 1 are `docs`, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1, 5], "classes": [[1], [5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let dropped = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Sign-in issues a session."}], "revised": [1]},
        {"kind": "apis", "blocks": [{"text": "Cancellations no longer reach the order store."}], "revised": [1]}
    ]}"#;
    let reworded_overview = "Sign-in issues a session; orders are placed (REQ-003) under it.";
    let reworded_stored = "Orders are kept in the order store, which REQ-003 writes.";
    let reworded = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{reworded_overview}"}}], "revised": [1]}},
            {{"kind": "apis", "blocks": [{{"text": "{reworded_stored}"}}], "revised": [1]}}
        ]}}"#
    );
    let mut provider = Provider::answering([grouping, dropped, reworded.as_str()]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("orders.create", "A signed-in user places an order."),
            retire("orders.cancel", WITHDRAWN),
        ])),
    );
    seed(&provider.files, SPEC, base.as_bytes(), PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("\n  design: overview, apis\n"), "{stdout}");
    assert_eq!(provider.model.seen().len(), 3, "the grouping, the design, corrected");
    let request = provider.model.seen()[1].messages.join("\n");
    assert!(
        request.contains("Name a paragraph `revised` to reword it, never to drop it"),
        "{request}"
    );
    let exchanges = provider.model.exchanges();
    let finding = exchanges[1].outcome.as_ref().expect_err("the first answer is refused");
    assert!(
        finding.contains(
            "`## Overview` [1] is revised and describes REQ-003, which the change keeps, yet no \
             paragraph of the section describes it now; reword it rather than dropping it"
        ),
        "{finding}"
    );
    assert!(
        finding.contains(
            "`## APIs and integrations` [1] is revised and describes REQ-003, which the change \
             keeps, yet no paragraph of the section describes it now; reword it rather than \
             dropping it"
        ),
        "{finding}"
    );
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": [], "changed": ["overview", "apis"]}),
        "{amendment}"
    );
    assert_eq!(
        amendment["design"]["sections"][0]["blocks"][0]["text"], reworded_overview,
        "{amendment}"
    );
    assert_eq!(
        amendment["design"]["sections"][2]["blocks"][0]["text"], reworded_stored,
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

// A behaviour source read whole retires what it falls silent on: under the
// stems its requirement claims cover, a standing requirement no claim of
// the change touched is retired with the source named in its place, the
// grouping the one turn spent. The entry is listed apart in the counts and
// under `## Retired` with a note, and the requirement leaves its part as
// one a `retire` claim withdrew does.
#[tokio::test]
async fn amend_scoped() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let mut provider = Provider::answering([SCOPED_GROUPING, SCOPED_GROUPING]);
    code(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", "--config", &config]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 1 retired (1 \
             by scope), 0 conflict, 3 unchanged\n  stems: orders\n  parts: orders (brief \
             kept)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    let expected: Value = serde_json::from_str(&SCOPED.replace("<revision>", &revision))
        .expect("the fixture is JSON");
    assert_eq!(amendment, expected);
    assert_eq!(provider.model.seen().len(), 1, "the grouping is the one turn");
    assert_eq!(provider.vcs.calls(), read_calls());

    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.contains("0 added, 0 changed, 1 retired, 0 conflict, 3 unchanged."),
        "the projection counts the class alone: {shown}"
    );
    assert!(
        shown.contains(
            "## Retired\n\n### Requirement: orders.cancel\n\nID: REQ-004\nSources: []\n\nA \
             signed-in user cancels an open order.\n\nNote: retired by scope: `code` was read \
             whole and states nothing under `orders`.\n"
        ),
        "{shown}"
    );
    assert!(shown.contains("### Part: orders [changed]\n\nID: PART-002\nRequirements: [REQ-003]"));

    // the envelope counts the scope-retired among the retired
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 1, "conflict": 0, "unchanged": 3, "by-scope": 1
        }),
        "{envelope}"
    );
    provider.model.assert_exhausted();
}

// The same source read as a delta retires nothing, since what it is silent
// on stands; nor does a documentation source read whole, since only a
// behaviour source's scope says what the code no longer does. Either way
// the change restates one requirement and nothing is staged.
#[tokio::test]
async fn amend_scoped_silent() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "delta");
    let mut provider = Provider::answering([SCOPED_GROUPING]);
    code(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", "--config", &config]).await;

    let unchanged = format!(
        "nothing staged: no change against the current revision\n  base {revision} at {}\n  0 \
         added, 0 changed, 0 retired, 0 conflict, 4 unchanged\n",
        &HEAD[..8]
    );
    assert_eq!(String::from_utf8_lossy(&resp.stdout), unchanged);
    assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");
    provider.model.assert_exhausted();

    let config = scoped(&scratch, DOCS, "whole");
    let mut provider = Provider::answering([SCOPED_GROUPING]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![requirement("orders.create", "A signed-in user places an order.")])),
    );
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", "--config", &config]).await;

    assert_eq!(String::from_utf8_lossy(&resp.stdout), unchanged);
    assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");
    provider.model.assert_exhausted();
}

// When an entire stem vanishes from a whole-scope behaviour source so the
// source emits zero claims under that stem, standing requirements whose
// non-empty base citations name that source alone retire by scope, emptying
// their part and its owned types, while a silent stem that also cites another
// source stands unchanged.
#[tokio::test]
async fn amend_scoped_vanished_stem() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");

    // index 0 is `code`'s `auth.login`, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 1], "classes": [[0, 1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let mut provider = Provider::answering([grouping, grouping]);
    code_with(&mut provider, vec![requirement("auth.login", "Users sign in with a credential.")]);
    let revision = seed(&provider.files, &spec_citing_code("code"), DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", "--config", &config]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 2 retired (2 \
             by scope), 0 conflict, 2 unchanged\n  stems: orders\n  design: domain-model\n  \
             parts: orders (removed)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 1, "the grouping is the one turn");
    assert_eq!(provider.vcs.calls(), read_calls());
    assert_eq!(amendment["changes"][0]["class"], "unchanged", "{amendment}");
    assert_eq!(amendment["changes"][1]["class"], "unchanged", "{amendment}");
    assert_eq!(amendment["changes"][2]["class"], "retired", "{amendment}");
    assert_eq!(amendment["changes"][2]["scope"], "code", "{amendment}");
    assert_eq!(amendment["changes"][3]["class"], "retired", "{amendment}");
    assert_eq!(amendment["changes"][3]["scope"], "code", "{amendment}");
    assert_eq!(
        amendment["parts"],
        serde_json::json!([
            {"id": "PART-002", "name": "orders", "class": "removed", "brief": "kept"}
        ]),
        "{amendment}"
    );
    assert_eq!(
        amendment["sections"],
        serde_json::json!({
            "preamble": false, "added": [], "removed": ["domain-model"], "changed": []
        }),
        "{amendment}"
    );

    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.contains(
            "## Retired\n\n### Requirement: orders.create\n\nID: REQ-003\nSources: []\n\nA \
             signed-in user places an order.\n\nNote: retired by scope: `code` was read whole \
             and states nothing under `orders`.\n\n### Requirement: orders.cancel\n\nID: \
             REQ-004\nSources: []\n\nA signed-in user cancels an open order.\n\nNote: retired \
             by scope: `code` was read whole and states nothing under `orders`.\n"
        ),
        "{shown}"
    );
    assert!(shown.ends_with("### Part: orders [removed]\n\nID: PART-002\n"), "{shown}");

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 2, "conflict": 0, "unchanged": 2, "by-scope": 2
        }),
        "{envelope}"
    );
    provider.model.assert_exhausted();
}

// A whole-scope behaviour source that emits no requirement claims at all
// retires every standing requirement citing it alone with no grouping turn.
#[tokio::test]
async fn amend_scoped_all_vanished() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let mut provider = Provider::idle();
    code_with(&mut provider, Vec::new());
    seed(&provider.files, &spec_citing_code("docs"), DESIGN, PLAN);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 2, "conflict": 0, "unchanged": 2, "by-scope": 2
        }),
        "{envelope}"
    );
    assert!(provider.model.seen().is_empty(), "no turn when no claim states a requirement");
}

// Under `scope = "whole"`, re-extracted claims on untouched behaviour whose
// wording or leaf/sub-stem slug drifted yet share the standing requirement's
// winning equivalence class stay `unchanged` byte for byte: original subjects,
// bodies, scenarios, triggers, design, and untouched parts are preserved,
// standing scenarios and extracted criteria appear in the grouping brief, and
// only the requirement placed in its own class is redrafted.
#[tokio::test]
async fn amend_scoped_equivalent() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");

    // base spec citing `code` alone, with an untriggered helper under `orders`
    // where `REQ-003` already holds `POST /orders`
    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
    for req in base_spec["requirements"].as_array_mut().expect("requirements") {
        req["sources"][0]["source"] = Value::from("code");
    }
    base_spec["requirements"][3]["subject"] = Value::from("orders.validate-sku");
    base_spec["requirements"][3]["trigger"] = Value::Null;
    base_spec["requirements"][3]["sources"][0]["claim"] = Value::from("orders.validate-sku");
    base_spec["requirements"][3]["body"] =
        serde_json::json!(["An order referencing an unknown SKU is rejected."]);
    let base_bytes = serde_json::to_vec(&base_spec).expect("serialise base spec");

    // indices 0..=3 are `code`'s re-extracted claims, 4..=7 the standing requirements
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1, 5], "classes": [[1, 5]]},
        {"claims": [2, 6], "classes": [[2], [6]]},
        {"claims": [3, 7], "classes": [[3, 7]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.post.place", "scenarios": [
            {"name": "Bulk cap", "when": "a signed-in caller places an order of up to 20 units", "then": "the order is created"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    code_with(
        &mut provider,
        vec![
            requirement("auth.sign-in", "Callers authenticate with a credential."),
            requirement("session.idle-expiry", "Idle sessions time out after sixty minutes."),
            requirement("orders.post.place", "A signed-in user places an order of up to 20 units."),
            claim(
                ClaimKind::Criterion,
                "orders.post.place.max-units",
                ("criterion", "MAX_UNITS is 20."),
            ),
            requirement(
                "orders.post.unknown-sku",
                "Placing an order rejects a line whose SKU is absent.",
            ),
            claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders")),
        ],
    );
    seed(&provider.files, &base_bytes, DESIGN, PLAN);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 1, "retired": 0, "conflict": 0, "unchanged": 3, "by-scope": 0
        }),
        "{envelope}"
    );
    assert_eq!(envelope["stems"], serde_json::json!(["orders"]), "{envelope}");
    assert_eq!(envelope["sections"], Value::Null, "design stands unchanged: {envelope}");
    assert_eq!(envelope["parts"], Value::Null, "no part membership changed: {envelope}");

    // standing scenarios and extracted criteria appear in the grouping brief
    let grouping_turn = provider.model.seen()[0].messages.join("\n");
    assert!(
        grouping_turn.contains("  - criterion `orders.post.place.max-units`: MAX_UNITS is 20."),
        "{grouping_turn}"
    );
    assert!(
        grouping_turn
            .contains("  - scenario `Timeout`: when a session is idle for an hour -> it times out"),
        "{grouping_turn}"
    );

    // only `REQ-003` is drafted anew; the other three stay byte for byte the base's
    let spec_turn = provider.model.seen()[1].messages.join("\n");
    assert!(spec_turn.contains("- REQ-003 `orders.post.place`"), "{spec_turn}");
    assert!(!spec_turn.contains("REQ-001"), "{spec_turn}");
    assert!(!spec_turn.contains("REQ-002"), "{spec_turn}");
    assert!(!spec_turn.contains("REQ-004"), "{spec_turn}");

    let (_, amendment) = staged_amendment(&provider);
    for (idx, req_id) in [(0, "REQ-001"), (1, "REQ-002"), (3, "REQ-004")] {
        assert_eq!(amendment["changes"][idx]["id"], req_id, "{amendment}");
        assert_eq!(amendment["changes"][idx]["class"], "unchanged", "{amendment}");
        assert_eq!(
            amendment["changes"][idx]["subject"], base_spec["requirements"][idx]["subject"],
            "{amendment}"
        );
        assert_eq!(
            amendment["changes"][idx]["after"], amendment["changes"][idx]["before"],
            "{amendment}"
        );
        assert_eq!(
            amendment["spec"]["requirements"][idx], base_spec["requirements"][idx],
            "{amendment}"
        );
    }
    let base_plan: Value = serde_json::from_slice(PLAN).expect("base plan JSON");
    assert_eq!(amendment["plan"]["parts"][0], base_plan["parts"][0], "{amendment}");
    provider.model.assert_exhausted();
}

// Under `scope = "whole"`, re-extraction granularity drift on untouched code
// neither manufactures spurious `added` requirements nor retires covered
// standing requirements by scope:
// - multiple finer-grained claims of one `(source, stem)` placed in one
//   standing requirement's class stay unsplit on that requirement (`unchanged`);
// - one consolidated claim covering distinct standing requirements may join
//   each standing requirement's group without merging the standing requirements
//   (while invalid multi-group placements are refused and corrected);
// - a standing requirement whose behaviour was extracted only as an `excerpt`
//   in `## Whole-source detail` and named in `evidenced` is preserved rather
//   than retired by scope.
#[tokio::test]
async fn amend_scoped_granularity() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");

    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
    for req in base_spec["requirements"].as_array_mut().expect("requirements") {
        req["sources"][0]["source"] = Value::from("code");
        req["trigger"] = Value::Null;
    }
    base_spec["requirements"][1]["sources"][0]["path"] = Value::from("src/session.ts#L10-L14");
    let base_bytes = serde_json::to_vec(&base_spec).expect("serialise base spec");

    // indices 0..=2 are `code`'s requirement claims; 3..=6 the standing requirements
    let invalid = r#"{"groups": [
        {"claims": [0, 1, 3], "classes": [[0, 1, 3]]},
        {"claims": [4], "classes": [[4]]},
        {"claims": [2, 5], "classes": [[2], [5]]},
        {"claims": [2, 6], "classes": [[2], [6]]}
    ], "evidenced": [3, 4]}"#;
    let valid = r#"{"groups": [
        {"claims": [0, 1, 3], "classes": [[0, 1, 3]]},
        {"claims": [4], "classes": [[4]]},
        {"claims": [2, 5], "classes": [[2, 5]]},
        {"claims": [2, 6], "classes": [[2, 6]]}
    ], "evidenced": [4]}"#;
    let design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}], "revised": []}},
            {{"kind": "technical-logic", "blocks": [{{"text": "Idle sessions expire after 60 minutes (REQ-002)."}}], "revised": []}}
        ]}}"#
    );
    let mut provider = Provider::answering([invalid, valid, design.as_str()]);
    let mut session_excerpt = Claim::new(ClaimKind::Excerpt)
        .at("src/session.ts#L10-L14")
        .with_extra("excerpt", "Idle sessions are evicted after 60 minutes without activity.");
    session_excerpt.id = None;
    code_with(
        &mut provider,
        vec![
            requirement("auth.login.credentials", "Callers sign in by presenting a credential."),
            requirement(
                "auth.login.normalise",
                "Presented sign-in identifiers are trimmed before lookup.",
            ),
            requirement(
                "orders.lifecycle",
                "A signed-in user places an order and may cancel an open order.",
            ),
            session_excerpt,
        ],
    );
    seed(&provider.files, &base_bytes, DESIGN, PLAN);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 0, "conflict": 0, "unchanged": 4, "by-scope": 0
        }),
        "{envelope}"
    );
    assert_eq!(envelope["stems"], Value::Null, "{envelope}");
    assert_eq!(envelope["parts"], Value::Null, "{envelope}");

    let grouping_turn = provider.model.seen()[0].messages.join("\n");
    assert!(grouping_turn.contains("## Whole-source detail"), "{grouping_turn}");
    assert!(
        grouping_turn.contains("`code` `src/session.ts#L10-L14` — Idle sessions are evicted"),
        "{grouping_turn}"
    );
    let exchanges = provider.model.exchanges();
    let finding = exchanges[0].outcome.as_ref().expect_err("invalid multi-group answer rejected");
    assert!(
        finding.contains("claim 2 is placed outside the standing requirement's class"),
        "{finding}"
    );
    assert!(finding.contains("standing requirement 3 is already matched to a claim"), "{finding}");

    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(amendment["spec"]["requirements"], base_spec["requirements"], "{amendment}");
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn amend_scoped_facets() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");

    // two facet claims revising one standing requirement join their statements
    let revised_grouping = r#"{"groups": [
        {"claims": [0, 1, 4], "classes": [[0, 1], [4]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [5], "classes": [[5]]}
    ]}"#;
    let revised_draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.post.totals", "scenarios": [
            {"name": "Bulk discount at 20", "when": "an order has 20 units", "then": "the 5 percent bulk discount applies"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([revised_grouping, revised_draft]);
    code_with(
        &mut provider,
        vec![
            requirement("orders.post.totals", "Order totals sum priced lines and shipping."),
            requirement(
                "orders.post.bulk-discount",
                "A 5 percent bulk discount applies from 20 units.",
            ),
        ],
    );
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 1, "retired": 1, "conflict": 0, "unchanged": 2, "by-scope": 1
        }),
        "{envelope}"
    );
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(
        amendment["changes"][2]["after"],
        "Order totals sum priced lines and shipping. A 5 percent bulk discount applies from 20 \
         units.",
        "{amendment}"
    );
    provider.model.assert_exhausted();

    // a second same-stem claim in a separate class spins out as added without self-conflict
    let mixed_class_grouping = r#"{"groups": [
        {"claims": [0, 1, 4], "classes": [[0, 4], [1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [5, 0], "classes": [[0, 5]]}
    ]}"#;
    let added_draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.post.gift-note", "scenarios": [
            {"name": "Gift note stored", "when": "an order includes a gift note", "then": "the gift note is stored"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([mixed_class_grouping, added_draft]);
    code_with(
        &mut provider,
        vec![
            requirement(
                "orders.lifecycle",
                "A signed-in user places an order and may cancel an open order.",
            ),
            requirement("orders.post.gift-note", "An order may carry an optional gift note."),
        ],
    );
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 1, "changed": 0, "retired": 0, "conflict": 0, "unchanged": 4, "by-scope": 0
        }),
        "{envelope}"
    );
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(amendment["changes"][4]["id"], "REQ-005", "{amendment}");
    assert_eq!(amendment["changes"][4]["subject"], "orders.post.gift-note", "{amendment}");
    assert_eq!(amendment["changes"][4]["class"], "added", "{amendment}");
    provider.model.assert_exhausted();
}

// Under `scope = "whole"`, co-grouping a triggered secondary claim from
// another stem (`health.get-ready.*`) onto an untriggered `start.*` standing
// requirement keeps the base's `trigger: null`, `start.*` subject, and
// `unchanged` standing; and co-grouping a `start.*` claim ahead of a
// `health.*` claim onto a `health.*` standing requirement keeps the `health.*`
// stem and `unchanged` standing too.
#[tokio::test]
async fn amend_scoped_co_grouped_trigger() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let fixture = co_grouped_trigger_fixture();

    // 0..=1 from `start` seam, 2..=3 from `health` seam, 4..=5 from `orders` seam;
    // 6..=9 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 2, 6], "classes": [[0, 2, 6]]},
        {"claims": [1, 3, 7], "classes": [[1, 3, 7]]},
        {"claims": [4, 8], "classes": [[4], [8]]},
        {"claims": [5, 9], "classes": [[5, 9]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.create", "scenarios": [
            {"name": "Place order", "when": "a signed-in caller places an order of up to 20 units", "then": "the order is created"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    code_with(&mut provider, fixture.claims);
    seed(&provider.files, &fixture.spec, DESIGN, &fixture.plan);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 1, "retired": 0, "conflict": 0, "unchanged": 3, "by-scope": 0
        }),
        "{envelope}"
    );
    assert_eq!(envelope["stems"], serde_json::json!(["orders"]), "{envelope}");

    let grouping_turn = provider.model.seen()[0].messages.join("\n");
    assert!(grouping_turn.contains("Whole-scope re-extraction (`code`)"), "{grouping_turn}");
    assert!(grouping_turn.contains("  - route `GET /health/ready`"), "{grouping_turn}");

    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(amendment["changes"][0]["class"], "unchanged", "{amendment}");
    assert_eq!(amendment["changes"][0]["subject"], "start.health-probe-timeout", "{amendment}");
    assert_eq!(amendment["spec"]["requirements"][0]["trigger"], Value::Null, "{amendment}");
    assert_eq!(amendment["changes"][1]["class"], "unchanged", "{amendment}");
    assert_eq!(amendment["changes"][1]["subject"], "health.get-ready.report-probes", "{amendment}");
    assert_eq!(
        amendment["spec"]["requirements"][1]["trigger"],
        serde_json::json!({"http": {"method": "GET", "path": "/health/ready"}}),
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

struct TriggerFixture {
    spec: Vec<u8>,
    plan: Vec<u8>,
    claims: Vec<Claim>,
}

fn co_grouped_trigger_fixture() -> TriggerFixture {
    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
    for requirement in base_spec["requirements"].as_array_mut().expect("requirements") {
        requirement["sources"][0]["source"] = Value::from("code");
    }
    base_spec["requirements"][0]["subject"] = Value::from("start.health-probe-timeout");
    base_spec["requirements"][0]["trigger"] = Value::Null;
    base_spec["requirements"][0]["sources"] = serde_json::json!([{
        "source": "code",
        "claim": "start.health-probe-timeout",
        "path": "src/routes/health.ts#L7-L16"
    }]);
    base_spec["requirements"][0]["body"] =
        serde_json::json!(["A readiness probe exceeding 2000 milliseconds fails."]);
    base_spec["requirements"][1]["subject"] = Value::from("health.get-ready.report-probes");
    base_spec["requirements"][1]["trigger"] =
        serde_json::json!({"http": {"method": "GET", "path": "/health/ready"}});
    base_spec["requirements"][1]["sources"] = serde_json::json!([{
        "source": "code",
        "claim": "health.get-ready.report-probes",
        "path": "src/routes/health.ts#L26-L38"
    }]);
    base_spec["requirements"][1]["body"] = serde_json::json!([
        "GET /health/ready reports whether the store and cache probes succeeded."
    ]);

    let mut base_plan: Value = serde_json::from_slice(PLAN).expect("base plan JSON");
    base_plan["parts"][0]["name"] = Value::from("health");

    let claims = vec![
        requirement(
            "start.health-probe-timeout",
            "A dependency probe exceeding 2000 milliseconds is treated as failed.",
        )
        .at("src/routes/health.ts#L7-L16"),
        requirement(
            "start.health-ready-body",
            "GET /health/ready includes boolean fields for the store and cache probes.",
        )
        .at("src/routes/health.ts#L33-L37"),
        requirement(
            "health.get-ready.probe-timeout",
            "A readiness probe that does not finish within 2000 milliseconds fails.",
        )
        .at("src/routes/health.ts#L26-L30"),
        requirement(
            "health.get-ready.report-probe-outcomes",
            "The readiness response reports whether the store and cache probes succeeded.",
        )
        .at("src/routes/health.ts#L33-L37"),
        claim(ClaimKind::Contract, "health.get-ready", ("route", "GET /health/ready"))
            .at("src/routes/health.ts#L26-L38"),
        claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders"))
            .at("src/routes/orders.ts#L10-L20"),
        claim(ClaimKind::Contract, "orders.cancel-msg", ("message", "orders.cancel-requested"))
            .at("src/consumers/orders.ts#L10-L20"),
        requirement("orders.create", "A signed-in user places an order of up to 20 units."),
        requirement("orders.cancel", "A signed-in user cancels an open order."),
    ];
    TriggerFixture {
        spec: serde_json::to_vec(&base_spec).expect("serialise base spec"),
        plan: serde_json::to_vec(&base_plan).expect("serialise base plan"),
        claims,
    }
}

// Retiring one route's requirement under `scope = "whole"` while whole-repo
// `call`, `excerpt`, and unchanged `type` claims for `start`, `customers`, and
// surviving `orders` remain in the extract neither marks `domain-model` or
// `technical-logic` informed nor rejects a declared `revised` edit trimming
// retired prose from a shared infrastructure paragraph citing unchanged ids.
#[tokio::test]
async fn amend_scoped_retired_design() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let fixture = retired_lines_fixture(None);

    let grouping = r#"{"groups": [
        {"claims": [0, 3], "classes": [[0, 3]]},
        {"claims": [1, 4], "classes": [[1, 4]]},
        {"claims": [2, 5], "classes": [[2, 5]]},
        {"claims": [6], "classes": [[6]]}
    ]}"#;
    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Sign-in issues a session; orders are placed under it."}], "revised": [1]},
        {"kind": "apis", "blocks": [
            {"text": "Inbound HTTP exposes POST /orders (REQ-003)."},
            {"text": "The store connects via DATABASE_URL (REQ-001) and runs transactional updates on transitions."}
        ], "revised": [1, 2]}
    ]}"#;
    let mut provider = Provider::answering([grouping, design]);
    code_with(&mut provider, fixture.claims);
    seed(&provider.files, &fixture.spec, fixture.design.as_bytes(), PLAN);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 1, "conflict": 0, "unchanged": 3, "by-scope": 1
        }),
        "{envelope}"
    );
    assert_eq!(
        envelope["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": [], "changed": ["overview", "apis"]}),
        "{envelope}"
    );
    let design_turn = provider.model.seen()[1].messages.join("\n");
    assert!(!design_turn.contains("### Domain model"), "{design_turn}");
    assert!(!design_turn.contains("### Technical logic"), "{design_turn}");
    assert!(!design_turn.contains("src/server.ts#L44"), "{design_turn}");
    assert!(!design_turn.contains("src/services/orders.ts#L23-L33"), "{design_turn}");
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn amend_scoped_retired_mixed() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let added_customer =
        requirement("customers.delete-id", "A customer with no orders may be deleted.")
            .at("src/routes/customers.ts#L41-L47");
    let fixture = retired_lines_fixture(Some(added_customer));

    let mut base_plan: Value = serde_json::from_slice(PLAN).expect("base plan JSON");
    base_plan["parts"][0]["name"] = Value::from("customers");
    let plan_bytes = serde_json::to_vec(&base_plan).expect("serialise base plan");

    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1, 5], "classes": [[1, 5]]},
        {"claims": [2, 6], "classes": [[2, 6]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [7], "classes": [[7]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "customers.delete-id", "scenarios": [
            {"name": "Delete customer", "when": "a customer with no orders is deleted", "then": "204 is returned"}
        ]}
    ]}"#;
    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Sign-in issues a session; customers and orders are managed under it (REQ-005)."}], "revised": [1]},
        {"kind": "apis", "blocks": [
            {"text": "Inbound HTTP exposes POST /orders (REQ-003) and DELETE /customers/{id} (REQ-005)."},
            {"text": "The store connects via DATABASE_URL (REQ-001) and runs transactional updates on transitions."}
        ], "revised": [1, 2]}
    ]}"#;

    let mut provider = Provider::answering([grouping, draft, design]);
    code_with(&mut provider, fixture.claims);
    seed(&provider.files, &fixture.spec, fixture.design.as_bytes(), &plan_bytes);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 1, "changed": 0, "retired": 1, "conflict": 0, "unchanged": 3, "by-scope": 1
        }),
        "{envelope}"
    );
    let design_turn = provider.model.seen()[2].messages.join("\n");
    assert!(!design_turn.contains("src/server.ts#L44"), "{design_turn}");
    assert!(!design_turn.contains("src/repositories/orders.ts#L94"), "{design_turn}");
    assert!(design_turn.contains("customers.delete-id"), "{design_turn}");
    provider.model.assert_exhausted();
}

struct RetiredLinesFixture {
    spec: Vec<u8>,
    design: String,
    claims: Vec<Claim>,
}

fn retired_lines_fixture(extra: Option<Claim>) -> RetiredLinesFixture {
    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
    for requirement in base_spec["requirements"].as_array_mut().expect("requirements") {
        requirement["sources"][0]["source"] = Value::from("code");
    }
    for (index, subject, body) in [
        (
            0,
            "start.require-database-url",
            "The program refuses to start when DATABASE_URL is unset.",
        ),
        (1, "customers.post.create", "A valid customer creation persists the customer."),
        (3, "orders.put-id-lines", "A pending order's lines may be replaced."),
    ] {
        base_spec["requirements"][index]["subject"] = Value::from(subject);
        base_spec["requirements"][index]["sources"][0]["claim"] = Value::from(subject);
        base_spec["requirements"][index]["body"] = serde_json::json!([body]);
    }
    let spec = serde_json::to_vec(&base_spec).expect("serialise base spec");
    let design = describing(&[
        "Inbound HTTP exposes POST /orders (REQ-003) and PUT /orders/{id}/lines (REQ-004).",
        "The store connects via DATABASE_URL (REQ-001) and runs transactional updates on line \
         updates and transitions.",
    ]);

    let db_call =
        |path: &str| Claim::new(ClaimKind::Call).at(path).with_extra("callee", "pg:Pool.query");
    let excerpt = |path: &str, text: &str| {
        Claim::new(ClaimKind::Excerpt).at(path).with_extra("excerpt", text)
    };
    let mut claims = vec![
        requirement(
            "start.require-database-url",
            "The program refuses to start when DATABASE_URL is unset.",
        )
        .at("src/config.ts#L5-L7"),
        requirement("customers.post.create", "A valid customer creation persists the customer.")
            .at("src/services/customers.ts#L18-L31"),
        requirement("orders.create", "A signed-in user places an order.")
            .at("src/services/orders.ts#L40-L58"),
        claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders"))
            .at("src/routes/orders.ts#L10-L20"),
        claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
        claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        db_call("src/server.ts#L44"),
        db_call("src/repositories/customers.ts#L20"),
        db_call("src/repositories/orders.ts#L94"),
        excerpt("src/server.ts#L42-L60", "Startup probes the store before listening on PORT."),
        excerpt("src/services/orders.ts#L23-L33", "Order reads check the cache under order:{id}."),
    ];
    claims.extend(extra);
    RetiredLinesFixture { spec, design, claims }
}

// When `PUT /api/orders/{id}/lines` vanishes from the extracted contracts of a
// whole-scope behaviour source while `orders.ts` and `server.ts` still carry
// `excerpt` and `call` detail (plus untouched `start.omit-powered-by` setup on
// `src/server.ts`), `verify_evidenced` refuses listing `REQ-047..049` in
// `evidenced`, the corrected grouping retires all three by scope, and the
// untouched bootstrap setup neither mints an added requirement nor dirties
// `PART-001`.
#[tokio::test]
async fn amend_scoped_retire_route_not_evidenced() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let fixture = ScopedFixture::vanished_route();

    // 0..=3 are `code`'s requirement claims; 4..=9 the standing requirements
    // (`REQ-001`, `REQ-002`, `REQ-003`, `REQ-047`, `REQ-048`, `REQ-049`)
    let invalid = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2, 5], "classes": [[2, 5]]},
        {"claims": [3, 6], "classes": [[3, 6]]},
        {"claims": [7], "classes": [[7]]},
        {"claims": [8], "classes": [[8]]},
        {"claims": [9], "classes": [[9]]}
    ], "evidenced": [7, 8, 9]}"#;
    let valid = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2, 5], "classes": [[2, 5]]},
        {"claims": [3, 6], "classes": [[3, 6]]},
        {"claims": [7], "classes": [[7]]},
        {"claims": [8], "classes": [[8]]},
        {"claims": [9], "classes": [[9]]}
    ]}"#;
    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Customers and orders are exposed over HTTP (REQ-001, REQ-002, REQ-003)."}], "revised": [1]},
        {"kind": "apis", "blocks": [{"text": "Inbound HTTP exposes POST /api/customers (REQ-002) and POST /api/orders (REQ-003)."}], "revised": [1]}
    ]}"#;
    let mut provider = Provider::answering([invalid, valid, design]);
    fixture.seed_into(&mut provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 3, "conflict": 0, "unchanged": 3, "by-scope": 3
        }),
        "{envelope}"
    );
    assert_eq!(envelope["stems"], serde_json::json!(["orders"]), "{envelope}");
    assert_eq!(
        envelope["parts"],
        serde_json::json!([
            {"id": "PART-002", "name": "orders", "class": "changed", "brief": "kept"}
        ]),
        "PART-001 (`start`) stays untouched: {envelope}"
    );

    let exchanges = provider.model.exchanges();
    let finding =
        exchanges[0].outcome.as_ref().expect_err("vanished route in `evidenced` rejected");
    for (index, subject) in [
        (7, "orders.put-id-lines.body-validation"),
        (8, "orders.put-id-lines.pending-only"),
        (9, "orders.put-id-lines.reprice-and-persist"),
    ] {
        assert!(
            finding.contains(&format!(
                "standing requirement {index} (`{subject}`) carries route \
                 `PUT /api/orders/{{id}}/lines`, which no contract of its cited whole-scope \
                 source declares"
            )),
            "{finding}"
        );
    }

    let (_, amendment) = staged_amendment(&provider);
    for (pos, id) in [(3, "REQ-047"), (4, "REQ-048"), (5, "REQ-049")] {
        assert_eq!(amendment["changes"][pos]["id"], id, "{amendment}");
        assert_eq!(amendment["changes"][pos]["class"], "retired", "{amendment}");
        assert_eq!(amendment["changes"][pos]["scope"], "code", "{amendment}");
    }
    provider.model.assert_exhausted();
}

// When `PUT /api/orders/{id}/lines` vanishes from the extracted contracts of a
// whole-scope behaviour source, standing requirements on that route are marked
// `absent from extracted contracts` in the grouping brief, `GroupingBrief::verify`
// refuses placing either an untriggered shared-helper claim
// (`orders.invalidate-cache-on-transition`) or another standing route's claim
// (`orders.post.create`) into their groups, and the corrected grouping retires
// all three `REQ-047..049` by scope.
#[tokio::test]
async fn amend_scoped_vanished_route_rejects_untriggered_claim() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let mut fixture = ScopedFixture::vanished_route();
    fixture.claims.insert(
        4,
        requirement(
            "orders.invalidate-cache-on-transition",
            "Every successful order state change removes the cached entry for that order.",
        )
        .at("src/services/orders.ts#L78"),
    );

    // 0..=4 are `code`'s requirement claims (`start.json-body-limit`,
    // `start.omit-powered-by`, `customers.post.create`, `orders.post.create`,
    // `orders.invalidate-cache-on-transition`); 5..=10 the standing
    // requirements (`REQ-001`, `REQ-002`, `REQ-003`, `REQ-047`, `REQ-048`, `REQ-049`)
    let invalid = r#"{"groups": [
        {"claims": [0, 5], "classes": [[0, 5]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2, 6], "classes": [[2, 6]]},
        {"claims": [3, 7], "classes": [[3, 7]]},
        {"claims": [8], "classes": [[8]]},
        {"claims": [3, 9], "classes": [[3, 9]]},
        {"claims": [4, 10], "classes": [[4, 10]]}
    ]}"#;
    let valid = r#"{"groups": [
        {"claims": [0, 5], "classes": [[0, 5]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2, 6], "classes": [[2, 6]]},
        {"claims": [3, 4, 7], "classes": [[3, 4, 7]]},
        {"claims": [8], "classes": [[8]]},
        {"claims": [9], "classes": [[9]]},
        {"claims": [10], "classes": [[10]]}
    ]}"#;
    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Customers and orders are exposed over HTTP (REQ-001, REQ-002, REQ-003)."}], "revised": [1]},
        {"kind": "apis", "blocks": [{"text": "Inbound HTTP exposes POST /api/customers (REQ-002) and POST /api/orders (REQ-003)."}], "revised": [1]}
    ]}"#;
    let mut provider = Provider::answering([invalid, valid, design]);
    fixture.seed_into(&mut provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 3, "conflict": 0, "unchanged": 3, "by-scope": 3
        }),
        "{envelope}"
    );
    assert_eq!(envelope["stems"], serde_json::json!(["orders"]), "{envelope}");

    let grouping_turn = provider.model.seen()[0].messages.join("\n");
    let absent_note = "route `PUT /api/orders/{id}/lines` — absent from extracted contracts";
    assert!(grouping_turn.contains(absent_note), "{grouping_turn}");

    let exchanges = provider.model.exchanges();
    let finding =
        exchanges[0].outcome.as_ref().expect_err("untriggered claim on vanished route rejected");
    let untriggered_finding = "standing requirement 10 (`orders.put-id-lines.reprice-and-persist`) \
                               carries route `PUT /api/orders/{id}/lines`, which is absent from \
                               the extracted contracts of `code`, so claim 4 \
                               (`orders.invalidate-cache-on-transition`) cannot keep it standing \
                               without a replacement contract";
    assert!(finding.contains(untriggered_finding), "{finding}");
    let borrowed_route_finding = "standing requirement 9 (`orders.put-id-lines.pending-only`) \
                                  carries route `PUT /api/orders/{id}/lines`, which is absent from \
                                  the extracted contracts of `code`, while claim 3 \
                                  (`orders.post.create`) belongs to existing route \
                                  `POST /api/orders`";
    assert!(finding.contains(borrowed_route_finding), "{finding}");

    let (_, amendment) = staged_amendment(&provider);
    for (pos, req_id) in [(3, "REQ-047"), (4, "REQ-048"), (5, "REQ-049")] {
        assert_eq!(amendment["changes"][pos]["id"], req_id, "{amendment}");
        assert_eq!(amendment["changes"][pos]["class"], "retired", "{amendment}");
        assert_eq!(amendment["changes"][pos]["scope"], "code", "{amendment}");
    }
    provider.model.assert_exhausted();
}

// When a standing requirement's whole-scope route vanished and a replacement
// claim revises it with no contract (from a delta source) or with a
// replacement contract (`PATCH /api/orders/{id}/lines`), `same_trigger` is
// `false` and the vanished `PUT /api/orders/{id}/lines` trigger is never
// carried forward onto the amended requirement.
#[tokio::test]
async fn amend_whole_vanished_trigger_not_preserved() {
    let scratch = Scratch::new();
    let fixture = ScopedFixture::vanished_route();
    assert_vanished_trigger_stripped(&scratch, &fixture).await;
    assert_vanished_trigger_replaced(&scratch, fixture).await;
}

async fn assert_vanished_trigger_stripped(scratch: &Scratch, fixture: &ScopedFixture) {
    let dual_config = scratch.config(&format!(
        "[[source]]\nadapter = \"{DOCS}\"\n\n[[source]]\nadapter = \"{CODE}\"\nscope = \
         \"whole\"\n\n[target]\nadapter = \"acme:builder@2.1.0\"\nrepository = \".\"\n"
    ));
    let grouping = r#"{"groups": [
        {"claims": [0, 10], "classes": [[0, 10]]},
        {"claims": [1, 5], "classes": [[1, 5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3, 6], "classes": [[3, 6]]},
        {"claims": [4, 7], "classes": [[4, 7]]},
        {"claims": [8], "classes": [[8]]},
        {"claims": [9], "classes": [[9]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.reprice-lines", "scenarios": [
            {"name": "Reprice lines", "when": "pending order lines are repriced", "then": "totals are recomputed"}
        ]}
    ]}"#;
    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Customers and orders are exposed over HTTP, with internal line repricing (REQ-001, REQ-002, REQ-003, REQ-049)."}], "revised": [1]},
        {"kind": "apis", "blocks": [{"text": "Inbound HTTP exposes POST /api/customers (REQ-002) and POST /api/orders (REQ-003); line repricing runs internally (REQ-049)."}], "revised": [1]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft, design]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![requirement(
            "orders.reprice-lines",
            "A valid amend-lines request replaces lines and recomputes totals.",
        )])),
    );
    fixture.seed_into(&mut provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &dual_config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 1, "retired": 2, "conflict": 0, "unchanged": 3, "by-scope": 2
        }),
        "{envelope}"
    );
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(amendment["changes"][5]["id"], "REQ-049", "{amendment}");
    assert_eq!(amendment["changes"][5]["class"], "changed", "{amendment}");
    let req_049 = amended_requirement(&amendment, "REQ-049");
    assert_eq!(req_049["trigger"], Value::Null, "vanished route stripped: {req_049}");
    provider.model.assert_exhausted();
}

async fn assert_vanished_trigger_replaced(scratch: &Scratch, mut fixture: ScopedFixture) {
    let whole_config = scoped(scratch, CODE, "whole");
    fixture.claims.insert(
        4,
        requirement(
            "orders.patch-id-lines.reprice-and-persist",
            "A valid patch-lines request replaces lines and recomputes totals.",
        )
        .at("src/services/orders.ts#L67-L78"),
    );
    fixture.claims.push(
        claim(
            ClaimKind::Contract,
            "orders.patch-id-lines",
            ("route", "PATCH /api/orders/{id}/lines"),
        )
        .at("src/routes/orders.ts#L47-L53"),
    );
    let grouping = r#"{"groups": [
        {"claims": [0, 5], "classes": [[0, 5]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2, 6], "classes": [[2, 6]]},
        {"claims": [3, 7], "classes": [[3, 7]]},
        {"claims": [8], "classes": [[8]]},
        {"claims": [9], "classes": [[9]]},
        {"claims": [4, 10], "classes": [[4, 10]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.patch-id-lines.reprice-and-persist", "scenarios": [
            {"name": "Patch lines", "when": "a caller patches lines on a pending order", "then": "lines and totals are updated"}
        ]}
    ]}"#;
    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Customers and orders are exposed over HTTP, including line patches (REQ-001, REQ-002, REQ-003, REQ-049)."}], "revised": [1]},
        {"kind": "apis", "blocks": [{"text": "Inbound HTTP exposes POST /api/customers (REQ-002), POST /api/orders (REQ-003), and PATCH /api/orders/{id}/lines (REQ-049)."}], "revised": [1]},
        {"kind": "technical-logic", "blocks": [{"text": "Order reads check the cache under order:{id} before repricing lines (REQ-049)."}], "revised": []}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft, design]);
    fixture.seed_into(&mut provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &whole_config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 1, "retired": 2, "conflict": 0, "unchanged": 3, "by-scope": 2
        }),
        "{envelope}"
    );
    let (_, amendment) = staged_amendment(&provider);
    let req_049 = amended_requirement(&amendment, "REQ-049");
    assert_eq!(
        req_049["trigger"],
        serde_json::json!({"http": {"method": "PATCH", "path": "/api/orders/{id}/lines"}}),
        "{req_049}"
    );
    provider.model.assert_exhausted();
}

fn amended_requirement(amendment: &Value, req_id: &str) -> Value {
    amendment["spec"]["requirements"]
        .as_array()
        .expect("requirements")
        .iter()
        .find(|req| req["id"] == req_id)
        .unwrap_or_else(|| panic!("{req_id} stands revised"))
        .clone()
}

// On an untouched live route (`GET /api/orders/{id}`) whose return-order
// standing requirement was read in a thin route wrapper (`src/routes/orders.ts`)
// while re-extracted claims anchored in `src/services/orders.ts`, `## Baseline`
// emits a cross-file route hint and placing `orders.get-id.load-and-cache` in
// both `REQ-034` and `REQ-036` (or evidencing `REQ-034` through stem detail on
// the live route) keeps every standing requirement unchanged.
#[tokio::test]
async fn amend_scoped_live_route_cross_layer() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let mut fixture = ScopedFixture::live_route_cross_layer();

    // 0..=3 are `code`'s claims; 4..=7 are `REQ-001`, `REQ-034`, `REQ-035`, `REQ-036`
    let multi_grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [3, 5], "classes": [[3, 5]]},
        {"claims": [2, 6], "classes": [[2, 6]]},
        {"claims": [1, 3, 7], "classes": [[1, 3, 7]]}
    ]}"#;
    let unchanged_apis_design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}], "revised": []}},
            {{"kind": "apis", "blocks": [{{"text": "Inbound HTTP exposes GET /api/orders/{{id}} (REQ-034, REQ-035, REQ-036)."}}], "revised": []}}
        ]}}"#
    );
    let mut provider = Provider::answering([multi_grouping, unchanged_apis_design.as_str()]);
    fixture.seed_into(&mut provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["staged"], false, "all standing requirements unchanged: {envelope}");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 0, "conflict": 0, "unchanged": 4, "by-scope": 0
        }),
        "{envelope}"
    );
    let grouping_turn = provider.model.seen()[0].messages.join("\n");
    let cross_file_hint = "- claims 1, 2, 3 share route `GET /api/orders/{id}` with standing \
                           requirements 5, 6, 7 across files";
    assert!(grouping_turn.contains(cross_file_hint), "{grouping_turn}");
    provider.model.assert_exhausted();

    // When `REQ-034` stands alone on a live route and `src/services/orders.ts`
    // holds stem `Excerpt` detail while `src/routes/orders.ts` holds none,
    // `evidenced` is accepted by both `verify_evidenced` and `admits_evidenced`.
    let evidenced_grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [5], "classes": [[5]]},
        {"claims": [2, 6], "classes": [[2, 6]]},
        {"claims": [1, 3, 7], "classes": [[1, 3, 7]]}
    ], "evidenced": [5]}"#;
    let design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}], "revised": []}},
            {{"kind": "apis", "blocks": [{{"text": "Inbound HTTP exposes GET /api/orders/{{id}} (REQ-034, REQ-035, REQ-036)."}}], "revised": []}},
            {{"kind": "technical-logic", "blocks": [{{"text": "Orders are cached under order:{{id}} on miss (REQ-034, REQ-036)."}}], "revised": []}}
        ]}}"#
    );
    let mut provider = Provider::answering([evidenced_grouping, design.as_str()]);
    fixture.claims.push(
        Claim::new(ClaimKind::Excerpt)
            .at("src/services/orders.ts#L23-L33")
            .with_extra("excerpt", "Order lookup checks the cache and returns the order."),
    );
    fixture.seed_into(&mut provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 0, "conflict": 0, "unchanged": 4, "by-scope": 0
        }),
        "{envelope}"
    );
    provider.model.assert_exhausted();
}

struct ScopedFixture {
    spec: Vec<u8>,
    design: String,
    plan: Vec<u8>,
    claims: Vec<Claim>,
}

impl ScopedFixture {
    fn seed_into(&self, provider: &mut Provider) {
        code_with(provider, self.claims.clone());
        seed(&provider.files, &self.spec, self.design.as_bytes(), &self.plan);
    }

    fn live_route_cross_layer() -> Self {
        let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
        let tpl = base_spec["requirements"][2].clone();
        base_spec["requirements"] = serde_json::json!([
            SpecReq::new(
                "REQ-001",
                "start.json-body-limit",
                "JSON request bodies exceeding 256 kilobytes are rejected.",
            )
            .at("src/server.ts#L28")
            .into_json(&tpl),
            SpecReq::new(
                "REQ-034",
                "orders.get-id.return-order",
                "A get-by-id request returns the order for the path id when it exists.",
            )
            .at("src/routes/orders.ts#L29-L31")
            .route("GET /api/orders/{id}")
            .into_json(&tpl),
            SpecReq::new(
                "REQ-035",
                "orders.get-id.not-found",
                "A get-by-id request is rejected when no order exists for the given id.",
            )
            .at("src/services/orders.ts#L29-L31")
            .route("GET /api/orders/{id}")
            .into_json(&tpl),
            SpecReq::new(
                "REQ-036",
                "orders.get-id.cache-hit",
                "A get-by-id request returns the cached order when one is held for that id.",
            )
            .at("src/services/orders.ts#L25-L33")
            .route("GET /api/orders/{id}")
            .into_json(&tpl),
        ]);
        let mut base_plan: Value = serde_json::from_slice(PLAN).expect("base plan JSON");
        base_plan["parts"][0]["name"] = Value::from("start");
        base_plan["parts"][0]["requirements"] = serde_json::json!(["REQ-001"]);
        base_plan["parts"][1]["requirements"] =
            serde_json::json!(["REQ-034", "REQ-035", "REQ-036"]);
        let design = format!(
            r#"{{"emery": 10, "parent": null, "preamble": [], "sections": [
                {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}]}},
                {DOMAIN_MODEL},
                {{"kind": "apis", "blocks": [{{"text": "Inbound HTTP exposes GET /api/orders/{{id}} (REQ-034, REQ-035, REQ-036)."}}]}},
                {{"kind": "technical-logic", "blocks": [{{"text": "Orders are cached under order:{{id}} on miss (REQ-034, REQ-036)."}}]}}
            ]}}"#
        );
        let claims = vec![
            requirement(
                "start.json-body-limit",
                "JSON request bodies exceeding 256 kilobytes are rejected.",
            )
            .at("src/server.ts#L28"),
            requirement(
                "orders.get-id.serve-from-cache",
                "When a cached copy of the order exists, that order is returned without reading the store.",
            )
            .at("src/services/orders.ts#L25-L27"),
            requirement(
                "orders.get-id.reject-missing",
                "A request for an order that does not exist in the store is rejected.",
            )
            .at("src/services/orders.ts#L29-L31"),
            requirement(
                "orders.get-id.load-and-cache",
                "When no cached copy exists, the order is read from the store, cached, then returned.",
            )
            .at("src/services/orders.ts#L28-L33"),
            claim(ClaimKind::Contract, "orders.get-id", ("route", "GET /api/orders/{id}"))
                .at("src/routes/orders.ts#L27-L33"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        ];
        Self {
            spec: serde_json::to_vec(&base_spec).expect("serialise base spec"),
            design,
            plan: serde_json::to_vec(&base_plan).expect("serialise base plan"),
            claims,
        }
    }

    fn vanished_route() -> Self {
        let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
        let tpl = base_spec["requirements"][2].clone();
        base_spec["requirements"] = serde_json::json!([
            SpecReq::new(
                "REQ-001",
                "start.json-body-limit",
                "JSON request bodies exceeding 256 kilobytes are rejected.",
            )
            .at("src/server.ts#L28")
            .into_json(&tpl),
            SpecReq::new(
                "REQ-002",
                "customers.post.create",
                "A valid customer request persists the customer.",
            )
            .at("src/routes/customers.ts#L18-L31")
            .route("POST /api/customers")
            .into_json(&tpl),
            SpecReq::new(
                "REQ-003",
                "orders.post.create",
                "A valid order request creates a pending order.",
            )
            .at("src/routes/orders.ts#L30-L42")
            .route("POST /api/orders")
            .into_json(&tpl),
            SpecReq::new(
                "REQ-047",
                "orders.put-id-lines.body-validation",
                "An amend-lines request rejects an invalid line list.",
            )
            .at("src/routes/orders.ts#L47")
            .route("PUT /api/orders/{id}/lines")
            .into_json(&tpl),
            SpecReq::new(
                "REQ-048",
                "orders.put-id-lines.pending-only",
                "An amend-lines request is rejected when the order is not pending.",
            )
            .at("src/services/orders.ts#L64-L66")
            .route("PUT /api/orders/{id}/lines")
            .into_json(&tpl),
            SpecReq::new(
                "REQ-049",
                "orders.put-id-lines.reprice-and-persist",
                "A valid amend-lines request replaces lines and recomputes totals.",
            )
            .at("src/services/orders.ts#L67-L78")
            .route("PUT /api/orders/{id}/lines")
            .into_json(&tpl),
        ]);
        let mut base_plan: Value = serde_json::from_slice(PLAN).expect("base plan JSON");
        base_plan["parts"][0]["name"] = Value::from("start");
        base_plan["parts"][0]["requirements"] = serde_json::json!(["REQ-001", "REQ-002"]);
        base_plan["parts"][1]["requirements"] =
            serde_json::json!(["REQ-003", "REQ-047", "REQ-048", "REQ-049"]);

        let design = format!(
            r#"{{"emery": 10, "parent": null, "preamble": [], "sections": [
                {{"kind": "overview", "blocks": [{{"text": "Customers and orders are exposed over HTTP, including line amendments (REQ-001, REQ-002, REQ-003, REQ-047, REQ-048, REQ-049)."}}]}},
                {DOMAIN_MODEL},
                {{"kind": "apis", "blocks": [{{"text": "Inbound HTTP exposes POST /api/customers (REQ-002), POST /api/orders (REQ-003), and PUT /api/orders/{{id}}/lines (REQ-047, REQ-048, REQ-049)."}}]}}
            ]}}"#
        );
        Self {
            spec: serde_json::to_vec(&base_spec).expect("serialise base spec"),
            design,
            plan: serde_json::to_vec(&base_plan).expect("serialise base plan"),
            claims: vanished_route_claims(),
        }
    }
}

struct SpecReq {
    id: String,
    subject: String,
    source: String,
    path: Option<String>,
    trigger: Option<Trigger>,
    body: String,
}

impl SpecReq {
    fn new(id: &str, subject: &str, body: &str) -> Self {
        Self {
            id: id.to_owned(),
            subject: subject.to_owned(),
            source: "code".to_owned(),
            path: None,
            trigger: None,
            body: body.to_owned(),
        }
    }

    fn source(mut self, source: &str) -> Self {
        self.source = source.to_owned();
        self
    }

    fn at(mut self, path: &str) -> Self {
        self.path = Some(path.to_owned());
        self
    }

    fn route(mut self, route: &str) -> Self {
        self.trigger = Some(Trigger::Http(Route::parse(route).expect("valid route")));
        self
    }

    fn into_json(self, template: &Value) -> Value {
        let mut entry = template.clone();
        entry["id"] = Value::from(self.id);
        entry["subject"] = Value::from(self.subject.as_str());
        entry["trigger"] = serde_json::to_value(self.trigger).expect("serialise trigger");
        entry["sources"] = serde_json::json!([{
            "source": self.source,
            "claim": self.subject,
            "path": self.path,
        }]);
        entry["body"] = serde_json::json!([self.body]);
        entry
    }
}

fn vanished_route_claims() -> Vec<Claim> {
    vec![
        requirement(
            "start.json-body-limit",
            "JSON request bodies exceeding 256 kilobytes are rejected.",
        )
        .at("src/server.ts#L28"),
        requirement("start.omit-powered-by", "Responses omit an X-Powered-By header.")
            .at("src/server.ts#L31"),
        requirement("customers.post.create", "A valid customer request persists the customer.")
            .at("src/routes/customers.ts#L18-L31"),
        requirement("orders.post.create", "A valid order request creates a pending order.")
            .at("src/routes/orders.ts#L30-L42"),
        claim(ClaimKind::Contract, "customers.post", ("route", "POST /api/customers"))
            .at("src/routes/customers.ts#L18-L31"),
        claim(ClaimKind::Contract, "orders.post", ("route", "POST /api/orders"))
            .at("src/routes/orders.ts#L30-L42"),
        claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
        claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        Claim::new(ClaimKind::Call).at("src/server.ts#L44").with_extra("callee", "pg:Pool.query"),
        Claim::new(ClaimKind::Call)
            .at("src/repositories/orders.ts#L94")
            .with_extra("callee", "pg:Pool.query"),
        Claim::new(ClaimKind::Excerpt)
            .at("src/routes/orders.ts#L15-L25")
            .with_extra("excerpt", "Order routes validate UUID path parameters."),
        Claim::new(ClaimKind::Excerpt)
            .at("src/services/orders.ts#L23-L33")
            .with_extra("excerpt", "Order reads check the cache under order:{id}."),
    ]
}

fn bootstrap_spec_and_plan() -> (Vec<u8>, Vec<u8>) {
    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
    let tpl = base_spec["requirements"][0].clone();
    base_spec["requirements"] = serde_json::json!([
        SpecReq::new(
            "REQ-001",
            "start.json-body-limit",
            "JSON request bodies exceeding 256 kilobytes are rejected.",
        )
        .at("src/server.ts#L28")
        .into_json(&tpl),
        SpecReq::new("REQ-002", "session.timeout", "Sessions expire after an hour of inactivity.",)
            .at("src/session.ts#L10-L14")
            .into_json(&tpl),
        SpecReq::new("REQ-003", "orders.create", "A signed-in user places an order.",)
            .at("src/orders.ts#L20-L35")
            .into_json(&tpl),
        SpecReq::new(
            "REQ-004",
            "orders.audit-note",
            "Order mutations record an operator audit note.",
        )
        .at("src/audit.ts#L8-L12")
        .into_json(&tpl),
    ]);
    let mut base_plan: Value = serde_json::from_slice(PLAN).expect("base plan JSON");
    base_plan["parts"][0]["name"] = Value::from("start");
    (
        serde_json::to_vec(&base_spec).expect("serialise base spec"),
        serde_json::to_vec(&base_plan).expect("serialise base plan"),
    )
}

// Listing an untriggered standing requirement in `evidenced` is refused when
// none of `## Whole-source detail` is anchored in a file its citations name.
#[tokio::test]
async fn amend_scoped_evidenced_requires_cited_file() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let (spec_bytes, plan_bytes) = bootstrap_spec_and_plan();

    let invalid = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2, 5], "classes": [[2, 5]]},
        {"claims": [3, 6], "classes": [[3, 6]]},
        {"claims": [7], "classes": [[7]]}
    ], "evidenced": [7]}"#;
    let valid = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2, 5], "classes": [[2, 5]]},
        {"claims": [3, 6], "classes": [[3, 6]]},
        {"claims": [7], "classes": [[7]]}
    ]}"#;
    let mut provider = Provider::answering([invalid, valid]);
    code_with(
        &mut provider,
        vec![
            requirement(
                "start.json-body-limit",
                "JSON request bodies exceeding 256 kilobytes are rejected.",
            )
            .at("src/server.ts#L28"),
            requirement("start.omit-powered-by", "Responses omit an X-Powered-By header.")
                .at("src/server.ts#L31"),
            requirement("session.timeout", "Sessions expire after an hour of inactivity.")
                .at("src/session.ts#L10-L14"),
            requirement("orders.create", "A signed-in user places an order.")
                .at("src/orders.ts#L20-L35"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
            Claim::new(ClaimKind::Excerpt)
                .at("src/orders.ts#L5-L12")
                .with_extra("excerpt", "Order creation normalises SKU casing."),
        ],
    );
    seed(&provider.files, &spec_bytes, DESIGN, &plan_bytes);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 1, "conflict": 0, "unchanged": 3, "by-scope": 1
        }),
        "{envelope}"
    );
    let exchanges = provider.model.exchanges();
    let finding = exchanges[0].outcome.as_ref().expect_err("uncited file in `evidenced` rejected");
    assert!(
        finding.contains(
            "standing requirement 7 (`orders.audit-note`) has no `## Whole-source detail` in any \
             file its citations name"
        ),
        "{finding}"
    );
    provider.model.assert_exhausted();
}

// On `start.*` bootstrap files under `scope = "whole"`, re-extraction noise on
// an untouched file stages nothing while an added `start.*` claim in a newly
// added file is admitted.
#[tokio::test]
async fn amend_scoped_bootstrap_guard() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let (spec_bytes, plan_bytes) = bootstrap_spec_and_plan();

    let grouping = r#"{"groups": [
        {"claims": [0, 5], "classes": [[0, 5]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2, 6], "classes": [[2, 6]]},
        {"claims": [3, 7], "classes": [[3, 7]]},
        {"claims": [4, 8], "classes": [[4, 8]]}
    ]}"#;
    let mut provider = Provider::answering([grouping]);
    code_with(
        &mut provider,
        vec![
            requirement(
                "start.json-body-limit",
                "JSON request bodies exceeding 256 kilobytes are rejected.",
            )
            .at("src/server.ts#L28"),
            requirement("start.omit-powered-by", "Responses omit an X-Powered-By header.")
                .at("src/server.ts#L31"),
            requirement("session.timeout", "Sessions expire after an hour of inactivity.")
                .at("src/session.ts#L10-L14"),
            requirement("orders.create", "A signed-in user places an order.")
                .at("src/orders.ts#L20-L35"),
            requirement("orders.audit-note", "Order mutations record an operator audit note.")
                .at("src/audit.ts#L8-L12"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        ],
    );
    seed(&provider.files, &spec_bytes, DESIGN, &plan_bytes);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["staged"], false, "nothing staged on untouched bootstrap: {envelope}");
    provider.model.assert_exhausted();

    let bootstrap_draft = r#"{"preamble": [], "requirements": [
        {"subject": "start.require-otlp-url", "scenarios": [
            {"name": "Missing OTLP URL", "when": "OTLP_URL is unset at startup", "then": "startup fails"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, bootstrap_draft]);
    code_with(
        &mut provider,
        vec![
            requirement(
                "start.json-body-limit",
                "JSON request bodies exceeding 256 kilobytes are rejected.",
            )
            .at("src/server.ts#L28"),
            requirement(
                "start.require-otlp-url",
                "The program refuses to start when OTLP_URL is unset.",
            )
            .at("src/telemetry.ts#L4-L8"),
            requirement("session.timeout", "Sessions expire after an hour of inactivity.")
                .at("src/session.ts#L10-L14"),
            requirement("orders.create", "A signed-in user places an order.")
                .at("src/orders.ts#L20-L35"),
            requirement("orders.audit-note", "Order mutations record an operator audit note.")
                .at("src/audit.ts#L8-L12"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        ],
    );
    seed(&provider.files, &spec_bytes, DESIGN, &plan_bytes);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 1, "changed": 0, "retired": 0, "conflict": 0, "unchanged": 4, "by-scope": 0
        }),
        "{envelope}"
    );
    provider.model.assert_exhausted();
}

// Revising `orders.post.discount-rate` in `src/services/pricing.ts` under
// `scope = "whole"` while untouched `src/lib/shipping.ts` (same `orders` stem)
// carries an HTTP `fetch` `call` claim and `src/routes/orders.ts` carries the
// standing `POST /api/orders` contract scopes `DesignAmend` to `pricing.ts`
// alone: `## APIs and integrations` is not marked informed, preserving a prior
// `intent` `x-api-key` decision paragraph byte for byte without conflict.
#[tokio::test]
async fn amend_scoped_drafted_file_isolates_design() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let fixture = isolated_design_fixture();

    // 0..=3 are `code`'s requirement claims; 4..=7 the standing requirements
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1, 5], "classes": [[1, 5]]},
        {"claims": [2, 6], "classes": [[2, 6]]},
        {"claims": [3, 7], "classes": [[3], [7]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.post.discount-rate", "scenarios": [
            {"name": "Gold discount at 10 units", "when": "a gold-tier order has 10 units", "then": "the 10 percent discount applies"}
        ]}
    ]}"#;
    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Orders fetch shipping quotes and apply tier discounts from 10 units (REQ-003, REQ-004)."}], "revised": [1]},
        {"kind": "technical-logic", "blocks": [{"text": "Gold-tier orders of 10 or more units receive a 10 percent discount before shipping (REQ-004)."}], "revised": [1]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft, design]);
    code_with(&mut provider, fixture.claims);
    seed(&provider.files, &fixture.spec, fixture.design.as_bytes(), &fixture.plan);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 1, "retired": 0, "conflict": 0, "unchanged": 3, "by-scope": 0
        }),
        "{envelope}"
    );
    assert_eq!(
        envelope["sections"],
        serde_json::json!({
            "preamble": false,
            "added": [],
            "removed": [],
            "changed": ["overview", "technical-logic"]
        }),
        "`apis` is untouched: {envelope}"
    );

    let design_turn = provider.model.seen()[2].messages.join("\n");
    assert!(!design_turn.contains("### APIs and integrations"), "{design_turn}");
    assert!(!design_turn.contains("src/lib/shipping.ts"), "{design_turn}");
    assert!(!design_turn.contains("Bearer"), "{design_turn}");
    assert!(design_turn.contains("DISCOUNT_THRESHOLD is 10 units"), "{design_turn}");

    let base_design: Value = serde_json::from_str(&fixture.design).expect("base design JSON");
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(
        amendment["design"]["sections"][2], base_design["sections"][2],
        "`apis` with `x-api-key` preserved byte for byte: {amendment}"
    );
    provider.model.assert_exhausted();
}

fn isolated_design_fixture() -> ScopedFixture {
    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
    let tpl = base_spec["requirements"][2].clone();
    base_spec["requirements"] = serde_json::json!([
        SpecReq::new(
            "REQ-001",
            "start.require-database-url",
            "The program refuses to start when DATABASE_URL is unset.",
        )
        .at("src/config.ts#L5-L7")
        .into_json(&tpl),
        SpecReq::new(
            "REQ-002",
            "start.upstream-error",
            "Upstream dependency failures return 502 upstream-unavailable.",
        )
        .at("src/server.ts#L50-L55")
        .into_json(&tpl),
        SpecReq::new(
            "REQ-003",
            "orders.post.shipping-quote",
            "Placing an order fetches a shipping quote for the destination postcode.",
        )
        .at("src/lib/shipping.ts#L15-L35")
        .route("POST /api/orders")
        .into_json(&tpl),
        SpecReq::new(
            "REQ-004",
            "orders.post.discount-rate",
            "Gold-tier orders of 20 or more units receive a 10 percent discount.",
        )
        .source("intent")
        .route("POST /api/orders")
        .into_json(&tpl),
    ]);
    let mut base_plan: Value = serde_json::from_slice(PLAN).expect("base plan JSON");
    base_plan["parts"][0]["name"] = Value::from("start");

    let design = format!(
        r#"{{"emery": 10, "parent": null, "preamble": [], "sections": [
            {{"kind": "overview", "blocks": [{{"text": "Orders fetch shipping quotes and apply tier discounts from 20 units (REQ-003, REQ-004)."}}]}},
            {DOMAIN_MODEL},
            {{"kind": "apis", "blocks": [
                {{"text": "Inbound HTTP exposes POST /api/orders (REQ-003, REQ-004)."}},
                {{"text": "Upstream delivery-rates calls GET ${{SHIPPING_API_URL}}/rates with header x-api-key: ${{SHIPPING_API_KEY}} (from intent) and surfaces failures as 502 upstream-unavailable (REQ-002, REQ-003)."}}
            ]}},
            {{"kind": "technical-logic", "blocks": [
                {{"text": "Gold-tier orders of 20 or more units receive a 10 percent discount before shipping (REQ-004)."}}
            ]}}
        ]}}"#
    );
    let claims = vec![
        requirement(
            "start.require-database-url",
            "The program refuses to start when DATABASE_URL is unset.",
        )
        .at("src/config.ts#L5-L7"),
        requirement(
            "start.upstream-error",
            "Upstream dependency failures return 502 upstream-unavailable.",
        )
        .at("src/server.ts#L50-L55"),
        requirement(
            "orders.post.shipping-quote",
            "Placing an order fetches a shipping quote for the destination postcode.",
        )
        .at("src/lib/shipping.ts#L15-L35"),
        requirement(
            "orders.post.discount-rate",
            "Gold-tier orders of 10 or more units receive a 10 percent discount.",
        )
        .at("src/services/pricing.ts#L15-L21"),
        claim(ClaimKind::Contract, "orders.post", ("route", "POST /api/orders"))
            .at("src/routes/orders.ts#L30-L42"),
        claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
        claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        Claim::new(ClaimKind::Call)
            .at("src/lib/shipping.ts#L22")
            .with_extra("callee", "fetch")
            .with_synopsis(
                "GET ${SHIPPING_API_URL}/rates with Authorization: Bearer ${SHIPPING_API_KEY}",
            ),
        Claim::new(ClaimKind::Excerpt).at("src/services/pricing.ts#L12-L24").with_extra(
            "excerpt",
            "DISCOUNT_THRESHOLD is 10 units for gold-tier 10 percent discount.",
        ),
    ];

    ScopedFixture {
        spec: serde_json::to_vec(&base_spec).expect("serialise base spec"),
        design,
        plan: serde_json::to_vec(&base_plan).expect("serialise base plan"),
        claims,
    }
}

// `SPEC` with `auth.login` citing `login_source`, `session.timeout` citing
// both `docs` and `code`, and `orders.*` citing `code` alone.
fn spec_citing_code(login_source: &str) -> Vec<u8> {
    let mut spec: Value = serde_json::from_slice(SPEC).expect("the base spec is JSON");
    spec["requirements"][0]["sources"] = serde_json::json!([
        {"source": login_source, "claim": "auth.login", "path": null}
    ]);
    spec["requirements"][1]["sources"] = serde_json::json!([
        {"source": "docs", "claim": "session.timeout", "path": null},
        {"source": "code", "claim": "session.timeout", "path": null}
    ]);
    for index in [2, 3] {
        spec["requirements"][index]["sources"][0]["source"] = Value::from("code");
    }
    serde_json::to_vec(&spec).expect("serialise base spec")
}

// The grouping over `code`'s one claim, `orders.create`, restating the
// standing requirement of that id: index 0 is `code`, 1..=4 the standing
// requirements in id order.
const SCOPED_GROUPING: &str = r#"{"groups": [
    {"claims": [0, 3], "classes": [[0, 3]]},
    {"claims": [1], "classes": [[1]]},
    {"claims": [2], "classes": [[2]]},
    {"claims": [4], "classes": [[4]]}
]}"#;

// A behaviour source `code`, its one claim restating `orders.create`.
fn code(provider: &mut Provider) {
    code_with(provider, vec![requirement("orders.create", "A signed-in user places an order.")]);
}

fn code_with(provider: &mut Provider, claims: Vec<Claim>) {
    provider.source.kinds.insert("acme:code".to_owned(), SourceKind::Behaviour);
    provider.source.evidence.insert("code".to_owned(), Ok(evidence(claims)));
}

// A change file naming one source read under `scope`, over the project
// checkout as its target.
fn scoped(scratch: &Scratch, adapter: &str, scope: &str) -> String {
    scratch.config(&format!(
        "[[source]]\nadapter = \"{adapter}\"\nscope = \"{scope}\"\n\n[target]\nadapter = \
         \"acme:builder@2.1.0\"\nrepository = \".\"\n"
    ))
}

// A change claim anchored where a standing requirement was read, from the
// same source, is hinted to revise it in the grouping request: a hint, not
// a pre-merge, so the answer is held to no group for it. Here the model
// takes the hint, and the requirement keeps its id under the claim's
// subject.
#[tokio::test]
async fn amend_anchored() {
    let base = String::from_utf8_lossy(SPEC).replace(
        "\"claim\": \"orders.cancel\",\n          \"path\": null",
        "\"claim\": \"orders.cancel\",\n          \"path\": \"docs/orders.md#L20-L30\"",
    );
    // index 0 is `docs`, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0], [4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.cancellation", "scenarios": [
            {"name": "Cancel", "when": "a signed-in caller cancels an order within an hour", "then": "the order is cancelled"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    let mut anchored = requirement(
        "orders.cancellation",
        "A signed-in user cancels an open order within an hour.",
    );
    anchored.path = Some("docs/orders.md#L25-L40".to_owned());
    provider.source.evidence.insert("docs".to_owned(), Ok(evidence(vec![anchored])));
    seed(&provider.files, base.as_bytes(), DESIGN, PLAN);

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let request = provider.model.seen()[0].messages.join("\n");
    assert!(
        request.contains(
            "## Baseline\n\nNo two claims share an id; every grouping is your judgement.\n- claim \
             0 anchors at `docs/orders.md#L25-L40`, where standing requirement 4 was read; they \
             are one requirement unless the code moved.\n"
        ),
        "{request}"
    );
    let (_, amendment) = staged_amendment(&provider);
    let changed = &amendment["changes"][3];
    assert_eq!(changed["id"], "REQ-004", "{amendment}");
    assert_eq!(changed["subject"], "orders.cancellation", "{amendment}");
    assert_eq!(changed["class"], "changed", "{amendment}");
    assert_eq!(changed["sources"][0]["path"], "docs/orders.md#L25-L40", "{amendment}");
    provider.model.assert_exhausted();
}

// A change restating a standing requirement under another stem keeps its id
// and takes the new subject, so the requirement is changed and moves part:
// it leaves the part of its old stem, which shrinks and keeps its types,
// for a part the slicing turn drafts over the current plan, numbered after
// the base's, with the base's trigger carried. The turn's brief lists the
// current parts fixed, the old one touched and its brief to redraft, the
// open stem with its trigger, and the requirement it places as drafted; the
// redrafted brief stands in the shrunk part, which the amendment lists as
// changed with its brief redrafted.
#[tokio::test]
async fn amend_moved() {
    let slicing = r#"{"parts": [
        {"name": "reporting", "requirements": ["REQ-004"], "types": [], "uses": ["orders.order"],
         "brief": ["Reports a cancellation; edits the cancel handler, so it compiles against `orders`."]}
    ], "joins": [], "briefs": [
        {"part": "PART-002", "brief": ["Delivers order creation for a signed-in caller; verified by placing an order under a session from `authentication`."]}
    ]}"#;
    let (provider, revision) = moved([slicing]);
    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 1 changed, 0 retired, 0 \
             conflict, 3 unchanged\n  stems: reporting\n  parts: orders, reporting (new)\n  \
             nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 3, "the grouping, the draft, and the slicing");
    let draft = provider.model.seen()[1].messages.join("\n");
    assert!(draft.contains("- REQ-004 `reporting.cancel`"), "{draft}");

    // the slicing turn: the current parts fixed, the old one touched and
    // its brief asked for, the open stem with the trigger it carries, and
    // the requirement it places; the schema asks one brief for the touched
    // part and at most one per current part
    let turn = &provider.model.seen()[2];
    let request = turn.messages.join("\n");
    assert!(request.starts_with("Amend the build plan"), "{request}");
    assert!(
        request.contains(
            "## Current plan\n\n- `PART-001` `authentication` — REQ-001, REQ-002\n- `PART-002` \
             `orders` — REQ-003 — types: `orders.line`, `orders.order` — touched by this \
             amendment\n\nA part marked touched"
        ),
        "{request}"
    );
    assert!(request.contains("Every other part keeps its brief.\n\n## Open\n"), "{request}");
    assert!(
        request.contains("- `reporting` — REQ-004 — triggers: `orders.cancel-requested`\n"),
        "{request}"
    );
    assert!(!request.contains("Type keys the design holds"), "{request}");
    assert!(request.contains("## Requirements\n\nThe requirements the change adds"), "{request}");
    assert!(request.contains("- REQ-004 `reporting.cancel` — Status: unknown"), "{request}");
    let SeenFormat::Schema { schema, .. } = &turn.format else {
        panic!("the slicing is steered by schema");
    };
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["briefs"]["minItems"], 1, "{schema}");
    assert_eq!(schema["properties"]["briefs"]["maxItems"], 2, "{schema}");
    assert_eq!(
        schema["$defs"]["Rebrief"]["properties"]["part"]["enum"],
        serde_json::json!(["PART-001", "PART-002"])
    );
    assert_eq!(schema["$defs"]["Rebrief"]["properties"]["brief"]["minItems"], 1, "{schema}");

    // the change keeps the id under the new subject, with the base's trigger
    let moved = &amendment["changes"][3];
    assert_eq!(
        (&moved["id"], &moved["subject"], &moved["class"]),
        (
            &serde_json::json!("REQ-004"),
            &serde_json::json!("reporting.cancel"),
            &serde_json::json!("changed")
        )
    );
    let drafted = &amendment["spec"]["requirements"][3];
    assert_eq!(drafted["subject"], "reporting.cancel", "{amendment}");
    assert_eq!(drafted["trigger"], serde_json::json!({"message": "orders.cancel-requested"}));

    // the plan: the old part shrunk with its types and its redrafted brief,
    // the new one after it, and no note on a redrafted brief
    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    let parts = amendment["plan"]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 3, "{amendment}");
    assert_eq!(parts[0], base["parts"][0], "{amendment}");
    assert_eq!(parts[1]["requirements"], serde_json::json!(["REQ-003"]), "{amendment}");
    assert_eq!(parts[1]["types"], base["parts"][1]["types"], "{amendment}");
    assert_eq!(
        parts[2],
        serde_json::json!({
            "id": "PART-003", "name": "reporting", "requirements": ["REQ-004"],
            "types": [], "uses": ["orders.order"], "depends-on": ["PART-002"],
            "brief": ["Reports a cancellation; edits the cancel handler, so it compiles against `orders`."]
        }),
        "{amendment}"
    );
    assert_eq!(
        amendment["parts"],
        serde_json::json!([
            {"id": "PART-002", "name": "orders", "class": "changed", "brief": "redrafted"},
            {"id": "PART-003", "name": "reporting", "class": "added", "brief": "redrafted"}
        ]),
        "{amendment}"
    );
    assert!(amendment["sections"]["changed"].as_array().is_some_and(Vec::is_empty), "{amendment}");
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.contains(
            "### Part: orders [changed]\n\nID: PART-002\nRequirements: [REQ-003]\nTypes: \
             [orders.line, orders.order]\n\nDelivers order creation for a signed-in caller; \
             verified by placing an order under a session from `authentication`.\n\n### Part: \
             reporting [added]"
        ),
        "{shown}"
    );
    provider.model.assert_exhausted();
}

// The slicing turn's `briefs` are held to the parts this amendment touched
// or its joins name: a brief for a part that stands as it was, one for no
// current part, one listed twice, an empty one, and a touched part left
// without one are each refused and corrected.
#[tokio::test]
async fn untouched_rebriefed() {
    let placed = r#""parts": [
        {"name": "reporting", "requirements": ["REQ-004"], "types": [], "uses": ["orders.order"],
         "brief": ["Reports a cancellation."]}
    ], "joins": []"#;
    let refused = format!(
        r#"{{{placed}, "briefs": [
            {{"part": "PART-001", "brief": ["Signs a caller in."]}},
            {{"part": "PART-009", "brief": ["Nothing."]}},
            {{"part": "PART-002", "brief": []}},
            {{"part": "PART-002", "brief": ["Delivers order creation."]}}
        ]}}"#
    );
    let missing = format!(r#"{{{placed}, "briefs": []}}"#);
    let corrected = format!(
        r#"{{{placed}, "briefs": [{{"part": "PART-002", "brief": ["Delivers order creation."]}}]}}"#
    );
    let (provider, _) = moved([refused.as_str(), missing.as_str(), corrected.as_str()]);

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let exchanges = provider.model.exchanges();
    assert_eq!(exchanges.len(), 5, "the grouping, the draft, and three slicing rounds");
    let finding = exchanges[2].outcome.as_ref().expect_err("the first answer is refused");
    for expected in [
        "`briefs` `PART-001`: `authentication` stands as it was; it keeps its brief, so list \
         no entry for it",
        "`briefs` `PART-009` names no part of the current plan",
        "`briefs` `PART-002` is listed more than once; redraft it once",
        "`briefs` `PART-002`: the brief is empty; redraft `orders`'s brief whole",
    ] {
        assert!(finding.contains(expected), "{expected}: {finding}");
    }
    let finding = exchanges[3].outcome.as_ref().expect_err("the second answer is refused");
    assert!(
        finding.contains(
            "`orders` (PART-002) is touched by this amendment, or joined by the answer, and its \
             brief is not redrafted; list it under `briefs`"
        ),
        "{finding}"
    );
    assert!(exchanges[4].outcome.is_ok(), "{:?}", exchanges[4].outcome);
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(
        amendment["plan"]["parts"][1]["brief"],
        serde_json::json!(["Delivers order creation."]),
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

// A documentation source restating the standing `orders.cancel` under
// `reporting`, over a scripted grouping and draft, with `slicing` queued for
// the slicing turn.
fn moved<const N: usize>(slicing: [&str; N]) -> (Provider, String) {
    // index 0 is `docs`, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "reporting.cancel", "scenarios": [
            {"name": "Cancel", "when": "a signed-in caller cancels an order", "then": "the cancellation is reported"}
        ]}
    ]}"#;
    let mut answers = vec![grouping, draft];
    answers.extend(slicing);
    let mut provider = Provider::answering(answers);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![requirement(
            "reporting.cancel",
            "A signed-in user cancels an open order.",
        )])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);
    (provider, revision)
}

// A change restating every requirement of a part under another stem empties
// the part, which is removed, and the types it owned are open with the new
// stem: the slicing turn's brief lists them, the new part owns them, and
// the design stands.
#[tokio::test]
async fn amend_moved_part() {
    // indices 0 and 1 are `docs`, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1, 5], "classes": [[1, 5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "reporting.create", "scenarios": [
            {"name": "Create", "when": "a signed-in caller places an order", "then": "the order is reported"}
        ]},
        {"subject": "reporting.cancel", "scenarios": [
            {"name": "Cancel", "when": "a signed-in caller cancels an order", "then": "the cancellation is reported"}
        ]}
    ]}"#;
    let slicing = r#"{"parts": [
        {"name": "reporting", "requirements": ["REQ-003", "REQ-004"],
         "types": ["orders.line", "orders.order"], "uses": [], "brief": []}
    ], "joins": [], "briefs": []}"#;
    let mut provider = Provider::answering([grouping, draft, slicing]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("reporting.create", "A signed-in user places an order."),
            requirement("reporting.cancel", "A signed-in user cancels an open order."),
        ])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 2 changed, 0 retired, 0 \
             conflict, 2 unchanged\n  stems: reporting\n  parts: orders (removed), reporting \
             (new)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 3, "the grouping, the draft, and the slicing");
    let request = provider.model.seen()[2].messages.join("\n");
    assert!(
        request.contains("## Current plan\n\n- `PART-001` `authentication` — REQ-001, REQ-002\n\n"),
        "the emptied part is gone: {request}"
    );
    assert!(
        request.contains(
            "- `reporting` — REQ-003, REQ-004 — triggers: `POST /orders`, \
             `orders.cancel-requested`\n"
        ),
        "{request}"
    );
    assert!(request.contains("Type keys the design holds that no current part owns"), "{request}");
    assert!(request.contains("- `orders.line`\n- `orders.order`\n"), "{request}");
    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    let parts = amendment["plan"]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 2, "{amendment}");
    assert_eq!(parts[0], base["parts"][0], "{amendment}");
    assert_eq!(
        parts[1],
        serde_json::json!({
            "id": "PART-003", "name": "reporting", "requirements": ["REQ-003", "REQ-004"],
            "types": ["orders.line", "orders.order"], "uses": [], "depends-on": [], "brief": []
        }),
        "{amendment}"
    );
    let design: Value = serde_json::from_slice(DESIGN).expect("the base design is JSON");
    assert_eq!(amendment["design"]["sections"], design["sections"], "{amendment}");

    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(!shown.contains("## Design"), "the design stands: {shown}");
    assert!(
        shown.ends_with(
            "## Plan\n\n### Part: orders [removed]\n\nID: PART-002\n\n### Part: reporting \
             [added]\n\nID: PART-003\nRequirements: [REQ-003, REQ-004]\nTypes: [orders.line, \
             orders.order]\n"
        ),
        "{shown}"
    );
    provider.model.assert_exhausted();
}

// The S1-shaped change: a documentation source stating two requirements
// under a stem no part holds. The grouping and the draft run as before,
// then one slicing turn over the current plan drafts the new part, which
// the engine numbers after the base's and whose dependency it maps to the
// part's id, the schema holding the answer to the open ids and the current
// part ids; no design turn is spent, since a requirement informs no
// section.
#[tokio::test]
async fn amend_sliced() {
    let slicing = r#"{"parts": [
        {"name": "rate-limit", "requirements": ["REQ-005", "REQ-006"], "types": [], "uses": ["orders.order"],
         "brief": ["Delivers the per-caller limit on order placement; edits the order handler, so it compiles against `orders`."]}
    ], "joins": [], "briefs": []}"#;
    let (provider, revision) = limited([slicing]);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  2 added, 0 changed, 0 retired, 0 \
             conflict, 4 unchanged\n  stems: rate-limit\n  parts: rate-limit (new)\n  nothing \
             sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 3, "the grouping, the draft, and the slicing");
    let turn = &provider.model.seen()[2];
    let request = turn.messages.join("\n");
    assert!(request.contains("- `rate-limit` — REQ-005, REQ-006\n"), "{request}");
    assert!(
        request.contains("- `PART-002` `orders` — REQ-003, REQ-004 — types:"),
        "untouched: {request}"
    );
    assert!(!request.contains("touched by this amendment"), "{request}");
    assert!(request.contains("- REQ-005 `rate-limit.count` — Status: unknown"), "{request}");
    assert!(request.contains("- REQ-006 `rate-limit.refuse` — Status: unknown"), "{request}");
    let SeenFormat::Schema { name, schema } = &turn.format else {
        panic!("the slicing is steered by schema");
    };
    assert_eq!(name, "slicing");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["parts"]["maxItems"], 2, "{schema}");
    assert_eq!(schema["properties"]["joins"]["maxItems"], 2, "{schema}");
    assert_eq!(
        schema["$defs"]["Draft"]["properties"]["requirements"]["items"]["enum"],
        serde_json::json!(["REQ-005", "REQ-006"]),
        "{schema}"
    );
    assert_eq!(schema["$defs"]["Draft"]["properties"]["types"]["maxItems"], 0, "{schema}");
    assert_eq!(
        schema["$defs"]["Join"]["properties"]["part"]["enum"],
        serde_json::json!(["PART-001", "PART-002"]),
        "{schema}"
    );
    // no part is touched, so no brief is required and one per join at most
    assert_eq!(schema["properties"]["briefs"]["minItems"], 0, "{schema}");
    assert_eq!(schema["properties"]["briefs"]["maxItems"], 2, "{schema}");

    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    let parts = amendment["plan"]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 3, "{amendment}");
    assert_eq!(parts[0], base["parts"][0], "{amendment}");
    assert_eq!(parts[1], base["parts"][1], "{amendment}");
    assert_eq!(
        parts[2],
        serde_json::json!({
            "id": "PART-003", "name": "rate-limit", "requirements": ["REQ-005", "REQ-006"],
            "types": [], "uses": ["orders.order"], "depends-on": ["PART-002"],
            "brief": ["Delivers the per-caller limit on order placement; edits the order handler, so it compiles against `orders`."]
        }),
        "{amendment}"
    );
    assert_eq!(amendment["plan"]["preamble"], base["preamble"], "{amendment}");
    assert_eq!(
        amendment["parts"],
        serde_json::json!([
            {"id": "PART-003", "name": "rate-limit", "class": "added", "brief": "redrafted"}
        ]),
        "{amendment}"
    );
    assert_eq!(provider.vcs.calls(), read_calls());
    provider.model.assert_exhausted();
}

// The same change answered by joining the new stem into a current part:
// the part holds the two beside its own, keeps its id, name, and types,
// takes the brief the answer redrafts for it, since a join names it, and no
// part is added.
#[tokio::test]
async fn amend_sliced_joined() {
    let slicing = r#"{"parts": [], "joins": [{"part": "PART-002", "requirements": ["REQ-005", "REQ-006"]}],
        "briefs": [{"part": "PART-002", "brief": ["Delivers orders and the per-caller limit on placing them."]}]}"#;
    let (provider, _) = limited([slicing]);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (_, amendment) = staged_amendment(&provider);
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("\n  parts: orders\n"), "{stdout}");
    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    let parts = amendment["plan"]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 2, "{amendment}");
    assert_eq!(parts[0], base["parts"][0], "{amendment}");
    assert_eq!(
        parts[1]["requirements"],
        serde_json::json!(["REQ-003", "REQ-004", "REQ-005", "REQ-006"]),
        "{amendment}"
    );
    assert_eq!(parts[1]["types"], base["parts"][1]["types"], "{amendment}");
    assert_eq!(
        parts[1]["brief"],
        serde_json::json!(["Delivers orders and the per-caller limit on placing them."]),
        "{amendment}"
    );
    assert_eq!(
        amendment["parts"],
        serde_json::json!([
            {"id": "PART-002", "name": "orders", "class": "changed", "brief": "redrafted"}
        ]),
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

// The slicing answer is held to the open set and the plan's rules: one that
// moves a requirement the current plan places, places an open one twice, or
// names a new part as a current one is refused naming each, and the
// correction lands.
#[tokio::test]
async fn invalid_part_amend() {
    let refused = r#"{"parts": [
        {"name": "orders", "requirements": ["REQ-005"], "types": [], "uses": [], "brief": []}
    ], "joins": [
        {"part": "PART-001", "requirements": ["REQ-003"]},
        {"part": "PART-002", "requirements": ["REQ-005", "REQ-006"], "uses": ["orders.order"]}
    ], "briefs": [
        {"part": "PART-001", "brief": ["Signs a caller in and places an order."]},
        {"part": "PART-002", "brief": ["Delivers orders and their rate limit."]}
    ]}"#;
    let corrected = r#"{"parts": [], "joins": [{"part": "PART-002", "requirements": ["REQ-005", "REQ-006"]}],
        "briefs": [{"part": "PART-002", "brief": ["Delivers orders and their rate limit."]}]}"#;
    let (provider, _) = limited([refused, corrected]);

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    assert_eq!(provider.model.seen().len(), 4, "the grouping, the draft, and two slicing rounds");
    let exchanges = provider.model.exchanges();
    let finding = exchanges[2].outcome.as_ref().expect_err("the first answer is refused");
    for expected in [
        "`orders` names a current part; list what joins it under `joins`",
        "`joins` `PART-001`: `REQ-003` is not open to this amendment; the current plan places \
         it, and it stays",
        "`REQ-005` appears in more than one part",
        "part `orders` uses `orders.order`, which it owns",
    ] {
        assert!(finding.contains(expected), "{expected}: {finding}");
    }
    assert!(exchanges[3].outcome.is_ok(), "{exchanges:?}");
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(
        amendment["plan"]["parts"][1]["requirements"],
        serde_json::json!(["REQ-003", "REQ-004", "REQ-005", "REQ-006"]),
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

// The S4-shaped change: two requirements under two new stems beside two
// `type` claims. The design turn redrafts `## Domain model` with the new
// keys, which no current part owns, so they are open with the stems: the
// slicing turn's brief lists them in the design's order, and each new part
// owns the key its requirement reads.
#[tokio::test]
async fn amend_sliced_types() {
    // indices 0 and 1 are `docs`'s requirements, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0], "classes": [[0]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]},
        {"claims": [5], "classes": [[5]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "rate-limit.count", "scenarios": [
            {"name": "Count", "when": "a caller places an order", "then": "the caller's count rises"}
        ]},
        {"subject": "audit.record", "scenarios": [
            {"name": "Record", "when": "an order is placed", "then": "an audit entry is recorded"}
        ]}
    ]}"#;
    let design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}], "revised": []}},
            {{"kind": "domain-model", "blocks": [
                {{"type": {{"key": "orders.order", "role": "data"}}}},
                {{"type": {{"key": "orders.line", "role": "data"}}}},
                {{"type": {{"key": "rate-limit.counter", "role": "data"}}}},
                {{"type": {{"key": "audit.entry", "role": "data"}}}}
            ], "revised": []}}
        ]}}"#
    );
    let slicing = r#"{"parts": [
        {"name": "rate-limit", "requirements": ["REQ-005"], "types": ["rate-limit.counter"], "uses": ["orders.order"], "brief": []},
        {"name": "audit", "requirements": ["REQ-006"], "types": ["audit.entry"], "uses": [], "brief": []}
    ], "joins": [], "briefs": []}"#;
    let mut provider = Provider::answering([grouping, draft, design.as_str(), slicing]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("rate-limit.count", "Each caller's order placements are counted."),
            requirement("audit.record", "Each order placed is recorded for audit."),
            claim(
                ClaimKind::Type,
                "rate-limit.counter",
                ("signature", "interface Counter { calls: number }"),
            ),
            claim(ClaimKind::Type, "audit.entry", ("signature", "interface Entry { at: string }")),
        ])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  2 added, 0 changed, 0 retired, 0 \
             conflict, 4 unchanged\n  stems: audit, rate-limit\n  design: domain-model\n  parts: \
             rate-limit (new), audit (new)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(
        provider.model.seen().len(),
        4,
        "the grouping, the draft, the design, and the slicing"
    );
    let turn = &provider.model.seen()[3];
    let request = turn.messages.join("\n");
    assert!(request.contains("- `rate-limit` — REQ-005\n- `audit` — REQ-006\n"), "{request}");
    assert!(
        request.contains(
            "Type keys the design holds that no current part owns, each to be owned by \
             exactly one part"
        ),
        "{request}"
    );
    assert!(request.contains("- `audit.entry`\n- `rate-limit.counter`\n"), "{request}");
    let SeenFormat::Schema { schema, .. } = &turn.format else {
        panic!("the slicing is steered by schema");
    };
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(
        schema["$defs"]["Draft"]["properties"]["types"]["items"]["enum"],
        serde_json::json!(["audit.entry", "rate-limit.counter"]),
        "{schema}"
    );
    assert_eq!(
        schema["$defs"]["Join"]["properties"]["types"]["items"]["enum"],
        serde_json::json!(["audit.entry", "rate-limit.counter"]),
        "{schema}"
    );

    let parts = amendment["plan"]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 4, "{amendment}");
    assert_eq!(parts[2]["id"], "PART-003", "{amendment}");
    assert_eq!(parts[2]["name"], "rate-limit", "{amendment}");
    assert_eq!(parts[2]["types"], serde_json::json!(["rate-limit.counter"]), "{amendment}");
    assert_eq!(parts[2]["depends-on"], serde_json::json!(["PART-002"]), "{amendment}");
    assert_eq!(parts[3]["id"], "PART-004", "{amendment}");
    assert_eq!(parts[3]["name"], "audit", "{amendment}");
    assert_eq!(parts[3]["types"], serde_json::json!(["audit.entry"]), "{amendment}");
    provider.model.assert_exhausted();
}

// The S1-shaped change: a documentation source stating two requirements
// under the stem `rate-limit`, which no part holds, scripted through the
// grouping and the draft of those two alone, then `slicing`, the answers
// of the slicing turn in order, over the seeded revision.
fn limited<const N: usize>(slicing: [&str; N]) -> (Provider, String) {
    // indices 0 and 1 are `docs`'s requirements, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0], "classes": [[0]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]},
        {"claims": [5], "classes": [[5]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "rate-limit.count", "scenarios": [
            {"name": "Count", "when": "a caller places an order", "then": "the caller's count rises"}
        ]},
        {"subject": "rate-limit.refuse", "scenarios": [
            {"name": "Refuse", "when": "a caller is over the limit", "then": "the order is refused"}
        ]}
    ]}"#;
    let mut answers = vec![grouping, draft];
    answers.extend(slicing);
    let mut provider = Provider::answering(answers);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("rate-limit.count", "Each caller's order placements are counted."),
            requirement("rate-limit.refuse", "A caller over the limit is refused."),
        ])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);
    (provider, revision)
}

// A change whose claims inform a design section spends one design turn over
// the current design: a documentation change carrying a `decision` and a
// requirement under a new stem redrafts `## APIs and integrations`, which
// the base lacks, and `## Overview`, which describes the whole, while
// `## Domain model` is kept byte for byte with no type block offered. The
// brief carries the change's claims, the current text numbered, the
// requirement drafted beside it, and the schema the redrafted kinds alone;
// the new stem is a new part the slicing turn drafts after the design,
// numbered after the base's; the amendment lists the sections added and
// changed and the part added, the output and the envelope name them, and
// `show amendment` renders the new text and the new part with its brief.
#[tokio::test]
async fn amend_design() {
    let (provider, revision) = designed();

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  1 added, 0 changed, 0 retired, 0 \
             conflict, 4 unchanged\n  stems: secrets\n  design: overview, apis\n  parts: \
             secrets (new)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    let expected: Value = serde_json::from_str(&DESIGNED.replace("<revision>", &revision))
        .expect("the fixture is JSON");
    assert_eq!(amendment, expected);
    assert_eq!(
        provider.model.seen().len(),
        4,
        "the grouping, the draft, the design, and the slicing"
    );

    // the design turn carries the change's claims, the current text of the
    // redrafted sections numbered, the kept-or-named rule, and the
    // requirement drafted beside it; the kept section is not listed and no
    // type block is offered
    let turn = &provider.model.seen()[2];
    let request = turn.messages.join("\n");
    assert!(request.starts_with("Amend `design.md`"), "{request}");
    assert!(request.contains("- decision `secrets.vault`"), "{request}");
    assert!(request.contains(&format!("### Overview\n\n[1] {OVERVIEW}\n")), "{request}");
    assert!(
        request.contains(
            "### APIs and integrations\n\nThe current design has no such section; draft it.\n"
        ),
        "{request}"
    );
    assert!(!request.contains("### Domain model"), "{request}");
    assert!(!request.contains("\n## Type blocks\n"), "{request}");
    assert!(request.contains("name every paragraph you reword or drop"), "{request}");
    assert!(request.contains("- REQ-005 `secrets.read` — Status: unknown"), "{request}");
    assert!(!request.contains("- REQ-003"), "a standing requirement is not drafted: {request}");
    let SeenFormat::Schema { name, schema } = &turn.format else {
        panic!("the design is steered by schema");
    };
    assert_eq!(name, "design-draft");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["sections"]["minItems"], 2, "{schema}");
    assert_eq!(schema["properties"]["sections"]["maxItems"], 2, "{schema}");
    assert_eq!(
        schema["$defs"]["Redraft"]["properties"]["kind"]["enum"],
        serde_json::json!(["overview", "apis"]),
        "{schema}"
    );
    assert_eq!(
        schema["$defs"]["Redraft"]["properties"]["revised"]["items"]["maximum"], 1,
        "the overview's one paragraph: {schema}"
    );
    assert!(schema["$defs"]["TypeRef"]["properties"]["key"].get("enum").is_none(), "{schema}");
    assert_eq!(provider.vcs.calls(), read_calls());

    // the projection carries what the design changed and the part the new
    // stem became, with the brief the slicing turn drafted
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.ends_with(&format!(
            "## Design\n\nChanged: Overview; added: APIs and integrations.\n\n### \
             Overview\n\n{OVERVIEW}\n\nHandlers read the secrets they need at use \
             (REQ-005).\n\n### APIs and integrations\n\nSecrets are read from a secret store, \
             the team vault (from docs), by REQ-005 at use.\n\n## Plan\n\n### Part: secrets \
             [added]\n\nID: PART-003\nRequirements: [REQ-005]\n\nDelivers the secret read \
             every handler makes at use; verified against a fake store.\n"
        )),
        "{shown}"
    );
    provider.model.assert_exhausted();
}

// The JSON envelope carries the sections the design changed and the parts
// the plan changed as the amendment records them.
#[tokio::test]
async fn amend_design_envelope() {
    let (provider, revision) = designed();

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "amend", DOCS, "--config", PROJECT_CONFIG],
    )
    .await;

    let expected: Value = serde_json::from_str(&DESIGNED.replace("<revision>", &revision))
        .expect("the fixture is JSON");
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["sections"], expected["sections"], "{envelope}");
    assert_eq!(envelope["parts"], expected["parts"], "{envelope}");
    provider.model.assert_exhausted();
}

// The S4-shaped change: a documentation source stating one requirement under
// a new stem beside a `decision`, scripted through the grouping, the draft of
// that requirement alone, and a design redrafting `## Overview` and
// `## APIs and integrations`, over the seeded revision.
fn designed() -> (Provider, String) {
    // index 0 is `docs`'s requirement, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0], "classes": [[0]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "secrets.read", "scenarios": [
            {"name": "Read", "when": "a handler needs a secret", "then": "it is read at use"}
        ]}
    ]}"#;
    let design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [
                {{"text": "{OVERVIEW}"}},
                {{"text": "Handlers read the secrets they need at use (REQ-005)."}}
            ], "revised": []}},
            {{"kind": "apis", "blocks": [
                {{"text": "Secrets are read from a secret store, the team vault (from docs), by REQ-005 at use."}}
            ], "revised": []}}
        ]}}"#
    );
    let slicing = r#"{"parts": [
        {"name": "secrets", "requirements": ["REQ-005"], "types": [], "uses": [],
         "brief": ["Delivers the secret read every handler makes at use; verified against a fake store."]}
    ], "joins": [], "briefs": []}"#;
    let mut provider = Provider::answering([grouping, draft, design.as_str(), slicing]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("secrets.read", "A handler reads a secret at use."),
            claim(ClaimKind::Decision, "secrets.vault", ("decision", VAULT)),
        ])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);
    (provider, revision)
}

// A `type` claim redeclaring a type the design holds informs
// `## Domain model`: the redraft is offered the kept keys and the change's,
// the redeclared key under the change's signature, and the accepted design
// carries the new signature beside the kept type; the change's requirement
// restates a standing one, so the specification stands and the amendment
// stages for the design alone.
#[tokio::test]
async fn amend_design_types() {
    // index 0 is `docs`'s requirement, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 3], "classes": [[0, 3]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}], "revised": []}},
            {{"kind": "domain-model", "blocks": [
                {{"type": {{"key": "orders.order", "role": "data"}}}},
                {{"type": {{"key": "orders.line", "role": "data"}}}}
            ], "revised": []}}
        ]}}"#
    );
    let signature = "interface Order { id: string; total: number }";
    let mut provider = Provider::answering([grouping, design.as_str()]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("orders.create", "A signed-in user places an order."),
            claim(ClaimKind::Type, "orders.order", ("signature", signature)),
        ])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 0 retired, 0 \
             conflict, 4 unchanged\n  design: domain-model\n  nothing sealed; `emery show \
             amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 2, "the grouping and the design; nothing drafted");
    let base: Value = serde_json::from_slice(SPEC).expect("the base is JSON");
    assert_eq!(amendment["spec"]["requirements"], base["requirements"], "{amendment}");
    assert_eq!(
        amendment["design"]["sections"][1]["blocks"],
        serde_json::json!([
            {"type": {"key": "orders.order", "signature": signature, "role": "data"}},
            {"type": {"key": "orders.line", "signature": "interface Line { sku: string }", "role": "data"}}
        ]),
        "{amendment}"
    );
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": [], "changed": ["domain-model"]}),
        "{amendment}"
    );

    // the brief lists the current type blocks numbered and the keys to
    // reference, the redeclared one marked
    let turn = &provider.model.seen()[1];
    let request = turn.messages.join("\n");
    assert!(
        request.contains(
            "### Domain model\n\n[1] Type: orders.order (data)\n[2] Type: orders.line (data)\n"
        ),
        "{request}"
    );
    assert!(
        request.contains("## Type blocks\n\nReference each key exactly once under `domain-model`"),
        "{request}"
    );
    assert!(request.contains("- `orders.line` — kept, data\n"), "{request}");
    assert!(
        request.contains("- `orders.order` — kept, data; the change redeclares its signature\n"),
        "{request}"
    );
    assert!(request.contains("The change adds or revises no requirement"), "{request}");
    let SeenFormat::Schema { schema, .. } = &turn.format else {
        panic!("the design is steered by schema");
    };
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(
        schema["$defs"]["TypeRef"]["properties"]["key"]["enum"],
        serde_json::json!(["orders.line", "orders.order"]),
        "{schema}"
    );
    assert_eq!(
        schema["$defs"]["Redraft"]["properties"]["revised"]["items"]["maximum"], 2,
        "the domain model's two blocks: {schema}"
    );
    provider.model.assert_exhausted();
}

// The redraft is held to the current text: a paragraph dropped without being
// named `revised` is a finding the next answer corrects by naming it. A
// change of a `decision` alone states no requirement, so no grouping is
// spent: every requirement stands unchanged, and the amendment stages for
// the design.
#[tokio::test]
async fn amend_design_loss() {
    let dropped = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Secrets are read at use."}], "revised": []},
        {"kind": "apis", "blocks": [{"text": "The team vault holds the secrets (from docs)."}], "revised": []}
    ]}"#;
    let named = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Secrets are read at use."}], "revised": [1]},
        {"kind": "apis", "blocks": [{"text": "The team vault holds the secrets (from docs)."}], "revised": []}
    ]}"#;
    let mut provider = Provider::answering([dropped, named]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![claim(ClaimKind::Decision, "secrets.vault", ("decision", VAULT))])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 0 retired, 0 \
             conflict, 4 unchanged\n  design: overview, apis\n  nothing sealed; `emery show \
             amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 2, "the design and its correction; no grouping");
    let exchanges = provider.model.exchanges();
    assert!(
        exchanges[0].outcome.as_ref().is_err_and(|finding| {
            finding.contains(
                "`## Overview` [1] is dropped or reworded without being named in `revised`",
            )
        }),
        "{exchanges:?}"
    );
    assert!(
        amendment["changes"]
            .as_array()
            .is_some_and(|changes| changes.iter().all(|change| change["class"] == "unchanged")),
        "{amendment}"
    );
    assert_eq!(
        amendment["design"]["sections"][0]["blocks"],
        serde_json::json!([{"text": "Secrets are read at use."}]),
        "{amendment}"
    );
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": ["apis"], "removed": [], "changed": ["overview"]}),
        "{amendment}"
    );
    assert_eq!(provider.vcs.calls(), read_calls());
    provider.model.assert_exhausted();
}

// A `call` claim in the change informs `## APIs and integrations`, and the
// amended design names the package it reaches through as a draft would: a
// redraft naming no term of it is a finding, and the next answer, naming it
// beside the role it plays, stages.
#[tokio::test]
async fn amend_design_call() {
    let unnamed = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Sign-in issues a session; orders are placed and cancelled under it."}], "revised": []},
        {"kind": "apis", "blocks": [{"text": "Postings reach the ledger over HTTP, by REQ-003 at use."}], "revised": []}
    ]}"#;
    let named = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Sign-in issues a session; orders are placed and cancelled under it."}], "revised": []},
        {"kind": "apis", "blocks": [{"text": "Postings reach the ledger through the ledger-client (acme-cloud), by REQ-003 at use."}], "revised": []}
    ]}"#;
    let mut provider = Provider::answering([unnamed, named]);
    let mut call =
        claim(ClaimKind::Call, "orders.post", ("callee", "@acme-cloud/ledger-client:post"));
    call.path = Some("src/orders.ts#L40".to_owned());
    provider.source.evidence.insert("docs".to_owned(), Ok(evidence(vec![call])));
    seed(&provider.files, SPEC, DESIGN, PLAN);

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(provider.model.seen().len(), 2, "the design and its correction; no grouping");
    let exchanges = provider.model.exchanges();
    assert!(
        exchanges[0].outcome.as_ref().is_err_and(|finding| {
            finding.contains(
                "the design names no technology for the `call` claims through \
                 `@acme-cloud/ledger-client` (`@acme-cloud/ledger-client:post` at \
                 `src/orders.ts#L40`); name it beside the role it plays",
            )
        }),
        "{exchanges:?}"
    );
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": ["apis"], "removed": [], "changed": []}),
        "{amendment}"
    );
    assert_eq!(
        amendment["design"]["sections"][2]["blocks"][0]["text"],
        "Postings reach the ledger through the ledger-client (acme-cloud), by REQ-003 at use.",
        "{amendment}"
    );
    assert_eq!(provider.vcs.calls(), read_calls());
    provider.model.assert_exhausted();
}

// A change that adds or revises no requirement carries technology alone,
// which lands in the paragraphs already describing the behaviour it serves:
// the turn is told so, a revised paragraph citing unchanged requirements is
// accepted named on the first round, and the amendment stages for the
// design with every requirement unchanged.
#[tokio::test]
async fn amend_design_decision() {
    let stored = "Orders are kept in the order store, which REQ-003 writes.";
    let base = describing(&[stored]);
    let revised = "Orders are kept in the order store, which REQ-003 writes under credentials \
                   read from the team vault (from docs).";
    let design = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}, {{"text": "Secrets are read from the vault at use."}}], "revised": []}},
            {{"kind": "apis", "blocks": [{{"text": "{revised}"}}], "revised": [1]}}
        ]}}"#
    );
    let mut provider = Provider::answering([design.as_str()]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![claim(ClaimKind::Decision, "secrets.vault", ("decision", VAULT))])),
    );
    let revision = seed(&provider.files, SPEC, base.as_bytes(), PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 0 retired, 0 \
             conflict, 4 unchanged\n  design: overview, apis\n  nothing sealed; `emery show \
             amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 1, "the design alone, accepted at once");
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(
        request.contains(
            "The change adds or revises no requirement; the specification's stand as they are. \
             What it carries lands in the paragraphs that already describe the behaviour it \
             serves, each one you revise named in `revised`."
        ),
        "{request}"
    );
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": [], "changed": ["overview", "apis"]}),
        "{amendment}"
    );
    assert_eq!(amendment["design"]["sections"][2]["blocks"][0]["text"], revised, "{amendment}");
    provider.model.assert_exhausted();
}

// Where the change drafts a requirement, a paragraph named `revised` that
// cites requirements the change leaves unchanged, and no other, is a finding
// the next answer corrects by keeping it verbatim.
#[tokio::test]
async fn amend_design_revised() {
    let stored = "Orders are kept in the order store, which REQ-003 writes.";
    let base = describing(&[stored]);
    // index 0 is `docs`'s requirement, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 2], "classes": [[0], [2]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "session.timeout", "scenarios": [
            {"name": "Timeout", "when": "a session is idle for 15 minutes", "then": "it times out"}
        ]}
    ]}"#;
    let reworded = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}], "revised": []}},
            {{"kind": "apis", "blocks": [
                {{"text": "Orders are kept in a relational order store, which REQ-003 writes."}},
                {{"text": "The team vault holds the secrets (from docs)."}}
            ], "revised": [1]}}
        ]}}"#
    );
    let kept = format!(
        r#"{{"sections": [
            {{"kind": "overview", "blocks": [{{"text": "{OVERVIEW}"}}], "revised": []}},
            {{"kind": "apis", "blocks": [
                {{"text": "{stored}"}},
                {{"text": "The team vault holds the secrets (from docs)."}}
            ], "revised": []}}
        ]}}"#
    );
    let mut provider = Provider::answering([grouping, draft, reworded.as_str(), kept.as_str()]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("session.timeout", "Sessions expire after 15 minutes of inactivity."),
            claim(ClaimKind::Decision, "secrets.vault", ("decision", VAULT)),
        ])),
    );
    seed(&provider.files, SPEC, base.as_bytes(), PLAN);

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    assert_eq!(provider.model.seen().len(), 4, "the grouping, the draft, the design, corrected");
    let exchanges = provider.model.exchanges();
    assert!(
        exchanges[2].outcome.as_ref().is_err_and(|finding| {
            finding.contains(
                "`## APIs and integrations` [1] describes REQ-003, which the change leaves \
                 unchanged; keep it verbatim",
            )
        }),
        "{exchanges:?}"
    );
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(amendment["changes"][1]["class"], "changed", "{amendment}");
    assert_eq!(
        amendment["sections"],
        serde_json::json!({"preamble": false, "added": [], "removed": [], "changed": ["apis"]}),
        "{amendment}"
    );
    assert_eq!(amendment["design"]["sections"][2]["blocks"][0]["text"], stored, "{amendment}");
    provider.model.assert_exhausted();
}

// A `retire` the grouping matched to no standing requirement withdraws a
// behaviour the specification does not hold: the run is refused typed,
// naming every such claim across groups, with nothing staged, whether a
// retire stands alone or beside a claim the change adds.
#[tokio::test]
async fn amend_retire_unmatched() {
    let alone = r#"{"groups": [
        {"claims": [0], "classes": [[0]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let mut provider = Provider::answering([alone]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![retire("orders.export", "Downloading the order book is withdrawn.")])),
    );
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let envelope = fail(
        &provider,
        &["emery", "amend", DOCS, "--config", PROJECT_CONFIG],
        1,
        "retire-unmatched",
    )
    .await;
    assert!(
        envelope["message"].as_str().is_some_and(|text| {
            text.contains("retire `orders.export` from `docs` withdraws a behaviour")
        }),
        "{envelope}"
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery show spec")),
        "{envelope}"
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");
    assert_eq!(provider.vcs.calls(), read_calls());
    provider.model.assert_exhausted();

    // another source's retire beside the added claim it shares an id with,
    // and a second in another group, are both named at once
    let added = r#"{"groups": [
        {"claims": [0, 2], "classes": [[0], [2]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]},
        {"claims": [5], "classes": [[5]]},
        {"claims": [6], "classes": [[6]]}
    ]}"#;
    let mut provider = Provider::answering([added]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("orders.export", "A signed-in user downloads the order book."),
            retire("orders.import", "Uploading orders from a file is withdrawn."),
        ])),
    );
    provider.source.evidence.insert(
        "wiki".to_owned(),
        Ok(evidence(vec![retire("orders.export", "Downloading the order book is withdrawn.")])),
    );
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let envelope = fail(
        &provider,
        &["emery", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG],
        1,
        "retire-unmatched",
    )
    .await;
    assert!(
        envelope["message"].as_str().is_some_and(|text| {
            text.contains(
                "retire `orders.export` from `wiki`, `orders.import` from `docs` withdraw \
                 behaviours",
            )
        }),
        "{envelope}"
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");
}

// `docs` withdraws `orders.cancel` while `wiki` at the same rank revises
// it, seeded over `SPEC` with the grouping and draft scripted.
fn contested_retire() -> (Provider, String) {
    // indices 0 is `docs`, 1 `wiki`, 2..=5 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 1, 5], "classes": [[0], [1], [5]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.cancel", "scenarios": [
            {"name": "Cancel", "when": "a signed-in caller cancels an order", "then": "[unknown]"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    provider
        .source
        .evidence
        .insert("docs".to_owned(), Ok(evidence(vec![retire("orders.cancel", WITHDRAWN)])));
    provider.source.evidence.insert(
        "wiki".to_owned(),
        Ok(evidence(vec![requirement(
            "orders.cancel",
            "A signed-in user cancels an order within an hour of placing it.",
        )])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);
    (provider, revision)
}

// A withdrawal and a revision of one requirement from two sources of one
// rank are a conflict, as two statements are: the requirement is listed in
// conflict with both claims behind it, nothing is retired, and the amended
// specification holds it as a specify holds a conflict, the withdrawal a
// competing statement among its notes.
#[tokio::test]
async fn amend_retire_contested() {
    let (provider, revision) = contested_retire();

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 0 changed, 0 retired, 1 \
             conflict, 3 unchanged\n  conflicts: orders.cancel\n  stems: orders\n  nothing \
             sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    let contested = &amendment["changes"][3];
    assert_eq!(contested["id"], "REQ-004", "{amendment}");
    assert_eq!(contested["class"], "conflict", "{amendment}");
    assert_eq!(contested["before"], "A signed-in user cancels an open order.", "{amendment}");
    assert_eq!(contested["after"], Value::Null, "{amendment}");
    assert_eq!(
        contested["sources"],
        serde_json::json!([
            {"source": "docs", "claim": "orders.cancel", "path": null},
            {"source": "wiki", "claim": "orders.cancel", "path": null}
        ]),
        "{amendment}"
    );
    let drafted = &amendment["spec"]["requirements"][3];
    assert_eq!(drafted["id"], "REQ-004", "{amendment}");
    assert_eq!(drafted["status"], "conflict", "{amendment}");
    assert_eq!(drafted["body"], serde_json::json!([]), "{amendment}");
    assert_eq!(drafted["sources"], contested["sources"], "{amendment}");
    assert_eq!(drafted["losers"][0]["statement"], WITHDRAWN, "{amendment}");
    assert_eq!(drafted["losers"][1]["sources"], serde_json::json!(["wiki"]), "{amendment}");
    assert_eq!(drafted["trigger"], serde_json::json!({"message": "orders.cancel-requested"}));
    assert_eq!(drafted["scenarios"][0]["name"], "Cancel", "{amendment}");
    provider.model.assert_exhausted();

    // the projection notes both statements before the reconciliation note
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.contains(&format!(
            "## Conflict\n\n### Requirement: orders.cancel [conflict]\n\nID: REQ-004\nSources: \
             [docs:orders.cancel, wiki:orders.cancel]\n\nNote: docs (documentation, rank 2, \
             orders.cancel): {WITHDRAWN}\nNote: wiki (documentation, rank 2, orders.cancel): A \
             signed-in user cancels an order within an hour of placing it.\nNote: Operator \
             reconciliation required.\n\n#### Scenario: Cancel\n"
        )),
        "{shown}"
    );
}

// The conflict seals as a specify's would: `--apply` writes the
// specification with the requirement in conflict, its competing statements
// among its notes, counted under no trailer, and nothing refuses it, since
// the next change is where the operator reconciles it.
#[tokio::test]
async fn retire_contested_apply() {
    let (provider, revision) = contested_retire();

    let resp =
        cli_ok(&provider, &["emery", "amend", DOCS, WIKI, "--apply", "--config", PROJECT_CONFIG])
            .await;

    let pointer = provider.storage.state(REVISION_KEY).expect("the pointer moves");
    let sealed = String::from_utf8_lossy(&pointer).into_owned();
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.starts_with("staged amendment "), "{stdout}");
    assert!(
        stdout.ends_with(&format!(
            "\n  base {revision} at {}\n  0 added, 0 changed, 0 retired, 1 conflict, 3 \
             unchanged\n  conflicts: orders.cancel\n  stems: orders\n  sealed revision {sealed} \
             as emery/{sealed} at Revision\n    amends {revision}\n",
            &HEAD[..8]
        )),
        "{stdout}"
    );
    let spec: Value =
        serde_json::from_slice(&written(&provider.files, "spec")).expect("the sealed spec is JSON");
    let sealed_cancel = &spec["requirements"][3];
    assert_eq!(sealed_cancel["id"], "REQ-004", "{spec}");
    assert_eq!(sealed_cancel["status"], "conflict", "{spec}");
    assert_eq!(sealed_cancel["body"], serde_json::json!([]), "{spec}");
    assert_eq!(sealed_cancel["losers"][0]["statement"], WITHDRAWN, "{spec}");
    assert_eq!(sealed_cancel["losers"][1]["sources"], serde_json::json!(["wiki"]), "{spec}");
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the slot is cleared");
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(message.parent.as_deref(), Some(revision.as_str()), "{message:?}");
    assert_eq!(
        message.summary,
        Some(Summary {
            added: 0,
            changed: 0,
            retired: 0,
            parts: Vec::new(),
        }),
        "{message:?}"
    );
    provider.model.assert_exhausted();
}

// A source cannot conflict with itself: a `retire` beside the same source's
// `requirement` under one id is passed over before the grouping, so the
// requirement is changed to the restatement, nothing is retired, and the
// grouping never sees the withdrawal.
#[tokio::test]
async fn amend_retire_own() {
    let restated = "A signed-in user cancels an order within an hour of placing it.";
    // index 0 is `docs`' requirement, its retire passed over, and 1..=4 the
    // standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0], [4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.cancel", "scenarios": [
            {"name": "Cancel", "when": "a signed-in caller cancels an order within an hour", "then": "the order is cancelled"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            retire("orders.cancel", WITHDRAWN),
            requirement("orders.cancel", restated),
        ])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 1 changed, 0 retired, 0 \
             conflict, 3 unchanged\n  stems: orders\n  nothing sealed; `emery show amendment` \
             reviews it\n",
            &HEAD[..8]
        )
    );
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(request.contains(&format!("- 0 `docs` `orders.cancel` — {restated}")), "{request}");
    assert!(!request.contains("(retire) —"), "the withdrawal is passed over: {request}");
    assert!(request.contains("- 4 `orders.cancel`"), "{request}");
    assert!(!request.contains("- 5 "), "the indices run on from the one claim: {request}");
    let changed = &amendment["changes"][3];
    assert_eq!(changed["id"], "REQ-004", "{amendment}");
    assert_eq!(changed["class"], "changed", "{amendment}");
    assert_eq!(changed["before"], "A signed-in user cancels an open order.", "{amendment}");
    assert_eq!(changed["after"], restated, "{amendment}");
    assert_eq!(
        changed["sources"],
        serde_json::json!([{"source": "docs", "claim": "orders.cancel", "path": null}]),
        "{amendment}"
    );
    let drafted = &amendment["spec"]["requirements"][3];
    assert_eq!(drafted["status"], "unknown", "{amendment}");
    assert_eq!(drafted["body"], serde_json::json!([restated]), "{amendment}");
    assert_eq!(drafted["losers"], serde_json::json!([]), "{amendment}");
    provider.model.assert_exhausted();
}

// A requirement the base holds in conflict stands as one requirement whose
// statement lists the competing statements, so a claim matched to it
// resolves the conflict: the grouping lists one standing entry for it, the
// change is `changed` with the competing statements as `before` and the
// claim's statement as `after`, the amended specification holds it under
// the status the change sources alone settle with its notes gone, and `show
// amendment` prints the competing statements on its `Before:` line.
#[tokio::test]
async fn conflict_claim() {
    let competing = "[conflict] A signed-in user cancels an open order. | A signed-in user \
                     cancels an order within an hour of placing it.";
    let resolved = "A signed-in user cancels an order within a day of placing it.";
    // index 0 is `docs`, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 4], "classes": [[0], [4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.cancel", "scenarios": [
            {"name": "Cancel", "when": "a signed-in caller cancels an order placed a day ago", "then": "the order is cancelled"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    provider
        .source
        .evidence
        .insert("docs".to_owned(), Ok(evidence(vec![requirement("orders.cancel", resolved)])));
    let revision = seed(&provider.files, CONFLICTED_SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 1 changed, 0 retired, 0 \
             conflict, 3 unchanged\n  stems: orders\n  nothing sealed; `emery show amendment` \
             reviews it\n",
            &HEAD[..8]
        )
    );

    // the conflicted requirement is one standing entry listing both
    // statements, pre-merged with the claim at its id
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(request.contains(&format!("- 4 `orders.cancel` — {competing}")), "{request}");
    assert!(!request.contains("- 5 "), "one entry per standing requirement: {request}");
    assert!(request.contains("claims 0, 4 share the id `orders.cancel`"), "{request}");
    let draft = provider.model.seen()[1].messages.join("\n");
    assert!(
        draft.contains(
            "- REQ-004 `orders.cancel` — Status: unknown — Sources: [docs:orders.cancel]"
        ),
        "the status is the change sources' alone: {draft}"
    );

    let changed = &amendment["changes"][3];
    assert_eq!(changed["id"], "REQ-004", "{amendment}");
    assert_eq!(changed["class"], "changed", "{amendment}");
    assert_eq!(changed["before"], competing, "{amendment}");
    assert_eq!(changed["after"], resolved, "{amendment}");
    let drafted = &amendment["spec"]["requirements"][3];
    assert_eq!(drafted["status"], "unknown", "{amendment}");
    assert_eq!(drafted["body"], serde_json::json!([resolved]), "{amendment}");
    assert_eq!(drafted["losers"], serde_json::json!([]), "{amendment}");
    assert_eq!(
        drafted["sources"],
        serde_json::json!([{"source": "docs", "claim": "orders.cancel", "path": null}]),
        "{amendment}"
    );
    assert_eq!(drafted["trigger"], serde_json::json!({"message": "orders.cancel-requested"}));

    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.contains(&format!(
            "## Changed\n\n### Requirement: orders.cancel\n\nID: REQ-004\nSources: \
             [docs:orders.cancel]\n\nBefore: {competing}\n\nAfter: {resolved}\n\n#### Scenario: \
             Cancel\n"
        )),
        "{shown}"
    );
    assert!(!shown.contains("Operator reconciliation required"), "{shown}");
    assert_eq!(provider.vcs.calls(), read_calls());
    provider.model.assert_exhausted();
}

// A requirement the base holds in conflict that no claim touches stands as
// it is, its conflict included: the entry is unchanged with the competing
// statements as its statement before and after, the amended specification
// holds it as the base does, and `show amendment` lists it so.
#[tokio::test]
async fn standing_conflict() {
    let competing = "[conflict] A signed-in user cancels an open order. | A signed-in user \
                     cancels an order within an hour of placing it.";
    // index 0 is `docs`, 1..=4 the standing requirements in id order
    let grouping = r#"{"groups": [
        {"claims": [0, 3], "classes": [[0], [3]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.create", "scenarios": [
            {"name": "Create", "when": "a signed-in caller places an order of ten lines", "then": "the order is created"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![requirement(
            "orders.create",
            "A signed-in user places an order of up to ten lines.",
        )])),
    );
    seed(&provider.files, CONFLICTED_SPEC, DESIGN, PLAN);

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (_, amendment) = staged_amendment(&provider);
    let standing = &amendment["changes"][3];
    assert_eq!(standing["class"], "unchanged", "{amendment}");
    assert_eq!(standing["before"], competing, "{amendment}");
    assert_eq!(standing["after"], competing, "{amendment}");
    let base: Value = serde_json::from_slice(CONFLICTED_SPEC).expect("the base is JSON");
    assert_eq!(amendment["spec"]["requirements"][3], base["requirements"][3], "{amendment}");
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.contains(&format!(
            "### Requirement: orders.cancel\n\nID: REQ-004\nSources: []\n\n{competing}\n"
        )),
        "{shown}"
    );
    provider.model.assert_exhausted();
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
        "budget-exhausted",
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

// `amend --apply` seals what it stages in the same run: the amendment lands
// in the slot, then its three documents are sealed as a revision over the
// base the run read, under the base as parent with the counts and the
// parts in the message, the slot is cleared, the pointer names the sealed
// revision, and the output carries the staged block and the seal in place
// of the `nothing sealed` line, the envelope `applied` as `apply` answers
// it and `staged` false since nothing stands staged after the seal.
#[tokio::test]
async fn amend_apply() {
    let mut provider = Provider::answering([CLASSES_GROUPING, CLASSES_DRAFT]);
    bind(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "amend", DOCS, WIKI, "--apply", "--config", PROJECT_CONFIG],
    )
    .await;

    // the envelope carries what apply answers beside the amendment's report
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let amendment = envelope["amendment"].as_str().expect("the amendment's id").to_owned();
    let sealed = envelope["applied"]["revision"].as_str().expect("the sealed id").to_owned();
    assert_ne!(sealed, revision);
    assert_eq!(envelope["staged"], false, "{envelope}");
    assert_eq!(envelope["tally"]["changed"], 1, "{envelope}");
    assert_eq!(envelope["applied"]["parent"], revision, "{envelope}");
    assert_eq!(envelope["applied"]["kind"], "amendment", "{envelope}");
    assert_eq!(envelope["applied"]["amendment"], amendment, "{envelope}");
    assert_eq!(envelope["applied"]["label"], format!("emery/{sealed}"), "{envelope}");
    assert_eq!(envelope["applied"]["commit"], "Revision-commit", "{envelope}");
    assert_eq!(envelope["applied"]["diff"]["from"], revision, "{envelope}");

    // the sealed documents are the amendment's
    let expected: Value = serde_json::from_str(&CLASSES.replace("<revision>", &revision))
        .expect("the fixture is JSON");
    let spec: Value =
        serde_json::from_slice(&written(&provider.files, "spec")).expect("the sealed spec is JSON");
    assert_eq!(spec, expected["spec"]);
    let pointer = provider.storage.state(REVISION_KEY).expect("the pointer moves");
    assert_eq!(String::from_utf8_lossy(&pointer), sealed);
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the slot is cleared");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "the amendment is pruned");
    let message = RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
    assert_eq!(
        message,
        RevisionMessage {
            revision: sealed.clone(),
            parent: Some(revision.clone()),
            base: Some(HEAD.to_owned()),
            summary: Some(Summary {
                added: 1,
                changed: 1,
                retired: 0,
                parts: vec![PartSummary {
                    id: "PART-002".parse().expect("a part id"),
                    name: "orders".to_owned(),
                    class: PartClass::Changed,
                }],
            }),
        }
    );
    assert!(
        provider.vcs.calls().contains(&format!("label . emery/{sealed} Revision-commit")),
        "{:?}",
        provider.vcs.calls()
    );
    provider.model.assert_exhausted();

    // the text render prints the staged block, then the seal in place of
    // the `nothing sealed` line
    let mut provider = Provider::answering([CLASSES_GROUPING, CLASSES_DRAFT]);
    bind(&mut provider);
    seed(&provider.files, SPEC, DESIGN, PLAN);
    let resp =
        cli_ok(&provider, &["emery", "amend", DOCS, WIKI, "--apply", "--config", PROJECT_CONFIG])
            .await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "staged amendment {amendment}\n  base {revision} at {}\n  1 added, 1 changed, 0 \
             retired, 1 conflict, 2 unchanged\n  conflicts: auth.login\n  stems: auth, orders, \
             session\n  parts: orders (brief kept)\n  sealed revision {sealed} as emery/{sealed} \
             at Revision\n    amends {revision}\n",
            &HEAD[..8]
        )
    );
    provider.model.assert_exhausted();
}

// The stage lands before the seal is attempted: a seal the repository
// refuses fails the run after the amendment is staged, so `emery apply` can
// seal it once the repository answers, and nothing of the current revision
// moves.
#[tokio::test]
async fn apply_dirty_checkout() {
    let mut provider = Provider::answering([CLASSES_GROUPING, CLASSES_DRAFT]);
    bind(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);
    provider.vcs.commits.script(REVISION, Err(Error::Access("index.lock held".to_owned())));

    let resp = cli(
        &provider,
        &["emery", "--format", "json", "amend", DOCS, WIKI, "--apply", "--config", PROJECT_CONFIG],
    )
    .await;

    assert_eq!(resp.exit, 4, "{}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], "bad_gateway", "{envelope}");
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.contains("index.lock held")),
        "{envelope}"
    );
    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(amendment["base"]["revision"], revision, "the amendment stands staged");
    assert!(provider.storage.state(REVISION_KEY).is_none(), "the pointer does not move");
    assert!(
        !provider.vcs.calls().iter().any(|call| call.starts_with("label ")),
        "nothing is labelled"
    );
    provider.model.assert_exhausted();

    // once the repository answers, apply seals what the run left staged
    let provider = Provider::over(Arc::clone(&provider.storage), Vec::<String>::new());
    seed(&provider.files, SPEC, DESIGN, PLAN);
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "apply", "--config", PROJECT_CONFIG])
            .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["kind"], "amendment", "{envelope}");
    assert_eq!(envelope["amendment"], id, "{envelope}");
    assert_eq!(envelope["parent"], revision, "{envelope}");
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the slot is cleared");
}

// A change that stages nothing seals nothing under `--apply`: the run says
// so and the repository is read and never written.
#[tokio::test]
async fn amend_apply_no_change() {
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

    let resp =
        cli_ok(&provider, &["emery", "amend", DOCS, "--apply", "--config", PROJECT_CONFIG]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "nothing staged: no change against the current revision\n  base {revision} at {}\n  \
             0 added, 0 changed, 0 retired, 0 conflict, 4 unchanged\n",
            &HEAD[..8]
        )
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");
    assert!(provider.storage.state(REVISION_KEY).is_none(), "nothing is sealed");
    assert_eq!(provider.vcs.calls(), read_calls());
    assert!(provider.vcs.messages().is_empty(), "nothing is sealed");

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "amend", DOCS, "--apply", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["amendment"], Value::Null, "{envelope}");
    assert_eq!(envelope["staged"], false, "{envelope}");
    assert_eq!(envelope["applied"], Value::Null, "{envelope}");
    provider.model.assert_exhausted();
}

// An amend replaces an amendment staged before it, pruning the displaced
// document, and never a revision candidate: over one it is
// `amendment-pending` before any source is extracted, the candidate staying
// where it is.
#[tokio::test]
async fn amend_over_staged() {
    let mut provider = Provider::answering([CLASSES_GROUPING, CLASSES_DRAFT]);
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
async fn corrupt_baseline() {
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

// A `spec/` at the head that holds three documents reading as no revision —
// a merge of two branches that each sealed one, left with its conflict
// markers — was edited or merged by hand, since a seal writes the documents
// whole: the amend is refused `spec-mismatch` before any load, naming the
// commit and what failed to read, and the hint says to keep one side's
// three files whole and amend again over them.
#[tokio::test]
async fn amend_hand_merged() {
    let provider = Provider::idle();
    seed(&provider.files, HAND_MERGED, DESIGN, PLAN);

    let envelope =
        fail(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG], 1, "spec-mismatch")
            .await;

    let message = envelope["message"].as_str().expect("a message");
    assert!(
        message.starts_with(&format!(
            "`spec/` at commit `{HEAD}` holds documents that read as no revision (`spec` is not \
             JSON"
        )),
        "{envelope}"
    );
    assert!(message.contains("edited or merged by hand"), "{envelope}");
    let hint = envelope["hint"].as_str().expect("a hint");
    assert!(hint.contains("git checkout <branch> -- spec/"), "{envelope}");
    assert!(hint.contains("emery amend"), "{envelope}");
    assert!(provider.source.calls().is_empty(), "no source is extracted over a hand-merged tree");
    assert_eq!(provider.vcs.calls(), read_calls(), "read, and nothing written");
}

// `tests/build/spec.json` as a hand merge of two branches leaves it, the
// conflict markers committed, so it is no JSON.
const HAND_MERGED: &[u8] =
    b"<<<<<<< a\n{\"emery\": 8}\n=======\n{\"emery\": 8, \"parent\": null}\n>>>>>>> b\n";

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

// The verb is a public one: the top-level help and the completions list it
// beside the others, and its own help names its flags.
#[tokio::test]
async fn amend_shown() {
    let provider = Provider::idle();

    let help = cli_ok(&provider, &["emery", "amend", "--help"]).await;
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("--description"), "{help}");
    assert!(help.contains("--config"), "{help}");
    assert!(help.contains("--apply"), "{help}");

    let top = cli_ok(&provider, &["emery", "--help"]).await;
    let top = String::from_utf8_lossy(&top.stdout);
    assert!(verbs(&top).contains(&"amend"), "{top}");

    let completions = cli_ok(&provider, &["emery", "completions", "zsh"]).await;
    let completions = String::from_utf8_lossy(&completions.stdout);
    assert!(completions.contains("'apply:"), "{completions}");
    assert!(completions.contains("'amend:"), "the verb is completed: {completions}");
    assert!(completions.contains("amendment"), "the artifact the verb stages is: {completions}");
}

// Base-snapshot and standing-citation files stay hashed while present and drop
// once gone, and newly cited directories join under the sealed revision's id.
#[tokio::test]
async fn amend_snapshot() {
    let kept = "# Kept guide\n";
    let cited = "# Standing order notes\n";
    let added = "# Session timeout 30m\n";
    let sibling = "# Session refresh\n";

    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("spec JSON");
    base_spec["requirements"][2]["sources"][0]["path"] = Value::from("notes/cited.md#L1-L4");
    base_spec["requirements"][3]["sources"][0]["path"] = Value::from("notes/vanished.md#L1-L2");
    let base_bytes = serde_json::to_vec(&base_spec).expect("serialise base spec");

    let grouping = r#"{"groups": [
        {"claims": [1], "classes": [[1]]},
        {"claims": [0, 2], "classes": [[0], [2]]},
        {"claims": [3], "classes": [[3]]},
        {"claims": [4], "classes": [[4]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "session.timeout", "scenarios": [
            {"name": "Timeout", "when": "a session is idle for 30 minutes", "then": "it times out"}
        ]}
    ]}"#;

    let mut provider = Provider::answering([grouping, draft]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![
            requirement("session.timeout", "Sessions expire after 30 minutes of inactivity.")
                .at("rfcs/timeout.md#L1-L3"),
        ])),
    );
    provider.files.file(".", "guides/kept.md", kept);
    provider.files.file(".", "notes/cited.md", cited);
    provider.files.file(".", "rfcs/timeout.md", added);
    provider.files.file(".", "rfcs/refresh.md", sibling);

    let base = seed(&provider.files, &base_bytes, DESIGN, PLAN);
    seed_snapshot(
        &provider.storage,
        &base,
        &serde_json::json!({
            "docs": {
                "guides/kept.md": sha256("# Older guide\n"),
                "guides/gone.md": sha256("# Removed guide\n"),
            }
        }),
    );

    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "amend", DOCS, "--apply", "--config", PROJECT_CONFIG],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let sealed = envelope["applied"]["revision"].as_str().expect("the sealed id");
    assert_ne!(sealed, base);
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "staged sources.json is pruned");
    assert_eq!(
        snapshot(&provider.storage, sealed),
        serde_json::json!({
            "docs": {
                "guides/kept.md": sha256(kept),
                "notes/cited.md": sha256(cited),
                "rfcs/refresh.md": sha256(sibling),
                "rfcs/timeout.md": sha256(added),
            }
        }),
        "keeps existing base and cited files, drops gone ones, and adds the newly cited directory"
    );
    assert!(
        provider.storage.object(SNAPSHOTS_CONTAINER, &format!("{base}.json")).is_some(),
        "the base revision's snapshot stands alongside the amended one"
    );
    provider.model.assert_exhausted();
}

struct WholeWorkspace {
    spec: Vec<u8>,
    plan: Vec<u8>,
    base: BTreeMap<String, String>,
    current: BTreeMap<String, String>,
}

impl WholeWorkspace {
    fn new(spec: Vec<u8>, plan: Vec<u8>) -> Self {
        Self {
            spec,
            plan,
            base: BTreeMap::new(),
            current: BTreeMap::new(),
        }
    }

    fn standard() -> Self {
        Self::new(whole_base_spec(), PLAN.to_vec())
            .with_unchanged("src/auth.ts", "export function login() {}\n")
            .with_unchanged("src/session.ts", "export const IDLE_MINUTES = 60;\n")
            .with_unchanged("src/routes/orders.ts", "export function createOrder() {}\n")
            .with_unchanged("src/services/pricing.ts", "export const DISCOUNT_UNITS = 20;\n")
    }

    fn with_unchanged(mut self, path: impl Into<String>, body: impl Into<String>) -> Self {
        let path = path.into();
        let body = body.into();
        self.base.insert(path.clone(), sha256(&body));
        self.current.insert(path, body);
        self
    }

    fn with_changed(
        mut self, path: impl Into<String>, before: impl AsRef<str>, after: impl Into<String>,
    ) -> Self {
        let path = path.into();
        self.base.insert(path.clone(), sha256(before.as_ref()));
        self.current.insert(path, after.into());
        self
    }

    fn with_edited(mut self, path: impl Into<String>, after: impl Into<String>) -> Self {
        self.current.insert(path.into(), after.into());
        self
    }

    fn with_gone(mut self, path: impl Into<String>, before: impl AsRef<str>) -> Self {
        let path = path.into();
        self.base.insert(path.clone(), sha256(before.as_ref()));
        self.current.remove(&path);
        self
    }

    fn without_current(mut self, path: &str) -> Self {
        self.current.remove(path);
        self
    }

    fn seed(&self, provider: &Provider) -> String {
        let revision = self.seed_unrecorded(provider);
        seed_snapshot(&provider.storage, &revision, &serde_json::json!({ "code": self.base }));
        revision
    }

    fn seed_unrecorded(&self, provider: &Provider) -> String {
        for (path, body) in &self.current {
            provider.files.file(".", path, body);
        }
        seed(&provider.files, &self.spec, DESIGN, &self.plan)
    }
}

fn whole_base_spec() -> Vec<u8> {
    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
    let tpl = base_spec["requirements"][0].clone();
    base_spec["requirements"] = serde_json::json!([
        SpecReq::new("REQ-001", "auth.login", "Users sign in with a credential.")
            .at("src/auth.ts#L10-L20")
            .into_json(&tpl),
        SpecReq::new("REQ-002", "session.timeout", "Sessions expire after an hour of inactivity.")
            .at("src/session.ts#L5-L12")
            .into_json(&tpl),
        SpecReq::new("REQ-003", "orders.create", "A signed-in user places an order.")
            .at("src/routes/orders.ts#L20-L35")
            .route("POST /orders")
            .into_json(&tpl),
        SpecReq::new(
            "REQ-004",
            "orders.discount",
            "Gold-tier orders of 20 units receive a discount."
        )
        .at("src/services/pricing.ts#L8-L18")
        .into_json(&tpl),
    ]);
    serde_json::to_vec(&base_spec).expect("serialise base spec")
}

// A whole-scope behaviour source whose workspace files match the base snapshot
// drops every behavioural and prose claim, spending no model turn and leaving
// every requirement unchanged.
#[tokio::test]
async fn amend_whole_unchanged_file() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");

    let mut provider = Provider::idle();
    code_with(
        &mut provider,
        vec![
            requirement("auth.sign-in", "Callers authenticate with a credential.")
                .at("src/auth.ts#L10-L20"),
            requirement("session.idle-expiry", "Idle sessions expire after sixty minutes.")
                .at("src/session.ts#L5-L12"),
            requirement("orders.post.place", "Signed-in callers place orders.")
                .at("src/routes/orders.ts#L20-L35"),
            requirement(
                "orders.post.discount",
                "Orders of twenty or more units receive the gold discount.",
            )
            .at("src/services/pricing.ts#L8-L18"),
            claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders"))
                .at("src/routes/orders.ts#L20-L35"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
            Claim::new(ClaimKind::Call)
                .at("src/services/pricing.ts#L12")
                .with_extra("callee", "pg:Pool.query"),
            Claim::new(ClaimKind::Excerpt)
                .at("src/routes/orders.ts#L22-L25")
                .with_extra("excerpt", "Orders validate line quantities before pricing."),
        ],
    );
    WholeWorkspace::standard().seed(&provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["staged"], false, "{envelope}");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 0, "conflict": 0, "unchanged": 4, "by-scope": 0
        }),
        "{envelope}"
    );
    assert!(provider.model.seen().is_empty(), "no turn when every file matches the base snapshot");
}

// When one file changes in a whole-scope behaviour source, only claims from
// that file and the standing requirements in play reach the grouping turn; the
// rest settle unchanged by code, and a same-stem standing requirement citing
// an unchanged file stays unchanged rather than retiring by scope.
#[tokio::test]
async fn amend_whole_changed_file() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");

    // index 0 is the one kept claim (`orders.discount`); 1 and 2 are the two
    // `orders.*` standing requirements in play (`REQ-003`, `REQ-004`)
    let grouping = r#"{"groups": [
        {"claims": [1], "classes": [[1]]},
        {"claims": [0, 2], "classes": [[0], [2]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.discount", "scenarios": [
            {"name": "Gold discount", "when": "a gold-tier order has 10 units", "then": "the discount applies"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    code_with(
        &mut provider,
        vec![
            requirement("auth.login", "Users sign in with a credential.").at("src/auth.ts#L10-L20"),
            requirement("session.timeout", "Sessions expire after an hour of inactivity.")
                .at("src/session.ts#L5-L12"),
            requirement("orders.create", "A signed-in user places an order.")
                .at("src/routes/orders.ts#L20-L35"),
            requirement("orders.discount", "Gold-tier orders of 10 units receive a discount.")
                .at("src/services/pricing.ts#L8-L18"),
            claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders"))
                .at("src/routes/orders.ts#L20-L35"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        ],
    );
    let workspace = WholeWorkspace::standard()
        .with_edited("src/services/pricing.ts", "export const DISCOUNT_UNITS = 10;\n");
    workspace.seed(&provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 1, "retired": 0, "conflict": 0, "unchanged": 3, "by-scope": 0
        }),
        "{envelope}"
    );

    let request = provider.model.seen()[0].messages.join("\n");
    assert!(request.contains("- 0 `code` `orders.discount`"), "{request}");
    assert!(!request.contains("`auth.login`"), "unchanged file claim dropped: {request}");
    assert!(!request.contains("`session.timeout`"), "unchanged file claim dropped: {request}");
    assert!(!request.contains("- 1 `code`"), "only one extracted claim kept: {request}");
    assert!(request.contains("- 1 `orders.create`"), "{request}");
    assert!(request.contains("- 2 `orders.discount`"), "{request}");

    let base_spec: Value = serde_json::from_slice(&workspace.spec).expect("base spec JSON");
    let (_, amendment) = staged_amendment(&provider);
    for idx in [0, 1, 2] {
        assert_eq!(amendment["changes"][idx]["class"], "unchanged", "{amendment}");
        assert_eq!(
            amendment["spec"]["requirements"][idx], base_spec["requirements"][idx],
            "{amendment}"
        );
    }
    assert_eq!(amendment["changes"][3]["class"], "changed", "{amendment}");
    provider.model.assert_exhausted();
}

fn gone_file_spec_and_plan() -> (Vec<u8>, Vec<u8>) {
    let mut base_spec: Value = serde_json::from_slice(SPEC).expect("base spec JSON");
    let tpl = base_spec["requirements"][0].clone();
    base_spec["requirements"] = serde_json::json!([
        SpecReq::new(
            "REQ-001",
            "start.require-database-url",
            "The program refuses to start when DATABASE_URL is unset.",
        )
        .at("src/server.ts#L10-L14")
        .into_json(&tpl),
        SpecReq::new("REQ-002", "orders.create", "A signed-in user places an order.")
            .at("src/routes/orders.ts#L20-L35")
            .route("POST /orders")
            .into_json(&tpl),
        SpecReq::new("REQ-003", "returns.post.create", "A signed-in user submits a return.")
            .at("src/routes/returns.ts#L15-L30")
            .route("POST /returns")
            .into_json(&tpl),
        SpecReq::new(
            "REQ-004",
            "returns.validate-window",
            "A return outside the thirty-day window is rejected.",
        )
        .at("src/services/returns.ts#L10-L24")
        .into_json(&tpl),
    ]);
    let mut base_plan: Value = serde_json::from_slice(PLAN).expect("base plan JSON");
    base_plan["parts"][0]["name"] = Value::from("start");
    base_plan["parts"][0]["requirements"] = serde_json::json!(["REQ-001"]);
    base_plan["parts"][1]["requirements"] = serde_json::json!(["REQ-002"]);
    base_plan["parts"].as_array_mut().expect("parts").push(serde_json::json!({
        "id": "PART-003",
        "name": "returns",
        "requirements": ["REQ-003", "REQ-004"],
        "types": [],
        "uses": [],
        "depends-on": [],
        "brief": ["Delivers customer returns."]
    }));
    (
        serde_json::to_vec(&base_spec).expect("serialise base spec"),
        serde_json::to_vec(&base_plan).expect("serialise base plan"),
    )
}

// Removing a stem's files from a whole-scope behaviour source retires the
// standing requirements citing those removed files by scope, drops a shared
// helper claim from an unchanged file before the grouping turn, and leaves
// every requirement outside the removed stem unchanged.
#[tokio::test]
async fn amend_whole_gone_file() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");
    let (spec, plan) = gone_file_spec_and_plan();

    // index 0 is `start.require-database-url` from changed `src/server.ts`;
    // 1..=3 are `REQ-001`, `REQ-003`, `REQ-004` (`REQ-002` is settled out of play)
    let grouping = r#"{"groups": [
        {"claims": [0, 1], "classes": [[0, 1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let mut provider = Provider::answering([grouping]);
    code_with(
        &mut provider,
        vec![
            requirement(
                "start.require-database-url",
                "The program refuses to start when DATABASE_URL is unset.",
            )
            .at("src/server.ts#L10-L14"),
            requirement("orders.create", "A signed-in user places an order.")
                .at("src/routes/orders.ts#L20-L35"),
            requirement("validation.positive-int", "Quantities must be positive integers.")
                .at("src/lib/validation.ts#L1-L3"),
            claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders"))
                .at("src/routes/orders.ts#L20-L35"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        ],
    );
    WholeWorkspace::new(spec, plan)
        .with_changed(
            "src/server.ts",
            "app.use('/orders', orders); app.use('/returns', returns);\n",
            "app.use('/orders', orders);\n",
        )
        .with_unchanged("src/routes/orders.ts", "export function createOrder() {}\n")
        .with_unchanged(
            "src/lib/validation.ts",
            "export function requirePositiveInt(n: number) { return n > 0; }\n",
        )
        .with_gone("src/routes/returns.ts", "export function createReturn() {}\n")
        .with_gone("src/services/returns.ts", "export function validateReturnWindow() {}\n")
        .seed(&provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 2, "conflict": 0, "unchanged": 2, "by-scope": 2
        }),
        "{envelope}"
    );
    assert_eq!(provider.model.seen().len(), 1, "grouping is the only turn");
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(
        !request.contains("validation.positive-int"),
        "shared helper claim in unchanged file is dropped: {request}"
    );
    assert!(!request.contains("`orders.create`"), "`orders.create` settled by code: {request}");
    assert!(request.contains("`returns.post.create`"), "{request}");
    assert!(request.contains("`returns.validate-window`"), "{request}");

    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(amendment["changes"][0]["class"], "unchanged", "{amendment}");
    assert_eq!(amendment["changes"][1]["class"], "unchanged", "{amendment}");
    assert_eq!(amendment["changes"][2]["class"], "retired", "{amendment}");
    assert_eq!(amendment["changes"][2]["scope"], "code", "{amendment}");
    assert_eq!(amendment["changes"][3]["class"], "retired", "{amendment}");
    assert_eq!(amendment["changes"][3]["scope"], "code", "{amendment}");
    provider.model.assert_exhausted();
}

// Changing a constant file that anchors a `criterion` keeps the leading
// `requirement` claim from an unchanged handler file under `revision::leads`,
// so the affected requirement is revised without dropping its statement.
#[tokio::test]
async fn amend_whole_constant_elsewhere() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");

    // index 0 is `orders.discount` (kept via `leads` from `src/services/pricing.ts`),
    // 1 and 2 are the `orders.*` standing requirements in play (`REQ-003`, `REQ-004`)
    let grouping = r#"{"groups": [
        {"claims": [1], "classes": [[1]]},
        {"claims": [0, 2], "classes": [[0], [2]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.discount", "scenarios": [
            {"name": "Gold discount", "when": "a gold-tier order has 10 units", "then": "the discount applies"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    code_with(
        &mut provider,
        vec![
            requirement("auth.login", "Users sign in with a credential.").at("src/auth.ts#L10-L20"),
            requirement("session.timeout", "Sessions expire after an hour of inactivity.")
                .at("src/session.ts#L5-L12"),
            requirement("orders.create", "A signed-in user places an order.")
                .at("src/routes/orders.ts#L20-L35"),
            requirement(
                "orders.discount",
                "Gold-tier orders meeting DISCOUNT_UNITS receive a discount.",
            )
            .at("src/services/pricing.ts#L8-L18"),
            claim(
                ClaimKind::Criterion,
                "orders.discount.threshold",
                ("criterion", "DISCOUNT_UNITS is 10."),
            )
            .at("src/constants.ts#L1"),
            claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders"))
                .at("src/routes/orders.ts#L20-L35"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        ],
    );
    WholeWorkspace::standard()
        .with_changed(
            "src/constants.ts",
            "export const DISCOUNT_UNITS = 20;\n",
            "export const DISCOUNT_UNITS = 10;\n",
        )
        .seed(&provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 1, "retired": 0, "conflict": 0, "unchanged": 3, "by-scope": 0
        }),
        "{envelope}"
    );

    let request = provider.model.seen()[0].messages.join("\n");
    assert!(request.contains("- 0 `code` `orders.discount`"), "{request}");
    assert!(
        request.contains("criterion `orders.discount.threshold`: DISCOUNT_UNITS is 10."),
        "{request}"
    );
    assert!(!request.contains("`code` `orders.create`"), "{request}");
    assert!(!request.contains("`auth.login`"), "{request}");
    provider.model.assert_exhausted();
}

// An unchanged whole-scope behaviour source beside a delta prose source
// stating a `decision` spends no grouping turn, retires nothing, keeps
// unchanged whole-source contracts/calls out of the design turn, and amends
// the design section.
#[tokio::test]
async fn amend_whole_beside_delta() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[[source]]\nadapter = \"{CODE}\"\nscope = \"whole\"\n\n[[source]]\nadapter = \
         \"{DOCS}\"\nscope = \"delta\"\n\n[target]\nadapter = \"acme:builder@2.1.0\"\nrepository \
         = \".\"\n"
    ));

    let design = r#"{"sections": [
        {"kind": "overview", "blocks": [{"text": "Sign-in issues a session; orders are placed and cancelled under it."}], "revised": []},
        {"kind": "apis", "blocks": [{"text": "Secrets live in the team vault (from docs)."}], "revised": []}
    ]}"#;
    let mut provider = Provider::answering([design]);
    code_with(
        &mut provider,
        vec![
            requirement("auth.login", "Users sign in with a credential.").at("src/auth.ts#L10-L20"),
            requirement("session.timeout", "Sessions expire after an hour of inactivity.")
                .at("src/session.ts#L5-L12"),
            requirement("orders.create", "A signed-in user places an order.")
                .at("src/routes/orders.ts#L20-L35"),
            requirement("orders.discount", "Gold-tier orders of 20 units receive a discount.")
                .at("src/services/pricing.ts#L8-L18"),
            claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders"))
                .at("src/routes/orders.ts#L20-L35"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
            Claim::new(ClaimKind::Call)
                .at("src/services/pricing.ts#L12")
                .with_extra("callee", "node-fetch:fetch"),
        ],
    );
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![claim(ClaimKind::Decision, "secrets.vault", ("decision", VAULT))])),
    );
    WholeWorkspace::standard().seed(&provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 0, "conflict": 0, "unchanged": 4, "by-scope": 0
        }),
        "{envelope}"
    );
    assert_eq!(
        envelope["sections"],
        serde_json::json!({"preamble": false, "added": ["apis"], "removed": [], "changed": []}),
        "{envelope}"
    );
    assert_eq!(provider.model.seen().len(), 1, "design amend is the only turn; no grouping");
    let design_turn = provider.model.seen()[0].messages.join("\n");
    assert!(design_turn.contains(VAULT), "{design_turn}");
    assert!(!design_turn.contains("node-fetch"), "unchanged code call dropped: {design_turn}");
    assert!(
        !design_turn.contains("POST /orders"),
        "unchanged code contract dropped: {design_turn}"
    );
    provider.model.assert_exhausted();
}

// When no snapshot is recorded for the base revision (for example, on a fresh
// clone), a whole-scope behaviour source lists every extracted claim and every
// standing requirement and retires untouched requirements under covered stems.
#[tokio::test]
async fn amend_whole_unrecorded() {
    let scratch = Scratch::new();
    let config = scoped(&scratch, CODE, "whole");

    // 0..=2 are extracted claims; 3..=6 are all four standing requirements
    let grouping = r#"{"groups": [
        {"claims": [0, 3], "classes": [[0, 3]]},
        {"claims": [1, 4], "classes": [[1, 4]]},
        {"claims": [2, 5], "classes": [[2, 5]]},
        {"claims": [6], "classes": [[6]]}
    ]}"#;
    let mut provider = Provider::answering([grouping]);
    code_with(
        &mut provider,
        vec![
            requirement("auth.login", "Users sign in with a credential.").at("src/auth.ts#L10-L20"),
            requirement("session.timeout", "Sessions expire after an hour of inactivity.")
                .at("src/session.ts#L5-L12"),
            requirement("orders.create", "A signed-in user places an order.")
                .at("src/routes/orders.ts#L20-L35"),
            claim(ClaimKind::Contract, "orders.post", ("route", "POST /orders"))
                .at("src/routes/orders.ts#L20-L35"),
            claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
            claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
        ],
    );
    WholeWorkspace::standard()
        .without_current("src/services/pricing.ts")
        .seed_unrecorded(&provider);

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "amend", "--config", &config]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        envelope["tally"],
        serde_json::json!({
            "added": 0, "changed": 0, "retired": 1, "conflict": 0, "unchanged": 3, "by-scope": 1
        }),
        "{envelope}"
    );
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(request.contains("- 0 `code` `auth.login`"), "{request}");
    assert!(request.contains("- 1 `code` `session.timeout`"), "{request}");
    assert!(request.contains("- 2 `code` `orders.create`"), "{request}");
    assert!(request.contains("- 3 `auth.login`"), "{request}");
    assert!(request.contains("- 6 `orders.discount`"), "{request}");
    provider.model.assert_exhausted();
}
