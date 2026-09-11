//! The `specify` → `show` product arc
//!
//! The scenarios an operator lives through: naming sources, generating a
//! specification, reviewing it, regenerating it, and hitting every refusal
//! along the way — an invalid source, an untrusted adapter, a model draft
//! that still does not fit the requirements or the plan once the backend's
//! rounds are spent.
//!
//! Each scenario drives the real command façade over scripted capabilities,
//! so it reads as usage documentation while still asserting the exact
//! envelope, exit code, and stored revision the operator would see. The
//! model answers are typed drafts, so the scripted turns are JSON; the
//! stored documents are the engine's canonical JSON, and the documents `show`
//! renders from them are the engine's canonical Markdown.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;
use std::path::Path;
use std::sync::Arc;

use emery_engine::{CONTAINER, CURRENT};
use emery_source::types::{Authority, ClaimKind, Evidence, SourceContent};
use omnia_guest::model::Error as ModelError;
use omnia_guest::plugins::{Digest, Error as LoadError, Location};
use omnia_guest::{BlobStore, StateStore, bad_gateway, bad_request};
use omnia_test::SeenFormat;
use omnia_test::guest::{Memory, Namespaced, Scripted};
use serde_json::Value;
use sha2::{Digest as _, Sha256};
use support::{Provider, claim, cli, cli_ok, digest, evidence, fail, requirement};

// Scripted drafts, the canonical documents the engine commits from them, and
// the documents it renders from those documents.
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

// Builds the grouping answer that merges `count` claims into one agreeing
// requirement — what a run over one id appearing several times expects.
fn baseline_grouping(count: usize) -> String {
    let indices = (0..count).map(|index| index.to_string()).collect::<Vec<_>>().join(", ");
    format!("{{\"groups\": [{{\"claims\": [{indices}], \"classes\": [[{indices}]]}}]}}")
}

// A scratch directory inside the project where one scenario's operator files
// live. Every path handed to the CLI must stay project-relative for the
// guest preopen, so each write answers with that relative path.
struct Scratch(tempfile::TempDir);

impl Scratch {
    fn new() -> Self {
        Self(tempfile::TempDir::new_in(env!("CARGO_MANIFEST_DIR")).expect("project tempdir"))
    }

    // Writes `body` under `name`, returning the project-relative path.
    fn write(&self, name: &str, body: impl AsRef<[u8]>) -> String {
        let path = self.0.path().join(name);
        fs::write(&path, body).unwrap_or_else(|err| panic!("write {name}: {err}"));
        path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .expect("path under project")
            .to_str()
            .expect("utf-8 path")
            .to_string()
    }

    // Writes the operator's `emery.toml`.
    fn config(&self, body: &str) -> String {
        self.write("emery.toml", body)
    }

    // Writes a stub `source.wasm`: the loader is scripted, so the component
    // only has to exist as a `.wasm` file.
    fn component(&self) -> String {
        self.write("source.wasm", b"\0asm-stub")
    }
}

// One `specify` loads, extracts, and commits the typed revision — no prior
// verb; `show` renders each document from the committed revision; an
// identical re-run drafts again from the sources alone, is byte-stable, and
// says so.
#[tokio::test]
async fn gen_spec() {
    // --------------------------------------------------
    // Arrange: only the operator-supplied component touches the
    // filesystem; engine state stays in scripted storage.
    // --------------------------------------------------
    let scratch = Scratch::new();
    let component = scratch.component();

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]);

    // --------------------------------------------------
    // Act: the first specify.
    // --------------------------------------------------
    cli_ok(&provider, &["emery", "specify", &component]).await;

    // --------------------------------------------------
    // Observe: the load, the current id, and the revision.
    // --------------------------------------------------
    let request = provider.plugins.loads().first().cloned().expect("one load request");
    assert_eq!(request.package, "source:source", "the adapter id is the loaded package identity");
    let Location::Path(path) = &request.location else {
        panic!("a local component loads by path");
    };
    assert!(path.ends_with("source.wasm"), "the preopen-relative path rides the request: {path}");
    assert!(request.digest.is_none(), "an unpinned source requests no digest");
    assert!(
        provider.storage.objects("adapters").is_empty(),
        "nothing mirrors into engine storage; the loader reads the file fresh"
    );
    assert!(provider.storage.state("project.yaml").is_none(), "no project record exists");
    let id = current(&provider.storage);
    // The stored documents are the engine's facts beside the drafts, as
    // canonical JSON: the id, status, coverage, and cited claims are all the
    // engine's.
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

    // Review is `show`: text stdout is the document rendered from the
    // revision — headings, provenance, the gap tag and note are all rendered.
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
    // The JSON envelope carries the revision, the projection, and the typed
    // document itself.
    let resp = cli_ok(&provider, &["emery", "--format", "json", "show", "spec"]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["body"], projection(SPEC_RENDERED, &id), "{envelope}");
    let document: Value =
        serde_json::from_str(SPEC_REVISION).expect("the revision fixture is JSON");
    assert_eq!(envelope["document"], document, "the envelope carries the typed document");

    // An identical re-run reads nothing of the stored revision: both drafts
    // are asked again, the same facts and drafts commit the same bytes, and
    // the empty diff is reported.
    let resp = cli_ok(&provider, &["emery", "specify", &component]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");
    assert_eq!(current(&provider.storage), id, "the same revision keeps its id");

    provider.model.assert_exhausted();
}

