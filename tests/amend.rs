//! Verifies `emery amend` over scripted sources, storage, and version control.

#![cfg(not(target_arch = "wasm32"))]

mod support;
#[path = "support/verbs.rs"]
mod verbs;

use emery_adapter::source::{ClaimKind, SourceKind};
use emery_engine::vcs::REVISION;
use emery_engine::{STAGED_CONTAINER, STAGED_KEY};
use omnia_test::SeenFormat;
use serde_json::Value;
use support::provider::{Provider, claim, cli, evidence, requirement, retire};
use support::scratch::Scratch;
use support::store::{seed, stage, stage_amendment};
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
const PLAN: &[u8] = include_bytes!("build/plan.json");
const CHAIN_PLAN: &[u8] = include_bytes!("amend/chain-plan.json");
const CLASSES: &str = include_str!("amend/classes.json");
const RETIRED: &str = include_str!("amend/retired.json");
const SCOPED: &str = include_str!("amend/scoped.json");
const DESIGNED: &str = include_str!("amend/designed.json");
const AMENDMENT: &[u8] = include_bytes!("specify/6-amendment.json");

const WITHDRAWN: &str = "Cancelling an open order is withdrawn.";
// The one paragraph of `DESIGN`'s overview.
const OVERVIEW: &str = "Sign-in issues a session; orders are placed and cancelled under it.";
// `DESIGN`'s domain model, the two types `PLAN`'s `orders` slice owns, for a
// base design a scenario spells with sections of its own.
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
// since nothing is retired, the added requirement joins the slice of its
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
             slices: orders\n  nothing sealed; `emery show amendment` reviews it\n",
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
    assert_eq!(
        envelope["slices"],
        serde_json::json!([{"id": "SLICE-002", "name": "orders", "class": "changed"}]),
        "{envelope}"
    );

    // what was staged is what show renders: a changed requirement's old and
    // new statement and its redrafted scenarios, an unchanged one's
    // statement alone, no design section, and the slice the added
    // requirement joined with its types and brief as they stood
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(!shown.contains("## Design"), "{shown}");
    assert!(
        shown.ends_with(
            "## Plan\n\n### Slice: orders [changed]\n\nID: SLICE-002\nRequirements: [REQ-003, \
             REQ-004, REQ-005]\nTypes: [orders.line, orders.order]\nDepends on: \
             [SLICE-001]\n\nDelivers order creation and cancellation for a signed-in caller; \
             verified by placing and cancelling an order under a session from \
             `authentication`.\n"
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
// nothing is drafted anew. The requirement leaves its slice, which shrinks
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
             conflict, 3 unchanged\n  stems: orders\n  slices: orders\n  nothing sealed; `emery \
             show amendment` reviews it\n",
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
            "## Plan\n\n### Slice: orders [changed]\n\nID: SLICE-002\nRequirements: \
             [REQ-003]\nTypes: [orders.line, orders.order]\nDepends on: [SLICE-001]\n\nDelivers \
             order creation and cancellation for a signed-in caller; verified by placing and \
             cancelling an order under a session from `authentication`.\n"
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

// Retiring every requirement of a slice removes the slice, and the design
// types it owned leave the design with it, a section left with no block
// going too: the amendment lists the slice removed and the section removed,
// the output names both, `show amendment` renders the two sections, and
// the grouping is the one turn spent.
#[tokio::test]
async fn amend_retired_slice() {
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
             conflict, 2 unchanged\n  stems: orders\n  design: domain-model\n  slices: orders \
             (removed)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 1, "the grouping is the one turn");

    // the plan keeps the slice left standing and the preamble, the design
    // its overview alone, each under the base as parent
    let plan = &amendment["plan"];
    assert_eq!(plan["parent"], revision, "{amendment}");
    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    assert_eq!(plan["preamble"], base["preamble"], "{amendment}");
    assert_eq!(plan["slices"].as_array().map(Vec::len), Some(1), "{amendment}");
    assert_eq!(plan["slices"][0], base["slices"][0], "{amendment}");
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
        amendment["slices"],
        serde_json::json!([{"id": "SLICE-002", "name": "orders", "class": "removed"}]),
        "{amendment}"
    );

    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.ends_with(
            "## Design\n\nRemoved: Domain model.\n\n## Plan\n\n### Slice: orders [removed]\n\nID: \
             SLICE-002\n"
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
    assert_eq!(envelope["slices"], amendment["slices"], "{envelope}");
    provider.model.assert_exhausted();
}

// A removed slice that stood between two others leaves its dependants
// depending on what it depended on, so the order it held survives it: the
// slice after it is changed in its dependencies alone, and the design,
// which the removed slice owned nothing of, stands.
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
             conflict, 3 unchanged\n  stems: session\n  slices: sessions (removed), orders\n  \
             nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    let base: Value = serde_json::from_slice(CHAIN_PLAN).expect("the base plan is JSON");
    let slices = amendment["plan"]["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 2, "{amendment}");
    assert_eq!(slices[0], base["slices"][0], "{amendment}");
    assert_eq!(slices[1]["id"], "SLICE-003", "{amendment}");
    assert_eq!(slices[1]["depends-on"], serde_json::json!(["SLICE-001"]), "{amendment}");
    assert_eq!(slices[1]["requirements"], base["slices"][2]["requirements"], "{amendment}");
    assert_eq!(slices[1]["types"], base["slices"][2]["types"], "{amendment}");
    assert_eq!(slices[1]["brief"], base["slices"][2]["brief"], "{amendment}");
    assert_eq!(
        amendment["slices"],
        serde_json::json!([
            {"id": "SLICE-002", "name": "sessions", "class": "removed"},
            {"id": "SLICE-003", "name": "orders", "class": "changed"}
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
        r#"{{"emery": 8, "parent": null, "preamble": [], "sections": [
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
// named; the types the emptied slice owned leave the design by code before
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
             slices: orders (removed)\n  nothing sealed; `emery show amendment` reviews it\n",
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
// kept verbatim, and the slice shrinks keeping its types.
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
             conflict, 3 unchanged\n  stems: orders\n  design: apis\n  slices: orders\n  nothing \
             sealed; `emery show amendment` reviews it\n",
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
        amendment["plan"]["slices"][1]["types"],
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
        r#"{{"emery": 8, "parent": null, "preamble": [], "sections": [
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

// A behaviour source read whole retires what it falls silent on: under the
// stems its requirement claims cover, a standing requirement no claim of
// the change touched is retired with the source named in its place, the
// grouping the one turn spent. The entry is listed apart in the counts and
// under `## Retired` with a note, and the requirement leaves its slice as
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
             by scope), 0 conflict, 3 unchanged\n  stems: orders\n  slices: orders\n  nothing \
             sealed; `emery show amendment` reviews it\n",
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
    assert!(
        shown.contains("### Slice: orders [changed]\n\nID: SLICE-002\nRequirements: [REQ-003]")
    );

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
    provider.source.kinds.insert("acme:code".to_owned(), SourceKind::Behaviour);
    provider.source.evidence.insert(
        "code".to_owned(),
        Ok(evidence(vec![requirement("orders.create", "A signed-in user places an order.")])),
    );
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
        {"claims": [0, 4], "classes": [[0, 4]]},
        {"claims": [1], "classes": [[1]]},
        {"claims": [2], "classes": [[2]]},
        {"claims": [3], "classes": [[3]]}
    ]}"#;
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.cancellation", "scenarios": [
            {"name": "Cancel", "when": "a signed-in caller cancels an order", "then": "the order is cancelled"}
        ]}
    ]}"#;
    let mut provider = Provider::answering([grouping, draft]);
    let mut anchored =
        requirement("orders.cancellation", "A signed-in user cancels an open order.");
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
// and takes the new subject, so the requirement is changed and moves slice:
// it leaves the slice of its old stem, which shrinks and keeps its types,
// for a slice the slicing turn drafts over the current plan, numbered after
// the base's, with the base's trigger carried. The turn's brief lists the
// current slices fixed, the open stem with its trigger, and the requirement
// it places as drafted.
#[tokio::test]
async fn amend_moved() {
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
    let slicing = r#"{"slices": [
        {"name": "reporting", "requirements": ["REQ-004"], "types": [], "depends-on": ["orders"],
         "brief": ["Reports a cancellation; edits the cancel handler, so it compiles against `orders`."]}
    ], "joins": []}"#;
    let mut provider = Provider::answering([grouping, draft, slicing]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![requirement(
            "reporting.cancel",
            "A signed-in user cancels an open order.",
        )])),
    );
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  0 added, 1 changed, 0 retired, 0 \
             conflict, 3 unchanged\n  stems: reporting\n  slices: orders, reporting (new)\n  \
             nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 3, "the grouping, the draft, and the slicing");
    let draft = provider.model.seen()[1].messages.join("\n");
    assert!(draft.contains("- REQ-004 `reporting.cancel`"), "{draft}");

    // the slicing turn: the current slices fixed, the old one touched, the
    // open stem with the trigger it carries, and the requirement it places
    let request = provider.model.seen()[2].messages.join("\n");
    assert!(request.starts_with("Amend the build plan"), "{request}");
    assert!(
        request.contains(
            "## Current plan\n\n- `SLICE-001` `authentication` — REQ-001, REQ-002\n- `SLICE-002` \
             `orders` — REQ-003 — types: `orders.line`, `orders.order` — depends on: \
             `authentication` — touched by this amendment\n\n## Open\n"
        ),
        "{request}"
    );
    assert!(
        request.contains("- `reporting` — REQ-004 — triggers: `orders.cancel-requested`\n"),
        "{request}"
    );
    assert!(!request.contains("Type keys the design holds"), "{request}");
    assert!(request.contains("## Requirements\n\nThe requirements the change adds"), "{request}");
    assert!(request.contains("- REQ-004 `reporting.cancel` — Status: unknown"), "{request}");

    // the change keeps the id under the new subject, with the base's trigger
    let moved = &amendment["changes"][3];
    assert_eq!(moved["id"], "REQ-004", "{amendment}");
    assert_eq!(moved["subject"], "reporting.cancel", "{amendment}");
    assert_eq!(moved["class"], "changed", "{amendment}");
    let drafted = &amendment["spec"]["requirements"][3];
    assert_eq!(drafted["subject"], "reporting.cancel", "{amendment}");
    assert_eq!(drafted["trigger"], serde_json::json!({"message": "orders.cancel-requested"}));

    // the plan: the old slice shrunk with its types, the new one after it
    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    let slices = amendment["plan"]["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 3, "{amendment}");
    assert_eq!(slices[0], base["slices"][0], "{amendment}");
    assert_eq!(slices[1]["requirements"], serde_json::json!(["REQ-003"]), "{amendment}");
    assert_eq!(slices[1]["types"], base["slices"][1]["types"], "{amendment}");
    assert_eq!(slices[1]["brief"], base["slices"][1]["brief"], "{amendment}");
    assert_eq!(
        slices[2],
        serde_json::json!({
            "id": "SLICE-003", "name": "reporting", "requirements": ["REQ-004"],
            "types": [], "depends-on": ["SLICE-002"],
            "brief": ["Reports a cancellation; edits the cancel handler, so it compiles against `orders`."]
        }),
        "{amendment}"
    );
    assert_eq!(
        amendment["slices"],
        serde_json::json!([
            {"id": "SLICE-002", "name": "orders", "class": "changed"},
            {"id": "SLICE-003", "name": "reporting", "class": "added"}
        ]),
        "{amendment}"
    );
    assert!(amendment["sections"]["changed"].as_array().is_some_and(Vec::is_empty), "{amendment}");
    provider.model.assert_exhausted();
}

