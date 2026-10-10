//! Verifies the operator journey from `specify` through `show` and regeneration.
//!
//! Each scenario drives the real command façade over scripted capabilities,
//! asserting the exact envelope, exit code, storage operations, and Markdown
//! projection an operator would observe.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use emery_adapter::source::{Claim, ClaimKind, Evidence, SourceContent, SourceKind};
use emery_engine::{CONTAINER, REVISION_KEY};
use omnia_sdk::model::{Error as ModelError, Reply};
use omnia_sdk::{BlobStore, StateStore, bad_gateway, bad_request};
use omnia_test::SeenFormat;
use omnia_test::guest::{Memory, Scripted};
use serde_json::Value;
use support::{
    Provider, Rendezvous, Scratch, claim, cli_ok, digest, evidence, fail, requirement, seed,
};

// The one-source reference most scenarios name; the scripted loader lands
// any exact package reference, so the namespace and version are nominal.
const SOURCE: &str = "acme:source@1.0.0";

// The reference an adapter `name` is named by, and the source it binds.
fn reference(name: &str) -> String {
    format!("acme:{name}@1.0.0")
}

// The guest a reference loads and dispatches as, which the scripts key on.
fn guest(name: &str) -> String {
    format!("acme:{name}")
}

// A `--description` value binding `text` under the adapter `name`.
fn describing(name: &str, text: &str) -> String {
    format!("{}={text}", reference(name))
}

const SPEC_ANSWER: &str = include_str!("specify/spec-draft.json");
const SPEC_REVISION: &str = include_str!("specify/1-spec.json");
const SPEC_RENDERED: &str = include_str!("specify/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("specify/design-draft.json");
const DESIGN_REVISION: &str = include_str!("specify/2-design.json");
const DESIGN_RENDERED: &str = include_str!("specify/2-design.md");
const GROUPING_ANSWER: &str = include_str!("specify/grouping.json");
const PRECEDENCE_ANSWER: &str = include_str!("specify/precedence-draft.json");
const PRECEDENCE_REVISION: &str = include_str!("specify/3-precedence.json");
const PRECEDENCE_RENDERED: &str = include_str!("specify/3-precedence.md");
const PLAN_REVISION: &str = include_str!("specify/4-plan.json");
const PLAN_RENDERED: &str = include_str!("specify/4-plan.md");
const SLICING_ANSWER: &str = include_str!("specify/slicing.json");
const SLICED_REVISION: &str = include_str!("specify/5-sliced.json");
const SLICED_RENDERED: &str = include_str!("specify/5-sliced.md");

// The grouping a run over `count` claims of one id expects: one agreeing requirement.
fn baseline_grouping(count: usize) -> String {
    let indices = (0..count).collect::<Vec<_>>();
    serde_json::json!({
        "groups": [{"claims": &indices, "classes": [&indices]}],
    })
    .to_string()
}

// The grouping a judged run over `count` claims of distinct ids expects
// when every claim stays its own requirement.
fn separate_grouping(count: usize) -> String {
    let groups: Vec<_> = (0..count)
        .map(|index| serde_json::json!({"claims": [index], "classes": [[index]]}))
        .collect();
    serde_json::json!({ "groups": groups }).to_string()
}

// A specification draft of one generated scenario per subject under `preamble`.
fn spec_draft(subjects: &[String], preamble: &[&str]) -> String {
    let requirements: Vec<Value> = subjects
        .iter()
        .map(|id| {
            serde_json::json!({"subject": id, "scenarios": [{
                "name": format!("Scenario for {id}"),
                "when": format!("{id} is triggered"),
                "then": format!("{id} is observed"),
            }]})
        })
        .collect();
    serde_json::json!({"preamble": preamble, "requirements": requirements}).to_string()
}

// The slicing a run over several stems expects when every stem stays its own
// slice, named for it, owning nothing and depending on nothing.
fn separate_slicing(stems: &[(&str, &[&str])]) -> String {
    let slices: Vec<_> = stems
        .iter()
        .map(|(name, requirements)| serde_json::json!({"name": name, "requirements": requirements}))
        .collect();
    serde_json::json!({ "preamble": [], "slices": slices }).to_string()
}

// --- journey ---

// One `specify` with no prior verb, then `show`, then an identical re-run that
// drafts again from the sources alone.
#[tokio::test]
async fn gen_spec() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]);

    // the first specify
    cli_ok(&provider, &["emery", "specify", SOURCE]).await;

    // observe the gate, the current id, and the revision
    assert_eq!(
        *provider.source.metadata.lock().expect("metadata"),
        [guest("source")],
        "the adapter is gated as the guest its reference names"
    );
    assert!(
        provider.storage.objects("adapters").is_empty(),
        "nothing of an adapter mirrors into engine storage"
    );
    assert!(provider.storage.state("project.yaml").is_none(), "no project record exists");
    let id = current(&provider.storage);

    // the committed documents are canonical JSON of the engine's facts
    let spec = document(&provider.storage, &id, "spec.json");
    assert_eq!(
        String::from_utf8_lossy(&spec),
        SPEC_REVISION,
        "spec.json is the canonical revision"
    );
    let design = document(&provider.storage, &id, "design.json");
    assert_eq!(
        String::from_utf8_lossy(&design),
        DESIGN_REVISION,
        "design.json is the canonical revision"
    );
    let plan = document(&provider.storage, &id, "plan.json");
    assert_eq!(
        String::from_utf8_lossy(&plan),
        PLAN_REVISION,
        "one stem is one slice, planned with no turn spent"
    );

    // review through show
    assert_eq!(
        shown(&provider, "spec").await,
        projection(SPEC_RENDERED, &id),
        "show renders spec.md"
    );
    assert_eq!(
        shown(&provider, "design").await,
        projection(DESIGN_RENDERED, &id),
        "show renders design.md"
    );
    assert_eq!(
        shown(&provider, "plan").await,
        projection(PLAN_RENDERED, &id),
        "show renders plan.md"
    );

    // the JSON envelope carries the revision, the projection, and the document
    let resp = cli_ok(&provider, &["emery", "--format", "json", "show", "spec"]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["body"], projection(SPEC_RENDERED, &id), "{envelope}");
    let document: Value =
        serde_json::from_str(SPEC_REVISION).expect("the revision fixture is JSON");
    assert_eq!(envelope["document"], document, "the envelope carries the typed document");

    // an identical re-run drafts again and commits the same bytes
    let resp = cli_ok(&provider, &["emery", "specify", SOURCE]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "committed revision {id}\n  plan: 1 slice in 1 wave, widest 1\n  diff vs {id}: none \
             (byte-stable)\n"
        ),
        "the one-stem plan is one slice in one wave"
    );
    assert_eq!(current(&provider.storage), id, "the same revision keeps its id");

    provider.model.assert_exhausted();
}

// Padding around a drafted line is the model's, not the specification's, so
// it is dropped before the revision is stored and never reaches the id.
#[tokio::test]
async fn padded_lines() {
    let mut answer: Value = serde_json::from_str(SPEC_ANSWER).expect("the draft fixture is JSON");
    let scenario = &mut answer["requirements"][0]["scenarios"][0];
    scenario["name"] = Value::String("  Greeting requested ".into());
    scenario["given"] = serde_json::json!([" the greeting surface is bound\t"]);
    scenario["when"] = Value::String("`/greeting` is requested  ".into());
    scenario["then"] = Value::String(" the response is `hello`".into());
    let padded = answer.to_string();
    let provider = Provider::answering([padded.as_str(), DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", SOURCE]).await;

    let id = current(&provider.storage);
    assert_eq!(
        String::from_utf8_lossy(&document(&provider.storage, &id, "spec.json")),
        SPEC_REVISION,
        "the stored lines carry none of the padding"
    );
    provider.model.assert_exhausted();
}

// A drafted `given` or `and` written as one bare string where the schema asks
// for a sequence is the model's slip, not a finding: it is read as the one-line
// sequence and stored as such, with no correction round spent.
#[tokio::test]
async fn lines_as_strings() {
    let mut answer: Value = serde_json::from_str(SPEC_ANSWER).expect("the draft fixture is JSON");
    let scenario = &mut answer["requirements"][0]["scenarios"][0];
    scenario["given"] = Value::String("the greeting surface is bound".into());
    scenario["and"] = Value::String("the greeting is logged".into());
    let bare = answer.to_string();
    let provider = Provider::answering([bare.as_str(), DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", SOURCE]).await;

    let id = current(&provider.storage);
    let spec: Value = serde_json::from_slice(&document(&provider.storage, &id, "spec.json"))
        .expect("the stored spec is JSON");
    let stored = &spec["requirements"][0]["scenarios"][0];
    assert_eq!(stored["given"], serde_json::json!(["the greeting surface is bound"]), "{spec}");
    assert_eq!(stored["and"], serde_json::json!(["the greeting is logged"]), "{spec}");
    provider.model.assert_exhausted();
}

// One adapter named by two sources is gated once, extracted twice.
#[tokio::test]
async fn shared_roots() {
    let adapter = "emery:documentation@1.2.0";
    let package = "emery:documentation";
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[[source]]\nname = \"docs\"\nadapter = \"{adapter}\"\npath = \"docs\"\n\n\
         [[source]]\nname = \"api\"\nadapter = \"{adapter}\"\npath = \"api\"\n"
    ));

    let grouping = baseline_grouping(2);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    assert!(
        shown(&provider, "spec")
            .await
            .contains("Sources: [docs:greeting.behaviour, api:greeting.behaviour]"),
        "both sources contribute to the one requirement"
    );

    let gated = provider.source.metadata.lock().expect("metadata").clone();
    assert_eq!(gated, [package], "one adapter identity is gated once");

    let calls = provider.source.calls();
    assert_eq!(calls.len(), 2, "each source extracts");
    assert_eq!(calls[0].0, package);
    assert_eq!(calls[0].1.name, "docs");
    assert_eq!(calls[1].0, package);
    assert_eq!(calls[1].1.name, "api");
    drop(calls);

    provider.model.assert_exhausted();
}

// The rendezvous holds each extract until the other is requested, so an engine
// extracting one source at a time fails naming the one that never came.
#[tokio::test]
async fn sources_together() {
    let grouping = baseline_grouping(2);
    let mut provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);
    provider.source.rendezvous = Some(Rendezvous::from_iter(["docs", "api"]));

    cli_ok(&provider, &["emery", "specify", &reference("docs"), &reference("api")]).await;

    let order: Vec<String> = provider
        .source
        .calls
        .lock()
        .expect("calls")
        .iter()
        .map(|(_, input)| input.name.clone())
        .collect();
    assert_eq!(order, ["docs", "api"], "dispatch keeps declaration order");
    assert!(
        shown(&provider, "spec")
            .await
            .contains("Sources: [docs:greeting.behaviour, api:greeting.behaviour]"),
        "both sources contribute, cited in declaration order"
    );
    provider.model.assert_exhausted();
}

// The CWD move is hermetic under nextest's process-per-test isolation.
#[tokio::test]
async fn discovery() {
    let project = tempfile::TempDir::new().expect("project dir");
    fs::write(
        project.path().join("emery.toml"),
        "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n",
    )
    .expect("write emery.toml");
    std::env::set_current_dir(project.path()).expect("enter project");

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify"]).await;

    assert!(shown(&provider, "spec").await.contains("Sources: [docs:greeting.behaviour]"));
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn description_source() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--description", &describing("intent", "Ship it.")])
        .await;

    let spec = shown(&provider, "spec").await;
    assert!(spec.contains("Sources: [intent:greeting.behaviour]"));
    let calls = provider.source.calls();
    let (id, input) = calls.first().expect("one extract dispatch");
    assert_eq!(*id, guest("intent"), "the adapter dispatches as the guest its reference names");
    assert_eq!(input.name, "intent", "the source is named for the package");
    assert_eq!(input.content, SourceContent::Value("Ship it.".to_string()));
    drop(calls);
    provider.model.assert_exhausted();
}

// --- precedence ---

// Identity is the grouping's, authority the engine's: intent outranks the
// behaviour claim bound into the timeout, tied documentation peers conflict,
// and the uncovered timeout keeps its gap tag.
#[tokio::test]
async fn authority_precedence() {
    let slicing = separate_slicing(&[("login", &["REQ-001"]), ("session", &["REQ-002"])]);
    let mut provider =
        Provider::answering([GROUPING_ANSWER, PRECEDENCE_ANSWER, DESIGN_ANSWER, slicing.as_str()]);

    // rank each adapter by its metadata; the unscripted ones read documentation
    provider.source.kinds.insert(guest("code"), SourceKind::Behaviour);
    provider.source.kinds.insert(guest("intent"), SourceKind::Intent);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("login.flow", "Users sign in with a magic link."),
            requirement("session.timeout", "Sessions expire after 30 minutes of inactivity."),
            claim(
                ClaimKind::Criterion,
                "login.flow.success",
                ("criterion", "A valid link signs the user in."),
            ),
            claim(ClaimKind::Decision, "auth.decision", ("body", "Sessions are cookie-bound.")),
        ])),
    );
    provider.source.evidence.insert(
        "wiki-live".to_string(),
        Ok(evidence(vec![requirement("login.flow", "Users sign in with a passkey.")])),
    );
    provider.source.evidence.insert(
        "code".to_string(),
        Ok(evidence(vec![
            requirement("login.flow", "Users sign in with email and password."),
            requirement("session-expiry", "Sessions expire after 15 minutes of inactivity."),
        ])),
    );
    provider.source.evidence.insert(
        "intent".to_string(),
        Ok(evidence(vec![requirement(
            "session.timeout",
            "Sessions must expire after 30 minutes of inactivity.",
        )])),
    );

    cli_ok(
        &provider,
        &[
            "emery",
            "specify",
            &reference("docs"),
            &reference("wiki-live"),
            &reference("code"),
            "--description",
            &describing("intent", "Sessions expire after 30."),
        ],
    )
    .await;

    // the grouping request indexes every claim and withholds authority
    let grouping = &provider.model.seen()[0];
    let request = grouping.messages.join("\n");
    assert!(request.contains("- 4 `code` `session-expiry`"), "{request}");
    assert!(
        request.contains("share the id `session.timeout`"),
        "the baseline is stated: {request}"
    );
    assert!(!request.contains("documentation"), "authority is withheld: {request}");
    assert!(!request.contains("behaviour"), "authority is withheld: {request}");
    let SeenFormat::Schema { name, schema } = &grouping.format else {
        panic!("the grouping is steered by schema");
    };
    assert_eq!(name, "grouping");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["groups"]["minItems"], 1);
    let group = &schema["$defs"]["Group"]["properties"];
    assert_eq!(group["claims"]["items"]["maximum"], 5, "six claims, last index 5");
    assert_eq!(group["claims"]["items"]["type"], "integer", "the derive is intact");
    assert_eq!(group["classes"]["items"]["items"]["maximum"], 5);

    let id = current(&provider.storage);
    let spec = document(&provider.storage, &id, "spec.json");
    assert_eq!(
        String::from_utf8_lossy(&spec),
        PRECEDENCE_REVISION,
        "every resolution is a fact in the revision"
    );
    assert_eq!(
        shown(&provider, "spec").await,
        projection(PRECEDENCE_RENDERED, &id),
        "every resolution is rendered inline"
    );
    provider.model.assert_exhausted();
}

