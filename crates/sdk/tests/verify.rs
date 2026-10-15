//! A verification is one turn whose verdict is held to itself before the
//! adapter answers; every scenario here puts one over a scripted model and a
//! scratch tree.

use emery_sdk::target::{Verdict, VerifyContext, Worker};
use emery_sdk::{Doc, Error};
use omnia_sdk::model::ToolCall;
use omnia_test::guest::Scripted;
use omnia_test::{Seen, SeenFormat, SeenWorker};

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
    ask_as(model, workspace, &Worker::NONE).await
}

async fn ask_as(model: &Scripted, workspace: &str, worker: &Worker) -> Result<Verdict, Error> {
    ask_checked(model, workspace, worker, |_| Ok(Vec::new())).await
}

async fn ask_checked(
    model: &Scripted, workspace: &str, worker: &Worker,
    check: impl FnMut(&Verdict) -> Result<Vec<String>, Error> + Send,
) -> Result<Verdict, Error> {
    let ctx = VerifyContext {
        adapter_id: "target:probe",
        workspace,
        worker,
        model,
    };
    emery_sdk::target::verify(&ctx, PROSE, check).await
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
    assert_eq!(request.worker, None, "a worker reaching nothing puts no grant");
    assert_eq!(
        request.tools,
        ["list_docs", "read_doc", "write_files", "edit_files"],
        "a verify turn repairs through the build's tools"
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
            "Where a check fails, repair the tree through this call's two tools alone, then run \
             the checks again: `write_files` writes one or more files beneath `$WORKSPACE` per \
             call"
        ),
        "{user}"
    );
    assert!(user.contains("A repair within a file is an `edit_files` call."), "{user}");
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

// A repair within a file lands through `edit_files` as a build's edit does,
// the rest of the file as it was, and a whole-file rewrite that would halve
// the file is refused as a build's is.
#[tokio::test]
async fn repairs_by_edit() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    std::fs::create_dir_all(tmp.path().join("src")).expect("src dir");
    let before = "pub struct Order;\npub fn total(&token: &str) {}\npub struct Line;\n";
    std::fs::write(tmp.path().join("src/orders.rs"), before).expect("seed");
    let model = Scripted::answering([PASSED]).calling(
        0,
        [
            ToolCall {
                id: "1".to_owned(),
                name: "write_files".to_owned(),
                arguments:
                    r#"{"files":[{"path":"src/orders.rs","content":"pub struct Order;\n"}]}"#
                        .to_owned(),
            },
            ToolCall {
                id: "2".to_owned(),
                name: "edit_files".to_owned(),
                arguments:
                    r#"{"edits":[{"path":"src/orders.rs","find":"&token","replace":"token"}]}"#
                        .to_owned(),
            },
        ],
    );

    let verdict = ask(&model, root).await.expect("accepted");
    assert!(verdict.passed);
    assert_eq!(
        std::fs::read_to_string(tmp.path().join("src/orders.rs")).expect("repaired"),
        "pub struct Order;\npub fn total(token: &str) {}\npub struct Line;\n"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "the refused rewrite, the edit, then the check");
    assert_eq!(
        exchanges[0].outcome,
        Err("write_files: nothing written: `src/orders.rs` would shrink from 65 to 18 bytes; a \
             file is replaced whole, so send the whole file, use `edit_files` for a change \
             within it, or set `shrink: true` if that is meant"
            .to_owned())
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"edited":[{"bytes":64,"path":"src/orders.rs"}]}"#)
    );
    assert_eq!(exchanges[2].tool, "check");
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

