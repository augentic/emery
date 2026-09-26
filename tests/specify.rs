//! Verifies the operator journey from `specify` through `show` and regeneration.
//!
//! The scenarios cover source selection, specification generation, review,
//! revision replacement, and every caller-visible refusal.
//!
//! Each scenario drives the real command façade over scripted capabilities,
//! asserting the exact envelope, exit code, storage operations, and Markdown
//! projection an operator would observe.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use emery_adapter::source::{ClaimKind, Evidence, SourceContent, SourceKind};
use emery_engine::{CONTAINER, CURRENT, ENGINE};
use omnia_sdk::model::Error as ModelError;
use omnia_sdk::plugins::{Error as LoadError, Location};
use omnia_sdk::{BlobStore, StateStore, bad_gateway, bad_request};
use omnia_test::SeenFormat;
use omnia_test::guest::{Memory, Namespaced, Scripted};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use support::{Provider, Rendezvous, claim, cli_ok, digest, evidence, fail, requirement};

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
const SOURCES: &str = include_str!("specify/emery.toml");

// The grouping a run over `count` claims of one id expects: one agreeing requirement.
fn baseline_grouping(count: usize) -> String {
    let indices = (0..count).collect::<Vec<_>>();
    serde_json::json!({
        "groups": [{"claims": &indices, "classes": [&indices]}],
    })
    .to_string()
}

// Inside the project: every path handed to the CLI must stay project-relative
// for the guest preopen, so each write answers with that path.
struct Scratch(tempfile::TempDir);

impl Scratch {
    fn new() -> Self {
        Self(tempfile::TempDir::new_in(env!("CARGO_MANIFEST_DIR")).expect("project tempdir"))
    }

    fn write(&self, name: &str, body: impl AsRef<[u8]>) -> String {
        let path = self.0.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|err| panic!("mkdir for {name}: {err}"));
        }
        fs::write(&path, body).unwrap_or_else(|err| panic!("write {name}: {err}"));
        path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .expect("path under project")
            .to_str()
            .expect("utf-8 path")
            .to_string()
    }

    fn config(&self, body: &str) -> String {
        self.write("emery.toml", body)
    }

    // The loader is scripted, so the component only has to exist as a `.wasm` file.
    fn component(&self) -> String {
        self.write("source.wasm", b"\0asm-stub")
    }
}

// --- journey ---

// One `specify` with no prior verb, then `show`, then an identical re-run that
// drafts again from the sources alone.
#[tokio::test]
async fn gen_spec() {
    // arrange: scripted storage; only the operator's component touches the filesystem
    let scratch = Scratch::new();
    let component = scratch.component();

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]);

    // the first specify
    cli_ok(&provider, &["emery", "specify", &component]).await;

    // observe the load, the current id, and the revision
    let loads = provider.plugins.loads();
    let [(Location::Path(path), None)] = loads.as_slice() else {
        panic!("a local component is one unpinned load by path: {loads:?}");
    };
    assert!(path.ends_with("source.wasm"), "the preopen-relative path rides the request: {path}");
    assert!(
        provider.storage.objects("adapters").is_empty(),
        "nothing mirrors into engine storage; the loader reads the file fresh"
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

    // the JSON envelope carries the revision, the projection, and the document
    let resp = cli_ok(&provider, &["emery", "--format", "json", "show", "spec"]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["body"], projection(SPEC_RENDERED, &id), "{envelope}");
    let document: Value =
        serde_json::from_str(SPEC_REVISION).expect("the revision fixture is JSON");
    assert_eq!(envelope["document"], document, "the envelope carries the typed document");

    // an identical re-run drafts again and commits the same bytes
    let resp = cli_ok(&provider, &["emery", "specify", &component]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");
    assert_eq!(current(&provider.storage), id, "the same revision keeps its id");

    provider.model.assert_exhausted();
}

// The component the entry names exists only beside the file, so the run must
// resolve it there.
#[tokio::test]
async fn from_file() {
    let scratch = Scratch::new();
    scratch.component();
    let config = scratch.config(SOURCES);

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    let loads = provider.plugins.loads();
    let [(Location::Path(path), None)] = loads.as_slice() else {
        panic!("a local component is one unpinned load by path: {loads:?}");
    };
    assert!(
        path.ends_with("source.wasm") && !path.starts_with("./"),
        "the file-relative reference resolves against the config directory: {path}"
    );

    assert!(
        shown(&provider, "spec").await.contains("Sources: [greeting:greeting.behaviour]"),
        "the entry name is the source key the renderer cites"
    );

    provider.model.assert_exhausted();
}

// One adapter named by two sources is loaded and gated once, extracted twice.
#[tokio::test]
async fn shared_roots() {
    let cases: &[(&str, &str)] =
        &[("emery:documentation@1.2.0", "emery:documentation"), ("./source.wasm", "source")];
    for (adapter, package) in cases {
        let scratch = Scratch::new();
        if Path::new(adapter).extension().is_some() {
            scratch.component();
        }
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
            "{adapter}: both sources contribute to the one requirement"
        );

        assert_eq!(provider.loaded(), [*package], "{adapter}: one adapter identity loads once");
        let gated = provider.source.metadata.lock().expect("metadata").clone();
        assert_eq!(gated, [*package], "{adapter}: one adapter identity is gated once");

        let calls = provider.source.calls.lock().expect("calls");
        assert_eq!(calls.len(), 2, "{adapter}: each source extracts");
        assert_eq!(calls[0].0, *package);
        assert_eq!(calls[0].1.key, "docs");
        assert_eq!(calls[1].0, *package);
        assert_eq!(calls[1].1.key, "api");
        drop(calls);

        provider.model.assert_exhausted();
    }
}