// A `[[source]] rank` stands in for the kind's default: documentation ranked
// beneath behaviour loses to it, and ranked level with it ties into a conflict.
#[tokio::test]
async fn configured_rank() {
    let bind = |provider: &mut Provider| {
        provider.source.kinds.insert(guest("code"), SourceKind::Behaviour);
        provider.source.evidence.insert(
            "docs".to_string(),
            Ok(evidence(vec![requirement("session.timeout", "Sessions expire after 30 minutes.")])),
        );
        provider.source.evidence.insert(
            "code".to_string(),
            Ok(evidence(vec![requirement("session.timeout", "Sessions expire after 15 minutes.")])),
        );
    };
    let grouping = r#"{"groups": [{"claims": [0, 1], "classes": [[0], [1]]}]}"#;
    let spec = SPEC_ANSWER.replace("greeting.behaviour", "session.timeout");
    let config = |rank: u32| {
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{}\"\nrank = {rank}\n\n\
             [[source]]\nname = \"code\"\nadapter = \"{}\"\n",
            reference("docs"),
            reference("code")
        )
    };

    // ranked beneath the code's default 3, the documentation loses
    let scratch = Scratch::new();
    let path = scratch.config(&config(4));
    let mut provider = Provider::answering([grouping, spec.as_str(), DESIGN_ANSWER]);
    bind(&mut provider);
    cli_ok(&provider, &["emery", "specify", "--config", &path]).await;

    let id = current(&provider.storage);
    let stored: Value = serde_json::from_slice(&document(&provider.storage, &id, "spec.json"))
        .expect("the stored spec is JSON");
    let requirement = &stored["requirements"][0];
    assert_eq!(requirement["status"], "divergence", "{stored}");
    assert_eq!(requirement["body"], serde_json::json!(["Sessions expire after 15 minutes."]));
    assert_eq!(
        requirement["losers"],
        serde_json::json!([{
            "sources": ["docs"],
            "kind": "documentation",
            "rank": 4,
            "claim": "session.timeout",
            "statement": "Sessions expire after 30 minutes.",
        }]),
        "the loser records the rank it lost under beside its kind"
    );
    let spec_md = shown(&provider, "spec").await;
    assert!(spec_md.contains("### Requirement: session.timeout [divergence]"), "{spec_md}");
    assert!(
        spec_md.contains("Sources: [code:session.timeout, docs:session.timeout]"),
        "citations follow rank, not kind: {spec_md}"
    );
    assert!(
        spec_md.contains(
            "Note: docs (documentation, rank 4, session.timeout): Sessions expire after 30 \
             minutes."
        ),
        "{spec_md}"
    );
    assert!(
        provider.model.seen().iter().skip(1).all(|draft| {
            draft.messages.join("\n").contains("docs (documentation, rank 4, `session.timeout`)")
        }),
        "the drafting turns see the rank the note carries"
    );
    provider.model.assert_exhausted();

    // ranked level with the code, the two tie into a conflict
    let path = scratch.config(&config(3));
    let mut provider = Provider::answering([grouping, spec.as_str(), DESIGN_ANSWER]);
    bind(&mut provider);
    cli_ok(&provider, &["emery", "specify", "--config", &path]).await;

    let spec_md = shown(&provider, "spec").await;
    assert!(spec_md.contains("### Requirement: session.timeout [conflict]"), "{spec_md}");
    assert!(
        spec_md.contains(
            "Note: docs (documentation, rank 3, session.timeout): Sessions expire after 30 \
             minutes.\nNote: code (behaviour, rank 3, session.timeout): Sessions expire after 15 \
             minutes.\nNote: Operator reconciliation required."
        ),
        "{spec_md}"
    );
    provider.model.assert_exhausted();
}

// A refused grouping is the next candidate's correction; a backend out of rounds
// fails typed and commits nothing.
#[tokio::test]
async fn grouping_refused() {
    let bind = |provider: &mut Provider| {
        provider.source.kinds.insert(guest("code"), SourceKind::Behaviour);
        provider.source.evidence.insert(
            "docs".to_string(),
            Ok(evidence(vec![requirement("session.timeout", "Sessions expire after 30 minutes.")])),
        );
        provider.source.evidence.insert(
            "code".to_string(),
            Ok(evidence(vec![requirement("session.timeout", "Sessions expire after 15 minutes.")])),
        );
    };
    let cases: &[(&str, &str)] = &[
        (
            r#"{"groups": [{"claims": [0], "classes": [[0]]}, {"claims": [1], "classes": [[1]]}]}"#,
            "claims sharing the id `session.timeout` are split across groups",
        ),
        (r#"{"groups": [{"claims": [0], "classes": [[0]]}]}"#, "claim 1 is in no group"),
        (
            r#"{"groups": [{"claims": [0, 1], "classes": [[0, 1], [1]]}]}"#,
            "claim 1 appears in more than one class",
        ),
        (r#"{"groups": [{"claims": [0, 1], "classes": [[0]]}]}"#, "claim 1 is in no class"),
        ("not json", "schema and answer type disagree"),
    ];
    for (answer, fragment) in cases {
        let mut provider = Provider::answering([*answer, *answer, *answer]);
        bind(&mut provider);
        let envelope = fail(
            &provider,
            &["emery", "specify", &reference("docs"), &reference("code")],
            1,
            "bad_request",
        )
        .await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }

    // the corrected answer commits
    let refused = r#"{"groups": [{"claims": [0], "classes": [[0]]}]}"#;
    let corrected = r#"{"groups": [{"claims": [0, 1], "classes": [[0], [1]]}]}"#;
    let spec = SPEC_ANSWER.replace("greeting.behaviour", "session.timeout");
    let mut provider = Provider::answering([refused, corrected, spec.as_str(), DESIGN_ANSWER]);
    bind(&mut provider);
    cli_ok(&provider, &["emery", "specify", &reference("docs"), &reference("code")]).await;
    let check = &provider.model.exchanges()[0];
    assert_eq!(check.tool, "check", "the engine judges each candidate over the check tool");
    let correction = check.outcome.as_ref().expect_err("the first grouping is rejected");
    assert!(correction.contains("## Findings"), "the findings ride the correction: {correction}");
    assert!(correction.contains("claim 1 is in no group"), "{correction}");
    let spec = shown(&provider, "spec").await;
    assert!(spec.contains("### Requirement: session.timeout [divergence]"), "{spec}");
    assert!(
        spec.contains(
            "Note: code (behaviour, rank 3, session.timeout): Sessions expire after 15 minutes."
        ),
        "{spec}"
    );
    provider.model.assert_exhausted();
}

// Two behaviour sources whose claims are all `type` pass the claim gate but
// leave nothing to reconcile.
#[tokio::test]
async fn no_claims() {
    let mut provider = Provider::idle();
    for name in ["api", "code"] {
        provider.source.kinds.insert(guest(name), SourceKind::Behaviour);
        provider.source.evidence.insert(
            name.to_string(),
            Ok(evidence(vec![claim(
                ClaimKind::Type,
                "greeting.type",
                ("signature", "interface Greeting { text: string }"),
            )])),
        );
    }

    let envelope = fail(
        &provider,
        &["emery", "specify", &reference("api"), &reference("code")],
        1,
        "bad_request",
    )
    .await;
    assert_message(&envelope, "no source contributed a requirement claim");
    provider.model.assert_exhausted();
}

// A second source with no requirement claim leaves one contributing source,
// so no grouping turn is spent.
#[tokio::test]
async fn one_claims_source() {
    let mut provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    provider.source.kinds.insert(guest("code"), SourceKind::Behaviour);
    provider.source.evidence.insert(
        "code".to_string(),
        Ok(evidence(vec![claim(
            ClaimKind::Decision,
            "greeting.decision",
            ("body", "The greeting is a static string."),
        )])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs"), &reference("code")]).await;

    let SeenFormat::Schema { name, .. } = &provider.model.seen()[0].format else {
        panic!("the first turn is steered by schema");
    };
    assert_eq!(name, "spec-draft", "the baseline stands with no grouping turn");
    let spec = shown(&provider, "spec").await;
    assert!(spec.contains("### Requirement: greeting.behaviour"), "{spec}");
    provider.model.assert_exhausted();
}

// One source whose seams describe one behaviour under two nouns is grouped
// by the model, and the merged requirement cites both claims.
#[tokio::test]
async fn seams_grouped() {
    let grouping = r#"{"groups": [{"claims": [0, 1], "classes": [[0, 1]]}]}"#;
    let spec = r#"{"preamble": ["One source, two seams, one behaviour."],
        "requirements": [{"subject": "start.persist",
            "scenarios": [{"name": "Persist", "when": "the service starts", "then": "the queue is persisted"}]}]}"#;
    let mut provider = Provider::answering([grouping, spec, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("start.persist", "Starting the service persists the queue."),
            requirement("worker.persist", "The worker persists the queue on start."),
        ])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let seen = provider.model.seen();
    let SeenFormat::Schema { name, .. } = &seen[0].format else {
        panic!("one source over two stems is grouped by the model");
    };
    assert_eq!(name, "grouping");
    let request = seen[0].messages.join("\n");
    assert!(
        request.contains("`start.persist`") && request.contains("`worker.persist`"),
        "{request}"
    );

    let id = current(&provider.storage);
    let spec: Value = serde_json::from_slice(&document(&provider.storage, &id, "spec.json"))
        .expect("the committed spec is JSON");
    assert_eq!(spec["requirements"].as_array().map(Vec::len), Some(1), "{spec}");
    let requirement = &spec["requirements"][0];
    assert_eq!(requirement["subject"], "start.persist", "{spec}");
    assert_eq!(
        requirement["sources"],
        serde_json::json!([
            {"source": "docs", "claim": "start.persist", "path": null},
            {"source": "docs", "claim": "worker.persist", "path": null}
        ]),
        "{spec}"
    );
    provider.model.assert_exhausted();
}

// The extract call that minted `start.persist` and `start.recover` under one
// stem told them apart, so the grouping may merge across stems but never
// within one source's stem: a group that does is split by the engine, the first
// id keeping the group's other members, with no correction round spent.
#[tokio::test]
async fn same_stem_split() {
    let merged = r#"{"groups": [
        {"claims": [0, 1, 2], "classes": [[0, 2], [1]]}]}"#;
    let spec = r#"{"preamble": ["One source, two stems."],
        "requirements": [
            {"subject": "start.persist",
             "scenarios": [{"name": "Persist", "when": "the service starts", "then": "the queue is persisted"}]},
            {"subject": "start.recover",
             "scenarios": [{"name": "Recover", "when": "the service restarts", "then": "the queue is restored"}]}]}"#;
    let mut provider = Provider::answering([merged, spec, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("start.persist", "Starting the service persists the queue."),
            requirement("start.recover", "Restarting the service restores the queue."),
            requirement("worker.persist", "The worker persists the queue on start."),
        ])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let check = &provider.model.exchanges()[0];
    assert_eq!(check.tool, "check");
    assert!(
        check.outcome.is_ok(),
        "the same-stem merge is split, not refused: {:?}",
        check.outcome
    );
    let request = provider.model.seen()[0].messages.join("\n");
    assert!(request.contains("a group that merges them is split"), "{request}");

    let id = current(&provider.storage);
    let spec: Value = serde_json::from_slice(&document(&provider.storage, &id, "spec.json"))
        .expect("the committed spec is JSON");
    assert_eq!(spec["requirements"].as_array().map(Vec::len), Some(2), "{spec}");
    assert_eq!(
        spec["requirements"][0]["sources"],
        serde_json::json!([
            {"source": "docs", "claim": "start.persist", "path": null},
            {"source": "docs", "claim": "worker.persist", "path": null}
        ]),
        "the first id keeps the cross-stem member: {spec}"
    );
    assert_eq!(spec["requirements"][1]["subject"], "start.recover", "{spec}");
    assert_eq!(
        spec["requirements"][1]["sources"],
        serde_json::json!([{"source": "docs", "claim": "start.recover", "path": null}]),
        "{spec}"
    );
    provider.model.assert_exhausted();
}