// A change restating every requirement of a slice under another stem empties
// the slice, which is removed, and the types it owned are open with the new
// stem: the slicing turn's brief lists them, the new slice owns them, and
// the design stands.
#[tokio::test]
async fn amend_moved_slice() {
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
    let slicing = r#"{"slices": [
        {"name": "reporting", "requirements": ["REQ-003", "REQ-004"],
         "types": ["orders.line", "orders.order"], "depends-on": [], "brief": []}
    ], "joins": []}"#;
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
             conflict, 2 unchanged\n  stems: reporting\n  slices: orders (removed), reporting \
             (new)\n  nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 3, "the grouping, the draft, and the slicing");
    let request = provider.model.seen()[2].messages.join("\n");
    assert!(
        request
            .contains("## Current plan\n\n- `SLICE-001` `authentication` — REQ-001, REQ-002\n\n"),
        "the emptied slice is gone: {request}"
    );
    assert!(
        request.contains(
            "- `reporting` — REQ-003, REQ-004 — triggers: `POST /orders`, \
             `orders.cancel-requested`\n"
        ),
        "{request}"
    );
    assert!(request.contains("Type keys the design holds that no current slice owns"), "{request}");
    assert!(request.contains("- `orders.line`\n- `orders.order`\n"), "{request}");
    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    let slices = amendment["plan"]["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 2, "{amendment}");
    assert_eq!(slices[0], base["slices"][0], "{amendment}");
    assert_eq!(
        slices[1],
        serde_json::json!({
            "id": "SLICE-003", "name": "reporting", "requirements": ["REQ-003", "REQ-004"],
            "types": ["orders.line", "orders.order"], "depends-on": [], "brief": []
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
            "## Plan\n\n### Slice: orders [removed]\n\nID: SLICE-002\n\n### Slice: reporting \
             [added]\n\nID: SLICE-003\nRequirements: [REQ-003, REQ-004]\nTypes: [orders.line, \
             orders.order]\n"
        ),
        "{shown}"
    );
    provider.model.assert_exhausted();
}

// The S1-shaped change: a documentation source stating two requirements
// under a stem no slice holds. The grouping and the draft run as before,
// then one slicing turn over the current plan drafts the new slice, which
// the engine numbers after the base's and whose dependency it maps to the
// slice's id, the schema holding the answer to the open ids and the current
// slice ids; no design turn is spent, since a requirement informs no
// section.
#[tokio::test]
async fn amend_sliced() {
    let slicing = r#"{"slices": [
        {"name": "rate-limit", "requirements": ["REQ-005", "REQ-006"], "types": [], "depends-on": ["orders"],
         "brief": ["Delivers the per-caller limit on order placement; edits the order handler, so it compiles against `orders`."]}
    ], "joins": []}"#;
    let (provider, revision) = limited([slicing]);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  2 added, 0 changed, 0 retired, 0 \
             conflict, 4 unchanged\n  stems: rate-limit\n  slices: rate-limit (new)\n  nothing \
             sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 3, "the grouping, the draft, and the slicing");
    let turn = &provider.model.seen()[2];
    let request = turn.messages.join("\n");
    assert!(request.contains("- `rate-limit` — REQ-005, REQ-006\n"), "{request}");
    assert!(
        request.contains("- `SLICE-002` `orders` — REQ-003, REQ-004 — types:"),
        "untouched: {request}"
    );
    assert!(!request.contains("touched by this amendment"), "{request}");
    assert!(request.contains("- REQ-005 `rate-limit.count` — Status: unknown"), "{request}");
    assert!(request.contains("- REQ-006 `rate-limit.refuse` — Status: unknown"), "{request}");
    assert!(!request.contains("The standing requirements the answer places"), "{request}");
    let SeenFormat::Schema { name, schema } = &turn.format else {
        panic!("the slicing is steered by schema");
    };
    assert_eq!(name, "slicing");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["slices"]["maxItems"], 2, "{schema}");
    assert_eq!(schema["properties"]["joins"]["maxItems"], 2, "{schema}");
    assert_eq!(
        schema["$defs"]["Draft"]["properties"]["requirements"]["items"]["enum"],
        serde_json::json!(["REQ-005", "REQ-006"]),
        "{schema}"
    );
    assert_eq!(schema["$defs"]["Draft"]["properties"]["types"]["maxItems"], 0, "{schema}");
    assert_eq!(
        schema["$defs"]["Join"]["properties"]["slice"]["enum"],
        serde_json::json!(["SLICE-001", "SLICE-002"]),
        "{schema}"
    );

    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    let slices = amendment["plan"]["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 3, "{amendment}");
    assert_eq!(slices[0], base["slices"][0], "{amendment}");
    assert_eq!(slices[1], base["slices"][1], "{amendment}");
    assert_eq!(
        slices[2],
        serde_json::json!({
            "id": "SLICE-003", "name": "rate-limit", "requirements": ["REQ-005", "REQ-006"],
            "types": [], "depends-on": ["SLICE-002"],
            "brief": ["Delivers the per-caller limit on order placement; edits the order handler, so it compiles against `orders`."]
        }),
        "{amendment}"
    );
    assert_eq!(amendment["plan"]["preamble"], base["preamble"], "{amendment}");
    assert_eq!(
        amendment["slices"],
        serde_json::json!([{"id": "SLICE-003", "name": "rate-limit", "class": "added"}]),
        "{amendment}"
    );
    assert_eq!(provider.vcs.calls(), read_calls());
    provider.model.assert_exhausted();
}