// The rendezvous holds each extract until the other is requested, so an engine
// extracting one source at a time fails naming the one that never came.
#[tokio::test]
async fn sources_together() {
    let grouping = baseline_grouping(2);
    let mut provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER])
        .declaring(["docs", "api"]);
    provider.source.rendezvous = Some(Rendezvous::from_iter(["docs", "api"]));

    cli_ok(&provider, &["emery", "specify", "docs", "api"]).await;

    let order: Vec<String> = provider
        .source
        .calls
        .lock()
        .expect("calls")
        .iter()
        .map(|(_, input)| input.key.clone())
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
        "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n",
    )
    .expect("write emery.toml");
    std::env::set_current_dir(project.path()).expect("enter project");

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]).declaring(["documentation"]);

    cli_ok(&provider, &["emery", "specify"]).await;

    assert!(shown(&provider, "spec").await.contains("Sources: [docs:greeting.behaviour]"));
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn description_source() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]).declaring(["intent"]);

    cli_ok(&provider, &["emery", "specify", "--description", "intent=Ship it."]).await;

    let spec = shown(&provider, "spec").await;
    assert!(spec.contains("Sources: [intent:greeting.behaviour]"));
    let calls = provider.source.calls.lock().expect("calls");
    let (id, input) = calls.first().expect("one extract dispatch");
    assert_eq!(id, "intent", "a bare adapter dispatches to the guest declared under its name");
    assert_eq!(input.key, "intent");
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
    let mut provider = Provider::answering([GROUPING_ANSWER, PRECEDENCE_ANSWER, DESIGN_ANSWER])
        .declaring(["docs", "wiki-live", "code", "intent"]);

    // rank each adapter by its metadata; the unscripted ones read documentation
    provider.source.kinds.insert("code".to_string(), SourceKind::Behaviour);
    provider.source.kinds.insert("intent".to_string(), SourceKind::Intent);
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
            "docs",
            "wiki-live",
            "code",
            "--description",
            "intent=Sessions expire after 30.",
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

// A refused grouping is the next candidate's correction; a backend out of rounds
// fails typed and commits nothing.
#[tokio::test]
async fn grouping_refused() {
    let bind = |provider: &mut Provider| {
        provider.source.kinds.insert("code".to_string(), SourceKind::Behaviour);
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
        let mut provider =
            Provider::answering([*answer, *answer, *answer]).declaring(["docs", "code"]);
        bind(&mut provider);
        let envelope =
            fail(&provider, &["emery", "specify", "docs", "code"], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }

    // the corrected answer commits
    let refused = r#"{"groups": [{"claims": [0], "classes": [[0]]}]}"#;
    let corrected = r#"{"groups": [{"claims": [0, 1], "classes": [[0], [1]]}]}"#;
    let spec = SPEC_ANSWER.replace("greeting.behaviour", "session.timeout");
    let mut provider = Provider::answering([refused, corrected, spec.as_str(), DESIGN_ANSWER])
        .declaring(["docs", "code"]);
    bind(&mut provider);
    cli_ok(&provider, &["emery", "specify", "docs", "code"]).await;
    let check = &provider.model.exchanges()[0];
    assert_eq!(check.tool, "check", "the engine judges each candidate over the check tool");
    let correction = check.outcome.as_ref().expect_err("the first grouping is rejected");
    assert!(correction.contains("## Findings"), "the findings ride the correction: {correction}");
    assert!(correction.contains("claim 1 is in no group"), "{correction}");
    let spec = shown(&provider, "spec").await;
    assert!(spec.contains("### Requirement: session.timeout [divergence]"), "{spec}");
    assert!(
        spec.contains("Note: code (behaviour, session.timeout): Sessions expire after 15 minutes."),
        "{spec}"
    );
    provider.model.assert_exhausted();
}

// --- regeneration ---

// Every subject is drafted again and nothing of the outgoing revision reaches
// the model; the envelope reports the diff by requirement.
#[tokio::test]
async fn remine_supersedes() {
    // first run: a greeting, a session timeout, and a legacy export
    let mut provider = Provider::answering([REMINE_FIRST, DESIGN_ANSWER]).declaring(["docs"]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence(&[
            ("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            ("session.timeout", "Sessions time out after an hour."),
            ("legacy.export", "Exports ship nightly."),
        ])),
    );
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let first = current(&provider.storage);

    // second run: the greeting changed, the export gone, the overview following it
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let mut provider =
        Provider::over(Arc::clone(&provider.storage), [REMINE_SECOND, second_design.as_str()])
            .declaring(["docs"]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence(&[
            ("greeting.behaviour", "GET /greeting returns the static string 'howdy'."),
            ("session.timeout", "Sessions time out after an hour."),
        ])),
    );
    let resp = cli_ok(&provider, &["emery", "specify", "docs"]).await;

    // observe the diff, the swap, and the prune
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains(&format!("diff vs {first}:\n")), "{stdout}");
    assert!(stdout.contains("spec.md ~ preamble\n"), "the reworded preamble is a change: {stdout}");
    assert!(!stdout.contains("design.md ~ preamble"), "a standing preamble is not: {stdout}");
    assert!(stdout.contains("spec.md - REQ-003 legacy.export"), "{stdout}");
    assert!(stdout.contains("spec.md ~ REQ-001 greeting.behaviour: body, scenarios"), "{stdout}");
    assert!(stdout.contains("design.md ~ overview"), "{stdout}");
    assert!(!stdout.contains("spec.md +"), "no requirement is new: {stdout}");
    assert!(
        !stdout.contains("session.timeout"),
        "a standing requirement is not a change: {stdout}"
    );

    let request = provider.model.seen()[0].messages.join("\n");
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
    let mut provider =
        Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, second_spec, second_design])
            .declaring(["docs"]);
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
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
    let resp = cli_ok(&provider, &["emery", "--format", "json", "specify", "docs"]).await;
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
    provider.model.assert_exhausted();
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

