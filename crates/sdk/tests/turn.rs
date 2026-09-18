//! Verifies the model exchange used to mine one seam.
//!
//! The scenarios cover the system prompt, seam description, evidence schema,
//! claim check, reference tools, and workspace grant. They also verify
//! pre-request validation, correction rounds, exhausted budgets, and model
//! error classification.
//!
//! Every call here mines one seam, so each is one turn and its outcome passes
//! through unchanged; the fan-out and join over several are `extract.rs`'s.

use emery_sdk::model::{Error as ModelError, ToolCall};
use emery_sdk::{Context, Doc, Error, Evidence, Seam, SourceInput};
use omnia_test::SeenFormat;
use omnia_test::guest::Scripted;

const PROSE: &[Doc] = &[
    Doc {
        path: "extract.md",
        body: "SYSTEM",
    },
    Doc {
        path: "references/greeting.md",
        body: "Greet warmly.",
    },
];

const VALID: &str = r#"{"claims":[
    {"kind":"requirement","id":"password-reset.request","statement":"Users reset by email."},
    {"kind":"decision"}
]}"#;

fn files<const N: usize>(paths: [&str; N]) -> Seam {
    Seam::Files(paths.into_iter().map(str::to_string).collect())
}

async fn ask(model: &Scripted, input: &SourceInput, seam: Seam) -> Result<Evidence, Error> {
    let ctx = Context {
        adapter_id: "source:probe",
        input,
        model,
    };
    emery_sdk::extract(&ctx, PROSE, &[seam]).await
}

// The request carries the embedded prompt, the turn describing the lent
// tree, the derived `Evidence` schema under `evidence` with the claim-id
// grammar as a steering pattern, the reference tools, the lend, and the check
// the backend loops on.
#[tokio::test]
async fn request_shape() {
    let model = Scripted::answering([VALID]);

    let accepted = ask(&model, &SourceInput::workspace("docs", "/lend/docs"), Seam::Whole)
        .await
        .expect("a valid answer is accepted first time");
    assert_eq!(accepted.claims.len(), 2);

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("SYSTEM"));
    assert_eq!(
        request.messages,
        [concat!(
            "Extract the claim set of the source bound to adapter `source:probe` (source key ",
            "`docs`).\n\n",
            "`$SOURCE_DIR` is the read-only view at `/lend/docs` — the source tree the prompt ",
            "walks. Nothing outside it is reachable; extract mines only this source.\n\n",
            "The prompt's references are available through this call's `read_doc` tool ",
            "(`list_docs` enumerates them); load referenced bodies on demand.\n\n",
            "Answer with one JSON object matching the gated claims schema. The caller persists ",
            "the document; do not write it yourself."
        )]
    );
    assert!(request.check, "acceptance is the check, not the reply text");
    assert_eq!(request.tools, ["list_docs", "read_doc"], "the corpus is offered through tools");
    assert_eq!(request.workspace.as_deref(), Some("/lend/docs"), "the lend follows the input");

    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("evidence is steered by schema");
    };
    assert_eq!(name, "evidence");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("generated schema parses");
    assert!(schema.pointer("/properties/kind").is_none(), "the answer is claims alone: {schema}");
    let claim = schema.pointer("/$defs/Claim").expect("Claim definition");
    assert_eq!(
        claim.pointer("/properties/id/pattern").and_then(serde_json::Value::as_str),
        Some("^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$")
    );
    assert!(claim.pointer("/properties/backing").is_some(), "schema tracks the DTO");
    assert_ne!(
        claim.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false)),
        "open claim extras stay admitted"
    );
    model.assert_exhausted();
}

// A corpus without `extract.md` is the adapter build's own defect, reported
// before a model call is spent.
#[tokio::test]
async fn missing_prompt() {
    let model = Scripted::default();
    let input = SourceInput::workspace("docs", ".");
    let ctx = Context {
        adapter_id: "source:mute",
        input: &input,
        model: &model,
    };

    let error =
        emery_sdk::extract(&ctx, &[], &[Seam::Whole]).await.expect_err("no prompt to ask with");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`extract.md` is not embedded"), "{error}");
    assert!(model.seen().is_empty(), "nothing was asked");
}

// An inline value rides the turn and lends nothing.
#[tokio::test]
async fn inline_value() {
    let model = Scripted::answering([VALID]);

    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole).await.expect("accepted");
    let request = &model.seen()[0];
    assert!(request.workspace.is_none(), "no lend for an inline value");
    let user = &request.messages[0];
    assert!(user.contains("(source key `brief`)"), "{user}");
    assert!(user.contains("no `$SOURCE_DIR` is lent:\n\nShip it.\n\n"), "{user}");
}

// A `Note` seam stands in the turn where the SDK's rendering of the input
// would be.
#[tokio::test]
async fn prepared_turn() {
    let model = Scripted::answering([VALID]);

    ask(&model, &SourceInput::value("brief", "ignored"), Seam::Note("THE NOTE".to_string()))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(user.contains("\n\nTHE NOTE\n\n"), "{user}");
    assert!(!user.contains("ignored"), "the note replaces the input rendering");
}