// The same change answered by joining the new stem into a current slice:
// the slice holds the two beside its own, keeps its id, name, types, and
// brief, and no slice is added.
#[tokio::test]
async fn amend_sliced_joined() {
    let slicing = r#"{"slices": [], "joins": [{"slice": "SLICE-002", "requirements": ["REQ-005", "REQ-006"]}]}"#;
    let (provider, _) = limited([slicing]);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (_, amendment) = staged_amendment(&provider);
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("\n  slices: orders\n"), "{stdout}");
    let base: Value = serde_json::from_slice(PLAN).expect("the base plan is JSON");
    let slices = amendment["plan"]["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 2, "{amendment}");
    assert_eq!(slices[0], base["slices"][0], "{amendment}");
    assert_eq!(
        slices[1]["requirements"],
        serde_json::json!(["REQ-003", "REQ-004", "REQ-005", "REQ-006"]),
        "{amendment}"
    );
    assert_eq!(slices[1]["types"], base["slices"][1]["types"], "{amendment}");
    assert_eq!(slices[1]["brief"], base["slices"][1]["brief"], "{amendment}");
    assert_eq!(
        amendment["slices"],
        serde_json::json!([{"id": "SLICE-002", "name": "orders", "class": "changed"}]),
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

// The slicing answer is held to the open set and the plan's rules: one that
// moves a requirement the current plan places, places an open one twice, or
// names a new slice as a current one is refused naming each, and the
// correction lands.
#[tokio::test]
async fn amend_sliced_refused() {
    let refused = r#"{"slices": [
        {"name": "orders", "requirements": ["REQ-005"], "types": [], "depends-on": [], "brief": []}
    ], "joins": [
        {"slice": "SLICE-001", "requirements": ["REQ-003"]},
        {"slice": "SLICE-002", "requirements": ["REQ-005", "REQ-006"]}
    ]}"#;
    let corrected = r#"{"slices": [], "joins": [{"slice": "SLICE-002", "requirements": ["REQ-005", "REQ-006"]}]}"#;
    let (provider, _) = limited([refused, corrected]);

    cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    assert_eq!(provider.model.seen().len(), 4, "the grouping, the draft, and two slicing rounds");
    let exchanges = provider.model.exchanges();
    let finding = exchanges[2].outcome.as_ref().expect_err("the first answer is refused");
    for expected in [
        "`orders` names a current slice; list what joins it under `joins`",
        "`joins` `SLICE-001`: `REQ-003` is not open to this amendment; the current plan places \
         it, and it stays",
        "`REQ-005` appears in more than one slice",
    ] {
        assert!(finding.contains(expected), "{expected}: {finding}");
    }
    assert!(exchanges[3].outcome.is_ok(), "{exchanges:?}");
    let (_, amendment) = staged_amendment(&provider);
    assert_eq!(
        amendment["plan"]["slices"][1]["requirements"],
        serde_json::json!(["REQ-003", "REQ-004", "REQ-005", "REQ-006"]),
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

// The S4-shaped change: two requirements under two new stems beside two
// `type` claims. The design turn redrafts `## Domain model` with the new
// keys, which no current slice owns, so they are open with the stems: the
// slicing turn's brief lists them in the design's order, and each new slice
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
    let slicing = r#"{"slices": [
        {"name": "rate-limit", "requirements": ["REQ-005"], "types": ["rate-limit.counter"], "depends-on": ["orders"], "brief": []},
        {"name": "audit", "requirements": ["REQ-006"], "types": ["audit.entry"], "depends-on": [], "brief": []}
    ], "joins": []}"#;
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
             conflict, 4 unchanged\n  stems: audit, rate-limit\n  design: domain-model\n  slices: \
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
            "Type keys the design holds that no current slice owns, each to be owned by \
             exactly one slice"
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

    let slices = amendment["plan"]["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 4, "{amendment}");
    assert_eq!(slices[2]["id"], "SLICE-003", "{amendment}");
    assert_eq!(slices[2]["name"], "rate-limit", "{amendment}");
    assert_eq!(slices[2]["types"], serde_json::json!(["rate-limit.counter"]), "{amendment}");
    assert_eq!(slices[2]["depends-on"], serde_json::json!(["SLICE-002"]), "{amendment}");
    assert_eq!(slices[3]["id"], "SLICE-004", "{amendment}");
    assert_eq!(slices[3]["name"], "audit", "{amendment}");
    assert_eq!(slices[3]["types"], serde_json::json!(["audit.entry"]), "{amendment}");
    provider.model.assert_exhausted();
}