// --- extraction ---

// Invalid adapter output is an internal error, not the operator's.
#[tokio::test]
async fn extras_missing() {
    let mut provider = Provider::idle().declaring(["docs"]);
    let mut bare = requirement("greeting.behaviour", "");
    bare.extras.clear();
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(vec![bare])));

    let envelope = fail(&provider, &["emery", "specify", "docs"], 3, "server_error").await;
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
    let mut provider = Provider::idle().declaring(["docs", "api"]);
    provider
        .source
        .evidence
        .insert("docs".to_string(), Err(bad_gateway!("source `docs`: the adapter exploded")));

    for argv in [&["emery", "specify", "docs"][..], &["emery", "specify", "docs", "api"][..]] {
        let envelope = fail(&provider, argv, 4, "bad_gateway").await;
        assert_eq!(envelope["message"], "source `docs`: the adapter exploded", "{argv:?}");
    }
}

// The failure dispatched first — declaration order — is the run's.
#[tokio::test]
async fn two_failures() {
    let mut provider = Provider::idle().declaring(["docs", "code"]);
    provider
        .source
        .evidence
        .insert("docs".to_string(), Err(bad_gateway!("source `docs`: the adapter exploded")));
    provider
        .source
        .evidence
        .insert("code".to_string(), Err(bad_gateway!("source `code`: the model is down")));

    let envelope = fail(&provider, &["emery", "specify", "docs", "code"], 4, "bad_gateway").await;
    assert_eq!(envelope["message"], "source `docs`: the adapter exploded");

    let envelope = fail(&provider, &["emery", "specify", "code", "docs"], 4, "bad_gateway").await;
    assert_eq!(envelope["message"], "source `code`: the model is down");
}

// A refusal is the operator's to fix, so it keeps the adapter's class.
#[tokio::test]
async fn extract_refuses() {
    let mut provider = Provider::idle().declaring(["docs"]);
    provider
        .source
        .evidence
        .insert("docs".to_string(), Err(bad_request!("source `docs`: the brief is empty")));

    let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
    assert_eq!(envelope["message"], "source `docs`: the brief is empty");
}