// `--config` is the other specify authority: entry names become
// source keys, and a local adapter resolves relative to the file.
#[tokio::test]
async fn from_file() {
    let scratch = Scratch::new();
    scratch.component();
    let config = scratch.config(SOURCES);

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    let request = provider.plugins.loads().first().cloned().expect("one load request");
    let Location::Path(path) = &request.location else {
        panic!("a local component loads by path");
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

// One adapter may name several roots: the loader is asked once, each
// source extracts over its own workspace, and the two claims of one id
// are one requirement citing both sources.
#[tokio::test]
async fn shared_roots() {
    let cases: &[(&str, &str, bool)] = &[
        ("emery:documentation@1.2.0", "emery:documentation@1.2.0", false),
        ("./source.wasm", "source:source", true),
    ];
    for (adapter, package, wasm) in cases {
        let scratch = Scratch::new();
        if *wasm {
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

        let loads = provider.plugins.loads();
        assert_eq!(loads.len(), 1, "{adapter}: one adapter identity loads once");
        assert_eq!(loads[0].package, *package, "{adapter}");

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

// A run naming no sources at all discovers the project-root
// `emery.toml` before failing — never merged with argv sources. The
// CWD move is hermetic under nextest's process-per-test isolation.
#[tokio::test]
async fn discovery() {
    let project = tempfile::TempDir::new().expect("project dir");
    fs::write(
        project.path().join("emery.toml"),
        "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n",
    )
    .expect("write emery.toml");
    std::env::set_current_dir(project.path()).expect("enter project");

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify"]).await;

    assert!(shown(&provider, "spec").await.contains("Sources: [docs:greeting.behaviour]"));
    provider.model.assert_exhausted();
}

// `--description` supplies inline text under the adapter's name: no
// filesystem lend reaches extract, and a bare adapter needs no local
// component.
#[tokio::test]
async fn description_source() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--description", "intent=Ship it."]).await;

    let spec = shown(&provider, "spec").await;
    assert!(spec.contains("Sources: [intent:greeting.behaviour]"));
    let calls = provider.source.calls.lock().expect("calls");
    let (id, input) = calls.first().expect("one extract dispatch");
    assert_eq!(id, "source:intent", "a bare adapter dispatches by routed name");
    assert_eq!(input.key, "intent");
    assert_eq!(input.content, SourceContent::Value("Ship it.".to_string()));
    drop(calls);
    provider.model.assert_exhausted();
}

// Requirement identity and agreement are one model partition over the
// byte-equal-id baseline, and authority derives the rest: the grouping
// binds `code`'s `session-expiry` into the timeout requirement, where
// the intent directive outranks it as [divergence] with one templated
// loser note; tied documentation peers surface as [conflict] with no
// body; and the uncovered timeout keeps its tag and gains the gap note
// — no synthetic gap requirement, so the rendered spec has two blocks.
#[tokio::test]
async fn authority_precedence() {
    let mut provider = Provider::answering([GROUPING_ANSWER, PRECEDENCE_ANSWER, DESIGN_ANSWER]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(
            Authority::Documentation,
            vec![
                requirement("login.flow", "Users sign in with a magic link."),
                requirement("session.timeout", "Sessions expire after 30 minutes of inactivity."),
                claim(
                    ClaimKind::Criterion,
                    "login.flow.success",
                    ("criterion", "A valid link signs the user in."),
                ),
                // Non-requirement kinds ride along as synthesis context.
                claim(ClaimKind::Decision, "auth.decision", ("body", "Sessions are cookie-bound.")),
            ],
        )),
    );
    provider.source.evidence.insert(
        "wiki-live".to_string(),
        Ok(evidence(
            Authority::Documentation,
            vec![requirement("login.flow", "Users sign in with a passkey.")],
        )),
    );
    provider.source.evidence.insert(
        "code".to_string(),
        Ok(evidence(
            Authority::Behaviour,
            vec![
                requirement("login.flow", "Users sign in with email and password."),
                // Behaviour names the timeout differently; the grouping
                // call, not the id, joins it to the requirement.
                requirement("session-expiry", "Sessions expire after 15 minutes of inactivity."),
            ],
        )),
    );
    provider.source.evidence.insert(
        "intent".to_string(),
        Ok(evidence(
            Authority::Intent,
            vec![requirement(
                "session.timeout",
                "Sessions must expire after 30 minutes of inactivity.",
            )],
        )),
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

    // The grouping request indexes every claim and withholds authority.
    // The schema bounds those indexes; a derive reshape that no-ops the
    // pointer would silently drop the hint.
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

// A grouping the partition rules refuse — a baseline pair split, a claim
// in no group, a claim in two classes — is sent back as the correction
// and the next candidate checked; a backend out of rounds fails with a
// typed error carrying the last correction and commits nothing.
#[tokio::test]
async fn grouping_refused() {
    let bind = |provider: &mut Provider| {
        provider.source.evidence.insert(
            "docs".to_string(),
            Ok(evidence(
                Authority::Documentation,
                vec![requirement("session.timeout", "Sessions expire after 30 minutes.")],
            )),
        );
        provider.source.evidence.insert(
            "code".to_string(),
            Ok(evidence(
                Authority::Behaviour,
                vec![requirement("session.timeout", "Sessions expire after 15 minutes.")],
            )),
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
        let envelope =
            fail(&provider, &["emery", "specify", "docs", "code"], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }

    // The corrected answer commits: the same statements in two classes
    // diverge, and the winner is the documentation.
    let refused = r#"{"groups": [{"claims": [0], "classes": [[0]]}]}"#;
    let corrected = r#"{"groups": [{"claims": [0, 1], "classes": [[0], [1]]}]}"#;
    let spec = SPEC_ANSWER.replace("greeting.behaviour", "session.timeout");
    let mut provider = Provider::answering([refused, corrected, spec.as_str(), DESIGN_ANSWER]);
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

// A re-run over changed evidence supersedes the revision: the old
// blobs are pruned, the current id swaps, and the success envelope
// reports the re-mine diff by requirement — removed and changed, naming
// the fields that differ — while a requirement whose facts and place stand
// is not a change. Every subject is drafted again; nothing of the outgoing
// revision reaches the model.
#[tokio::test]
async fn remine_supersedes() {
    // --------------------------------------------------
    // First run: the docs describe a greeting, a session timeout, and a
    // legacy export.
    // --------------------------------------------------
    let mut provider = Provider::answering([REMINE_FIRST, DESIGN_ANSWER]);
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

    // --------------------------------------------------
    // Second run: the greeting changed, the export is gone, and the
    // design overview follows the greeting. The draft covers every
    // remaining subject.
    // --------------------------------------------------
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let mut provider =
        Provider::over(Arc::clone(&provider.storage), [REMINE_SECOND, second_design.as_str()]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(docs_evidence(&[
            ("greeting.behaviour", "GET /greeting returns the static string 'howdy'."),
            ("session.timeout", "Sessions time out after an hour."),
        ])),
    );
    let resp = cli_ok(&provider, &["emery", "specify", "docs"]).await;

    // --------------------------------------------------
    // Observe: the diff, the swap, and the prune.
    // --------------------------------------------------
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains(&format!("diff vs {first}:\n")), "{stdout}");
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

// The JSON envelope carries the re-mine diff per document: `spec` lists
// requirements by id and subject, `changed` naming the differing fields;
// `design` lists sections by kind. The second run's evidence changes the
// greeting's statement, adds an audit requirement, and adds a `type` claim.
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
        Provider::answering([SPEC_ANSWER, DESIGN_ANSWER, second_spec, second_design]);
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let first = current(&provider.storage);

    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(
            Authority::Documentation,
            vec![
                requirement(
                    "greeting.behaviour",
                    "GET /greeting returns the static string 'howdy'.",
                ),
                requirement("access.audit", "Access is audited."),
                claim(
                    ClaimKind::Type,
                    "greeting.type",
                    ("signature", "interface Greeting { text: string }"),
                ),
            ],
        )),
    );
    let resp = cli(&provider, &["emery", "--format", "json", "specify", "docs"]).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let diff = &envelope["diff"];
    assert_eq!(diff["from"], first, "{envelope}");
    assert!(diff.get("documents").is_none(), "the diff is typed, not by file: {envelope}");
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
    assert_eq!(diff["design"]["changed"], serde_json::json!(["overview"]), "{envelope}");
    assert_eq!(diff["design"]["added"], serde_json::json!(["domain-model"]), "{envelope}");
    assert_eq!(diff["design"]["removed"], serde_json::json!([]), "{envelope}");
    provider.model.assert_exhausted();
}

// Builds documentation evidence over `(subject, statement)` requirements in
// document order, each covered by its own criterion.
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
    evidence(Authority::Documentation, claims)
}

// The drafts are keyed by subject, so their order is immaterial; the
// renderer places each under its requirement.
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

// The second draft answers for every remaining subject, the timeout's
// scenario word for word.
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

// A requirement claim missing its `statement` extra fails the whole run
// with a typed error (the A8 claim gate) before anything commits.
#[tokio::test]
async fn extras_missing() {
    let mut provider = Provider::idle();
    let mut bare = requirement("greeting.behaviour", "");
    bare.extras.clear();
    provider
        .source
        .evidence
        .insert("docs".to_string(), Ok(evidence(Authority::Documentation, vec![bare])));

    fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
}

// An adapter failure surfaces as the upstream error it is.
#[tokio::test]
async fn extract_fails() {
    let mut provider = Provider::idle();
    provider
        .source
        .evidence
        .insert("docs".to_string(), Err(bad_gateway!("source `docs`: the adapter exploded")));

    let envelope = fail(&provider, &["emery", "specify", "docs"], 4, "bad_gateway").await;
    assert_eq!(envelope["message"], "source `docs`: the adapter exploded");
}

// An adapter refusing its input is the operator's error, not the
// adapter's: the refusal keeps its class through the engine.
#[tokio::test]
async fn extract_refuses() {
    let mut provider = Provider::idle();
    provider
        .source
        .evidence
        .insert("docs".to_string(), Err(bad_request!("source `docs`: the brief is empty")));

    let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
    assert_eq!(envelope["message"], "source `docs`: the brief is empty");
}

// An adapter declaring a newer minimum `emery-version` than the binary
// refuses with the dedicated version exit code.
#[tokio::test]
async fn version_too_new() {
    let mut provider = Provider::idle();
    provider.source.versions.insert("docs".to_string(), "99.0.0".to_string());

    fail(&provider, &["emery", "specify", "docs"], 1, "unsupported-version").await;
}

// A spec draft outside its schema or its requirements is refused once the
// backend's rounds are spent, one finding per case: not JSON, a requirement
// left undrafted, a subject that is not a requirement, a subject drafted
// twice, no scenario, and a preamble paragraph opening with a reserved
// marker. The operator never sees a half-committed run.
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
        let provider = Provider::answering([answer.as_str(), answer.as_str(), answer.as_str()]);
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }
}

// The schema steers the draft toward this run's requirements; the check is
// the gate. A finding is fed back as the correction with the previous answer,
// and the corrected draft commits: the operator sees one committed
// revision, not the intermediate miss.
#[tokio::test]
async fn repaired_draft() {
    let missing_scenario =
        SPEC_ANSWER.replace(r#""then": "the response is `hello`""#, r#""then": """#);
    assert_ne!(missing_scenario, SPEC_ANSWER, "the fixture carries the patched line");
    let provider = Provider::answering([missing_scenario.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

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

// The design leg is gated the same way: a draft outside its schema or plan
// is refused once the backend's rounds are spent, one finding per case —
// not JSON, the required overview absent, a section outside the closed
// vocabulary, a requirement heading smuggled into a paragraph, and a
// citation of a source the run never bound.
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
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }
}

// The evidence plans `design.md`'s sections: a `type` claim requires a
// `domain-model` section referencing it exactly once, and a section no
// claim informs may not appear. Every dishonest draft is refused; the
// honest one commits with the signature rendered verbatim, and `show`
// renders it.
#[tokio::test]
async fn dishonest_design() {
    let signature = "interface Greeting { text: string }";
    let evidence = || {
        Ok(evidence(
            Authority::Documentation,
            vec![
                requirement(
                    "greeting.behaviour",
                    "GET /greeting returns the static string 'hello'.",
                ),
                claim(ClaimKind::Type, "greeting.type", ("signature", signature)),
            ],
        ))
    };
    let draft = |sections: &str| format!(r#"{{"preamble": [], "sections": [{sections}]}}"#);
    // `(from the browser)` is prose — a citation key is one token.
    let overview = r#"{"kind": "overview", "blocks": [{"text": "Requests arrive (from the browser) and (from docs) they route."}]}"#;
    let domain = r#"{"kind": "domain-model", "blocks": [{"text": "The greeting payload is one string field."}, {"type": "greeting.type"}]}"#;
    let honest = draft(&format!("{overview}, {domain}"));
    let cases: Vec<(String, &str)> = vec![
        // The required `domain-model` is missing.
        (draft(overview), "`## Domain model` is required but absent"),
        // `ui-layout` appears with no spatial claim behind it.
        (
            draft(&format!(
                r#"{overview}, {domain}, {{"kind": "ui-layout", "blocks": [{{"text": "- page"}}]}}"#
            )),
            "`## UI / layout` is present but no claim informs it",
        ),
        // The signature is quoted as prose instead of referenced.
        (
            draft(&format!(
                r#"{overview}, {{"kind": "domain-model", "blocks": [{{"text": "`{signature}`"}}]}}"#
            )),
            "type `greeting.type` is never referenced",
        ),
        // The type is referenced twice.
        (
            draft(&format!(
                r#"{overview}, {{"kind": "domain-model", "blocks": [{{"type": "greeting.type"}}, {{"type": "greeting.type"}}]}}"#
            )),
            "type `greeting.type` is referenced 2 times",
        ),
        // A type block outside `domain-model`, naming no type claim.
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
        let envelope = fail(&provider, &["emery", "specify", "docs"], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }

    let mut provider = Provider::answering([SPEC_ANSWER, honest.as_str()]);
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

// A model transport failure surfaces as one typed synthesis error.
#[tokio::test]
async fn model_fails() {
    let provider = Provider {
        model: Scripted::new([Err(ModelError::Backend("scripted transport failure".into()))]),
        ..Provider::idle()
    };
    fail(&provider, &["emery", "specify", "docs"], 4, "bad_gateway").await;
    provider.model.assert_exhausted();
}

// Every malformed operator-owned `emery.toml` is refused with a typed error
// before anything commits.
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
        // The superseded `[sources.<key>]` / `value` schema fails loudly.
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
        ("[[source]]\nadapter = \"documentation\"\n", 1, "bad_request", "missing field `name`"),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\npath = \"docs\"\n\
             description = \"text\"\n",
            1,
            "bad_request",
            "both `path` and `description`",
        ),
        // Duplicate names reuse argv's typed duplicate error.
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n\n\
             [[source]]\nname = \"docs\"\nadapter = \"intent\"\n",
            1,
            "bad_request",
            "appears twice",
        ),
        // A malformed pin on a local component refuses before any load.
        (
            "[[source]]\nname = \"pinned\"\nadapter = \"./source.wasm\"\n\
             digest = \"sha256:9f2c44aa\"\n",
            1,
            "bad_request",
            "64 hex characters",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"documentation\"\ngit = \"https://github.com/acme/api@v2\"\n",
            1,
            "bad_request",
            "`git` and `url` are not supported",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"documentation\"\nurl = \"https://example.com/openapi.yaml\"\n",
            1,
            "bad_request",
            "`git` and `url` are not supported",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"documentation\"\ngit = \"git+https://github.com/acme/api#deadbeef\"\n",
            1,
            "bad_request",
            "drop the `git+` prefix",
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

    // An unreadable file is a typed filesystem error.
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--config", "nonexistent/emery.toml"], 3, "server_error")
        .await;

    // Host-absolute and escaping paths never cross into the guest namespace.
    for path in ["/nonexistent/emery.toml", "../emery.toml"] {
        fail(&provider, &["emery", "specify", "--config", path], 1, "bad_request").await;
    }
}

// The loader keys are gated by reference kind: `registry` only steers
// registry acquisition, so it rides only a package-shaped reference,
// and a `digest` pin binds exact bytes the loader acquires, so a bare
// name — which never loads — cannot carry one.
#[tokio::test]
async fn loader_keys_gated() {
    let pinned_bare = format!(
        "[[source]]\nname = \"pinned\"\nadapter = \"documentation\"\ndigest = \"{}\"\n",
        digest("ab")
    );
    let cases: &[(&str, &str)] = &[
        (
            "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\n\
             registry = \"registry.acme.example\"\n",
            "`registry` requires a package adapter",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"documentation\"\n\
             registry = \"registry.acme.example\"\n",
            "`registry` requires a package adapter",
        ),
        (pinned_bare.as_str(), "not a bare name"),
    ];
    for (body, fragment) in cases {
        let scratch = Scratch::new();
        let config = scratch.config(body);
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
        assert_message(&envelope, fragment);
    }
}

// File-relative `path` entries anchor at the file's directory, fold
// `.` and `..` lexically, and stay `.`-relative so the guest preopen
// can open them; `description` entries lend nothing; `[[source]]`
// entries extract in declaration order, not name order — all observed on
// the `SourceInput` the adapter receives.
#[tokio::test]
async fn source_paths() {
    let scratch = Scratch::new();
    let config = scratch.config(
        "[[source]]\nname = \"zulu\"\nadapter = \"documentation\"\npath = \"nested/../docs\"\n\n\
         [[source]]\nname = \"intent\"\nadapter = \"intent\"\ndescription = \"Ship it.\"\n\n\
         [[source]]\nname = \"alpha\"\nadapter = \"local\"\npath = \"./docs\"\n",
    );

    // Three sources contribute one id: the grouping turn merges them.
    let grouping = baseline_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);
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

// Local components are read fresh on every run — nothing mirrors, so a
// re-run after the operator deletes the source file fails with a typed error.
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

// A path that is not a `.wasm` component file is refused with a typed error.
#[tokio::test]
async fn component_missing() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "./missing.wasm"], 2, "not_found").await;
    for path in ["/tmp/missing.wasm", "../missing.wasm"] {
        fail(&provider, &["emery", "specify", path], 1, "bad_request").await;
    }
}

// A pin that matches the resolved bytes loads and extracts; the pin
// rides the load request.
#[tokio::test]
async fn pinned_component() {
    let scratch = Scratch::new();
    scratch.component();
    let config = scratch.config(&format!(
        "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\ndigest = \"{}\"\n",
        digest("ab")
    ));

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    let loads = provider.plugins.loads();
    let request = loads.first().expect("one load request");
    assert_eq!(request.digest, Some(digest("ab")), "the source's pin rides the load request");
    provider.model.assert_exhausted();
}

// A pinned local component must hash to exactly the pinned bytes: the
// loader's typed mismatch refusal surfaces on the exit contract before
// anything extracts or commits.
#[tokio::test]
async fn digest_mismatch() {
    let scratch = Scratch::new();
    scratch.component();
    let config = scratch.config(&format!(
        "[[source]]\nname = \"local\"\nadapter = \"./source.wasm\"\ndigest = \"{}\"\n",
        digest("11")
    ));

    let mut provider = Provider::idle();
    provider.plugins = provider.plugins.clone().digest("source:source", digest("ab"));

    fail(&provider, &["emery", "specify", "--config", &config], 1, "refused").await;
}

// GitHub URLs are refused: a source checkout is not an adapter.
#[tokio::test]
async fn github_refused() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "https://github.com/acme/api"], 1, "bad_request").await;
}