// The S1-shaped change: a documentation source stating two requirements
// under the stem `rate-limit`, which no slice holds, scripted through the
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

// A stem the amendment leaves past the slice cap is cut again along its
// sub-stems by the slicing turn: every requirement of the stem is open with
// the types its slice owned, the current slice keeps its id and takes some
// back, and the rest become a new slice. An answer cutting the stem finer
// than the cap asks is refused naming the pair that would fit one slice,
// and the correction lands.
#[tokio::test]
async fn amend_recut() {
    let (spec, plan) = wide(12);
    // index 0 is `docs`, 1..=12 the standing requirements in id order
    let groups: Vec<Value> = (0..=12)
        .map(|index| serde_json::json!({"claims": [index], "classes": [[index]]}))
        .collect();
    let grouping = serde_json::json!({"groups": groups}).to_string();
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "orders.refund", "scenarios": [
            {"name": "Refund", "when": "a signed-in caller requests a refund", "then": "the refund is recorded"}
        ]}
    ]}"#;
    let fine = r#"{"slices": [
        {"name": "orders-mid", "requirements": ["REQ-005", "REQ-006", "REQ-007", "REQ-008"], "types": [], "depends-on": [], "brief": []},
        {"name": "orders-late", "requirements": ["REQ-009", "REQ-010", "REQ-011", "REQ-012", "REQ-013"], "types": [], "depends-on": [], "brief": []}
    ], "joins": [
        {"slice": "SLICE-001", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.line", "orders.order"]}
    ]}"#;
    let cut = r#"{"slices": [
        {"name": "orders-late", "requirements": ["REQ-007", "REQ-008", "REQ-009", "REQ-010", "REQ-011", "REQ-012", "REQ-013"],
         "types": [], "depends-on": ["orders"], "brief": ["Delivers the later items and the refund over the order book `orders` keeps."]}
    ], "joins": [
        {"slice": "SLICE-001", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004", "REQ-005", "REQ-006"], "types": ["orders.line", "orders.order"]}
    ]}"#;
    let mut provider = Provider::answering([grouping.as_str(), draft, fine, cut]);
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![requirement("orders.refund", "A signed-in user requests a refund.")])),
    );
    let revision = seed(&provider.files, &spec, DESIGN, &plan);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  1 added, 0 changed, 0 retired, 0 \
             conflict, 12 unchanged\n  stems: orders\n  slices: orders, orders-late (new)\n  \
             nothing sealed; `emery show amendment` reviews it\n",
            &HEAD[..8]
        )
    );
    assert_eq!(provider.model.seen().len(), 4, "the grouping, the draft, and two slicing rounds");

    // the brief: the slice emptied by the recut, the stem by its sub-stems
    // with the slice that held it, the types open with them
    let request = provider.model.seen()[2].messages.join("\n");
    assert!(
        request.contains(
            "## Current plan\n\n- `SLICE-001` `orders` — no requirement; every one it held is \
             open — touched by this amendment\n\n## Open\n"
        ),
        "{request}"
    );
    assert!(
        request.contains(
            "- `orders` — 13 requirements, past the cap of 12, so its sub-stems are the floor; \
             held by `SLICE-001` `orders`:\n  - `orders.item-1` — REQ-001\n"
        ),
        "{request}"
    );
    assert!(request.contains("  - `orders.refund` — REQ-013\n"), "{request}");
    assert!(request.contains("- `orders.line`\n- `orders.order`\n"), "{request}");
    assert!(request.contains("A stem past the cap is listed by its sub-stems"), "{request}");
    assert!(
        request.contains("- REQ-001 `orders.item-1` — Item 1 of the order book is handled.\n"),
        "{request}"
    );

    // the fine cut refused naming the pair, the correction accepted
    let exchanges = provider.model.exchanges();
    assert!(
        exchanges[2].outcome.as_ref().is_err_and(|finding| {
            finding.contains(
                "the stem `orders` is cut into 3 slices, yet `orders` (4 requirements) and \
                 `orders-mid` (4) fit one slice of 8 within the cap of 12",
            )
        }),
        "{exchanges:?}"
    );
    assert!(exchanges[3].outcome.is_ok(), "{exchanges:?}");

    let slices = amendment["plan"]["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 2, "{amendment}");
    assert_eq!(slices[0]["id"], "SLICE-001", "{amendment}");
    assert_eq!(slices[0]["name"], "orders", "{amendment}");
    assert_eq!(
        slices[0]["requirements"],
        serde_json::json!(["REQ-001", "REQ-002", "REQ-003", "REQ-004", "REQ-005", "REQ-006"]),
        "{amendment}"
    );
    assert_eq!(
        slices[0]["types"],
        serde_json::json!(["orders.line", "orders.order"]),
        "{amendment}"
    );
    assert_eq!(
        slices[1],
        serde_json::json!({
            "id": "SLICE-002", "name": "orders-late",
            "requirements": ["REQ-007", "REQ-008", "REQ-009", "REQ-010", "REQ-011", "REQ-012", "REQ-013"],
            "types": [], "depends-on": ["SLICE-001"],
            "brief": ["Delivers the later items and the refund over the order book `orders` keeps."]
        }),
        "{amendment}"
    );
    provider.model.assert_exhausted();
}

