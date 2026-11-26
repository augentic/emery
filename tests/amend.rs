//! Verifies `emery amend` over scripted sources, storage, and version control.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use emery_adapter::source::{ClaimKind, SourceKind};
use emery_engine::vcs::REVISION;
use emery_engine::{STAGED_CONTAINER, STAGED_KEY};
use omnia_test::SeenFormat;
use serde_json::Value;
use support::cli_ok;
use support::provider::{Provider, Rendezvous, claim, evidence, requirement, retire};
use support::store::seed;
use support::vcs::HEAD;

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
const CHAIN_PLAN: &[u8] = include_bytes!("amend/chain-plan.json");
const CLASSES: &str = include_str!("amend/classes.json");
const RETIRED: &str = include_str!("amend/retired.json");
const SCOPED: &str = include_str!("amend/scoped.json");
const DESIGNED: &str = include_str!("amend/designed.json");

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
    provider.source.rendezvous = Some(Rendezvous::from_iter(["docs", "wiki"]));
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

async fn staged_amendment(provider: &Provider) -> (String, Value) {
    let raw = provider.storage.state(STAGED_KEY).await.expect("staged record");
    let record: Value = serde_json::from_slice(&raw).expect("staged record JSON");
    assert_eq!(record["kind"], "amendment", "{record}");
    let id = record["id"].as_str().expect("staged record id").to_owned();
    let body = provider
        .storage
        .object(STAGED_CONTAINER, &format!("{id}/amendment.json"))
        .await
        .expect("the staged amendment document");
    (id, serde_json::from_slice(&body).expect("the amendment is JSON"))
}
#[tokio::test]
async fn amend_classes() {
    let mut provider =
        Provider::answering([CLASSES_GROUPING, CLASSES_DRAFT, CLASSES_GROUPING, CLASSES_DRAFT]);
    bind(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", DOCS, WIKI, "--config", PROJECT_CONFIG]).await;

    let (id, amendment) = staged_amendment(&provider).await;
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
    provider.vcs.assert_subsequence(&read_calls(), "read the revision, nothing written");
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

    let (id, amendment) = staged_amendment(&provider).await;
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
    provider.vcs.assert_subsequence(&read_calls(), "read the revision, nothing written");

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
    let (_, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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
    let (_, amendment) = staged_amendment(&provider).await;
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
    let (_, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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
    let (_, amendment) = staged_amendment(&provider).await;
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
    let (_, amendment) = staged_amendment(&provider).await;
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
    let mut provider = Provider::answering([SCOPED_GROUPING, SCOPED_GROUPING]);
    let config = scoped(&provider, CODE, "whole");
    code(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", "--config", &config]).await;

    let (id, amendment) = staged_amendment(&provider).await;
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
    provider.vcs.assert_subsequence(&read_calls(), "read the revision, nothing written");

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
    let mut provider = Provider::answering([SCOPED_GROUPING]);
    let config = scoped(&provider, CODE, "delta");
    code(&mut provider);
    let revision = seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", "--config", &config]).await;

    let unchanged = format!(
        "nothing staged: no change against the current revision\n  base {revision} at {}\n  0 \
         added, 0 changed, 0 retired, 0 conflict, 4 unchanged\n",
        &HEAD[..8]
    );
    assert_eq!(String::from_utf8_lossy(&resp.stdout), unchanged);
    assert!(provider.storage.state(STAGED_KEY).await.is_none(), "nothing is staged");
    provider.model.assert_exhausted();

    let mut provider = Provider::answering([SCOPED_GROUPING]);
    let config = scoped(&provider, DOCS, "whole");
    provider.source.evidence.insert(
        "docs".to_owned(),
        Ok(evidence(vec![requirement("orders.create", "A signed-in user places an order.")])),
    );
    seed(&provider.files, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "amend", "--config", &config]).await;

    assert_eq!(String::from_utf8_lossy(&resp.stdout), unchanged);
    assert!(provider.storage.state(STAGED_KEY).await.is_none(), "nothing is staged");
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

// A project-mounted config naming one source read under `scope`.
fn scoped(provider: &Provider, adapter: &str, scope: &str) -> String {
    let slug = adapter.split(':').nth(1).unwrap_or(adapter).replace('.', "_");
    provider.write_project(
        &format!("amend-{slug}-{scope}.toml"),
        format!(
            "[[source]]\nadapter = \"{adapter}\"\nscope = \"{scope}\"\n\n[target]\nadapter = \
             \"acme:builder@2.1.0\"\nrepository = \".\"\n"
        ),
    )
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
    let (_, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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
    let (_, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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
    provider.vcs.assert_subsequence(&read_calls(), "read the revision, nothing written");
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

    let (_, amendment) = staged_amendment(&provider).await;
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
    let (_, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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

    let (id, amendment) = staged_amendment(&provider).await;
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
    provider.vcs.assert_subsequence(&read_calls(), "read the revision, nothing written");

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
