//! Verifies `emery specify` and `emery show` over scripted capabilities.

#![cfg(not(target_arch = "wasm32"))]
#![expect(dead_code, reason = "fixtures and helpers resume in A22–A24")]

mod support;

use emery_adapter::source::{Claim, ClaimKind, Evidence, SourceContent, SourceKind};
use emery_engine::{REVISION_KEY, STAGED_CONTAINER, STAGED_KEY};
use omnia_test::SeenFormat;
use serde_json::Value;
use support::provider::{Provider, Rendezvous, claim, evidence, requirement, retire};
use support::storage::{Storage, staged};
use support::store::{revision as revision_id, written};
use support::{cli_ok, fail};

const SOURCE: &str = "acme:source@1.0.0";
// The target whose repository every sealed revision lives in: the project checkout.
const PROJECT_CONFIG: &str = "tests/build/project.toml";

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

// `DESIGN_ANSWER` with the `## APIs and integrations` section a `call`,
// `contract`, or `decision` claim requires.
fn design_with_apis() -> String {
    let mut answer: Value =
        serde_json::from_str(DESIGN_ANSWER).expect("the design fixture is JSON");
    let sections = answer["sections"].as_array_mut().expect("sections");
    sections.push(serde_json::json!({"kind": "apis", "blocks": [{
        "text": "The system reaches one upstream service, as the claims spell it."
    }]}));
    answer.to_string()
}

// The slicing a run over several stems expects when every stem stays its own
// part, named for it, owning nothing and depending on nothing.
fn separate_slicing(stems: &[(&str, &[&str])]) -> String {
    let parts: Vec<_> = stems
        .iter()
        .map(|(name, requirements)| serde_json::json!({"name": name, "requirements": requirements}))
        .collect();
    serde_json::json!({ "preamble": [], "parts": parts }).to_string()
}

// --- journey ---

// One `specify` with no prior verb, then `show`, then an identical re-run that
// drafts again from the sources alone.
#[tokio::test]
async fn gen_spec() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]);

    // the first specify stages a candidate and seals nothing
    let resp = cli_ok(&provider, &["emery", "specify", SOURCE]).await;
    let id = staged(&provider.storage).await;
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged candidate {id}\n  plan: 1 part in 1 wave, widest 1\n  nothing sealed; `emery \
             apply` seals it\n"
        ),
        "a first run has no revision to diff against"
    );
    assert!(provider.storage.state(REVISION_KEY).await.is_none(), "nothing is committed yet");

    // observe the gate and that nothing else lands
    let calls = provider.source.calls();
    assert_eq!(calls.len(), 1, "one source extracts");
    assert_eq!(
        calls[0].0,
        guest("source"),
        "the adapter dispatches as the guest its reference names"
    );
    drop(calls);
    assert!(
        provider.storage.list_objects("adapters").await.is_empty(),
        "nothing of an adapter mirrors into engine storage"
    );
    assert!(provider.storage.state("project.yaml").await.is_none(), "no project record exists");

    // the staged documents are canonical JSON of the engine's facts; one
    // stem is one part, planned with no turn spent
    let spec = staged_document(&provider.storage, &id, "spec.json").await;
    let design = staged_document(&provider.storage, &id, "design.json").await;
    let plan = staged_document(&provider.storage, &id, "plan.json").await;
    assert_eq!(String::from_utf8_lossy(&spec), SPEC_REVISION, "spec.json is canonical");
    assert_eq!(String::from_utf8_lossy(&design), DESIGN_REVISION, "design.json is canonical");
    assert_eq!(String::from_utf8_lossy(&plan), PLAN_REVISION, "plan.json is canonical");

    // review the candidate through show --staged; the sealed slot is empty
    assert_eq!(
        shown(&provider, "spec").await,
        projection(SPEC_RENDERED, &id),
        "show --staged renders spec.md"
    );
    assert_eq!(
        shown(&provider, "design").await,
        projection(DESIGN_RENDERED, &id),
        "show --staged renders design.md"
    );
    assert_eq!(
        shown(&provider, "plan").await,
        projection(PLAN_RENDERED, &id),
        "show --staged renders plan.md"
    );
    let envelope = fail(
        &provider,
        &["emery", "show", "spec", "--config", PROJECT_CONFIG],
        2,
        "spec-not-generated",
    )
    .await;
    assert_message(&envelope, &format!("candidate `{id}` is staged"));
    assert!(
        envelope["hint"].as_str().unwrap_or("").contains("emery apply"),
        "the hint names the seal: {envelope}"
    );

    // apply seals it: the documents land under `spec/` in the repository and the slot clears
    let resp = cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!("sealed revision {id} as emery/{id} at Revision\n")
    );
    assert_eq!(committed(&provider.storage).await, id, "the candidate is the current revision");
    assert_eq!(committed_document(&provider, "spec"), spec);
    assert_eq!(committed_document(&provider, "design"), design);
    assert_eq!(committed_document(&provider, "plan"), plan);
    assert!(provider.storage.state(STAGED_KEY).await.is_none(), "the staging slot is cleared");
    assert!(
        provider.storage.list_objects(STAGED_CONTAINER).await.is_empty(),
        "the candidate is pruned"
    );

    // the JSON envelope carries the revision, the projection, and the document
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "show", "spec", "--config", PROJECT_CONFIG],
    )
    .await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["body"], projection(SPEC_RENDERED, &id), "{envelope}");
    let document: Value =
        serde_json::from_str(SPEC_REVISION).expect("the revision fixture is JSON");
    assert_eq!(envelope["document"], document, "the envelope carries the typed document");
    fail(&provider, &["emery", "show", "spec", "--staged"], 2, "nothing-staged").await;

    // an identical re-run drafts again and stages the same bytes beside the revision
    let resp = cli_ok(&provider, &["emery", "specify", SOURCE, "--config", PROJECT_CONFIG]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout,
        format!(
            "staged candidate {id}\n  plan: 1 part in 1 wave, widest 1\n  diff vs {id}: none \
             (byte-stable)\n  nothing sealed; `emery apply` seals it\n"
        ),
        "the one-stem plan is one part in one wave"
    );
    assert_eq!(staged(&provider.storage).await, id, "the same revision keeps its id");
    assert_eq!(committed(&provider.storage).await, id, "the sealed revision stands");

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

    let id = staged(&provider.storage).await;
    assert_eq!(
        String::from_utf8_lossy(&staged_document(&provider.storage, &id, "spec.json").await),
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

    let id = staged(&provider.storage).await;
    let spec: Value =
        serde_json::from_slice(&staged_document(&provider.storage, &id, "spec.json").await)
            .expect("the stored spec is JSON");
    let stored = &spec["requirements"][0]["scenarios"][0];
    assert_eq!(stored["given"], serde_json::json!(["the greeting surface is bound"]), "{spec}");
    assert_eq!(stored["and"], serde_json::json!(["the greeting is logged"]), "{spec}");
    provider.model.assert_exhausted();
}