// `docs` is held forever, so a run waiting for every source would exceed the bound.
#[tokio::test]
async fn refusal_fails_fast() {
    let mut provider = Provider::idle().declaring(["docs", "code"]);
    provider.source.held.insert("docs".to_string());
    provider
        .source
        .evidence
        .insert("code".to_string(), Err(bad_request!("source `code`: the brief is empty")));

    let envelope = tokio::time::timeout(
        Duration::from_secs(5),
        fail(&provider, &["emery", "specify", "docs", "code"], 1, "bad_request"),
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
        .map(|(_, input)| input.key.clone())
        .collect();
    assert_eq!(dispatched, ["docs", "code"], "both sources were dispatched before the refusal");
}

#[tokio::test]
async fn version_too_new() {
    let mut provider = Provider::idle().declaring(["docs"]);
    provider.source.versions.insert("docs".to_string(), "99.0.0".to_string());

    fail(&provider, &["emery", "specify", "docs"], 1, "unsupported-version").await;
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
    ];
    for (answer, fragment) in cases {
        let provider = Provider::answering([answer.as_str(), answer.as_str(), answer.as_str()])
            .declaring(["docs"]);
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        assert_message(&envelope, fragment);
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
    let provider = Provider::answering([missing_scenario.as_str(), SPEC_ANSWER, DESIGN_ANSWER])
        .declaring(["source"]);

    cli_ok(&provider, &["emery", "specify", "source"]).await;

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
            Provider::answering([SPEC_ANSWER, answer.as_str(), answer.as_str(), answer.as_str()])
                .declaring(["docs"]);
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
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
            Provider::answering([SPEC_ANSWER, answer.as_str(), answer.as_str(), answer.as_str()])
                .declaring(["docs"]);
        provider.source.evidence.insert("docs".to_string(), evidence());
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }

    let mut provider = Provider::answering([SPEC_ANSWER, honest.as_str()]).declaring(["docs"]);
    provider.source.evidence.insert("docs".to_string(), evidence());
    cli_ok(&provider, &["emery", "specify", "docs"]).await;

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
        "---\nemery: 2\nrevision: {id}\n---\n\n# Design\n\n## Overview\n\n\
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

#[tokio::test]
async fn model_fails() {
    let provider = Provider {
        model: Scripted::new([Err(ModelError::Backend("scripted transport failure".into()))]),
        ..Provider::idle().declaring(["docs"])
    };
    fail(&provider, &["emery", "specify", "docs"], 4, "bad_gateway").await;
    provider.model.assert_exhausted();
}

// --- config file ---

#[tokio::test]
async fn config_file() {
    let cases: &[(&str, u8, &str, &str)] = &[
        ("not toml [", 1, "bad_request", "TOML parse error"),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\nbranch = \"main\"\n",
            1,
            "bad_request",
            "unknown field `branch`",
        ),
        ("", 1, "specify-source-required", ""),
        (
            "[sources.docs]\nadapter = \"documentation\"\n",
            1,
            "bad_request",
            "unknown field `sources`",
        ),
        (
            "[[source]]\nname = \"intent\"\nadapter = \"intent\"\nvalue = \"text\"\n",
            1,
            "bad_request",
            "unknown field `value`",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\npath = \"docs\"\n\
             description = \"text\"\n",
            1,
            "bad_request",
            "both `path` and `description`",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n\n\
             [[source]]\nname = \"docs\"\nadapter = \"intent\"\n",
            1,
            "bad_request",
            "appears twice",
        ),
        (
            "[[source]]\nname = \"Docs\"\nadapter = \"documentation\"\n",
            1,
            "bad_request",
            "is not a kebab-case key",
        ),
        (
            "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\n\
             digest = \"sha256:9f2c44aa\"\n",
            1,
            "bad_request",
            "is not 64 hex characters",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"documentation\"\ngit = \"https://github.com/acme/api@v2\"\n",
            1,
            "bad_request",
            "unknown field `git`",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"documentation\"\nurl = \"https://example.com/openapi.yaml\"\n",
            1,
            "bad_request",
            "unknown field `url`",
        ),
        (
            "[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\nregistry = \"registry.acme.io\"\n",
            1,
            "bad_request",
            "unknown field `registry`",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\npath = \"../../outside\"\n",
            1,
            "bad_request",
            "../../outside",
        ),
        (
            "[[source]]\nname = \"local\"\nadapter = \"/tmp/source.wasm\"\n",
            1,
            "bad_request",
            "`/tmp/source.wasm`",
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

// Path anchoring, inline descriptions, and declaration order, all observed on
// the `SourceInput` the adapter receives.
#[tokio::test]
async fn source_paths() {
    let scratch = Scratch::new();
    let config = scratch.config(
        "[[source]]\nname = \"zulu\"\nadapter = \"documentation\"\npath = \"nested/../docs\"\n\n\
         [[source]]\nname = \"intent\"\nadapter = \"intent\"\ndescription = \"Ship it.\"\n\n\
         [[source]]\nname = \"alpha\"\nadapter = \"local\"\npath = \"./docs\"\n",
    );

    // three sources contribute one id, so a grouping turn is scripted
    let grouping = baseline_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER])
        .declaring(["documentation", "intent", "local"]);
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    let calls = provider.source.calls.lock().expect("calls");
    let order: Vec<&str> = calls.iter().map(|(_, input)| input.key.as_str()).collect();
    assert_eq!(order, ["zulu", "intent", "alpha"], "entries extract in declaration order");
    for key in ["zulu", "alpha"] {
        let (_, input) = calls.iter().find(|(_, input)| input.key == key).expect("dispatched");
        let SourceContent::Workspace(root) = &input.content else {
            panic!("a path source lends a workspace");
        };
        assert!(
            !Path::new(root).is_absolute(),
            "the lend must stay `.`-relative for the guest preopen: {root}"
        );
        assert!(
            root.ends_with("docs") && !root.contains(".."),
            "`.` and `..` fold away lexically against the file's directory: {root}"
        );
    }
    let (_, input) = calls.iter().find(|(_, input)| input.key == "intent").expect("dispatched");
    assert_eq!(
        input.content,
        SourceContent::Value("Ship it.".to_string()),
        "a file `description` entry supplies inline text"
    );
    drop(calls);
    provider.model.assert_exhausted();
}

// --- adapter references ---

// Nothing mirrors a local component, so a re-run reads it fresh.
#[tokio::test]
async fn deleted_wasm() {
    let scratch = Scratch::new();
    let component = scratch.component();

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", &component]).await;

    fs::remove_file(&component).expect("remove the operator's file");
    fail(&provider, &["emery", "specify", &component], 2, "not_found").await;
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn component_missing() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "./missing.wasm"], 2, "not_found").await;
    for path in ["/tmp/missing.wasm", "../missing.wasm"] {
        fail(&provider, &["emery", "specify", path], 1, "bad_request").await;
    }
}