// --- regeneration ---

// Every subject is drafted again and nothing of the outgoing revision reaches
// the model; text reports a one-line diff summary while JSON carries the entries.
#[tokio::test]
async fn remine_supersedes() {
    // first run: a greeting, a session timeout, and a legacy export
    let first_grouping = separate_grouping(3);
    let first_slicing = separate_slicing(&[
        ("greeting", &["REQ-001"]),
        ("session", &["REQ-002"]),
        ("legacy", &["REQ-003"]),
    ]);
    let mut provider = Provider::answering([
        first_grouping.as_str(),
        REMINE_FIRST,
        DESIGN_ANSWER,
        first_slicing.as_str(),
    ]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence(&[
            ("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            ("session.timeout", "Sessions time out after an hour."),
            ("legacy.export", "Exports ship nightly."),
        ])),
    );
    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
    let first = current(&provider.storage);

    // second run: the greeting changed, the export gone, the overview following it
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let second_grouping = separate_grouping(2);
    let second_slicing = separate_slicing(&[("greeting", &["REQ-001"]), ("session", &["REQ-002"])]);
    let mut provider = Provider::over(
        Arc::clone(&provider.storage),
        [second_grouping.as_str(), REMINE_SECOND, second_design.as_str(), second_slicing.as_str()],
    );
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence(&[
            ("greeting.behaviour", "GET /greeting returns the static string 'howdy'."),
            ("session.timeout", "Sessions time out after an hour."),
        ])),
    );
    let resp = cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    // observe the summary, the swap, and the prune
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout.lines().count(),
        3,
        "a changed run prints the revision, the plan's shape, and one summary line: {stdout}"
    );
    assert!(
        stdout.contains("  plan: 2 slices in 1 wave, widest 2\n"),
        "two independent slices are one wave: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "diff vs {first}: spec +0 -1 ~1 preamble, design +0 -0 ~1, plan +0 -1 ~0"
        )),
        "the summary counts the changes: {stdout}"
    );
    assert!(!stdout.contains("REQ-"), "no per-requirement entry rides text mode: {stdout}");
    assert!(!stdout.contains("SLICE-"), "no per-slice entry rides text mode: {stdout}");

    let seen = provider.model.seen();
    let SeenFormat::Schema { name, .. } = &seen[0].format else {
        panic!("one source over two stems is grouped by the model");
    };
    assert_eq!(name, "grouping");
    let request = seen[1].messages.join("\n");
    assert!(request.contains("- REQ-001 `greeting.behaviour`"), "{request}");
    assert!(request.contains("- REQ-002 `session.timeout`"), "{request}");
    assert!(!request.contains("Unchanged"), "nothing stands in from the outgoing run: {request}");

    let second = current(&provider.storage);
    assert_ne!(first, second, "changed documents commit a new revision");
    assert!(
        provider.storage.object(CONTAINER, &format!("{first}/spec.json")).is_none(),
        "the superseded revision is pruned"
    );
    let spec = shown(&provider, "spec").await;
    assert!(spec.contains("howdy"), "{spec}");
    assert!(spec.contains("ID: REQ-002\n") && spec.contains("it times out"), "{spec}");
    assert!(!spec.contains("REQ-003"), "{spec}");
    provider.model.assert_exhausted();
}

// The second run changes the greeting, adds an audit requirement and a `type`
// claim, and drops both preambles.
#[tokio::test]
async fn diff_envelope() {
    let second_spec = r#"{"preamble": [], "requirements": [
        {"subject": "greeting.behaviour", "scenarios": [{"name": "Greeting", "when": "`/greeting` is requested", "then": "the response is `howdy`"}]},
        {"subject": "access.audit", "scenarios": [{"name": "Audit", "when": "access occurs", "then": "it is audited"}]}
    ]}"#;
    let second_design = r#"{"preamble": [], "sections": [
        {"kind": "overview", "blocks": [{"text": "One static `GET /greeting` endpoint returning `'howdy'`."}]},
        {"kind": "domain-model", "blocks": [{"type": "greeting.type"}]}
    ]}"#;
    let second_grouping = separate_grouping(2);
    let second_slicing = r#"{"preamble": [], "slices": [
        {"name": "greeting", "requirements": ["REQ-001"], "types": ["greeting.type"]},
        {"name": "access", "requirements": ["REQ-002"]}
    ]}"#;
    let mut provider = Provider::answering([
        SPEC_ANSWER,
        DESIGN_ANSWER,
        second_grouping.as_str(),
        second_spec,
        second_design,
        second_slicing,
    ]);
    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
    let first = current(&provider.storage);

    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'howdy'."),
            requirement("access.audit", "Access is audited."),
            claim(
                ClaimKind::Type,
                "greeting.type",
                ("signature", "interface Greeting { text: string }"),
            ),
        ])),
    );
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let diff = &envelope["diff"];
    assert_eq!(diff["from"], first, "{envelope}");
    assert!(diff.get("documents").is_none(), "the diff is typed, not by file: {envelope}");
    assert_eq!(diff["spec"]["preamble"], serde_json::json!(true), "{envelope}");
    assert_eq!(
        diff["spec"]["added"],
        serde_json::json!([{"id": "REQ-002", "subject": "access.audit"}]),
        "{envelope}"
    );
    assert_eq!(diff["spec"]["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(
        diff["spec"]["changed"],
        serde_json::json!([{
            "id": "REQ-001",
            "subject": "greeting.behaviour",
            "fields": ["body", "scenarios"],
        }]),
        "{envelope}"
    );
    assert_eq!(diff["design"]["preamble"], serde_json::json!(true), "{envelope}");
    assert_eq!(diff["design"]["changed"], serde_json::json!(["overview"]), "{envelope}");
    assert_eq!(diff["design"]["added"], serde_json::json!(["domain-model"]), "{envelope}");
    assert_eq!(diff["design"]["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(diff["plan"]["preamble"], serde_json::json!(false), "{envelope}");
    assert_eq!(
        diff["plan"]["added"],
        serde_json::json!([{"id": "SLICE-002", "name": "access"}]),
        "{envelope}"
    );
    assert_eq!(diff["plan"]["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(
        diff["plan"]["changed"],
        serde_json::json!([{"id": "SLICE-001", "name": "greeting", "fields": ["types"]}]),
        "the greeting slice now owns the new type: {envelope}"
    );
    provider.model.assert_exhausted();
}

// The second run extracts the same two requirements in the other order, so
// the position ids swap; each is matched to its displaced self by the stem
// and the anchor it cites — one range shifted by a line, still overlapping —
// and reads as a change of `id` naming the id it carried, never as a removal
// and an addition.
#[tokio::test]
async fn remine_reordered() {
    let grouping = separate_grouping(2);
    let first_slicing = separate_slicing(&[("greeting", &["REQ-001"]), ("session", &["REQ-002"])]);
    let mut provider = Provider::answering([
        grouping.as_str(),
        REMINE_SECOND,
        DESIGN_ANSWER,
        first_slicing.as_str(),
    ]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            anchored(
                "greeting.behaviour",
                "GET /greeting returns 'howdy'.",
                "src/greeting.ts#L3-L9",
            ),
            anchored("session.timeout", "Sessions time out after an hour.", "src/session.ts#L4-L8"),
        ])),
    );
    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
    let first = current(&provider.storage);

    let second_slicing = separate_slicing(&[("session", &["REQ-001"]), ("greeting", &["REQ-002"])]);
    let mut provider = Provider::over(
        Arc::clone(&provider.storage),
        [grouping.as_str(), REMINE_SECOND, DESIGN_ANSWER, second_slicing.as_str()],
    );
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            anchored("session.timeout", "Sessions time out after an hour.", "src/session.ts#L5-L9"),
            anchored(
                "greeting.behaviour",
                "GET /greeting returns 'howdy'.",
                "src/greeting.ts#L3-L9",
            ),
        ])),
    );
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let diff = &envelope["diff"];
    assert_eq!(diff["from"], first, "{envelope}");
    assert_eq!(diff["spec"]["added"], serde_json::json!([]), "{envelope}");
    assert_eq!(diff["spec"]["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(
        diff["spec"]["changed"],
        serde_json::json!([
            {"id": "REQ-001", "subject": "session.timeout", "was": "REQ-002", "fields": ["id", "sources"]},
            {"id": "REQ-002", "subject": "greeting.behaviour", "was": "REQ-001", "fields": ["id"]},
        ]),
        "each requirement is matched where it anchors and reads as a moved id: {envelope}"
    );
    let spec: Value = serde_json::from_slice(&document(
        &provider.storage,
        &current(&provider.storage),
        "spec.json",
    ))
    .expect("the committed spec is JSON");
    assert_eq!(
        spec["requirements"][0]["sources"],
        serde_json::json!([{"source": "docs", "claim": "session.timeout", "path": "src/session.ts#L5-L9"}]),
        "the citation carries the anchor: {spec}"
    );
    provider.model.assert_exhausted();
}

// Two requirements under one stem whose anchors meet, re-mined with the ids
// shifted: the timeout's anchor has grown over the lines the renewal's held,
// so the renewal's first candidate is the one the timeout takes; it pairs
// with its own next rather than reading as a removal and an addition.
#[tokio::test]
async fn remine_contended_anchor() {
    let grouping = separate_grouping(3);
    let first_slicing =
        separate_slicing(&[("session", &["REQ-001", "REQ-002"]), ("greeting", &["REQ-003"])]);
    let mut provider = Provider::answering([
        grouping.as_str(),
        REMINE_SESSION,
        DESIGN_ANSWER,
        first_slicing.as_str(),
    ]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            anchored("session.timeout", "Sessions time out after an hour.", "src/session.ts#L1-L3"),
            anchored(
                "session.renewal",
                "Renewing a session restarts its hour.",
                "src/session.ts#L4-L8",
            ),
            anchored(
                "greeting.behaviour",
                "GET /greeting returns 'howdy'.",
                "src/greeting.ts#L3-L9",
            ),
        ])),
    );
    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
    let first = current(&provider.storage);

    let second_slicing =
        separate_slicing(&[("greeting", &["REQ-001"]), ("session", &["REQ-002", "REQ-003"])]);
    let mut provider = Provider::over(
        Arc::clone(&provider.storage),
        [grouping.as_str(), REMINE_SESSION, DESIGN_ANSWER, second_slicing.as_str()],
    );
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            anchored(
                "greeting.behaviour",
                "GET /greeting returns 'howdy'.",
                "src/greeting.ts#L3-L9",
            ),
            anchored("session.timeout", "Sessions time out after an hour.", "src/session.ts#L1-L6"),
            anchored(
                "session.renewal",
                "Renewing a session restarts its hour.",
                "src/session.ts#L7-L10",
            ),
        ])),
    );
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let diff = &envelope["diff"];
    assert_eq!(diff["from"], first, "{envelope}");
    assert_eq!(diff["spec"]["added"], serde_json::json!([]), "{envelope}");
    assert_eq!(diff["spec"]["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(
        diff["spec"]["changed"],
        serde_json::json!([
            {"id": "REQ-001", "subject": "greeting.behaviour", "was": "REQ-003", "fields": ["id"]},
            {"id": "REQ-002", "subject": "session.timeout", "was": "REQ-001", "fields": ["id", "sources"]},
            {"id": "REQ-003", "subject": "session.renewal", "was": "REQ-002", "fields": ["id", "sources"]},
        ]),
        "a requirement whose first candidate is taken pairs with its next: {envelope}"
    );
    provider.model.assert_exhausted();
}

// A requirement claim anchored at `path`.
fn anchored(subject: &str, statement: &str, path: &str) -> Claim {
    let mut claim = requirement(subject, statement);
    claim.path = Some(path.to_owned());
    claim
}

fn docs_evidence(requirements: &[(&str, &str)]) -> Evidence {
    let claims = requirements
        .iter()
        .flat_map(|(subject, statement)| {
            [
                requirement(subject, statement),
                claim(
                    ClaimKind::Criterion,
                    &format!("{subject}.check"),
                    ("criterion", "The behaviour is observable."),
                ),
            ]
        })
        .collect();
    evidence(claims)
}

// Out of requirement order on purpose: drafts are keyed by subject.
const REMINE_FIRST: &str = r#"{
  "preamble": ["The docs describe a greeting, a session timeout, and a legacy export."],
  "requirements": [
    {
      "subject": "legacy.export",
      "scenarios": [{"name": "Export", "when": "exports are produced", "then": "they ship nightly"}]
    },
    {
      "subject": "greeting.behaviour",
      "scenarios": [{"name": "Greeting", "when": "the greeting is requested", "then": "the response is hello"}]
    },
    {
      "subject": "session.timeout",
      "scenarios": [{"name": "Timeout", "when": "a session is idle for an hour", "then": "it times out"}]
    }
  ]
}"#;

// The timeout's scenario word for word, so it is not a change.
const REMINE_SECOND: &str = r#"{
  "preamble": ["The docs describe a greeting and a session timeout."],
  "requirements": [
    {
      "subject": "greeting.behaviour",
      "scenarios": [{"name": "Greeting", "when": "the greeting is requested", "then": "the response is howdy"}]
    },
    {
      "subject": "session.timeout",
      "scenarios": [{"name": "Timeout", "when": "a session is idle for an hour", "then": "it times out"}]
    }
  ]
}"#;

