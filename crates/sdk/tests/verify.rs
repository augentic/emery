//! Tests `target::verify` verdict gating and workspace repairs.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use emery_sdk::target::Grant;
use omnia_test::host::{Scratch, ScriptedModel, scratch};
use omnia_test::{Seen, SeenFormat, SeenWorker};
use support::harness::{DriverRun, verify};
use test_programs::protocol::{VERIFY_WORKSPACE, VerifyCheck, VerifyOutcome, VerifyScenario};

const PASSED: &str = r#"{"passed":true,"failures":[]}"#;

async fn ask(model: &ScriptedModel, project: &Scratch) -> DriverRun<VerifyOutcome> {
    verify(project, &VerifyScenario::default(), model.clone()).await
}

async fn ask_as(
    model: &ScriptedModel, project: &Scratch, grants: impl Into<Vec<Grant>>,
) -> DriverRun<VerifyOutcome> {
    verify(project, &VerifyScenario::default().with_grants(grants), model.clone()).await
}

async fn ask_checked(
    model: &ScriptedModel, project: &Scratch, check: VerifyCheck,
) -> DriverRun<VerifyOutcome> {
    verify(project, &VerifyScenario::default().with_check(check), model.clone()).await
}

// The turn carries the verify prompt as its system, lends the integrated
// tree writable, offers `write_files` beside the reference tools, and is
// steered by the verdict schema.
#[tokio::test]
async fn request_shape() {
    let project = scratch();
    let model = ScriptedModel::answering([PASSED]);

    let verdict = ask(&model, &project).await.verdict();
    assert!(verdict.passed);
    assert_eq!(verdict.failures, Vec::<String>::new());

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("VERIFY"));
    assert_eq!(request.workspace.as_deref(), Some(""), "the integrated tree is lent");
    assert_eq!(model.lent(), [Some(project.path().to_path_buf())], "the integrated tree is lent");
    assert_eq!(request.worker, None, "no grants put no worker grant");
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
    let host_root = project.path().to_str().expect("UTF-8 scratch root");
    assert!(
        !user.contains(VERIFY_WORKSPACE) && !user.contains(host_root),
        "the lend carries the root, not the brief: {user}"
    );
    assert!(user.ends_with("rather than what the tree should hold."), "{user}");
    model.assert_exhausted();
}