// GitHub URLs are refused: a source checkout is not an adapter.
#[tokio::test]
async fn github_refused() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "https://github.com/acme/api"], 1, "bad_request").await;
}

// `demo@1.2.0` is sugar for the `emery` namespace, which no table routes
// anywhere but augentic's registry.
#[tokio::test]
async fn package_loads() {
    for reference in ["emery:demo@1.2.0", "demo@1.2.0"] {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", reference]).await;

        assert_eq!(
            provider.plugins.loads(),
            [(registry("emery:demo@1.2.0", "augentic.io"), None)],
            "the exact reference is fetched, and the unpinned load names its registry: \
             {reference}"
        );
        assert_eq!(provider.loaded(), ["emery:demo"], "the package registers without its version");
        let calls = provider.source.calls.lock().expect("calls");
        let (id, input) = calls.first().expect("one extract dispatch");
        assert_eq!(id, "emery:demo", "the adapter id is the registered guest");
        assert_eq!(input.key, "demo", "the source key is the adapter name");
        drop(calls);
        provider.model.assert_exhausted();
    }
}

#[tokio::test]
async fn bare_declared() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]).declaring(["intent"]);

    cli_ok(&provider, &["emery", "specify", "intent"]).await;

    assert_eq!(
        provider.plugins.loads(),
        [(Location::Declared("intent".to_string()), None)],
        "a declared guest is one load, by name, never pinned"
    );
    let gated = provider.source.metadata.lock().expect("metadata").clone();
    assert_eq!(gated, ["intent"], "the version gate reads the attested name");
    let calls = provider.source.calls.lock().expect("calls");
    assert_eq!(calls[0].0, "intent", "extract dispatches by the attested name");
    drop(calls);
    provider.model.assert_exhausted();
}

// The loader refuses, typed, before any dispatch could trap.
#[tokio::test]
async fn bare_undeclared() {
    let provider = Provider::idle();

    let envelope = fail(&provider, &["emery", "specify", "nonesuch"], 1, "refused").await;

    assert_message(&envelope, "no guest `nonesuch` is declared");
    assert_eq!(provider.plugins.loads().len(), 1, "the load is what refuses");
    assert!(
        provider.source.metadata.lock().expect("metadata").is_empty(),
        "nothing is gated: the guest was never routable"
    );
}