// Two session requirements beside the greeting, answered to both runs.
const REMINE_SESSION: &str = r#"{
  "preamble": ["The docs describe a greeting and a session that times out and renews."],
  "requirements": [
    {
      "subject": "greeting.behaviour",
      "scenarios": [{"name": "Greeting", "when": "the greeting is requested", "then": "the response is howdy"}]
    },
    {
      "subject": "session.timeout",
      "scenarios": [{"name": "Timeout", "when": "a session is idle for an hour", "then": "it times out"}]
    },
    {
      "subject": "session.renewal",
      "scenarios": [{"name": "Renewal", "when": "a session is renewed", "then": "its hour restarts"}]
    }
  ]
}"#;

// --- extraction ---

// Invalid adapter output is an internal error, not the operator's.
#[tokio::test]
async fn extras_missing() {
    let mut provider = Provider::idle();
    let mut bare = requirement("greeting.behaviour", "");
    bare.extras.clear();
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(vec![bare])));

    let envelope =
        fail(&provider, &["emery", "specify", &reference("docs")], 3, "server_error").await;
    assert!(
        envelope["message"].as_str().is_some_and(|message| message.contains(
            "- claim 0: `requirement` `greeting.behaviour` is missing extra `statement`"
        )),
        "{envelope}"
    );
}

// The failure reaches the boundary as the adapter put it, alone or beside a
// source that succeeded.
#[tokio::test]
async fn extract_fails() {
    let mut provider = Provider::idle();
    provider
        .source
        .evidence
        .insert("docs".to_string(), Err(bad_gateway!("source `docs`: the adapter exploded")));

    for argv in [
        &["emery", "specify", &reference("docs")][..],
        &["emery", "specify", &reference("docs"), &reference("api")][..],
    ] {
        let envelope = fail(&provider, argv, 4, "bad_gateway").await;
        assert_eq!(envelope["message"], "source `docs`: the adapter exploded", "{argv:?}");
    }
}

// The failure dispatched first — declaration order — is the run's.
#[tokio::test]
async fn two_failures() {
    let mut provider = Provider::idle();
    provider
        .source
        .evidence
        .insert("docs".to_string(), Err(bad_gateway!("source `docs`: the adapter exploded")));
    provider
        .source
        .evidence
        .insert("code".to_string(), Err(bad_gateway!("source `code`: the model is down")));

    let envelope = fail(
        &provider,
        &["emery", "specify", &reference("docs"), &reference("code")],
        4,
        "bad_gateway",
    )
    .await;
    assert_eq!(envelope["message"], "source `docs`: the adapter exploded");

    let envelope = fail(
        &provider,
        &["emery", "specify", &reference("code"), &reference("docs")],
        4,
        "bad_gateway",
    )
    .await;
    assert_eq!(envelope["message"], "source `code`: the model is down");
}

// A refusal is the operator's to fix, so it keeps the adapter's class.
#[tokio::test]
async fn extract_refuses() {
    let mut provider = Provider::idle();
    provider
        .source
        .evidence
        .insert("docs".to_string(), Err(bad_request!("source `docs`: the brief is empty")));

    let envelope =
        fail(&provider, &["emery", "specify", &reference("docs")], 1, "bad_request").await;
    assert_eq!(envelope["message"], "source `docs`: the brief is empty");
}

// `docs` is held forever, so a run waiting for every source would exceed the bound.
#[tokio::test]
async fn held_source() {
    let mut provider = Provider::idle();
    provider.source.held.insert("docs".to_string());
    provider
        .source
        .evidence
        .insert("code".to_string(), Err(bad_request!("source `code`: the brief is empty")));

    let envelope = tokio::time::timeout(
        Duration::from_secs(5),
        fail(
            &provider,
            &["emery", "specify", &reference("docs"), &reference("code")],
            1,
            "bad_request",
        ),
    )
    .await
    .expect("the refusal ends the run while `docs` is still pending");

    assert_eq!(envelope["message"], "source `code`: the brief is empty");
    let dispatched: Vec<String> = provider
        .source
        .calls
        .lock()
        .expect("calls")
        .iter()
        .map(|(_, input)| input.name.clone())
        .collect();
    assert_eq!(dispatched, ["docs", "code"], "both sources were dispatched before the refusal");
}

#[tokio::test]
async fn version_too_new() {
    let mut provider = Provider::idle();
    provider.source.versions.insert(guest("docs"), "99.0.0".to_string());

    fail(&provider, &["emery", "specify", &reference("docs")], 1, "unsupported-version").await;
}

// --- synthesis ---

// Each refusal is one finding once the backend's rounds are spent; nothing
// half-commits.
#[tokio::test]
async fn invalid_draft() {
    let one = |preamble: &str, subject: &str, scenarios: &str| {
        format!(
            r#"{{"preamble": [{preamble}], "requirements": [{{"subject": "{subject}", "scenarios": [{scenarios}]}}]}}"#
        )
    };
    let scenario = r#"{"name": "Greeting", "when": "greeted", "then": "hello"}"#;
    let restated = r#"{"name": "Greeting", "when": "get /greeting returns the static string 'hello'", "then": "hello"}"#;
    let refrain = r#"{"name": "Greeting", "when": "greeted", "then": "GET /greeting returns the static string 'hello'."}"#;
    let cases: Vec<(String, &str)> = vec![
        ("Not a spec at all.".to_string(), "schema and answer type disagree"),
        (
            r#"{"preamble": [], "requirements": []}"#.to_string(),
            "requirement `greeting.behaviour` is not drafted",
        ),
        (one("", "greeting.renamed", scenario), "`greeting.renamed` is not a requirement"),
        (
            format!(
                r#"{{"preamble": [], "requirements": [{{"subject": "greeting.behaviour", "scenarios": [{scenario}]}}, {{"subject": "greeting.behaviour", "scenarios": [{scenario}]}}]}}"#
            ),
            "drafted more than once",
        ),
        (one("", "greeting.behaviour", ""), "has no scenario"),
        (
            one("\"### Requirement: smuggled\"", "greeting.behaviour", scenario),
            "opens with the reserved marker `#`",
        ),
        (
            one(r#""Hello.\nSources: [other]""#, "greeting.behaviour", scenario),
            "opens with the reserved marker `Sources:`",
        ),
        (one("", "greeting.behaviour", restated), "scenario `when` restates the requirement"),
        (one("", "greeting.behaviour", refrain), "scenario `then` restates the requirement"),
    ];
    for (answer, fragment) in cases {
        let provider = Provider::answering([answer.as_str(), answer.as_str(), answer.as_str()]);
        let envelope =
            fail(&provider, &["emery", "specify", &reference("docs")], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }
}

// One `then` across requirements may be two behaviours sharing an outcome, so
// it is accepted as drafted and left to the log.
#[tokio::test]
async fn shared_then() {
    let draft = r#"{"preamble": [], "requirements": [
        {"subject": "greeting.behaviour", "scenarios": [{"name": "Greeting", "when": "the greeting is requested", "then": "the response is a greeting"}]},
        {"subject": "greeting.formal", "scenarios": [{"name": "Formal", "when": "the formal greeting is requested", "then": "the response is a greeting"}]}
    ]}"#;
    let mut provider = Provider::answering([draft, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            requirement("greeting.formal", "GET /greeting/formal returns 'good day'."),
        ])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let check = &provider.model.exchanges()[0];
    assert_eq!(check.tool, "check");
    assert!(check.outcome.is_ok(), "the shared outcome is no finding: {:?}", check.outcome);
    let id = current(&provider.storage);
    let spec: Value = serde_json::from_slice(&document(&provider.storage, &id, "spec.json"))
        .expect("the committed spec is JSON");
    let outcomes: Vec<&Value> = spec["requirements"]
        .as_array()
        .expect("requirements")
        .iter()
        .map(|requirement| &requirement["scenarios"][0]["then"])
        .collect();
    assert_eq!(outcomes, vec!["the response is a greeting", "the response is a greeting"]);
    provider.model.assert_exhausted();
}

// Past `SPEC_CHUNK` requirements the draft is chunked by stem: two turns run
// beside the design, the second pinned to no preamble, and the specification
// is assembled in id order whichever answers first.
#[tokio::test]
async fn chunked_draft() {
    let chunk = emery_engine::specify::SPEC_CHUNK;
    // `alpha` fills a chunk, `beta` and `gamma` merge into the next
    let stems: Vec<(&str, usize)> = vec![("alpha", chunk), ("beta", 2), ("gamma", 1)];
    let mut claims = Vec::new();
    for (stem, count) in &stems {
        for index in 0..*count {
            let statement = format!("`{stem}` does thing {index}.");
            claims.push(requirement(&format!("{stem}.thing-{index}"), &statement));
        }
    }
    let ids: Vec<String> = claims.iter().filter_map(|claim| claim.id.clone()).collect();
    let (first, second) = ids.split_at(chunk);
    let first_draft = spec_draft(first, &["Many things, drafted in chunks."]);
    let second_draft = spec_draft(second, &[]);
    let grouping = separate_grouping(claims.len());
    let numbered: Vec<String> = (1..=claims.len()).map(|n| format!("REQ-{n:03}")).collect();
    let alpha: Vec<&str> = numbered[..chunk].iter().map(String::as_str).collect();
    let beta: Vec<&str> = numbered[chunk..chunk + 2].iter().map(String::as_str).collect();
    let gamma: Vec<&str> = numbered[chunk + 2..].iter().map(String::as_str).collect();
    let slicing = separate_slicing(&[("alpha", &alpha), ("beta", &beta), ("gamma", &gamma)]);
    let mut provider = Provider::answering([
        grouping.as_str(),
        first_draft.as_str(),
        second_draft.as_str(),
        DESIGN_ANSWER,
        slicing.as_str(),
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(claims)));

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    // the first chunk asks for the preamble, the second is pinned to none
    let seen = provider.model.seen();
    let schemas: Vec<(&str, Value)> = seen[1..=2]
        .iter()
        .map(|turn| {
            let SeenFormat::Schema { name, schema } = &turn.format else {
                panic!("the draft is steered by schema");
            };
            (name.as_str(), serde_json::from_str(schema).expect("the steering schema is JSON"))
        })
        .collect();
    assert_eq!(schemas[0].0, "spec-draft");
    assert_eq!(schemas[1].0, "spec-draft");
    assert_eq!(schemas[0].1["properties"]["requirements"]["maxItems"], chunk);
    assert!(schemas[0].1["properties"]["preamble"].get("maxItems").is_none());
    assert_eq!(schemas[1].1["properties"]["requirements"]["maxItems"], 3);
    assert_eq!(schemas[1].1["properties"]["preamble"]["maxItems"], 0);
    assert_eq!(
        schemas[1].1["$defs"]["Draft"]["properties"]["subject"]["enum"],
        serde_json::json!(["beta.thing-0", "beta.thing-1", "gamma.thing-0"]),
        "the second chunk is the merged small stems"
    );
    let second_request = seen[2].messages.join("\n");
    assert!(second_request.contains("leave it empty"), "{second_request}");
    assert!(!second_request.contains("- REQ-001 `alpha.thing-0`"), "{second_request}");
    assert!(second_request.contains("- REQ-028 `gamma.thing-0`"), "{second_request}");

    // one specification, the preamble from the first chunk, every id in order
    let id = current(&provider.storage);
    let spec: Value = serde_json::from_slice(&document(&provider.storage, &id, "spec.json"))
        .expect("the committed spec is JSON");
    assert_eq!(spec["preamble"], serde_json::json!(["Many things, drafted in chunks."]));
    let committed: Vec<&str> = spec["requirements"]
        .as_array()
        .expect("requirements")
        .iter()
        .map(|requirement| requirement["id"].as_str().expect("id"))
        .collect();
    assert_eq!(committed, numbered);
    assert_eq!(spec["requirements"][chunk]["subject"], "beta.thing-0");
    provider.model.assert_exhausted();
}

mod unknown {
    use serde_json::Value;

    use super::{
        DESIGN_ANSWER, Provider, SPEC_ANSWER, assert_message, cli_ok, current, docs_evidence,
        document, fail, reference, shown,
    };