// One adapter named by two sources is gated once, extracted twice.
#[tokio::test]
async fn shared_roots() {
    let adapter = "acme:docs@1.0.0";
    let package = "acme:docs";
    let grouping = baseline_grouping(2);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);
    let config = provider.write_project(
        "shared-roots.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{adapter}\"\npath = \"docs\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"{adapter}\"\npath = \"api\"\n"
        ),
    );

    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    assert!(
        shown(&provider, "spec")
            .await
            .contains("Sources: [docs:greeting.behaviour, api:greeting.behaviour]"),
        "both sources contribute to the one requirement"
    );

    let calls = provider.source.calls();
    assert_eq!(calls.len(), 2, "each source extracts");
    assert!(
        calls.iter().all(|(id, _)| id == package),
        "one adapter identity is gated once: {calls:?}"
    );
    assert_eq!(calls[0].0, package);
    assert_eq!(calls[0].1.name, "docs");
    assert_eq!(calls[1].0, package);
    assert_eq!(calls[1].1.name, "api");
    drop(calls);

    provider.model.assert_exhausted();
}

// Command-line sources layer over the file's: one named as a `[[source]]`
// is, by its package name, takes that entry's place whole, and any other is
// appended after the file's, in argv order.
#[tokio::test]
async fn layered_sources() {
    let grouping = baseline_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);
    let config = provider.write_project(
        "layered-sources.toml",
        format!(
            "[[source]]\nadapter = \"acme:docs@1.2.0\"\npath = \"docs\"\nrank = 4\n\n\
             [[source]]\nname = \"api\"\nadapter = \"{}\"\npath = \"api\"\n",
            reference("api")
        ),
    );

    cli_ok(
        &provider,
        &[
            "emery",
            "specify",
            &reference("docs"),
            "--description",
            &describing("intent", "Ship it."),
            "--config",
            &config,
        ],
    )
    .await;

    let calls = provider.source.calls();
    let by_name: std::collections::BTreeMap<_, _> = calls
        .iter()
        .map(|(id, input)| (input.name.as_str(), (id.as_str(), &input.content)))
        .collect();
    assert_eq!(by_name.len(), 3, "the file's two sources and the appended description");
    let (docs_id, docs_content) = by_name["docs"];
    assert_eq!(docs_id, guest("docs"), "the command-line adapter stands in the file's place");
    assert_eq!(
        *docs_content,
        SourceContent::Workspace(".".into()),
        "the entry's `path` goes with it"
    );
    assert_eq!(
        by_name["api"].0,
        guest("api"),
        "an entry no argument names is read as the file has it"
    );
    assert_eq!(by_name["intent"].1, &SourceContent::Value("Ship it.".into()));
    drop(calls);
    let spec = shown(&provider, "spec").await;
    for citation in
        ["docs:greeting.behaviour", "api:greeting.behaviour", "intent:greeting.behaviour"]
    {
        assert!(spec.contains(citation), "the specification cites every source: {spec}");
    }
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

    let mut order: Vec<String> = provider
        .source
        .calls
        .lock()
        .expect("calls")
        .iter()
        .map(|(_, input)| input.name.clone())
        .collect();
    order.sort();
    assert_eq!(order, ["api", "docs"], "both sources extracted under the rendezvous");
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
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    provider.write_project(
        "emery.toml",
        "[[source]]\nname = \"docs\"\nadapter = \"acme:docs@1.0.0\"\n",
    );

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
    let design = design_with_apis();
    let mut provider = Provider::answering([
        GROUPING_ANSWER,
        PRECEDENCE_ANSWER,
        design.as_str(),
        slicing.as_str(),
    ]);

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

    let id = staged(&provider.storage).await;
    let spec = staged_document(&provider.storage, &id, "spec.json").await;
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
    let mut provider = Provider::answering([grouping, spec.as_str(), DESIGN_ANSWER]);
    let path = provider.write_project("configured-rank-4.toml", config(4));
    bind(&mut provider);
    cli_ok(&provider, &["emery", "specify", "--config", &path]).await;

    let id = staged(&provider.storage).await;
    let stored: Value =
        serde_json::from_slice(&staged_document(&provider.storage, &id, "spec.json").await)
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
    let mut provider = Provider::answering([grouping, spec.as_str(), DESIGN_ANSWER]);
    let path = provider.write_project("configured-rank-3.toml", config(3));
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
async fn invalid_grouping() {
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
        let (exit, code) = if *answer == "not json" { (3, "invalid") } else { (1, "exhausted") };
        let envelope = fail(
            &provider,
            &["emery", "specify", &reference("docs"), &reference("code")],
            exit,
            code,
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
    let design = design_with_apis();
    let mut provider = Provider::answering([SPEC_ANSWER, design.as_str()]);
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

// A source may answer `retire` claims beside its requirements; synthesis
// drafts from the requirements alone, so the staged candidate is the one the
// run without them stages.
#[tokio::test]
async fn retire_claim() {
    let mut provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "source".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            retire("greeting.farewell", "The farewell greeting is withdrawn."),
        ])),
    );

    cli_ok(&provider, &["emery", "specify", SOURCE]).await;

    let id = staged(&provider.storage).await;
    for (name, fixture) in [
        ("spec.json", SPEC_REVISION),
        ("design.json", DESIGN_REVISION),
        ("plan.json", PLAN_REVISION),
    ] {
        assert_eq!(
            String::from_utf8_lossy(&staged_document(&provider.storage, &id, name).await),
            fixture,
            "{name} is byte-identical to the run without the retire"
        );
    }
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

    let id = staged(&provider.storage).await;
    let spec: Value =
        serde_json::from_slice(&staged_document(&provider.storage, &id, "spec.json").await)
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

    let id = staged(&provider.storage).await;
    let spec: Value =
        serde_json::from_slice(&staged_document(&provider.storage, &id, "spec.json").await)
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
async fn remine_candidate() {
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
    cli_ok(
        &provider,
        &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
    )
    .await;
    let first = committed(&provider.storage).await;

    // second run: the greeting changed, the export gone, the overview following it
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let second_grouping = separate_grouping(2);
    let second_slicing = separate_slicing(&[("greeting", &["REQ-001"]), ("session", &["REQ-002"])]);
    let mut provider = provider.continuing([
        second_grouping.as_str(),
        REMINE_SECOND,
        second_design.as_str(),
        second_slicing.as_str(),
    ]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence(&[
            ("greeting.behaviour", "GET /greeting returns the static string 'howdy'."),
            ("session.timeout", "Sessions time out after an hour."),
        ])),
    );
    let resp = cli_ok(
        &provider,
        &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
    )
    .await;

    // observe the summary, the swap, and the prune
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert_eq!(
        stdout.lines().count(),
        4,
        "a changed run prints the candidate, the plan's shape, one summary line, and the seal: \
         {stdout}"
    );
    assert!(
        stdout.contains("  plan: 2 parts in 1 wave, widest 2\n"),
        "two independent parts are one wave: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "diff vs {first}: spec +0 -1 ~1 preamble, design +0 -0 ~1, plan +0 -1 ~0"
        )),
        "the summary counts the changes: {stdout}"
    );
    assert!(!stdout.contains("REQ-"), "no per-requirement entry rides text mode: {stdout}");
    assert!(!stdout.contains("PART-"), "no per-part entry rides text mode: {stdout}");

    let seen = provider.model.seen();
    let SeenFormat::Schema { name, .. } = &seen[0].format else {
        panic!("one source over two stems is grouped by the model");
    };
    assert_eq!(name, "grouping");
    let request = seen[1].messages.join("\n");
    assert!(request.contains("- REQ-001 `greeting.behaviour`"), "{request}");
    assert!(request.contains("- REQ-002 `session.timeout`"), "{request}");
    assert!(!request.contains("Unchanged"), "nothing stands in from the outgoing run: {request}");

    let second = revision_id(
        &written(&provider.files, "spec"),
        &written(&provider.files, "design"),
        &written(&provider.files, "plan"),
    );
    assert_ne!(first, second, "changed documents commit a new revision");
    assert_eq!(
        committed(&provider.storage).await,
        second,
        "the pointer names what is under `spec/`"
    );
    assert!(stdout.contains(&format!("  sealed revision {second} as emery/{second}")), "{stdout}");
    assert!(provider.storage.state(STAGED_KEY).await.is_none(), "the seal clears the slot");
    let spec = shown_sealed(&provider, "spec").await;
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
        {"kind": "domain-model", "blocks": [{"type": {"key": "greeting.type", "role": "data"}}]}
    ]}"#;
    let second_grouping = separate_grouping(2);
    let second_slicing = r#"{"preamble": [], "parts": [
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
    cli_ok(
        &provider,
        &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
    )
    .await;
    let first = committed(&provider.storage).await;

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
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "specify", &reference("docs"), "--config", PROJECT_CONFIG],
    )
    .await;
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
        diff["spec"]["retired"],
        serde_json::json!([]),
        "a candidate drafted apart retires nothing: {envelope}"
    );
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
        serde_json::json!([{"id": "PART-002", "name": "access"}]),
        "{envelope}"
    );
    assert_eq!(diff["plan"]["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(
        diff["plan"]["changed"],
        serde_json::json!([{"id": "PART-001", "name": "greeting", "fields": ["types"]}]),
        "the greeting part now owns the new type: {envelope}"
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
    cli_ok(
        &provider,
        &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
    )
    .await;
    let first = committed(&provider.storage).await;

    let second_slicing = separate_slicing(&[("session", &["REQ-001"]), ("greeting", &["REQ-002"])]);
    let mut provider = provider.continuing([
        grouping.as_str(),
        REMINE_SECOND,
        DESIGN_ANSWER,
        second_slicing.as_str(),
    ]);
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
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "specify", &reference("docs"), "--config", PROJECT_CONFIG],
    )
    .await;

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
    let id = envelope["revision"].as_str().expect("staged revision id").to_owned();
    let spec: Value =
        serde_json::from_slice(&staged_document(&provider.storage, &id, "spec.json").await)
            .expect("the staged spec is JSON");
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
    cli_ok(
        &provider,
        &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
    )
    .await;
    let first = committed(&provider.storage).await;

    let second_slicing =
        separate_slicing(&[("greeting", &["REQ-001"]), ("session", &["REQ-002", "REQ-003"])]);
    let mut provider = provider.continuing([
        grouping.as_str(),
        REMINE_SESSION,
        DESIGN_ANSWER,
        second_slicing.as_str(),
    ]);
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
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "specify", &reference("docs"), "--config", PROJECT_CONFIG],
    )
    .await;

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
    requirement(subject, statement).at(path)
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

// --- helpers (lines 1–1314 portion) ---

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

async fn committed(storage: &Storage) -> String {
    let raw = storage.state(REVISION_KEY).await.expect("current revision pointer");
    String::from_utf8(raw).expect("utf-8 revision id")
}

async fn staged_document(storage: &Storage, id: &str, name: &str) -> Vec<u8> {
    storage
        .object(STAGED_CONTAINER, &format!("{id}/{name}"))
        .await
        .unwrap_or_else(|| panic!("staged {name}"))
}

fn committed_document(provider: &Provider, name: &str) -> Vec<u8> {
    written(&provider.files, name)
}

fn projection(fixture: &str, id: &str) -> String {
    fixture.replace("<revision>", id)
}

async fn shown(provider: &Provider, document: &str) -> String {
    let resp = cli_ok(provider, &["emery", "show", document, "--staged"]).await;
    String::from_utf8(resp.stdout).expect("utf-8 document")
}

async fn shown_sealed(provider: &Provider, document: &str) -> String {
    let resp = cli_ok(provider, &["emery", "show", document, "--config", PROJECT_CONFIG]).await;
    String::from_utf8(resp.stdout).expect("utf-8 document")
}