#[tokio::test]
async fn file_named_by_stem() {
    let scratch = Scratch::new();
    let component = scratch.write("custom.wasm", b"\0asm-stub");
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", &component]).await;

    assert_eq!(provider.loaded(), ["custom"], "the path registers as its stem");
    let gated = provider.source.metadata.lock().expect("metadata").clone();
    assert_eq!(gated, ["custom"]);
    let calls = provider.source.calls.lock().expect("calls");
    let (id, input) = calls.first().expect("one extract dispatch");
    assert_eq!(id, "custom", "extract dispatches by the stem");
    assert_eq!(input.key, "custom", "the source key is the adapter's kebab stem");
    drop(calls);
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn package_routed() {
    let cases: &[(&str, &str, &str)] = &[
        // (table, package, the registry the load names)
        ("", "emery:demo@1.2.0", "augentic.io"),
        ("[registries]\nacme = \"registry.acme.io\"\n", "acme:ledger@2.1.0", "registry.acme.io"),
        (
            "[registries]\nemery = \"staging.augentic.io\"\n",
            "emery:demo@1.2.0",
            "staging.augentic.io",
        ),
    ];
    for (table, package, endpoint) in cases {
        let scratch = Scratch::new();
        let config = scratch
            .config(&format!("[[source]]\nname = \"docs\"\nadapter = \"{package}\"\n{table}"));
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

        assert_eq!(
            provider.plugins.loads(),
            [(registry(package, endpoint), None)],
            "{package} under {table:?}"
        );
        provider.model.assert_exhausted();
    }
}

#[tokio::test]
async fn package_unrouted() {
    let scratch = Scratch::new();
    let config = scratch.config("[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\n");
    let provider = Provider::idle();

    let envelope =
        fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

    assert_message(&envelope, "no registry routes `acme:ledger@2.1.0`");
    assert_message(&envelope, "add `acme = \"<registry>\"` under `[registries]`");
    assert!(provider.plugins.loads().is_empty(), "an unrouted package is never fetched");
}

// Refused before either loads: the loader's own answer would blame whichever
// file happened to load second.
#[tokio::test]
async fn file_stem_collision() {
    let scratch = Scratch::new();
    let first = scratch.write("a/tool.wasm", b"\0asm-stub");
    let second = scratch.write("b/tool.wasm", b"\0asm-stub");
    let config = scratch.config(
        "[[source]]\nname = \"first\"\nadapter = \"./a/tool.wasm\"\n\n\
         [[source]]\nname = \"second\"\nadapter = \"./b/tool.wasm\"\n",
    );
    let provider = Provider::idle();

    let envelope =
        fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

    assert_message(&envelope, &format!("adapters `{first}` and `{second}`"));
    assert_message(&envelope, "would both register as `tool`");
    assert!(provider.plugins.loads().is_empty(), "a colliding list loads nothing");
}

// Two versions of one package register as one guest, as two components sharing
// a stem do.
#[tokio::test]
async fn package_version_collision() {
    let scratch = Scratch::new();
    let config = scratch.config(
        "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
         [[source]]\nname = \"api\"\nadapter = \"documentation@1.3.0\"\n",
    );
    let provider = Provider::idle();

    let envelope =
        fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

    assert_message(
        &envelope,
        "adapters `emery:documentation@1.2.0` and `emery:documentation@1.3.0`",
    );
    assert_message(&envelope, "would both register as `emery:documentation`");
    assert!(provider.plugins.loads().is_empty(), "a colliding list fetches nothing");
}

// Asked, the loader would attest the engine in the adapter's place and extract
// would be dispatched to it.
#[tokio::test]
async fn engine_as_adapter() {
    let scratch = Scratch::new();
    let component = scratch.write("adapters/emery.wasm", b"\0asm-stub");
    let provider = Provider::idle().declaring([ENGINE]);

    let envelope = fail(&provider, &["emery", "specify", &component], 1, "bad_request").await;
    assert_message(&envelope, &format!("adapter `{component}` would register as `{ENGINE}`"));
    assert_message(&envelope, "the engine itself; rename the component");

    let envelope = fail(&provider, &["emery", "specify", ENGINE], 1, "bad_request").await;
    assert_message(&envelope, &format!("adapter `{ENGINE}` is the engine itself"));

    assert!(provider.plugins.loads().is_empty(), "the engine is never asked for as an adapter");
    assert!(
        provider.source.metadata.lock().expect("metadata").is_empty(),
        "nothing is dispatched to the engine"
    );
}

// The project-root table routes an argv source while the file's own sources
// stay out. The CWD move is hermetic under nextest's process-per-test isolation.
#[tokio::test]
async fn package_argv_registries() {
    let project = tempfile::TempDir::new().expect("project dir");
    fs::write(
        project.path().join("emery.toml"),
        "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n\n\
         [registries]\nacme = \"registry.acme.io\"\n",
    )
    .expect("write emery.toml");
    std::env::set_current_dir(project.path()).expect("enter project");
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "acme:ledger@2.1.0"]).await;

    assert_eq!(
        provider.plugins.loads(),
        [(registry("acme:ledger@2.1.0", "registry.acme.io"), None)],
        "argv names the run's only source, routed by the project's table"
    );
    assert!(
        shown(&provider, "spec").await.contains("Sources: [ledger:greeting.behaviour]"),
        "the file's `[[source]]` entries stay out of an argv run"
    );
    provider.model.assert_exhausted();
}

// The `[[source]]` entries stay undecoded when argv names the sources, so a
// malformed one never refuses the run. The CWD move is hermetic under nextest.
#[tokio::test]
async fn package_argv_malformed_sources() {
    let project = tempfile::TempDir::new().expect("project dir");
    std::env::set_current_dir(project.path()).expect("enter project");
    for entry in [
        "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\npath = \"../../outside\"\n",
        "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\nbranch = \"main\"\n",
        "[[source]]\nname = \"local\"\nadapter = \"/tmp/source.wasm\"\n",
    ] {
        fs::write(
            project.path().join("emery.toml"),
            format!("{entry}\n[registries]\nacme = \"registry.acme.io\"\n"),
        )
        .expect("write emery.toml");
        fail(&Provider::idle(), &["emery", "specify"], 1, "bad_request").await;
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", "acme:ledger@2.1.0"]).await;

        assert_eq!(
            provider.plugins.loads(),
            [(registry("acme:ledger@2.1.0", "registry.acme.io"), None)],
            "{entry}"
        );
        provider.model.assert_exhausted();
    }
}

#[tokio::test]
async fn source_digest_pinned() {
    let scratch = Scratch::new();
    scratch.component();
    let pin = digest("cd");
    let config = scratch.config(&format!(
        "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\ndigest = \"{pin}\"\n\n\
         [[source]]\nname = \"demo\"\nadapter = \"emery:demo@1.2.0\"\ndigest = \"{pin}\"\n"
    ));
    let grouping = baseline_grouping(2);
    let mut provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);
    provider.plugins =
        provider.plugins.clone().digest("source", pin.clone()).digest("emery:demo", pin.clone());

    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    assert_eq!(provider.loaded(), ["source", "emery:demo"]);
    for (location, carried) in provider.plugins.loads() {
        assert_eq!(carried, Some(pin.clone()), "{location} carries its pin");
    }
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn source_digest_mismatch() {
    let scratch = Scratch::new();
    scratch.component();
    let config = scratch.config(&format!(
        "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\ndigest = \"{}\"\n",
        digest("cd")
    ));
    let mut provider = Provider::idle();
    provider.plugins = provider.plugins.clone().digest("source", digest("ab"));

    let envelope = fail(&provider, &["emery", "specify", "--config", &config], 1, "refused").await;

    assert_message(&envelope, "source.wasm` resolved to");
    assert_message(&envelope, &format!("not its pinned digest {}", digest("cd")));
    assert!(
        provider.source.metadata.lock().expect("metadata").is_empty(),
        "a refused load is never gated"
    );
}