// The verify turn offers the adapter's references alone through the
// reference tools: neither prompt is listed, a read still answers each, and
// the runtime references a source's turns import are neither listed nor
// read.
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
            ToolCall {
                id: "3".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"claims.md"}"#.to_string(),
            },
        ],
    );

    ask(&model, root).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4, "three reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"paths":["references/checks.md"]}"#),
        "`list_docs` lists the adapter's references alone"
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"VERIFY","path":"verify.md"}"#),
        "`read_doc` still answers an unlisted document"
    );
    assert_eq!(
        exchanges[2].outcome.as_ref().map(String::as_str).map_err(String::as_str),
        Err("no document `claims.md`"),
        "a source's runtime reference is no verify's to read"
    );
    assert_eq!(exchanges[3].tool, "check");
    model.assert_exhausted();
}

// The adapter's worker rides the verify turn as it rides a build's: the
// checks run the same toolchain over the integrated tree.
#[tokio::test]
async fn worker_grant() {
    const WORKER: Worker = Worker {
        egress: &["registry.npmjs.org"],
        read: &["~/.nvm/versions/node"],
        env: &[],
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([PASSED]);

    ask_as(&model, root, &WORKER).await.expect("accepted");

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].workspace.as_deref(), Some(root));
    assert_eq!(
        seen[0].worker,
        Some(SeenWorker {
            egress: vec!["registry.npmjs.org".to_owned()],
            read: vec!["~/.nvm/versions/node".to_owned()],
            env: vec![],
        })
    );
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
        worker: &Worker::NONE,
        model: &model,
    };

    let error = emery_sdk::target::verify(&ctx, &PROSE[..1], |_| Ok(Vec::new()))
        .await
        .expect_err("no prompt");
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`verify.md` is not embedded"), "{error}");
    assert_eq!(model.seen(), [] as [Seen; 0]);
}

// The adapter's check reads the tree as the repairs left it: a passing
// verdict over a tree that breaks the adapter's rule is refused with the
// finding, and the candidate after the repair is accepted.
#[tokio::test]
async fn check_findings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    std::fs::write(tmp.path().join("Cargo.toml"), "[workspace]\nmembers = []\n").expect("seed");
    let model = Scripted::answering([PASSED, PASSED]).calling(
        1,
        [ToolCall {
            id: "1".to_owned(),
            name: "edit_files".to_owned(),
            arguments: r#"{"edits":[{"path":"Cargo.toml","find":"members = []","replace":"members = [\"crates/*\"]"}]}"#
                .to_owned(),
        }],
    );
    let mut seen = 0;

    let verdict = ask_checked(&model, root, &Worker::NONE, |verdict| {
        seen += 1;
        let manifest = std::fs::read_to_string(tmp.path().join("Cargo.toml"))
            .map_err(|err| omnia_sdk::server_error!("reading `Cargo.toml`: {err}"))?;
        let drifted = verdict.passed && !manifest.contains("crates/*");
        Ok(drifted
            .then(|| "- `Cargo.toml` lacks the members list; put it back".to_owned())
            .into_iter()
            .collect())
    })
    .await
    .expect("corrected");

    assert!(verdict.passed);
    assert_eq!(seen, 2, "the check saw each candidate");
    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the first verdict is refused");
    assert!(
        correction.contains("- `Cargo.toml` lacks the members list; put it back"),
        "{correction}"
    );
    assert_eq!(exchanges[1].tool, "edit_files", "the repair follows the correction");
    assert_eq!(exchanges[2].outcome, Ok(String::new()), "the repaired tree passes");
    model.assert_exhausted();
}

// A check that fails ends the turn with the adapter's error, one candidate
// seen and no correction put.
#[tokio::test]
async fn check_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([PASSED]);

    let error = ask_checked(&model, root, &Worker::NONE, |_| {
        Err(omnia_sdk::bad_request!("a directory stands at `Cargo.toml`"))
    })
    .await
    .expect_err("the check's error is the call's");

    assert_eq!(error.code(), "bad_request", "{error}");
    assert!(error.to_string().contains("a directory stands at"), "{error}");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 1, "one candidate, no correction: {exchanges:?}");
    assert!(exchanges[0].outcome.is_ok(), "the candidate ends the turn: {exchanges:?}");
    model.assert_exhausted();
}