// An exact package reference (`emery:<name>@<semver>`, or the
// first-party shorthand as sugar for the `emery` namespace) loads
// through the deployment loader from the acquirer's default registry
// and is addressed by its own package identity — no parallel adapter id.
#[tokio::test]
async fn package_loads() {
    for reference in ["emery:demo@1.2.0", "demo@1.2.0"] {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", reference]).await;

        let loads = provider.plugins.loads();
        let request = loads.first().expect("one load request");
        assert_eq!(
            request.package, "emery:demo@1.2.0",
            "the package reference is the load identity: {reference}"
        );
        assert_eq!(
            request.location,
            Location::Registry(None),
            "no override selects the acquirer's default registry"
        );
        assert!(request.digest.is_none(), "an unpinned source requests no digest");
        let calls = provider.source.calls.lock().expect("calls");
        let (id, input) = calls.first().expect("one extract dispatch");
        assert_eq!(id, "emery:demo@1.2.0", "the adapter id is the loaded package identity");
        assert_eq!(input.key, "demo", "the source key is the adapter name");
        drop(calls);
        provider.model.assert_exhausted();
    }
}

// The source's `registry` key overrides the acquirer's default
// endpoint per source.
#[tokio::test]
async fn registry_override() {
    let scratch = Scratch::new();
    let config = scratch.config(
        "[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\n\
         registry = \"registry.acme.example\"\n",
    );

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    let loads = provider.plugins.loads();
    let request = loads.first().expect("one load request");
    assert_eq!(request.package, "acme:ledger@2.1.0", "third-party namespaces pass through");
    assert_eq!(
        request.location,
        Location::Registry(Some("registry.acme.example".to_string())),
        "the source's override rides the load request"
    );
    provider.model.assert_exhausted();
}