    // `SPEC_ANSWER` with its one outcome left `[unknown]`.
    fn draft() -> String {
        let unknown =
            SPEC_ANSWER.replace(r#""then": "the response is `hello`""#, r#""then": "[unknown]""#);
        assert_ne!(unknown, SPEC_ANSWER, "the fixture carries the patched line");
        unknown
    }

    // No criterion covers the requirement, so `[unknown]` commits in place of an
    // invented outcome.
    #[tokio::test]
    async fn uncovered() {
        let unknown = draft();
        let provider = Provider::answering([unknown.as_str(), DESIGN_ANSWER]);
        cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

        let id = current(&provider.storage);
        let spec: Value = serde_json::from_slice(&document(&provider.storage, &id, "spec.json"))
            .expect("the committed spec is JSON");
        assert_eq!(spec["requirements"][0]["scenarios"][0]["then"], "[unknown]", "{spec}");
        let shown = shown(&provider, "spec").await;
        assert!(shown.contains("- **THEN** [unknown]"), "{shown}");
        provider.model.assert_exhausted();
    }

    // A criterion covers the requirement, so its outcome is evidenced and
    // `[unknown]` is refused.
    #[tokio::test]
    async fn covered() {
        let unknown = draft();
        let mut provider =
            Provider::answering([unknown.as_str(), unknown.as_str(), unknown.as_str()]);
        provider.source.evidence.insert(
            "docs".to_string(),
            Ok(docs_evidence(&[(
                "greeting.behaviour",
                "GET /greeting returns the static string 'hello'.",
            )])),
        );

        let envelope =
            fail(&provider, &["emery", "specify", &reference("docs")], 1, "bad_request").await;
        assert_message(&envelope, "is `[unknown]` but the requirement is covered");
        provider.model.assert_exhausted();
    }
}

// The schema steers and the check gates: the finding rides the correction, and
// the corrected draft commits.
#[tokio::test]
async fn repaired_draft() {
    let missing_scenario =
        SPEC_ANSWER.replace(r#""then": "the response is `hello`""#, r#""then": """#);
    assert_ne!(missing_scenario, SPEC_ANSWER, "the fixture carries the patched line");
    let provider = Provider::answering([missing_scenario.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", &reference("source")]).await;

    let draft = &provider.model.seen()[0];
    assert!(draft.check, "acceptance is the engine's check, not the reply text");
    let SeenFormat::Schema { name, schema } = &draft.format else {
        panic!("the draft is steered by schema");
    };
    assert_eq!(name, "spec-draft");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["requirements"]["minItems"], 1);
    assert_eq!(schema["properties"]["requirements"]["maxItems"], 1);
    let entry = &schema["$defs"]["Draft"]["properties"];
    assert_eq!(
        entry["subject"]["enum"],
        serde_json::json!(["greeting.behaviour"]),
        "the requirement subjects ride the schema as a hint"
    );
    assert_eq!(entry["subject"]["type"], "string", "the derive is intact");
    assert_eq!(entry["scenarios"]["minItems"], 1);

    let check = &provider.model.exchanges()[0];
    assert_eq!(check.tool, "check");
    let correction = check.outcome.as_ref().expect_err("the first draft is rejected");
    assert!(correction.contains("## Previous answer (rejected)"), "{correction}");
    assert!(correction.contains("## Findings"), "{correction}");
    assert!(correction.contains("scenario `then` is blank"), "{correction}");
    let id = current(&provider.storage);
    let spec = document(&provider.storage, &id, "spec.json");
    assert_eq!(String::from_utf8_lossy(&spec), SPEC_REVISION, "the repaired draft is committed");
    provider.model.assert_exhausted();
}

// The design leg is gated as the spec leg is, one finding per case.
#[tokio::test]
async fn invalid_design() {
    let overview = |text: &str| {
        format!(
            r#"{{"preamble": [], "sections": [{{"kind": "overview", "blocks": [{{"text": "{text}"}}]}}]}}"#
        )
    };
    let cases: Vec<(String, &str)> = vec![
        ("   ".to_string(), "schema and answer type disagree"),
        (r#"{"preamble": [], "sections": []}"#.to_string(), "`## Overview` is required but absent"),
        (
            r#"{"preamble": [], "sections": [{"kind": "decisions", "blocks": [{"text": "Static."}]}]}"#
                .to_string(),
            "schema and answer type disagree",
        ),
        (overview("### Requirement: greeting.behaviour"), "opens with the reserved marker `#`"),
        (overview("The endpoint is static (from nobody)."), "cites source `nobody`, which is not bound"),
    ];
    for (answer, fragment) in cases {
        let provider =
            Provider::answering([SPEC_ANSWER, answer.as_str(), answer.as_str(), answer.as_str()]);
        let envelope =
            fail(&provider, &["emery", "specify", &reference("docs")], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }
}

// The evidence plans the sections: a `type` claim requires one `domain-model`
// reference, and a section no claim informs may not appear.
#[tokio::test]
async fn dishonest_design() {
    let signature = "interface Greeting { text: string }";
    let evidence = || {
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            claim(ClaimKind::Type, "greeting.type", ("signature", signature)),
        ]))
    };
    let draft = |sections: &str| format!(r#"{{"preamble": [], "sections": [{sections}]}}"#);

    // `(from the browser)` is prose: a citation key is one token
    let overview = r#"{"kind": "overview", "blocks": [{"text": "Requests arrive (from the browser) and (from docs) they route."}]}"#;
    let domain = r#"{"kind": "domain-model", "blocks": [{"text": "The greeting payload is one string field."}, {"type": "greeting.type"}]}"#;
    let honest = draft(&format!("{overview}, {domain}"));
    let cases: Vec<(String, &str)> = vec![
        (draft(overview), "`## Domain model` is required but absent"),
        (
            draft(&format!(
                r#"{overview}, {domain}, {{"kind": "ui-layout", "blocks": [{{"text": "- page"}}]}}"#
            )),
            "`## UI / layout` is present but no claim informs it",
        ),
        (
            draft(&format!(
                r#"{overview}, {{"kind": "domain-model", "blocks": [{{"text": "`{signature}`"}}]}}"#
            )),
            "type `greeting.type` is never referenced",
        ),
        (
            draft(&format!(
                r#"{overview}, {{"kind": "domain-model", "blocks": [{{"type": "greeting.type"}}, {{"type": "greeting.type"}}]}}"#
            )),
            "type `greeting.type` is referenced 2 times",
        ),
        (
            draft(&format!(
                r#"{{"kind": "overview", "blocks": [{{"type": "greeting.other"}}]}}, {domain}"#
            )),
            "type blocks belong under `## Domain model`",
        ),
    ];
    for (answer, fragment) in cases {
        let mut provider =
            Provider::answering([SPEC_ANSWER, answer.as_str(), answer.as_str(), answer.as_str()]);
        provider.source.evidence.insert("docs".to_string(), evidence());
        let envelope =
            fail(&provider, &["emery", "specify", &reference("docs")], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }

    let mut provider = Provider::answering([SPEC_ANSWER, honest.as_str()]);
    provider.source.evidence.insert("docs".to_string(), evidence());
    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let draft = &provider.model.seen()[1];
    let SeenFormat::Schema { name, schema } = &draft.format else {
        panic!("the design is steered by schema");
    };
    assert_eq!(name, "design-draft");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["sections"]["minItems"], 2);
    let kind = &schema["$defs"]["Section"]["properties"]["kind"];
    assert_eq!(
        kind["enum"],
        serde_json::json!(["overview", "domain-model", "technical-logic", "observability"])
    );
    assert_eq!(kind["type"], "string");
    assert!(kind.get("$ref").is_none() && schema["$defs"].get("SectionKind").is_none());
    let variants = schema["$defs"]["Block"]["oneOf"].as_array().expect("Block is a oneOf");
    let block = variants
        .iter()
        .find(|variant| variant["required"] == serde_json::json!(["type"]))
        .expect("the type block variant");
    assert_eq!(block["properties"]["type"]["enum"], serde_json::json!(["greeting.type"]));

    let id = current(&provider.storage);
    let rendered = format!(
        "---\nemery: 5\nrevision: {id}\n---\n\n# Design\n\n## Overview\n\n\
         Requests arrive (from the browser) and (from docs) they route.\n\n\
         ## Domain model\n\nThe greeting payload is one string field.\n\n\
         Type: greeting.type\n```\n{signature}\n```\n"
    );
    assert_eq!(
        shown(&provider, "design").await,
        rendered,
        "the signature is rendered verbatim, labelled by its key, where the draft placed it"
    );
    provider.model.assert_exhausted();
}

// A `type` claim without an id is keyed by its path with the line anchor
// stripped, so two runs that re-anchor the same declaration commit
// byte-identical designs.
#[tokio::test]
async fn type_reanchored() {
    let signature = "interface Greeting { text: string }";
    let evidence = |anchor: &str| {
        let mut typed = claim(ClaimKind::Type, "unused", ("signature", signature));
        typed.id = None;
        typed.path = Some(format!("src/greeting.ts#{anchor}"));
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            typed,
        ]))
    };
    let design = r#"{"preamble": [], "sections": [
        {"kind": "overview", "blocks": [{"text": "The greeting is one static endpoint."}]},
        {"kind": "domain-model", "blocks": [{"type": "src/greeting.ts"}]}
    ]}"#;

    let mut provider = Provider::answering([SPEC_ANSWER, design]);
    provider.source.evidence.insert("docs".to_string(), evidence("L1-L4"));
    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
    let id = current(&provider.storage);
    let first = document(&provider.storage, &id, "design.json");

    let mut provider = Provider::over(Arc::clone(&provider.storage), [SPEC_ANSWER, design]);
    provider.source.evidence.insert("docs".to_string(), evidence("L10-L14"));
    let resp = cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");
    assert_eq!(current(&provider.storage), id, "the re-anchored run keeps its id");
    assert_eq!(document(&provider.storage, &id, "design.json"), first, "design bytes are stable");
    let rendered = shown(&provider, "design").await;
    assert!(rendered.contains("Type: src/greeting.ts\n"), "{rendered}");
    provider.model.assert_exhausted();
}

// Three `type` claims declaring one name: the second is keyed apart by its
// path, the third — at that same path — by a counter, so every signature
// reaches the plan and the design.
#[tokio::test]
async fn type_collisions() {
    let signatures = [
        "interface Greeting { text: string }",
        "type Greeting = { text: string }",
        "class Greeting { text = '' }",
    ];
    let typed = |anchor: Option<&str>, signature: &str| {
        let mut typed = claim(ClaimKind::Type, "greeting.type", ("signature", signature));
        typed.path = anchor.map(str::to_string);
        typed
    };
    let mut provider = Provider::answering([
        SPEC_ANSWER,
        r#"{"preamble": [], "sections": [
            {"kind": "overview", "blocks": [{"text": "The greeting is one static endpoint."}]},
            {"kind": "domain-model", "blocks": [
                {"type": "greeting.type"},
                {"type": "greeting.type (src/greeting.ts#L1)"},
                {"type": "greeting.type (src/greeting.ts#L1, 2)"}
            ]}
        ]}"#,
    ]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            typed(None, signatures[0]),
            typed(Some("src/greeting.ts#L1"), signatures[1]),
            typed(Some("src/greeting.ts#L1"), signatures[2]),
        ])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let SeenFormat::Schema { schema, .. } = &provider.model.seen()[1].format else {
        panic!("the design is steered by schema");
    };
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    let block = schema["$defs"]["Block"]["oneOf"]
        .as_array()
        .and_then(|variants| {
            variants.iter().find(|variant| variant["required"] == serde_json::json!(["type"]))
        })
        .expect("the type block variant");
    assert_eq!(
        block["properties"]["type"]["enum"],
        serde_json::json!([
            "greeting.type",
            "greeting.type (src/greeting.ts#L1)",
            "greeting.type (src/greeting.ts#L1, 2)"
        ]),
        "every declaration is offered under its own key"
    );
    let rendered = shown(&provider, "design").await;
    for signature in signatures {
        assert!(rendered.contains(&format!("```\n{signature}\n```")), "{rendered}");
    }
    let plan = shown(&provider, "plan").await;
    assert!(
        plan.contains(
            "Types: [greeting.type, greeting.type (src/greeting.ts#L1), greeting.type \
             (src/greeting.ts#L1, 2)]"
        ),
        "the one slice owns every key in the design's order: {plan}"
    );
    provider.model.assert_exhausted();
}

// Two `type` claims under one id declare two identifiers: each is keyed by
// its `name`, so neither displaces the other and no key carries a path.
#[tokio::test]
async fn type_named() {
    let typed = |name: &str, signature: &str| {
        let mut typed = claim(ClaimKind::Type, "greeting.type", ("signature", signature));
        typed.extras.insert("name".to_string(), Value::String(name.to_string()));
        typed.path = Some(format!("src/{}.ts#L1", name.to_lowercase()));
        typed
    };
    let mut provider = Provider::answering([
        SPEC_ANSWER,
        r#"{"preamble": [], "sections": [
            {"kind": "overview", "blocks": [{"text": "The greeting is one static endpoint."}]},
            {"kind": "domain-model", "blocks": [{"type": "Greeting"}, {"type": "Salutation"}]}
        ]}"#,
    ]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            typed("Greeting", "interface Greeting { text: string }"),
            typed("Salutation", "type Salutation = Greeting"),
        ])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let SeenFormat::Schema { schema, .. } = &provider.model.seen()[1].format else {
        panic!("the design is steered by schema");
    };
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    let block = schema["$defs"]["Block"]["oneOf"]
        .as_array()
        .and_then(|variants| {
            variants.iter().find(|variant| variant["required"] == serde_json::json!(["type"]))
        })
        .expect("the type block variant");
    assert_eq!(block["properties"]["type"]["enum"], serde_json::json!(["Greeting", "Salutation"]));
    let plan = shown(&provider, "plan").await;
    assert!(plan.contains("Types: [Greeting, Salutation]"), "{plan}");
    provider.model.assert_exhausted();
}

// A failing model is asked once more; a second failure is the run's.
#[tokio::test]
async fn model_fails() {
    let provider = Provider {
        model: Scripted::new([
            Err(ModelError::Backend("scripted transport failure".into())),
            Err(ModelError::Backend("scripted transport failure".into())),
        ]),
        ..Provider::idle()
    };
    fail(&provider, &["emery", "specify", &reference("docs")], 4, "bad_gateway").await;
    provider.model.assert_exhausted();
}

// One transport failure on the spec draft is retried and the run completes.
#[tokio::test]
async fn model_recovers() {
    let reply = |answer: &str| {
        Ok(Reply {
            answer: answer.to_owned(),
            usage: None,
        })
    };
    let provider = Provider {
        model: Scripted::new([
            Err(ModelError::Backend("scripted transport failure".into())),
            reply(SPEC_ANSWER),
            reply(DESIGN_ANSWER),
        ]),
        ..Provider::idle()
    };
    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
    let seen = provider.model.seen();
    assert_eq!(seen.len(), 3, "the failed ask and the two answered ones");
    assert_eq!(seen[0].messages, seen[1].messages, "the retry puts the same question");
    provider.model.assert_exhausted();
}