// A declared guest is attested, never fetched, so a digest has nothing to check.
#[tokio::test]
async fn source_digest_on_bare() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\ndigest = \"{}\"\n",
        digest("cd")
    ));
    let provider = Provider::idle();

    let envelope =
        fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

    assert_message(&envelope, "adapter `documentation`");
    assert_message(&envelope, "takes no digest");
    assert!(provider.plugins.loads().is_empty(), "a refused list loads nothing");
}

// A digest on a later entry sharing the guest is refused whether or not it
// matches what the loader attests.
#[tokio::test]
async fn source_digest_on_bare_repeated() {
    for pin in [digest("ab"), digest("cd")] {
        let scratch = Scratch::new();
        let config = scratch.config(&format!(
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n\n\
             [[source]]\nname = \"guide\"\nadapter = \"documentation\"\ndigest = \"{pin}\"\n"
        ));
        let provider = Provider::idle().declaring(["documentation"]);

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(&envelope, "adapter `documentation`");
        assert_message(&envelope, "takes no digest");
        assert!(provider.plugins.loads().is_empty(), "a refused list loads nothing: {pin}");
    }
}

#[tokio::test]
async fn source_name_defaulted() {
    let scratch = Scratch::new();
    scratch.component();
    let config = scratch.config(
        "[[source]]\nadapter = \"documentation\"\n\n\
         [[source]]\nadapter = \"emery:demo@1.2.0\"\n\n\
         [[source]]\nadapter = \"./source.wasm\"\n",
    );
    let grouping = baseline_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER])
        .declaring(["documentation"]);

    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    assert!(
        shown(&provider, "spec").await.contains(
            "Sources: [documentation:greeting.behaviour, demo:greeting.behaviour, \
             source:greeting.behaviour]"
        ),
        "each entry is keyed by its adapter"
    );
    provider.model.assert_exhausted();
}

// The list is checked whole before a single adapter loads.
#[tokio::test]
async fn bad_key_package() {
    let cases = [
        (
            "[[source]]\nname = \"Docs\"\nadapter = \"emery:documentation@1.2.0\"\n",
            "is not a kebab-case key",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
             [[source]]\nname = \"docs\"\nadapter = \"emery:intent@1.0.0\"\n",
            "appears twice",
        ),
    ];
    for (body, fragment) in cases {
        let scratch = Scratch::new();
        let config = scratch.config(body);
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        assert!(provider.plugins.loads().is_empty(), "a refused list loads nothing: {fragment}");
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a refused list gates nothing: {fragment}"
        );
    }
}

// `unavailable` lands on the BadGateway exit and `refused` on the BadRequest
// exit; the loader answers by the registered name, the package without its version.
#[tokio::test]
async fn load_failures() {
    let mut provider = Provider::idle();
    provider.plugins = provider.plugins.clone().refuse(
        "emery:demo",
        LoadError::Unavailable("resolving `emery:demo@1.2.0`: endpoint unreachable".to_string()),
    );
    fail(&provider, &["emery", "specify", "emery:demo@1.2.0"], 4, "unavailable").await;

    let mut provider = Provider::idle();
    provider.plugins = provider
        .plugins
        .clone()
        .refuse("emery:demo", LoadError::Refused("not a raw wasm component".to_string()));
    fail(&provider, &["emery", "specify", "emery:demo@1.2.0"], 1, "refused").await;
}

#[tokio::test]
async fn package_ref() {
    let cases: &[(&str, &str)] = &[
        ("emery:demo", "missing `@<version>`"),
        ("emery:demo@main", "invalid version `main`"),
        ("emery:@1.2.0", "missing a name before `@`"),
    ];
    for (reference, fragment) in cases {
        let provider = Provider::idle();
        let envelope = fail(&provider, &["emery", "specify", reference], 1, "bad_request").await;
        assert_message(&envelope, fragment);
    }
}

// --- store ---

// A current id naming no revision is corruption, never an empty result.
#[tokio::test]
async fn corrupt_current() {
    let provider = Provider::idle();
    provider.storage.insert_state(CURRENT, b"0123456789abcdef");
    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;
}

// A document rewritten under its id no longer hashes to it.
#[tokio::test]
async fn tampered_revision() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]).declaring(["docs"]);
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let id = current(&provider.storage);

    provider.storage.insert_object(CONTAINER, &format!("{id}/spec.json"), b"{}\n");

    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;
}