// A base of `count` requirements under the one stem `orders`, in one slice
// owning `DESIGN`'s two types.
fn wide(count: usize) -> (Vec<u8>, Vec<u8>) {
    let ids: Vec<String> = (1..=count).map(|number| format!("REQ-{number:03}")).collect();
    let requirements: Vec<Value> = ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            serde_json::json!({
                "id": id, "subject": format!("orders.item-{}", index + 1), "status": "unknown",
                "covered": false, "trigger": null,
                "sources": [{"source": "docs", "claim": format!("orders.item-{}", index + 1), "path": null}],
                "body": [format!("Item {} of the order book is handled.", index + 1)],
                "losers": [], "scenarios": []
            })
        })
        .collect();
    let spec = serde_json::json!({
        "emery": 8, "parent": null, "preamble": [], "requirements": requirements
    });
    let plan = serde_json::json!({
        "emery": 8, "parent": null, "preamble": [],
        "slices": [{
            "id": "SLICE-001", "name": "orders", "requirements": ids,
            "types": ["orders.line", "orders.order"], "depends-on": [], "brief": []
        }]
    });
    (spec.to_string().into_bytes(), plan.to_string().into_bytes())
}

// A change whose claims inform a design section spends one design turn over
// the current design: a documentation change carrying a `decision` and a
// requirement under a new stem redrafts `## APIs and integrations`, which
// the base lacks, and `## Overview`, which describes the whole, while
// `## Domain model` is kept byte for byte with no type block offered. The
// brief carries the change's claims, the current text numbered, the
// requirement drafted beside it, and the schema the redrafted kinds alone;
// the new stem is a new slice the slicing turn drafts after the design,
// numbered after the base's; the amendment lists the sections added and
// changed and the slice added, the output and the envelope name them, and
// `show amendment` renders the new text and the new slice with its brief.
#[tokio::test]
async fn amend_design() {
    let (provider, revision) = designed();

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged amendment {id}\n  base {revision} at {}\n  1 added, 0 changed, 0 retired, 0 \
             conflict, 4 unchanged\n  stems: secrets\n  design: overview, apis\n  slices: \
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

    // the projection carries what the design changed and the slice the new
    // stem became, with the brief the slicing turn drafted
    let shown = cli_ok(&provider, &["emery", "show", "amendment"]).await;
    let shown = String::from_utf8_lossy(&shown.stdout);
    assert!(
        shown.ends_with(&format!(
            "## Design\n\nChanged: Overview; added: APIs and integrations.\n\n### \
             Overview\n\n{OVERVIEW}\n\nHandlers read the secrets they need at use \
             (REQ-005).\n\n### APIs and integrations\n\nSecrets are read from a secret store, \
             the team vault (from docs), by REQ-005 at use.\n\n## Plan\n\n### Slice: secrets \
             [added]\n\nID: SLICE-003\nRequirements: [REQ-005]\n\nDelivers the secret read \
             every handler makes at use; verified against a fake store.\n"
        )),
        "{shown}"
    );
    provider.model.assert_exhausted();
}