// --- slicing ---

// Three stems and two types: the model merges two stems, the engine refuses
// the draft that leaves a requirement out, then numbers the corrected slices by
// their lowest requirement and writes every list in canonical order — the
// requirements the answer listed backwards in id order, the types in key
// order. The slicing runs beside the two drafts from the bases, so its request
// carries the requirement outline and the type keys, never a rendered document.
#[tokio::test]
async fn sliced() {
    let refused = r#"{"preamble": [], "slices": [
        {"name": "authentication", "requirements": ["REQ-001", "REQ-002"]},
        {"name": "orders", "requirements": ["REQ-003"], "types": ["orders.order", "orders.line"], "depends-on": ["authentication"]}
    ]}"#;
    let grouping = separate_grouping(4);
    let mut provider = Provider::answering([
        grouping.as_str(),
        SLICED_SPEC,
        SLICED_DESIGN,
        refused,
        SLICING_ANSWER,
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(sliced_evidence()));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    // the envelope carries the committed plan's waves
    let committed: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        committed["waves"],
        serde_json::json!([["SLICE-001"], ["SLICE-002"]]),
        "a chain of two is two waves: {committed}"
    );

    // the slicing request carries the stems, the keys, and the requirement outline
    let slicing = &provider.model.seen()[3];
    let SeenFormat::Schema { name, schema } = &slicing.format else {
        panic!("the slicing is steered by schema");
    };
    assert_eq!(name, "slicing");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["slices"]["minItems"], 1);
    assert_eq!(schema["properties"]["slices"]["maxItems"], 3, "one slice per stem at most");
    let draft = &schema["$defs"]["Draft"]["properties"];
    assert_eq!(draft["requirements"]["minItems"], 1);
    assert_eq!(
        draft["requirements"]["items"]["enum"],
        serde_json::json!(["REQ-001", "REQ-002", "REQ-003", "REQ-004"]),
        "the run's requirement ids ride the schema as a hint"
    );
    assert_eq!(
        draft["types"]["items"]["enum"],
        serde_json::json!(["orders.line", "orders.order"]),
        "the design's type keys ride the schema in key order"
    );
    let request = slicing.messages.join("\n");
    assert!(request.contains("- `auth` — REQ-001\n"), "{request}");
    assert!(request.contains("- `orders` — REQ-003, REQ-004\n"), "{request}");
    assert!(request.contains("- `orders.line`\n- `orders.order`\n"), "the keys ride: {request}");
    assert!(
        request.contains("- REQ-004 `orders.cancel` — Status: unknown"),
        "the requirement outline rides: {request}"
    );
    assert!(!request.contains("### Requirement:"), "no rendered spec rides: {request}");
    assert!(!request.contains("## Domain model"), "no rendered design rides: {request}");

    // the design request carries the same outline in place of a rendered spec
    let design = provider.model.seen()[2].messages.join("\n");
    assert!(design.contains("- REQ-004 `orders.cancel` — Status: unknown"), "{design}");
    assert!(!design.contains("### Requirement:"), "no rendered spec rides: {design}");

    // the refusal is the correction; the corrected slicing commits
    let check = &provider.model.exchanges()[3];
    assert_eq!(check.tool, "check");
    let correction = check.outcome.as_ref().expect_err("the first slicing is rejected");
    assert!(correction.contains("`REQ-004` is in no slice"), "{correction}");
    let id = current(&provider.storage);
    assert_eq!(
        String::from_utf8_lossy(&document(&provider.storage, &id, "plan.json")),
        SLICED_REVISION,
        "plan.json is the canonical revision"
    );
    assert_eq!(
        shown(&provider, "plan").await,
        projection(SLICED_RENDERED, &id),
        "show renders plan.md"
    );
    let resp = cli_ok(&provider, &["emery", "--format", "json", "show", "plan"]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let document: Value =
        serde_json::from_str(SLICED_REVISION).expect("the revision fixture is JSON");
    assert_eq!(envelope["document"], document, "the envelope carries the typed plan");
    provider.model.assert_exhausted();
}

// Past `SLICE_CAP` requirements a stem is floored at its sub-stems: the brief
// lists them under it, the schema admits one slice per floor, a draft that
// splits a sub-stem is refused, and one that splits the stem along them is
// numbered like any other.
#[tokio::test]
async fn oversized_stem() {
    let cap = emery_engine::specify::SLICE_CAP;
    // three sub-stems past the cap together, one stem within it, one `SPEC_CHUNK` in all
    let reads = cap / 2;
    let posts = cap / 2 + 1;
    let deletes = 2;
    let verbs: Vec<(&str, usize)> = vec![("get", reads), ("post", posts), ("delete", deletes)];
    let mut claims = Vec::new();
    for (verb, count) in &verbs {
        for index in 0..*count {
            let statement = format!("`{verb}` does thing {index}.");
            claims.push(requirement(&format!("orders.{verb}.thing-{index}"), &statement));
        }
    }
    claims.push(requirement("auth.login", "Users sign in with a credential."));
    let subjects: Vec<String> = claims.iter().filter_map(|claim| claim.id.clone()).collect();
    let numbered: Vec<String> = (1..=claims.len()).map(|n| format!("REQ-{n:03}")).collect();
    let ids: Vec<&str> = numbered.iter().map(String::as_str).collect();
    let (get, others) = ids.split_at(reads);
    let (post, others) = others.split_at(posts);
    let (delete, auth) = others.split_at(deletes);
    let writes: Vec<&str> = post.iter().chain(delete).copied().collect();
    let mut straddling = vec![get[reads - 1]];
    straddling.extend_from_slice(&writes);
    let grouping = separate_grouping(claims.len());
    let draft = spec_draft(&subjects, &["Orders in three verbs, and sign-in."]);
    let refused = separate_slicing(&[
        ("orders-reading", &get[..reads - 1]),
        ("orders-writing", &straddling),
        ("authentication", auth),
    ]);
    let accepted = separate_slicing(&[
        ("orders-reading", get),
        ("orders-writing", &writes),
        ("authentication", auth),
    ]);
    let mut provider = Provider::answering([
        grouping.as_str(),
        draft.as_str(),
        DESIGN_ANSWER,
        refused.as_str(),
        accepted.as_str(),
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(claims)));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    // the brief lists the sub-stems under the stem and admits one slice per floor
    let slicing = &provider.model.seen()[3];
    let SeenFormat::Schema { name, schema } = &slicing.format else {
        panic!("the slicing is steered by schema");
    };
    assert_eq!(name, "slicing");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["slices"]["maxItems"], 4, "three sub-stems and a stem");
    let request = slicing.messages.join("\n");
    let baseline = format!(
        "- `orders` — {total} requirements, past the cap of {cap}, so its sub-stems are the \
         floor:\n  - `orders.get` — {}\n  - `orders.post` — {}\n  - `orders.delete` — {}\n- \
         `auth` — {}\n",
        get.join(", "),
        post.join(", "),
        delete.join(", "),
        auth.join(", "),
        total = reads + posts + deletes,
    );
    assert!(request.contains(&baseline), "{request}");
    assert!(request.contains("an answer that splits a sub-stem across slices is refused"));

    // a sub-stem split is the correction; the split along the sub-stems commits
    let check = &provider.model.exchanges()[3];
    assert_eq!(check.tool, "check");
    let correction = check.outcome.as_ref().expect_err("the straddling slicing is rejected");
    assert!(
        correction.contains(
            "requirements sharing the stem `orders.get` are split across `orders-reading`, \
             `orders-writing`"
        ),
        "{correction}"
    );
    let committed: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        committed["waves"],
        serde_json::json!([["SLICE-001", "SLICE-002", "SLICE-003"]]),
        "three slices depending on nothing are one wave: {committed}"
    );
    let id = current(&provider.storage);
    let plan: Value = serde_json::from_slice(&document(&provider.storage, &id, "plan.json"))
        .expect("the committed plan is JSON");
    let slices: Vec<(&str, &str)> = plan["slices"]
        .as_array()
        .expect("slices")
        .iter()
        .map(|slice| (slice["id"].as_str().expect("id"), slice["name"].as_str().expect("name")))
        .collect();
    assert_eq!(
        slices,
        vec![
            ("SLICE-001", "orders-reading"),
            ("SLICE-002", "orders-writing"),
            ("SLICE-003", "authentication"),
        ]
    );
    assert_eq!(plan["slices"][1]["requirements"], serde_json::json!(writes));
    provider.model.assert_exhausted();
}

// A stem past the cap cut one slice per sub-stem is refused where two of
// its slices would fit one within the cap, the smallest pair named; the
// answer merging them commits, the slice sizes it leaves no longer fitting
// together.
#[tokio::test]
async fn oversized_stem_cut_fine() {
    let cap = emery_engine::specify::SLICE_CAP;
    let reads = cap / 2;
    let posts = cap / 2 + 1;
    let deletes = 2;
    let verbs: Vec<(&str, usize)> = vec![("get", reads), ("post", posts), ("delete", deletes)];
    let mut claims = Vec::new();
    for (verb, count) in &verbs {
        for index in 0..*count {
            let statement = format!("`{verb}` does thing {index}.");
            claims.push(requirement(&format!("orders.{verb}.thing-{index}"), &statement));
        }
    }
    claims.push(requirement("auth.login", "Users sign in with a credential."));
    let subjects: Vec<String> = claims.iter().filter_map(|claim| claim.id.clone()).collect();
    let numbered: Vec<String> = (1..=claims.len()).map(|n| format!("REQ-{n:03}")).collect();
    let ids: Vec<&str> = numbered.iter().map(String::as_str).collect();
    let (get, others) = ids.split_at(reads);
    let (post, others) = others.split_at(posts);
    let (delete, auth) = others.split_at(deletes);
    let reads_and_deletes: Vec<&str> = get.iter().chain(delete).copied().collect();
    let grouping = separate_grouping(claims.len());
    let draft = spec_draft(&subjects, &["Orders in three verbs, and sign-in."]);
    let refused = separate_slicing(&[
        ("orders-reading", get),
        ("orders-posting", post),
        ("orders-deleting", delete),
        ("authentication", auth),
    ]);
    let accepted = separate_slicing(&[
        ("orders-reading", &reads_and_deletes),
        ("orders-posting", post),
        ("authentication", auth),
    ]);
    let mut provider = Provider::answering([
        grouping.as_str(),
        draft.as_str(),
        DESIGN_ANSWER,
        refused.as_str(),
        accepted.as_str(),
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(claims)));

    cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    let request = provider.model.seen()[3].messages.join("\n");
    assert!(
        request.contains(
            "Cut the stem at as few sub-stem boundaries as the cap allows, never one slice per \
             sub-stem"
        ),
        "{request}"
    );
    let check = &provider.model.exchanges()[3];
    assert_eq!(check.tool, "check");
    let correction = check.outcome.as_ref().expect_err("one slice per sub-stem is rejected");
    assert!(
        correction.contains(&format!(
            "the stem `orders` is cut into 3 slices, yet `orders-deleting` ({deletes} \
             requirements) and `orders-reading` ({reads}) fit one slice of {} within the cap of \
             {cap}: merge the stem's slices until no two fit together",
            reads + deletes
        )),
        "{correction}"
    );
    assert!(
        !correction.contains("split across"),
        "no sub-stem is split, so the cut is the one finding: {correction}"
    );
    let id = current(&provider.storage);
    let plan: Value = serde_json::from_slice(&document(&provider.storage, &id, "plan.json"))
        .expect("the committed plan is JSON");
    assert_eq!(plan["slices"][0]["requirements"], serde_json::json!(reads_and_deletes));
    assert_eq!(plan["slices"].as_array().map(Vec::len), Some(3));
    provider.model.assert_exhausted();
}

// The slicing leg is gated as the drafts are, one finding per case.
#[tokio::test]
async fn invalid_plan() {
    let plan = |slices: &str| format!(r#"{{"preamble": [], "slices": [{slices}]}}"#);
    let all = |extra: &str| {
        plan(&format!(
            r#"{{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]{extra}}}"#
        ))
    };
    let cases: Vec<(String, &str)> = vec![
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002", "REQ-003"]}, {"name": "orders", "requirements": ["REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "requirements sharing the stem `orders` are split across `authentication`, `orders`",
        ),
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002"]}, {"name": "orders", "requirements": ["REQ-003"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "`REQ-004` is in no slice",
        ),
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002", "REQ-003"]}, {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "`REQ-003` appears in more than one slice",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004", "REQ-009"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "slice `all`: `REQ-009` is not a requirement",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}, {"name": "empty", "requirements": []}"#,
            ),
            "slice `empty` has no requirement",
        ),
        (
            plan(
                r#"{"name": "All Slices", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "slice name `All Slices` is not kebab-case",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002"]}, {"name": "all", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "slice `all` is drafted more than once",
        ),
        (
            all(r#", "depends-on": ["nothing"]"#),
            "slice `all` depends on `nothing`, which is not a slice",
        ),
        (all(r#", "depends-on": ["all"]"#), "slice `all` depends on itself"),
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "depends-on": ["orders"]}, {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"], "depends-on": ["authentication"]}"#,
            ),
            "slices `authentication`, `orders` cannot be ordered",
        ),
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "types": ["orders.order"]}, {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "type `orders.order` is owned by 2 slices: `authentication`, `orders`",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.line"]}"#,
            ),
            "type `orders.order` is owned by no slice",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.order", "orders.line", "orders.receipt"]}"#,
            ),
            "`orders.receipt` is not a type in the design",
        ),
        (
            all(r#", "brief": ["Depends on: nothing"]"#),
            "slice `all` brief: a paragraph line opens with the reserved marker `Depends on:`",
        ),
        ("not json".to_string(), "schema and answer type disagree"),
    ];
    for (answer, fragment) in cases {
        let grouping = separate_grouping(4);
        let mut provider = Provider::answering([
            grouping.as_str(),
            SLICED_SPEC,
            SLICED_DESIGN,
            answer.as_str(),
            answer.as_str(),
            answer.as_str(),
        ]);
        provider.source.evidence.insert("docs".to_string(), Ok(sliced_evidence()));
        let envelope =
            fail(&provider, &["emery", "specify", &reference("docs")], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }
}

