//! A verification is one turn whose verdict is held to itself before the
//! adapter answers; every scenario here puts one over a scripted model and a
//! scratch tree.

use emery_sdk::target::{Verdict, VerifyContext};
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
        path: "verify.md",
        body: "VERIFY",
    },
    Doc {
        path: "references/checks.md",
        body: "Run `cargo test`.",
    },
];

const PASSED: &str = r#"{"passed":true,"failures":[]}"#;

async fn ask(model: &Scripted, workspace: &str) -> Result<Verdict, Error> {
    let ctx = VerifyContext {
        adapter_id: "target:probe",
        workspace,
        model,
    };
    emery_sdk::target::verify(&ctx, PROSE).await
}

// The turn carries the verify prompt as its system, lends the integrated
// tree writable, offers `write_files` beside the reference tools, and is
// steered by the verdict schema.
#[tokio::test]
async fn request_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([PASSED]);

    let verdict = ask(&model, root).await.expect("accepted");
    assert!(verdict.passed);
    assert_eq!(verdict.failures, Vec::<String>::new());

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("VERIFY"));
    assert_eq!(request.workspace.as_deref(), Some(root), "the integrated tree is lent");
    assert_eq!(
        request.tools,
        ["list_docs", "read_doc", "write_files"],
        "a verify turn repairs through the build's tool"
    );
    assert!(request.check, "acceptance is the check");
    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("the verdict is steered by schema");
    };
    assert_eq!(name, "verify");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("schema parses");
    assert!(schema.pointer("/properties/passed").is_some(), "{schema}");
    assert!(schema.pointer("/properties/failures").is_some(), "{schema}");

    let user = &request.messages[0];
    assert!(
        user.starts_with(
            "Verify the integrated project tree, bound to adapter `target:probe`.\n\n\
             `$WORKSPACE` is the tree every slice of the wave has merged into, lent writable \
             with the shell"
        ),
        "{user}"
    );
    assert!(
        user.contains(
            "Where a check fails, repair the tree through this call's `write_files` tool alone"
        ),
        "{user}"
    );
    assert!(user.contains("then run the checks again"), "{user}");
    assert!(
        user.contains("`failures` names each check that still failed, with the tail of its output"),
        "{user}"
    );
    assert!(!user.contains(root), "the lend carries the root, not the brief: {user}");
    assert!(user.ends_with("rather than what the tree should hold."), "{user}");
    model.assert_exhausted();
}

// A repair lands beneath the lent tree through `write_files`, as a build's
// write does, and the verdict stands as answered: what the repair left in
// the tree is the caller's to seal.
#[tokio::test]
async fn repairs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    std::fs::create_dir_all(tmp.path().join("src")).expect("src dir");
    std::fs::write(tmp.path().join("src/orders.rs"), "pub struct Order\n").expect("seed");
    let model = Scripted::answering([PASSED]).calling(
        0,
        [ToolCall {
            id: "1".to_owned(),
            name: "write_files".to_owned(),
            arguments: r#"{"files":[{"path":"src/orders.rs","content":"pub struct Order;\n"}]}"#
                .to_owned(),
        }],
    );

    let verdict = ask(&model, root).await.expect("accepted");
    assert!(verdict.passed);
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("src/orders.rs")).expect("repaired"),
        "pub struct Order;\n"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "the repair, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":18,"path":"src/orders.rs"}]}"#)
    );
    assert_eq!(exchanges[1].tool, "check");
    assert_eq!(exchanges[1].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A verdict that failed is an answer the gate accepts, since `passed` agrees
// with its failures; what a failed wave means is the caller's.
#[tokio::test]
async fn failed_verdict() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"passed":false,"failures":["cargo test: 2 failed"]}"#]);

    let verdict = ask(&model, root).await.expect("a failed verdict is an answer");
    assert!(!verdict.passed);
    assert_eq!(verdict.failures, ["cargo test: 2 failed"]);
    model.assert_exhausted();
}

// `passed` disagreeing with `failures` is a finding either way, returned for
// correction, and the corrected verdict is accepted.
#[tokio::test]
async fn gate_findings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"passed":true,"failures":["cargo test: 1 failed"]}"#,
        r#"{"passed":false,"failures":[]}"#,
        r#"{"passed":false,"failures":["cargo test: 1 failed"]}"#,
    ]);

    let verdict = ask(&model, root).await.expect("corrected");
    assert!(!verdict.passed);

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "two refused checks, then the accepted one");
    let correction = exchanges[0].outcome.as_ref().expect_err("passed with failures");
    assert!(correction.contains("- `passed` is true, yet 1 failure is listed"), "{correction}");
    let correction = exchanges[1].outcome.as_ref().expect_err("failed with none named");
    assert!(correction.contains("- `passed` is false, yet no failure is listed"), "{correction}");
    assert_eq!(exchanges[2].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// The verify turn offers the adapter's references and the runtime's through
// the reference tools; neither prompt is listed, and a read still answers
// each.
#[tokio::test]
async fn doc_refs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([PASSED]).calling(
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
                arguments: r#"{"path":"verify.md"}"#.to_string(),
            },
        ],
    );

    ask(&model, root).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "two reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"paths":["references/checks.md","reconciliation.md"]}"#),
        "`list_docs` lists neither prompt"
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"VERIFY","path":"verify.md"}"#),
        "`read_doc` still answers an unlisted document"
    );
    assert_eq!(exchanges[2].tool, "check");
    model.assert_exhausted();
}

// A corpus without `verify.md` is the adapter build's own defect, reported
// before a turn is spent.
#[tokio::test]
async fn missing_prompt() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::default();
    let ctx = VerifyContext {
        adapter_id: "target:mute",
        workspace: root,
        model: &model,
    };

    let error = emery_sdk::target::verify(&ctx, &PROSE[..1]).await.expect_err("no prompt");
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`verify.md` is not embedded"), "{error}");
    assert_eq!(model.seen(), [] as [Seen; 0]);
}