// A registry package pin verifies like a local component pin: the pin
// rides the load request, and a mismatch is refused with a typed error
// before anything extracts or commits.
#[tokio::test]
async fn pinned_package() {
    let pinned = |pin: &Digest| {
        format!("[[source]]\nname = \"demo\"\nadapter = \"emery:demo@1.2.0\"\ndigest = \"{pin}\"\n")
    };

    let scratch = Scratch::new();
    let config = scratch.config(&pinned(&digest("ab")));

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
    let request = provider.plugins.loads().first().cloned().expect("one load request");
    assert_eq!(request.digest, Some(digest("ab")), "the source's pin rides the load request");
    provider.model.assert_exhausted();

    let mismatched = Scratch::new();
    let config = mismatched.config(&pinned(&digest("11")));

    let mut provider = Provider::idle();
    provider.plugins = provider.plugins.clone().digest("emery:demo@1.2.0", digest("ab"));
    fail(&provider, &["emery", "specify", "--config", &config], 1, "refused").await;
}

// A second source that re-pins an already-loaded adapter is refused as
// `already-active`: the loader cannot re-bind the identity.
#[tokio::test]
async fn conflicting_pin() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[[source]]\nname = \"a\"\nadapter = \"emery:demo@1.2.0\"\ndigest = \"{}\"\n\n\
         [[source]]\nname = \"b\"\nadapter = \"emery:demo@1.2.0\"\ndigest = \"{}\"\n",
        digest("ab"),
        digest("cd"),
    ));

    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--config", &config], 1, "already-active").await;
    let loads = provider.plugins.loads();
    assert_eq!(loads.len(), 1, "the conflicting pin never reaches the loader");
}