// Sign-in, its session, and two order behaviours over two `type` claims: three
// stems, so the slicing is the model's.
fn sliced_evidence() -> Evidence {
    evidence(vec![
        requirement("auth.login", "Users sign in with a credential."),
        requirement("session.timeout", "Sessions expire after an hour of inactivity."),
        requirement("orders.create", "A signed-in user places an order."),
        requirement("orders.cancel", "A signed-in user cancels an open order."),
        claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
        claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
    ])
}

const SLICED_SPEC: &str = r#"{"preamble": ["Sign-in, its session, and orders over both."], "requirements": [
    {"subject": "auth.login", "scenarios": [{"name": "Login", "when": "a valid credential is presented", "then": "the caller is signed in"}]},
    {"subject": "session.timeout", "scenarios": [{"name": "Timeout", "when": "a session is idle for an hour", "then": "it times out"}]},
    {"subject": "orders.create", "scenarios": [{"name": "Create", "when": "a signed-in caller places an order", "then": "the order is created"}]},
    {"subject": "orders.cancel", "scenarios": [{"name": "Cancel", "when": "a signed-in caller cancels an order", "then": "the order is cancelled"}]}
]}"#;

const SLICED_DESIGN: &str = r#"{"preamble": [], "sections": [
    {"kind": "overview", "blocks": [{"text": "Sign-in issues a session; orders are placed and cancelled under it."}]},
    {"kind": "domain-model", "blocks": [{"type": "orders.order"}, {"type": "orders.line"}]}
]}"#;

// --- config file ---

// The list is checked whole before a single adapter loads.
#[tokio::test]
async fn config_file() {
    let cases: &[(&str, u8, &str, &str)] = &[
        ("not toml [", 1, "bad_request", "TOML parse error"),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\nbranch = \
             \"main\"\n",
            1,
            "bad_request",
            "unknown field `branch`",
        ),
        ("", 1, "specify-source-required", ""),
        (
            "[sources.docs]\nadapter = \"emery:documentation@1.2.0\"\n",
            1,
            "bad_request",
            "unknown field `sources`",
        ),
        (
            "[[source]]\nname = \"intent\"\nadapter = \"emery:intent@1.0.0\"\nvalue = \"text\"\n",
            1,
            "bad_request",
            "unknown field `value`",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
             \"docs\"\ndescription = \"text\"\n",
            1,
            "bad_request",
            "both `path` and `description`",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
             [[source]]\nname = \"docs\"\nadapter = \"emery:intent@1.0.0\"\n",
            1,
            "bad_request",
            "appears twice",
        ),
        (
            "[[source]]\nname = \"Docs\"\nadapter = \"emery:documentation@1.2.0\"\n",
            1,
            "bad_request",
            "is not a kebab-case name",
        ),
        (
            "[[source]]\nname = \"local\"\nadapter = \"acme:source@1.0.0\"\n\
             digest = \"sha256:9f2c44aa\"\n",
            1,
            "bad_request",
            "is not 64 hex characters",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\ngit = \
             \"https://github.com/acme/api@v2\"\n",
            1,
            "bad_request",
            "unknown field `git`",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\nurl = \
             \"https://example.com/openapi.yaml\"\n",
            1,
            "bad_request",
            "unknown field `url`",
        ),
        (
            "[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\nregistry = \
             \"registry.acme.io\"\n",
            1,
            "bad_request",
            "unknown field `registry`",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
             \"../../outside\"\n",
            1,
            "bad_request",
            "../../outside",
        ),
    ];
    for (body, exit, code, fragment) in cases {
        let scratch = Scratch::new();
        let config = scratch.config(body);
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], *exit, code).await;
        if !fragment.is_empty() {
            assert_message(&envelope, fragment);
        }
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a refused list gates nothing: {body}"
        );
    }

    // an unreadable file is a filesystem error
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--config", "nonexistent/emery.toml"], 3, "server_error")
        .await;

    // host-absolute and escaping paths never reach the guest
    for path in ["/nonexistent/emery.toml", "../emery.toml"] {
        fail(&provider, &["emery", "specify", "--config", path], 1, "bad_request").await;
    }
}

// A file's `adapter` is an exact package reference, refused at the field
// otherwise: no path, no bare name, no unversioned or unnamespaced package.
#[tokio::test]
async fn config_reference() {
    let cases: &[(&str, &str)] = &[
        ("./source.wasm", "adapter `./source.wasm` names no version"),
        ("/tmp/source.wasm", "adapter `/tmp/source.wasm` names no version"),
        ("documentation", "adapter `documentation` names no version"),
        ("documentation@1.2.0", "adapter `documentation@1.2.0` names no namespace"),
        ("emery:documentation", "adapter `emery:documentation` names no version"),
        (
            "emery:documentation@latest",
            "adapter `emery:documentation@latest` has an invalid version `latest`",
        ),
    ];
    for (adapter, fragment) in cases {
        let scratch = Scratch::new();
        let config =
            scratch.config(&format!("[[source]]\nname = \"docs\"\nadapter = \"{adapter}\"\n"));
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a refused reference loads nothing: {adapter}"
        );
    }
}

// A file's `rank` is a positive integer, refused at the field otherwise.
#[tokio::test]
async fn config_rank() {
    let cases: &[(&str, &str)] = &[
        ("0", "invalid value: integer `0`, expected a nonzero u32"),
        ("-1", "invalid value: integer `-1`, expected a nonzero u32"),
        ("1.5", "invalid type: floating point `1.5`, expected a nonzero u32"),
        ("\"high\"", "invalid type: string \"high\", expected a nonzero u32"),
    ];
    for (rank, fragment) in cases {
        let scratch = Scratch::new();
        let config = scratch.config(&format!(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\nrank = {rank}\n"
        ));
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a refused rank loads nothing: {rank}"
        );
    }
}

// Path anchoring, inline descriptions, and declaration order, all observed on
// the `SourceInput` the adapter receives.
#[tokio::test]
async fn source_paths() {
    let scratch = Scratch::new();
    let config = scratch.config(
        "[[source]]\nname = \"zulu\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
         \"nested/../docs\"\n\n\
         [[source]]\nname = \"intent\"\nadapter = \"emery:intent@1.0.0\"\ndescription = \"Ship \
         it.\"\n\n\
         [[source]]\nname = \"alpha\"\nadapter = \"acme:local@1.0.0\"\npath = \"./docs\"\n",
    );

    // three sources contribute one id, so a grouping turn is scripted
    let grouping = baseline_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    let calls = provider.source.calls();
    let order: Vec<&str> = calls.iter().map(|(_, input)| input.name.as_str()).collect();
    assert_eq!(order, ["zulu", "intent", "alpha"], "entries extract in declaration order");
    for name in ["zulu", "alpha"] {
        let (_, input) = calls.iter().find(|(_, input)| input.name == name).expect("dispatched");
        let SourceContent::Workspace(root) = &input.content else {
            panic!("a path source lends a workspace");
        };
        assert!(
            !std::path::Path::new(root).is_absolute(),
            "the lend must stay `.`-relative for the guest preopen: {root}"
        );
        assert!(
            root.ends_with("docs") && !root.contains(".."),
            "`.` and `..` fold away lexically against the file's directory: {root}"
        );
    }
    let (_, input) = calls.iter().find(|(_, input)| input.name == "intent").expect("dispatched");
    assert_eq!(
        input.content,
        SourceContent::Value("Ship it.".to_string()),
        "a file `description` entry supplies inline text"
    );
    drop(calls);
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn name_defaulted() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[[source]]\nadapter = \"emery:documentation@1.2.0\"\n\n\
         [[source]]\nadapter = \"emery:demo@1.2.0\"\n\n\
         [[source]]\nadapter = \"{SOURCE}\"\n",
    ));
    let grouping = baseline_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    assert!(
        shown(&provider, "spec").await.contains(
            "Sources: [documentation:greeting.behaviour, demo:greeting.behaviour, \
             source:greeting.behaviour]"
        ),
        "each entry is named for its adapter's package"
    );
    provider.model.assert_exhausted();
}

// --- repository sources ---

mod repository {
    use omnia_sdk::vcs::Error;

    use super::*;

    const URL: &str = "https://example.com/acme/shop.git";
    // `URL` normalised and hashed, the clone every run of it shares.
    const CLONE: &str = "./.emery/vcs/repos/f4c82940a714c3c5";
    const CHECKOUT: &str = "./.emery/vcs/sources/upstream";

    fn upstream(scratch: &Scratch, body: &str) -> String {
        scratch.config(&format!(
            "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"{URL}\"\n{body}"
        ))
    }

    // A source naming a repository is read in a working copy of it at the
    // revision: the clone made or fetched after the adapters load, the copy cut
    // and lent, at the `path` within it, and removed once the source answered.
    #[tokio::test]
    async fn repository_source() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"v1.2.0\"\npath = \"docs/api\"\n");
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        let resp = cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(
            stdout.contains(&format!("\n  upstream read from {URL} at v1.2.0: v1.2.0-commit\n")),
            "{stdout}"
        );
        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} v1.2.0"),
                format!("add {CLONE} {CHECKOUT} v1.2.0-commit"),
                format!("remove {CHECKOUT}"),
            ]
        );
        let calls = provider.source.calls();
        let (_, input) = calls.first().expect("one extract dispatch");
        assert_eq!(input.name, "upstream");
        assert_eq!(
            input.content,
            SourceContent::Workspace(format!("{CHECKOUT}/docs/api")),
            "the lend is the path within the working copy"
        );
        assert_eq!(
            provider.source.metadata.lock().expect("metadata").len(),
            1,
            "the adapter loads before the repository is touched"
        );
        drop(calls);
        provider.model.assert_exhausted();

        // the JSON envelope names what was read, and a path omitted lends the copy whole
        let config = upstream(&scratch, "revision = \"main\"\n");
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        provider.vcs.clones.script(CLONE, Err(Error::Exists(CLONE.to_owned())));
        let resp =
            cli_ok(&provider, &["emery", "--format", "json", "specify", "--config", &config]).await;
        let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
        assert_eq!(
            envelope["repositories"],
            serde_json::json!([
                {"source": "upstream", "repository": URL, "revision": "main", "commit": "main-commit"}
            ]),
            "{envelope}"
        );
        assert_eq!(
            provider.vcs.calls()[..2],
            [format!("clone {URL} {CLONE}"), format!("fetch {CLONE} origin")],
            "found, so fetched"
        );
        let calls = provider.source.calls();
        assert_eq!(calls[0].1.content, SourceContent::Workspace(CHECKOUT.to_owned()));
        drop(calls);
        provider.model.assert_exhausted();
    }

    // Two sources naming one repository, however they spell its URL, share
    // one clone made once and are read in working copies of their own.
    #[tokio::test]
    async fn repository_shared_clone() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"HTTPS://Example.com/acme/shop/\"\nrevision = \"v2\"\npath = \"api\"\n",
        );
        let grouping = baseline_grouping(2);
        let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} main"),
                format!("add {CLONE} ./.emery/vcs/sources/docs main-commit"),
                format!("resolve {CLONE} v2"),
                format!("add {CLONE} ./.emery/vcs/sources/api v2-commit"),
                "remove ./.emery/vcs/sources/docs".to_owned(),
                "remove ./.emery/vcs/sources/api".to_owned(),
            ]
        );
        let roots: Vec<SourceContent> =
            provider.source.calls().into_iter().map(|(_, input)| input.content).collect();
        assert_eq!(
            roots,
            [
                SourceContent::Workspace("./.emery/vcs/sources/docs".to_owned()),
                SourceContent::Workspace("./.emery/vcs/sources/api/api".to_owned()),
            ]
        );
        provider.model.assert_exhausted();
    }

    // The working copy goes whether or not the source answered; a stale one
    // from a failed run is removed before the copy is cut again.
    #[tokio::test]
    async fn repository_removed_on_failure() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"main\"\n");
        let mut provider = Provider::idle();
        provider
            .source
            .evidence
            .insert("upstream".to_string(), Err(bad_gateway!("the adapter exploded")));
        provider.vcs.adds.script(CHECKOUT, Err(Error::Exists(CHECKOUT.to_owned())));

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 4, "bad_gateway").await;

        assert_eq!(envelope["message"], "the adapter exploded");
        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} main"),
                format!("add {CLONE} {CHECKOUT} main-commit"),
                format!("remove {CHECKOUT}"),
                format!("add {CLONE} {CHECKOUT} main-commit"),
                format!("remove {CHECKOUT}"),
            ]
        );
        provider.vcs.assert_exhausted();
    }

    // A revision the repository lacks, or a URL that names none, is
    // `revision-not-found` before any source is asked; a remote that cannot be
    // reached is the gateway's.
    #[tokio::test]
    async fn repository_revision_missing() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"v9\"\n");
        let provider = Provider::idle();

        provider.vcs.resolves.script(CLONE, Err(Error::NotFound("v9".to_owned())));
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 2, "revision-not-found")
                .await;
        assert_message(&envelope, &format!("repository `{URL}` has no `v9`"));
        assert!(
            envelope["hint"].as_str().is_some_and(|hint| hint.contains("`revision`")),
            "{envelope}"
        );
        assert!(provider.source.calls().is_empty(), "no source is asked");

        provider.vcs.clones.script(CLONE, Err(Error::Access("authentication failed".to_owned())));
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 4, "bad_gateway").await;
        assert_message(&envelope, &format!("repository `{URL}`: authentication failed"));
        provider.vcs.assert_exhausted();
    }

    // A repository is read at a revision and at a path within it, never
    // beside inline text or above the clone; the file is refused before any
    // adapter loads.
    #[tokio::test]
    async fn repository_config() {
        let cases: &[(&str, &str)] = &[
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\n",
                "source `upstream` sets `repository` without `revision`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 revision = \"main\"\n",
                "source `upstream` sets `revision` without `repository`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:intent@1.0.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\
                 description = \"Ship it.\"\n",
                "source `upstream` sets both `repository` and `description`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\
                 path = \"../sibling\"\n",
                "source `upstream`: path `../sibling` must lie within the repository",
            ),
        ];
        for (body, fragment) in cases {
            let scratch = Scratch::new();
            let config = scratch.config(body);
            let provider = Provider::idle();
            let envelope =
                fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
            assert_message(&envelope, fragment);
            assert!(
                provider.source.metadata.lock().expect("metadata").is_empty(),
                "a refused list gates nothing: {body}"
            );
            assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
        }
    }
}