// Outdated is not corrupt: `show` refuses typed, and the next `specify`
// regenerates over it.
#[tokio::test]
async fn spec_outdated() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]).declaring(["docs"]);
    let outdated = seed(
        &provider.storage,
        br#"{"emery": 1, "requirements": []}"#,
        br#"{"emery": 1, "sections": []}"#,
    );

    let envelope = fail(&provider, &["emery", "show", "spec"], 1, "spec-outdated").await;
    assert_message(&envelope, "grammar 1");
    assert!(
        envelope["hint"].as_str().unwrap_or("").contains("emery specify"),
        "the hint names the way out: {envelope}"
    );

    let resp = cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(!stdout.contains("diff vs"), "an outdated outgoing revision yields no diff: {stdout}");
    let id = current(&provider.storage);
    assert_ne!(id, outdated, "the regenerated revision is current");
    assert!(
        provider.storage.object(CONTAINER, &format!("{outdated}/spec.json")).is_none(),
        "the outdated revision is pruned"
    );
    cli_ok(&provider, &["emery", "show", "spec"]).await;
    provider.model.assert_exhausted();
}

// Regeneration is the recovery path; only the advisory diff is suppressed.
#[tokio::test]
async fn repair_tampered() {
    let second_spec = SPEC_ANSWER.replace("hello", "howdy");
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let provider = Provider::answering([
        SPEC_ANSWER,
        DESIGN_ANSWER,
        second_spec.as_str(),
        second_design.as_str(),
    ])
    .declaring(["docs"]);

    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let first = current(&provider.storage);
    provider.storage.insert_object(CONTAINER, &format!("{first}/spec.json"), b"{}\n");

    let resp = cli_ok(&provider, &["emery", "specify", "docs"]).await;

    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(
        !stdout.contains("diff vs"),
        "an unreadable outgoing revision yields no diff: {stdout}"
    );

    let second = current(&provider.storage);
    assert_ne!(first, second, "the repaired store names the new revision");

    for name in ["spec.json", "design.json"] {
        assert!(
            provider.storage.object(CONTAINER, &format!("{first}/{name}")).is_none(),
            "the tampered outgoing revision is pruned: {name}"
        );
    }

    assert!(shown(&provider, "spec").await.contains("howdy"), "show renders the repair");

    provider.model.assert_exhausted();
}

// Bytes that decode to no id fail `show` closed, yet the next `specify` swaps
// over them.
#[tokio::test]
async fn repair_current() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]).declaring(["docs"]);
    provider.storage.insert_state(CURRENT, b"\xff\xfe");
    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;

    cli_ok(&provider, &["emery", "specify", "docs"]).await;

    let id = current(&provider.storage);
    assert!(provider.storage.object(CONTAINER, &format!("{id}/spec.json")).is_some());
    cli_ok(&provider, &["emery", "show", "spec"]).await;
    provider.model.assert_exhausted();
}

// Isolation is host policy over the engine's flat keys.
#[tokio::test]
async fn multi_project() {
    let scratch = Scratch::new();
    let component = scratch.component();

    // one shared store, two project-scoped views
    let shared = Memory::default();
    let alpha = Provider::over(
        Arc::new(Namespaced::new("alpha", shared.clone())),
        [SPEC_ANSWER, DESIGN_ANSWER],
    );
    let beta_spec = SPEC_ANSWER.replace("hello", "howdy");
    let beta_design = DESIGN_ANSWER.replace("hello", "howdy");
    let beta =
        Provider::over(Arc::new(Namespaced::new("beta", shared.clone())), [beta_spec, beta_design]);

    cli_ok(&alpha, &["emery", "specify", &component]).await;
    cli_ok(&beta, &["emery", "specify", &component]).await;

    // every write landed under its project prefix
    assert!(shared.state(CURRENT).is_none(), "no unprefixed current id exists");
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
    assert_eq!(String::from_utf8_lossy(&spec_alpha), SPEC_REVISION, "alpha committed the revision");
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

// --- helpers ---

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

fn registry(package: &str, endpoint: &str) -> Location {
    Location::Registry {
        package: package.to_string(),
        endpoint: Some(endpoint.to_string()),
    }
}

fn current(storage: &Memory) -> String {
    stored_id(storage, CURRENT)
}

fn project_current(shared: &Memory, project: &str) -> String {
    stored_id(shared, &format!("{project}/{CURRENT}"))
}

fn stored_id(storage: &Memory, key: &str) -> String {
    String::from_utf8(storage.state(key).expect("current")).expect("utf-8 revision id")
}

fn document(storage: &Memory, id: &str, name: &str) -> Vec<u8> {
    storage.object(CONTAINER, &format!("{id}/{name}")).unwrap_or_else(|| panic!("{name}"))
}

// The engine's content id: SHA-256 over the length-prefixed bodies, spec then design.
fn revision(spec: &[u8], design: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for body in [spec, design] {
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}

fn seed(storage: &Memory, spec: &[u8], design: &[u8]) -> String {
    let id = revision(spec, design);
    storage.insert_object(CONTAINER, &format!("{id}/spec.json"), spec);
    storage.insert_object(CONTAINER, &format!("{id}/design.json"), design);
    storage.insert_state(CURRENT, id.as_bytes());
    id
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