// Load failures land on the exit contract: an acquisition (registry
// or network) failure is the loader's `unavailable` on the
// BadGateway exit; a component refused host-side validation is
// `refused` on the BadRequest exit.
#[tokio::test]
async fn load_failures() {
    let mut provider = Provider::idle();
    provider.plugins = provider.plugins.clone().refuse(
        "emery:demo@1.2.0",
        LoadError::Unavailable("resolving `emery:demo@1.2.0`: endpoint unreachable".to_string()),
    );
    fail(&provider, &["emery", "specify", "emery:demo@1.2.0"], 4, "unavailable").await;

    let mut provider = Provider::idle();
    provider.plugins = provider
        .plugins
        .clone()
        .refuse("emery:demo@1.2.0", LoadError::Refused("not a raw wasm component".to_string()));
    fail(&provider, &["emery", "specify", "emery:demo@1.2.0"], 1, "refused").await;
}

// Package references pin an exact SemVer — no branches, tags, or
// namespace-less names.
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

// A current id naming a missing revision is corruption, never an empty
// result.
#[tokio::test]
async fn corrupt_current() {
    let provider = Provider::idle();
    provider.storage.insert_state(CURRENT, b"0123456789abcdef");
    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;
}

// The store is content-addressed: a committed document rewritten under
// its id no longer hashes to it, and `show` refuses rather than render
// bytes the id never named.
#[tokio::test]
async fn tampered_revision() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", "docs"]).await;
    let id = current(&provider.storage);

    provider.storage.insert_object(CONTAINER, &format!("{id}/spec.json"), b"{}\n");

    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;
}