// A `Files` seam lends the root — the read-only mount is the boundary — and
// lists the files to mine relative to it, sorted, once each, `.` segments
// dropped, so every anchor the model answers is already root-relative.
#[tokio::test]
async fn files_turn() {
    let model = Scripted::answering([VALID]);
    let seam = files(["guide/setup.md", "./guide/intro.md", "guide/intro.md", "api.md"]);

    ask(&model, &SourceInput::workspace("docs", "/lend/docs"), seam).await.expect("accepted");

    let request = &model.seen()[0];
    assert_eq!(request.workspace.as_deref(), Some("/lend/docs"), "the root is lent");
    let user = &request.messages[0];
    assert!(
        user.contains(
            "`$SOURCE_DIR` is the read-only view at `/lend/docs` — the source tree. Mine these \
             files beneath it and nothing else:\n\n\
             - `api.md`\n- `guide/intro.md`\n- `guide/setup.md`\n\n\
             Anchor every `path` relative to `$SOURCE_DIR`."
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// Reference calls are answered in-process before the candidate is checked:
// from the adapter's corpus, then from the SDK's runtime references, which
// the adapter never lists.
#[tokio::test]
async fn doc_refs() {
    let model = Scripted::answering([VALID]).calling(
        0,
        [
            ToolCall {
                id: "1".to_string(),
                name: "list_docs".to_string(),
                arguments: "{}".to_string(),
            },
            ToolCall {
                id: "2".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"references/greeting.md"}"#.to_string(),
            },
            ToolCall {
                id: "3".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"claims.md"}"#.to_string(),
            },
        ],
    );

    ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4, "three reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(concat!(
            r#"{"paths":["extract.md","references/greeting.md","#,
            r#""claims.md","reconciliation.md"]}"#
        ))
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"Greet warmly.","path":"references/greeting.md"}"#)
    );
    let runtime: serde_json::Value =
        serde_json::from_str(exchanges[2].outcome.as_deref().expect("the runtime table answers"))
            .expect("a JSON answer");
    assert_eq!(runtime["path"], "claims.md");
    assert_eq!(
        runtime["body"],
        emery_sdk::body(emery_sdk::RUNTIME, "claims.md").expect("embedded")
    );
    assert_eq!(exchanges[3].tool, "check");
    assert_eq!(exchanges[3].outcome, Ok(String::new()));
}

// A candidate the claim gate rejects — a missing id, a missing extra — is
// sent back as the correction and the next candidate is checked again, so
// the engine never sees the claim it would otherwise refuse.
#[tokio::test]
async fn gate_findings() {
    let model = Scripted::answering([r#"{"claims":[{"kind":"requirement"}]}"#, VALID]);

    let accepted = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole)
        .await
        .expect("the second candidate passes the gate");
    assert_eq!(accepted.claims.len(), 2);

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "one rejection, one acceptance");
    assert_eq!(exchanges[0].tool, "check");
    let correction = exchanges[0].outcome.as_ref().expect_err("the first candidate is rejected");
    assert!(correction.contains("## Previous answer (rejected)"), "{correction}");
    assert!(correction.contains("## Findings"), "{correction}");
    assert!(correction.contains("`requirement` claims require an id"), "{correction}");
    assert!(correction.contains("missing extra `statement`"), "{correction}");
    assert_eq!(exchanges[1].outcome, Ok(String::new()));
    assert_eq!(model.requests().len(), 2, "one scripted answer per attempt");
}

// When the backend spends its rounds on a rejected candidate the last
// findings surface as `bad_request`, so the host's own error is never the
// adapter's answer.
#[tokio::test]
async fn rounds_exhausted() {
    let model = Scripted::answering([
        r#"{"claims":[{"kind":"criterion","id":"Not.Valid","criterion":"x"}]}"#,
    ]);

    let error = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole)
        .await
        .expect_err("the only candidate fails the gate");
    let Error::BadRequest { code, description } = error else {
        panic!("spent rounds are a bad request: {error}");
    };
    assert_eq!(code, "bad_request");
    assert!(description.contains("budget exhausted"), "{description}");
    assert!(description.contains("id `Not.Valid` does not match"), "{description}");
    assert_eq!(model.exchanges().len(), 1, "one check, rejected");
}

// A request the host refuses is a `bad_request` carrying the host's reason.
#[tokio::test]
async fn invalid_request() {
    let model = Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]);

    let error = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole)
        .await
        .expect_err("the host refused");
    assert!(
        matches!(&error, Error::BadRequest { description, .. } if description == "invalid request: no such model"),
        "{error}"
    );
    assert!(model.exchanges().is_empty(), "nothing to check");
}

// A stray `kind` key is a schema miss: the answer is claims alone, and the
// kind of source is the adapter's metadata, never the model's to state.
#[tokio::test]
async fn stray_kind() {
    let model = Scripted::answering([r#"{"kind":"intent","claims":[{"kind":"decision"}]}"#, VALID]);

    let accepted = ask(&model, &SourceInput::value("brief", "Ship it."), Seam::Whole)
        .await
        .expect("the second candidate is claims-only");
    assert_eq!(accepted.claims.len(), 2);

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the stray key is refused");
    assert!(
        correction.contains("schema") || correction.contains("unknown field"),
        "a stray kind key is a schema miss: {correction}"
    );
    model.assert_exhausted();
}
