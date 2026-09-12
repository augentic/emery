//! The evidence call
//!
//! What an adapter can rely on from `SourceAdapter::evidence`: the request it
//! builds (the embedded prompt as the system, the SDK-owned turn around the
//! adapter's material, the `Evidence` schema with the claim-id pattern,
//! `check` set, the reference tools, and the workspace lend following the
//! input), reference calls answered from the embedded corpus, a candidate the
//! claim gate rejects corrected in place, the backend's spent rounds surfacing
//! as `bad_request` with the last findings, and a host refusal passing through
//! as `bad_request`.

use emery_prose::registry::Doc;
use emery_sdk::model::{Error as ModelError, ToolCall};
use emery_sdk::{
    Context, Error, Evidence, Material, Model, SourceAdapter, SourceContent, SourceInput,
};
use omnia_test::SeenFormat;
use omnia_test::guest::Scripted;

const DOCS: &[Doc] = &[
    Doc {
        path: "prompts/extract.md",
        body: "SYSTEM",
    },
    Doc {
        path: "references/greeting.md",
        body: "Greet warmly.",
    },
];

const VALID: &str = r#"{"authority":"documentation","claims":[
    {"kind":"requirement","id":"password-reset.request","statement":"Users reset by email."},
    {"kind":"decision"}
]}"#;

struct Probe;

impl SourceAdapter for Probe {
    const SOURCE: &'static str = "probe";

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(model: &P, ctx: &Context<'_>) -> Result<Evidence, Error> {
        Self::evidence(model, ctx, Material::Bound).await
    }
}

fn workspace(root: &str) -> SourceInput {
    SourceInput {
        key: "docs".to_string(),
        content: SourceContent::Workspace(root.to_string()),
    }
}

fn value(text: &str) -> SourceInput {
    SourceInput {
        key: "brief".to_string(),
        content: SourceContent::Value(text.to_string()),
    }
}

async fn ask(model: &Scripted, input: &SourceInput, material: Material) -> Result<Evidence, Error> {
    let ctx = Context {
        adapter_id: "source:probe",
        input,
    };
    Probe::evidence(model, &ctx, material).await
}

// The request carries the embedded prompt, the turn describing the lent
// tree, the derived `Evidence` schema under `evidence` with the claim-id
// grammar as a steering pattern, the reference tools, the lend, and the check
// the backend loops on.
#[tokio::test]
async fn request_shape() {
    let model = Scripted::answering([VALID]);

    let accepted = ask(&model, &workspace("/lend/docs"), Material::Bound)
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
            "Extract the claim set of the probe source bound to adapter `source:probe` (source ",
            "key `docs`).\n\n",
            "`$SOURCE_DIR` is the read-only view at `/lend/docs` — the probe source tree the ",
            "prompt walks. Nothing outside it is reachable; extract mines only this source.\n\n",
            "The prompt's references are available through this call's `read_doc` tool ",
            "(`list_docs` enumerates them); load referenced bodies on demand.\n\n",
            "Answer with one JSON object matching the gated Evidence schema. The caller persists ",
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

// An inline value rides the turn and lends nothing.
#[tokio::test]
async fn inline_value() {
    let model = Scripted::answering([VALID]);

    ask(&model, &value("Ship it."), Material::Bound).await.expect("accepted");
    let request = &model.seen()[0];
    assert!(request.workspace.is_none(), "no lend for an inline value");
    let user = &request.messages[0];
    assert!(user.contains("(source key `brief`)"), "{user}");
    assert!(user.contains("no `$SOURCE_DIR` is lent:\n\nShip it.\n\n"), "{user}");
}

#[tokio::test]
async fn prepared_turn() {
    let model = Scripted::answering([VALID]);

    ask(&model, &value("ignored"), Material::Prepared("PREPARED MATERIAL".to_string()))
        .await
        .expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(user.contains("\n\nPREPARED MATERIAL\n\n"), "{user}");
    assert!(!user.contains("ignored"), "the prepared note replaces the input rendering");
}

// Reference calls are answered in-process from the corpus before the
// candidate is checked.
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
        ],
    );

    ask(&model, &value("Ship it."), Material::Bound).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "two reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"paths":["prompts/extract.md","references/greeting.md"]}"#)
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"Greet warmly.","path":"references/greeting.md"}"#)
    );
    assert_eq!(exchanges[2].tool, "check");
    assert_eq!(exchanges[2].outcome, Ok(String::new()));
}

// A candidate the claim gate rejects — a missing id, a missing extra — is
// sent back as the correction and the next candidate is checked again, so
// the engine never sees the claim it would otherwise refuse.
#[tokio::test]
async fn gate_findings() {
    let model = Scripted::answering([
        r#"{"authority":"documentation","claims":[{"kind":"requirement"}]}"#,
        VALID,
    ]);

    let accepted = ask(&model, &value("Ship it."), Material::Bound)
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
        r#"{"authority":"documentation","claims":[{"kind":"criterion","id":"Not.Valid","criterion":"x"}]}"#,
    ]);

    let error = ask(&model, &value("Ship it."), Material::Bound)
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

    let error =
        ask(&model, &value("Ship it."), Material::Bound).await.expect_err("the host refused");
    assert!(
        matches!(&error, Error::BadRequest { description, .. } if description == "invalid request: no such model"),
        "{error}"
    );
    assert!(model.exchanges().is_empty(), "nothing to check");
}