// --- adapter references ---

mod reference {
    use std::fs;

    use super::{
        DESIGN_ANSWER, Provider, SPEC_ANSWER, Scratch, assert_message, cli_ok, fail, guest, shown,
    };

    // The exact reference loads as the guest it names without its version,
    // and the source is named for the package.
    #[tokio::test]
    async fn exact_ref() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", "emery:demo@1.2.0"]).await;

        let gated = provider.source.metadata.lock().expect("metadata").clone();
        assert_eq!(gated, ["emery:demo"], "the package is gated without its version");
        let calls = provider.source.calls();
        let (id, input) = calls.first().expect("one extract dispatch");
        assert_eq!(id, "emery:demo", "the adapter id is the registered guest");
        assert_eq!(input.name, "demo", "the source name is the package name");
        drop(calls);
        provider.model.assert_exhausted();
    }

    // A reference parses whole or not at all: no default namespace, nothing
    // resolving to a latest, no path, no bare name, no source checkout.
    #[tokio::test]
    async fn malformed_ref() {
        let cases: &[(&str, &str)] = &[
            ("emery:demo", "adapter `emery:demo` names no version"),
            ("demo@1.2.0", "adapter `demo@1.2.0` names no namespace"),
            ("demo", "adapter `demo` names no version"),
            ("emery:demo@main", "adapter `emery:demo@main` has an invalid version `main`"),
            ("emery:@1.2.0", "adapter `emery:@1.2.0` is not `namespace:name@version`"),
            ("Emery:demo@1.2.0", "adapter `Emery:demo@1.2.0` is not `namespace:name@version`"),
            ("./demo.wasm", "adapter `./demo.wasm` names no version"),
            ("/tmp/demo.wasm", "adapter `/tmp/demo.wasm` names no version"),
            ("https://github.com/acme/api", "GitHub URLs are not supported"),
        ];
        for (reference, fragment) in cases {
            let provider = Provider::idle();
            let envelope =
                fail(&provider, &["emery", "specify", reference], 1, "adapter-reference").await;
            assert_message(&envelope, fragment);
            assert_eq!(
                envelope["hint"],
                "an adapter is an exact package reference, `namespace:name@version`",
                "{envelope}"
            );
            assert!(
                provider.source.metadata.lock().expect("metadata").is_empty(),
                "a malformed reference loads nothing: {reference}"
            );
        }
    }

    // Routing is the deployment's, so the file reserves no table for it.
    #[tokio::test]
    async fn registries_table() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\n\n\
             [registries]\nacme = \"registry.acme.io\"\n",
        );
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(&envelope, "unknown field `registries`");
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a file that does not parse loads nothing"
        );
    }

    // Two versions of one package would register as one guest, the second
    // load attesting the first's bytes; the list is refused before either
    // loads.
    #[tokio::test]
    async fn version_collision() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"emery:documentation@1.3.0\"\n",
        );
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(
            &envelope,
            "adapters `emery:documentation@1.2.0` and `emery:documentation@1.3.0`",
        );
        assert_message(&envelope, "would both register as `emery:documentation`");
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a colliding list loads nothing"
        );

        // the same two on the command line are two sources of one package name
        let envelope = fail(
            &provider,
            &["emery", "specify", "emery:documentation@1.2.0", "emery:documentation@1.3.0"],
            1,
            "bad_request",
        )
        .await;
        assert_message(&envelope, "source `documentation` appears twice");
    }

    // An argv run reads no project-root file: one that does not parse, or
    // whose source is located outside its root, refuses the discovered run
    // alone. The CWD move is hermetic under nextest's process-per-test
    // isolation.
    #[tokio::test]
    async fn argv_beside_project_file() {
        let project = tempfile::TempDir::new().expect("project dir");
        std::env::set_current_dir(project.path()).expect("enter project");
        let argv = ["emery", "specify", "acme:ledger@2.1.0"];

        for (entry, refusal) in [
            (
                "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\nbranch = \
                 \"main\"\n",
                "unknown field `branch`",
            ),
            (
                "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
                 \"../../outside\"\n",
                "must be relative to the project root",
            ),
            ("[[source]]\nname = \"local\"\nadapter = \"/tmp/source.wasm\"\n", "names no version"),
        ] {
            fs::write(project.path().join("emery.toml"), entry).expect("write emery.toml");
            let envelope = fail(&Provider::idle(), &["emery", "specify"], 1, "bad_request").await;
            assert_message(&envelope, refusal);
            let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

            cli_ok(&provider, &argv).await;

            let gated = provider.source.metadata.lock().expect("metadata").clone();
            assert_eq!(gated, [guest("ledger")], "argv names the run's only source: {entry}");
            assert!(
                shown(&provider, "spec").await.contains("Sources: [ledger:greeting.behaviour]"),
                "the file's `[[source]]` entries stay out of an argv run"
            );
            provider.model.assert_exhausted();
        }
    }
}

mod digest {
    use super::{Provider, SOURCE, Scratch, assert_message, digest, fail};

    // One adapter loads once under one pin, so two entries pinning it differently
    // cannot both be honoured; the list is refused before either is asked for.
    #[tokio::test]
    async fn conflict() {
        let scratch = Scratch::new();
        let config = scratch.config(&format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\ndigest = \"{}\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"{SOURCE}\"\ndigest = \"{}\"\n",
            digest("cd"),
            digest("ab")
        ));
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(&envelope, &format!("adapter `{SOURCE}` is pinned to two digests"));
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a conflicting pin loads nothing"
        );
    }
}

mod store {
    use std::sync::Arc;

    use emery_engine::{CONTAINER, REVISION_KEY};
    use omnia_test::guest::{Memory, Namespaced};

    use super::{
        DESIGN_ANSWER, Provider, SOURCE, SPEC_ANSWER, SPEC_RENDERED, SPEC_REVISION, assert_message,
        cli_ok, current, fail, project_current, projection, reference, seed, shown,
    };

    // Bytes at the current key that name no revision — a well-formed id with no
    // documents, or bytes that decode to no id — are corruption: `show` fails
    // closed, never an empty result, and the next `specify` swaps over them.
    #[tokio::test]
    async fn corrupt_current() {
        for bytes in [&b"0123456789abcdef"[..], &b"\xff\xfe"[..]] {
            let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
            provider.storage.insert_state(REVISION_KEY, bytes);
            fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;

            cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

            let id = current(&provider.storage);
            assert!(provider.storage.object(CONTAINER, &format!("{id}/spec.json")).is_some());
            cli_ok(&provider, &["emery", "show", "spec"]).await;
            provider.model.assert_exhausted();
        }
    }

    // A document rewritten under its id no longer hashes to it, so `show` fails
    // closed; regeneration is the recovery path, with only the advisory diff
    // suppressed.
    #[tokio::test]
    async fn tampered_revision() {
        let second_spec = SPEC_ANSWER.replace("hello", "howdy");
        let second_design = DESIGN_ANSWER.replace("hello", "howdy");
        let provider = Provider::answering([
            SPEC_ANSWER,
            DESIGN_ANSWER,
            second_spec.as_str(),
            second_design.as_str(),
        ]);
        cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
        let first = current(&provider.storage);
        provider.storage.insert_object(CONTAINER, &format!("{first}/spec.json"), b"{}\n");

        fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;

        let resp = cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(
            !stdout.contains("diff vs"),
            "an unreadable outgoing revision yields no diff: {stdout}"
        );
        let second = current(&provider.storage);
        assert_ne!(first, second, "the repaired store names the new revision");
        for name in ["spec.json", "design.json", "plan.json"] {
            assert!(
                provider.storage.object(CONTAINER, &format!("{first}/{name}")).is_none(),
                "the tampered outgoing revision is pruned: {name}"
            );
        }
        assert!(shown(&provider, "spec").await.contains("howdy"), "show renders the repair");
        provider.model.assert_exhausted();
    }

    // Outdated is not corrupt: `show` refuses typed, and the next `specify`
    // regenerates over it.
    #[tokio::test]
    async fn spec_outdated() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        let outdated = seed(
            &provider.storage,
            br#"{"emery": 1, "requirements": []}"#,
            br#"{"emery": 1, "sections": []}"#,
            br#"{"emery": 1, "slices": []}"#,
        );

        let envelope = fail(&provider, &["emery", "show", "spec"], 1, "spec-outdated").await;
        assert_message(&envelope, "grammar 1");
        assert!(
            envelope["hint"].as_str().unwrap_or("").contains("emery specify"),
            "the hint names the way out: {envelope}"
        );

        let resp = cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(
            !stdout.contains("diff vs"),
            "an outdated outgoing revision yields no diff: {stdout}"
        );
        let id = current(&provider.storage);
        assert_ne!(id, outdated, "the regenerated revision is current");
        assert!(
            provider.storage.object(CONTAINER, &format!("{outdated}/spec.json")).is_none(),
            "the outdated revision is pruned"
        );
        cli_ok(&provider, &["emery", "show", "spec"]).await;
        provider.model.assert_exhausted();
    }

    // Isolation is host policy over the engine's flat keys.
    #[tokio::test]
    async fn multi_project() {
        // one shared store, two project-scoped views
        let shared = Memory::default();
        let alpha = Provider::over(
            Arc::new(Namespaced::new("alpha", shared.clone())),
            [SPEC_ANSWER, DESIGN_ANSWER],
        );
        let beta_spec = SPEC_ANSWER.replace("hello", "howdy");
        let beta_design = DESIGN_ANSWER.replace("hello", "howdy");
        let beta = Provider::over(
            Arc::new(Namespaced::new("beta", shared.clone())),
            [beta_spec, beta_design],
        );

        cli_ok(&alpha, &["emery", "specify", SOURCE]).await;
        cli_ok(&beta, &["emery", "specify", SOURCE]).await;

        // every write landed under its project prefix
        assert!(shared.state(REVISION_KEY).is_none(), "no unprefixed current id exists");
        assert!(shared.objects(CONTAINER).is_empty(), "no unprefixed revision exists");

        let id_alpha = project_current(&shared, "alpha");
        let id_beta = project_current(&shared, "beta");
        assert_ne!(id_alpha, id_beta, "distinct documents commit distinct revisions");

        // each project shows its own revision
        let spec_alpha = shared
            .object(&format!("alpha/{CONTAINER}"), &format!("{id_alpha}/spec.json"))
            .expect("spec.json");
        let spec_beta = shared
            .object(&format!("beta/{CONTAINER}"), &format!("{id_beta}/spec.json"))
            .expect("spec.json");
        assert_eq!(
            String::from_utf8_lossy(&spec_alpha),
            SPEC_REVISION,
            "alpha committed the revision"
        );
        assert!(String::from_utf8_lossy(&spec_beta).contains("howdy"));
        assert_eq!(
            shown(&alpha, "spec").await,
            projection(SPEC_RENDERED, &id_alpha),
            "alpha shows its own revision"
        );
        assert!(shown(&beta, "spec").await.contains("howdy"), "beta shows its own revision");

        alpha.model.assert_exhausted();
        beta.model.assert_exhausted();
    }
}

// --- helpers ---

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

fn current(storage: &Memory) -> String {
    stored_id(storage, REVISION_KEY)
}

fn project_current(shared: &Memory, project: &str) -> String {
    stored_id(shared, &format!("{project}/{REVISION_KEY}"))
}

fn stored_id(storage: &Memory, key: &str) -> String {
    String::from_utf8(storage.state(key).expect("current")).expect("utf-8 revision id")
}

fn document(storage: &Memory, id: &str, name: &str) -> Vec<u8> {
    storage.object(CONTAINER, &format!("{id}/{name}")).unwrap_or_else(|| panic!("{name}"))
}

fn projection(fixture: &str, id: &str) -> String {
    fixture.replace("<revision>", id)
}

async fn shown<S>(provider: &Provider<S>, document: &str) -> String
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let resp = cli_ok(provider, &["emery", "show", document]).await;
    String::from_utf8(resp.stdout).expect("utf-8 document")
}