// A stored revision written under another grammar is outdated, not corrupt:
// `show` refuses typed with `spec-outdated`, and the next `specify`
// regenerates over it — no diff, the outdated blobs pruned.
#[tokio::test]
async fn spec_outdated() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
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

// Regeneration is the recovery path: a `specify` over a tampered
// outgoing revision commits, prunes the tampered blobs, and suppresses
// only the advisory diff.
#[tokio::test]
async fn repair_tampered() {
    let second_spec = SPEC_ANSWER.replace("hello", "howdy");
    let second_design = DESIGN_ANSWER.replace("hello", "howdy");
    let provider = Provider::answering([
        SPEC_ANSWER,
        DESIGN_ANSWER,
        second_spec.as_str(),
        second_design.as_str(),
    ]);

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

// The current id is a raw compare-and-swap token: bytes that decode to
// no id fail `show` closed, yet the next `specify` swaps over them, so
// a corrupt store never dead-ends the grammar.
#[tokio::test]
async fn repair_current() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    provider.storage.insert_state(CURRENT, b"\xff\xfe");
    fail(&provider, &["emery", "show", "spec"], 3, "server_error").await;

    cli_ok(&provider, &["emery", "specify", "docs"]).await;

    let id = current(&provider.storage);
    assert!(provider.storage.object(CONTAINER, &format!("{id}/spec.json")).is_some());
    cli_ok(&provider, &["emery", "show", "spec"]).await;
    provider.model.assert_exhausted();
}