// A repair lands beneath the lent tree through `write_files`, as a build's
// write does, and the verdict stands as answered: what the repair left in
// the tree is the caller's to seal.
#[tokio::test]
async fn repair_by_write() {
    let project = scratch();
    project.write("src/orders.rs", "pub struct Order\n");
    let model = ScriptedModel::answering([PASSED]).calling(
        0,
        [(
            "write_files",
            r#"{"files":[{"path":"src/orders.rs","content":"pub struct Order;\n"}]}"#,
        )],
    );

    let verdict = ask(&model, &project).await.verdict();
    assert!(verdict.passed);
    assert_eq!(
        std::fs::read_to_string(project.path().join("src/orders.rs")).expect("repaired"),
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
async fn repair_by_edit() {
    let project = scratch();
    let before = "pub struct Order;\npub fn total(&token: &str) {}\npub struct Line;\n";
    project.write("src/orders.rs", before);
    let model = ScriptedModel::answering([PASSED]).calling(
        0,
        [
            (
                "write_files",
                r#"{"files":[{"path":"src/orders.rs","content":"pub struct Order;\n"}]}"#,
            ),
            (
                "edit_files",
                r#"{"edits":[{"path":"src/orders.rs","find":"&token","replace":"token"}]}"#,
            ),
        ],
    );

    let verdict = ask(&model, &project).await.verdict();
    assert!(verdict.passed);
    assert_eq!(
        std::fs::read_to_string(project.path().join("src/orders.rs")).expect("repaired"),
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
    let project = scratch();
    let model =
        ScriptedModel::answering([r#"{"passed":false,"failures":["cargo test: 2 failed"]}"#]);

    let verdict = ask(&model, &project).await.verdict();
    assert!(!verdict.passed);
    assert_eq!(verdict.failures, ["cargo test: 2 failed"]);
    model.assert_exhausted();
}

// `passed` disagreeing with `failures` is a finding either way, returned for
// correction, and the corrected verdict is accepted.
#[tokio::test]
async fn gate_findings() {
    let project = scratch();
    let model = ScriptedModel::answering([
        r#"{"passed":true,"failures":["cargo test: 1 failed"]}"#,
        r#"{"passed":false,"failures":[]}"#,
        r#"{"passed":false,"failures":["cargo test: 1 failed"]}"#,
    ]);

    let verdict = ask(&model, &project).await.verdict();
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
    let project = scratch();
    let model = ScriptedModel::answering([PASSED]).calling(
        0,
        [
            ("list_docs", "{}"),
            ("read_doc", r#"{"path":"verify.md"}"#),
            ("read_doc", r#"{"path":"claims.md"}"#),
        ],
    );

    ask(&model, &project).await.verdict();
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4, "three reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"paths":["references/checks.md"]}"#),
        "`list_docs` lists the adapter's references alone"
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"path":"verify.md","body":"VERIFY"}"#),
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

// The adapter's grants ride the verify turn as they ride a build's: the
// checks run the same toolchain over the integrated tree.
#[tokio::test]
async fn worker_grant() {
    let grants =
        [Grant::new().egress(["registry.npmjs.org"]), Grant::new().read(["~/.nvm/versions/node"])];
    let project = scratch();
    let model = ScriptedModel::answering([PASSED]);

    ask_as(&model, &project, grants).await.verdict();

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].workspace.as_deref(), Some(""));
    assert_eq!(model.lent(), [Some(project.path().to_path_buf())]);
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
    let project = scratch();
    let model = ScriptedModel::default();
    let scenario = VerifyScenario {
        adapter_id: "target:mute".to_owned(),
        omit_verify_prompt: true,
        ..VerifyScenario::default()
    };

    let error = verify(&project, &scenario, model.clone()).await.error();
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`verify.md` is not embedded"), "{error}");
    assert_eq!(model.seen(), [] as [Seen; 0]);
}

// A grant that breaks the shape is the adapter build's own defect, every
// broken entry named and the call ended before a turn is spent, so a grant
// the host would refuse never reaches it.
#[tokio::test]
async fn invalid_worker_grant() {
    let grants = [
        Grant::new()
            .egress(["https://registry.npmjs.org", "registry.npmjs.org"])
            .read(["/opt/node", "~/", "~/a/../b", "~/.volta"])
            .env([("VOLTA-HOME", "~/.volta"), ("NODE_ENV", "test")]),
        Grant::new().env([("NODE_ENV", "production")]),
    ];
    let project = scratch();
    let model = ScriptedModel::default();

    let error = ask_as(&model, &project, grants).await.error();

    assert_eq!(error.code(), "server_error");
    let description = error.description();
    for finding in [
        "egress `https://registry.npmjs.org` is not a host name",
        "read root `/opt/node` is not `~/<path>`",
        "read root `~/` is not `~/<path>`",
        "read root `~/a/../b` is not `~/<path>`",
        "variable `VOLTA-HOME` is not a name",
        "variable `NODE_ENV` is set to `test` and to `production`",
    ] {
        assert!(description.contains(finding), "{description}");
    }
    assert!(!description.contains("`registry.npmjs.org` is not"), "{description}");
    assert!(!description.contains("`~/.volta` is not"), "{description}");
    assert_eq!(model.seen(), [] as [Seen; 0]);
}

// A grant of empty lists grants nothing, so a list of it puts no worker
// grant, as no grants put none.
#[tokio::test]
async fn empty_worker_grant() {
    let grants = [Grant::new()];
    let project = scratch();
    let model = ScriptedModel::answering([PASSED]);

    ask_as(&model, &project, grants).await.verdict();

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].worker, None);
    model.assert_exhausted();
}

// The adapter's check reads the tree as the repairs left it: a passing
// verdict over a tree that breaks the adapter's rule is refused with the
// finding, and the candidate after the repair is accepted.
#[tokio::test]
async fn check_findings() {
    let project = scratch();
    project.write("Cargo.toml", "[workspace]\nmembers = []\n");
    let model = ScriptedModel::answering([PASSED, PASSED]).calling(
        1,
        [(
            "edit_files",
            r#"{"edits":[{"path":"Cargo.toml","find":"members = []","replace":"members = [\"crates/*\"]"}]}"#,
        )],
    );

    let outcome = ask_checked(&model, &project, VerifyCheck::MembersList).await.outcome();

    assert!(outcome.verdict.passed);
    assert_eq!(outcome.checks, 2, "the check saw each candidate");
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
    let project = scratch();
    let model = ScriptedModel::answering([PASSED]);

    let error = ask_checked(&model, &project, VerifyCheck::DirectoryAtManifest).await.error();

    assert_eq!(error.code(), "bad_request", "{error}");
    assert!(error.to_string().contains("a directory stands at"), "{error}");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 1, "one candidate, no correction: {exchanges:?}");
    assert_eq!(exchanges[0].outcome, Ok(String::new()), "the candidate ends the turn");
    model.assert_exhausted();
}
