//! A build is one turn whose report is held to the slice before the adapter
//! answers; every scenario here puts one over a scripted model.

use emery_sdk::target::{Context, Report, Slice};
use emery_sdk::{Doc, Error};
use omnia_sdk::model::ToolCall;
use omnia_test::guest::Scripted;
use omnia_test::{Seen, SeenFormat};

const PROSE: &[Doc] = &[
    Doc {
        path: "build.md",
        body: "BUILD",
    },
    Doc {
        path: "references/layout.md",
        body: "One module per requirement.",
    },
];

const VALID: &str = r#"{"covered":["REQ-001","REQ-002"],"written":["src/orders.rs","Cargo.toml"]}"#;

fn slice() -> Slice {
    Slice {
        id: "SLICE-001".to_owned(),
        name: "orders".to_owned(),
        requirements: vec!["REQ-001".to_owned(), "REQ-002".to_owned()],
        spec: "## REQ-001\n\nOrders are created.\n\n## REQ-002\n\nOrders are listed.\n".to_owned(),
        design: "## Types\n\n`Order`\n".to_owned(),
        plan: "## SLICE-001 orders\n\nBuild the orders module.\n".to_owned(),
    }
}

async fn ask(model: &Scripted, slice: &Slice, workspace: &str) -> Result<Report, Error> {
    let ctx = Context {
        adapter_id: "target:probe",
        slice,
        workspace,
        model,
    };
    emery_sdk::target::build(&ctx, PROSE).await
}

// The turn carries the slice's three documents and the lend, writable; the
// answer is steered by the report schema.
#[tokio::test]
async fn request_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    let slice = slice();

    let report = ask(&model, &slice, root).await.expect("accepted");
    assert_eq!(report.covered, ["REQ-001", "REQ-002"]);
    assert_eq!(report.written, ["src/orders.rs", "Cargo.toml"]);

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("BUILD"));
    assert_eq!(request.workspace.as_deref(), Some(root), "the tree is lent");
    assert_eq!(request.tools, ["list_docs", "read_doc"]);
    assert!(request.check, "acceptance is the check");
    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("the report is steered by schema");
    };
    assert_eq!(name, "build-SLICE-001");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("schema parses");
    assert!(schema.pointer("/properties/covered").is_some(), "{schema}");
    assert!(schema.pointer("/properties/written").is_some(), "{schema}");

    let user = &request.messages[0];
    assert!(
        user.starts_with(
            "Build the slice `orders` (SLICE-001) of the plan, bound to adapter \
             `target:probe`.\n\n`$WORKSPACE` is the project tree, lent writable"
        ),
        "{user}"
    );
    assert!(
        user.contains(
            "The slice's entry in the plan:\n\n## SLICE-001 orders\n\nBuild the orders \
             module.\n\nThe specification, cut to the slice's requirements:\n\n## \
             REQ-001\n\nOrders are created.\n\n## REQ-002\n\nOrders are listed.\n\nThe design, \
             whole:\n\n## Types\n\n`Order`\n\n"
        ),
        "{user}"
    );
    assert!(user.contains("— `REQ-001`, `REQ-002` — and no other."), "{user}");
    assert!(!user.contains(root), "the lend carries the root, not the brief: {user}");
    assert!(user.ends_with("rather than claim what the tree does not hold."), "{user}");
    model.assert_exhausted();
}

// The build turn offers the adapter's references and the runtime's through
// the same tools a mining turn has; no system document is listed — `build.md`
// rides this turn's system, and the claim rules teach a build nothing — and a
// read still answers each.
#[tokio::test]
async fn doc_refs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
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
                arguments: r#"{"path":"references/layout.md"}"#.to_string(),
            },
            ToolCall {
                id: "3".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"build.md"}"#.to_string(),
            },
        ],
    );

    ask(&model, &slice(), root).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4, "three reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"paths":["references/layout.md","reconciliation.md"]}"#),
        "`list_docs` lists no system document"
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"One module per requirement.","path":"references/layout.md"}"#)
    );
    assert_eq!(
        exchanges[2].outcome.as_deref(),
        Ok(r#"{"body":"BUILD","path":"build.md"}"#),
        "`read_doc` still answers an unlisted document"
    );
    assert_eq!(exchanges[3].tool, "check");
    assert_eq!(exchanges[3].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A corpus without `build.md` is the adapter build's own defect.
#[tokio::test]
async fn missing_prompt() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::default();
    let slice = slice();
    let ctx = Context {
        adapter_id: "target:mute",
        slice: &slice,
        workspace: root,
        model: &model,
    };

    let error = emery_sdk::target::build(&ctx, &[]).await.expect_err("no prompt to ask with");
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`build.md` is not embedded"), "{error}");
    assert_eq!(model.seen(), [] as [Seen; 0]);
}

// Every rule the slice can hold the report to is a finding, all returned
// together, and the corrected report is accepted; a requirement left
// uncovered is no finding, since the gate holds what is claimed, not what is
// missing.
#[tokio::test]
async fn gate_findings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"covered":["REQ-001","REQ-009","REQ-001"],
            "written":["src/orders.rs","../escape.rs","./","src/orders.rs",".omnia/storage/x"]}"#,
        r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#,
    ]);
    let slice = slice();

    let report = ask(&model, &slice, root).await.expect("corrected");
    assert_eq!(report.covered, ["REQ-001"]);
    assert_eq!(report.uncovered(&slice), ["REQ-002"]);

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the first report is refused");
    for finding in [
        "- covered `REQ-009` is not a requirement of slice `SLICE-001`; its requirements are \
         REQ-001, REQ-002",
        "- covered `REQ-001` is listed twice",
        "- written `../escape.rs` escapes the root",
        "- written `./` names no file",
        "- written `src/orders.rs` is listed twice",
        "- written `.omnia/storage/x` is under the engine's own `.omnia/`",
    ] {
        assert!(correction.contains(finding), "{finding}: {correction}");
    }
    assert!(exchanges[1].outcome.is_ok(), "the corrected report is accepted");
    model.assert_exhausted();
}

// A report that never lands within the rounds is the operator's `bad_request`,
// as a mining turn's is.
#[tokio::test]
async fn rounds_exhausted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-404"],"written":[]}"#]);

    let error = ask(&model, &slice(), root).await.expect_err("never accepted");
    assert_eq!(error.code(), "bad_request", "{error}");
    model.assert_exhausted();
}