// One shared store, two project-scoped views: multi-project isolation
// is host policy over the engine's flat keys, with no engine change
// (portable-storage step 8).
#[tokio::test]
async fn multi_project() {
    let scratch = Scratch::new();
    let component = scratch.component();

    // `Memory` is a shared handle: every clone reads the same store.
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

    // Every write landed under its project prefix; nothing landed flat.
    assert!(shared.state(CURRENT).is_none(), "no unprefixed current id exists");
    assert!(shared.objects(CONTAINER).is_empty(), "no unprefixed revision exists");

    let id_alpha = project_current(&shared, "alpha");
    let id_beta = project_current(&shared, "beta");
    assert_ne!(id_alpha, id_beta, "distinct documents commit distinct revisions");

    // Each project's `show` renders its own committed revision alone.
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

// Asserts that a failure envelope's message carries `fragment`.
fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

// Reads the current revision id from a project's store.
fn current(storage: &Memory) -> String {
    stored_id(storage, CURRENT)
}

// Reads a namespaced project's current revision id from the shared store.
fn project_current(shared: &Memory, project: &str) -> String {
    stored_id(shared, &format!("{project}/{CURRENT}"))
}

// Decodes the revision id `storage` holds under `key`.
fn stored_id(storage: &Memory, key: &str) -> String {
    String::from_utf8(storage.state(key).expect("current")).expect("utf-8 revision id")
}

// Reads a committed revision document from the store.
fn document(storage: &Memory, id: &str, name: &str) -> Vec<u8> {
    storage.object(CONTAINER, &format!("{id}/{name}")).unwrap_or_else(|| panic!("{name}"))
}

// The content id a revision holding `spec` and `design` sits under —
// SHA-256 over the length-prefixed file names and bodies, `spec.json` then
// `design.json`.
fn revision(spec: &[u8], design: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for (name, body) in [("spec.json", spec), ("design.json", design)] {
        hasher.update((name.len() as u64).to_be_bytes());
        hasher.update(name.as_bytes());
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}

// Seeds `storage` with a current revision holding `spec` and `design` as its
// stored documents, returning the content id it sits under.
fn seed(storage: &Memory, spec: &[u8], design: &[u8]) -> String {
    let id = revision(spec, design);
    storage.insert_object(CONTAINER, &format!("{id}/spec.json"), spec);
    storage.insert_object(CONTAINER, &format!("{id}/design.json"), design);
    storage.insert_state(CURRENT, id.as_bytes());
    id
}

// Fills a Markdown fixture's `<revision>` placeholder with the id `show`
// stamps in the front matter.
fn projection(fixture: &str, id: &str) -> String {
    fixture.replace("<revision>", id)
}

// Renders `document` of the current revision through `show`.
async fn shown<S>(provider: &Provider<S>, document: &str) -> String
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let resp = cli_ok(provider, &["emery", "show", document]).await;
    String::from_utf8(resp.stdout).expect("utf-8 document")
}