// The JSON envelope carries the sections the design changed and the slices
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
    assert_eq!(envelope["slices"], expected["slices"], "{envelope}");
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
    let slicing = r#"{"slices": [
        {"name": "secrets", "requirements": ["REQ-005"], "types": [], "depends-on": [],
         "brief": ["Delivers the secret read every handler makes at use; verified against a fake store."]}
    ], "joins": []}"#;
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

    // a retire beside the added claim it shares an id with, and a second in
    // another group, are both named at once
    let added = r#"{"groups": [
        {"claims": [0, 1], "classes": [[0], [1]]},
        {"claims": [2], "classes": [[2]]},
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
            retire("orders.export", "Downloading the order book is withdrawn."),
            retire("orders.import", "Uploading orders from a file is withdrawn."),
        ])),
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
            text.contains(
                "retire `orders.export` from `docs`, `orders.import` from `docs` withdraw \
                 behaviours",
            )
        }),
        "{envelope}"
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");
}

// A withdrawal and a revision of one requirement from two sources of one
// rank are a conflict, as two statements are: the requirement is listed in
// conflict with both claims behind it, nothing is retired, and the amended
// specification holds it as a specify holds a conflict, the withdrawal a
// competing statement among its notes.
#[tokio::test]
async fn amend_retire_contested() {
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
