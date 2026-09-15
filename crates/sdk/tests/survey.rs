//! Asserts what the survey helpers decide for a tree adapter.
//!
//! `files` and `by_directory` spare the adapter the walk: skip roots, the
//! keep filter, the grain floor, and that a symlink is not a file to mine.
//! `by_model` is the one survey call an adapter may make:
//!
//! - the request it builds: the embedded survey prompt as the system, the
//!   root lent, the candidate files listed, the `survey` schema;
//! - the fold of its accepted partition under the floor with every
//!   unassigned file;
//! - the corrections a partition earns;
//! - the backend's spent rounds as `bad_request`;
//! - the refusals that spend no turn.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use emery_prose::registry::Doc;
use emery_sdk::survey::{self, Entry};
use emery_sdk::{Context, Error, SourceContent, SourceInput};
use omnia_test::SeenFormat;
use omnia_test::guest::Scripted;

const DOCS: &[Doc] = &[
    Doc {
        path: "prompts/extract.md",
        body: "EXTRACT",
    },
    Doc {
        path: "prompts/survey.md",
        body: "SURVEY",
    },
];

// A corpus without a survey prompt.
const MUTE: &[Doc] = &[Doc {
    path: "prompts/extract.md",
    body: "EXTRACT",
}];

const FILES: &[&str] = &[
    "index.ts",
    "jobs/nightly.ts",
    "routes/orders.ts",
    "routes/users.ts",
    "services/orders.ts",
    "services/users.ts",
];

fn workspace(root: &str) -> SourceInput {
    SourceInput {
        key: "code".to_string(),
        content: SourceContent::Workspace(root.to_string()),
    }
}

async fn survey(
    model: &Scripted, docs: &'static [Doc], input: &SourceInput, floor: usize,
) -> Result<Vec<Vec<String>>, Error> {
    let ctx = Context {
        adapter_id: "source:probe",
        input,
    };
    survey::by_model(model, &ctx, docs, &owned(FILES), floor).await
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(path, body).expect("write");
}

fn owned(files: &[&str]) -> Vec<String> {
    files.iter().copied().map(str::to_string).collect()
}

// Every regular file beneath the root is listed relative to it, sorted, with
// `/` separators — the path space a claim's `path` anchor cites.
#[test]
fn lists_relative() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "b.md", "");
    write(tmp.path(), "a/y.md", "");
    write(tmp.path(), "a/x.md", "");

    let found = survey::files(tmp.path(), |_, _| true).expect("walk");

    assert_eq!(found, ["a/x.md", "a/y.md", "b.md"]);
}

// The engine's own files are never offered, wherever they sit: a projection
// of the last revision is output, not a source to mine.
#[test]
fn skip_roots() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "readme.md", "");
    write(tmp.path(), "spec.md", "");
    write(tmp.path(), "design.md", "");
    write(tmp.path(), ".omnia/store.json", "");
    write(tmp.path(), "nested/spec.md", "");
    write(tmp.path(), "nested/design.md", "");
    write(tmp.path(), "nested/.omnia/x", "");
    write(tmp.path(), "nested/keep.md", "");

    let found = survey::files(tmp.path(), |_, _| true).expect("walk");

    assert_eq!(found, ["nested/keep.md", "readme.md"]);
}

// A refused directory is not entered; a refused file is omitted. Every other
// entry is the adapter's.
#[test]
fn keep_filter() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "keep.md", "");
    write(tmp.path(), "skip.lock", "");
    write(tmp.path(), "vendor/lib.md", "");
    write(tmp.path(), "src/main.rs", "");

    let found = survey::files(tmp.path(), |path, kind| match kind {
        Entry::Dir => path != Path::new("vendor"),
        Entry::File => path.extension().is_none_or(|ext| ext != "lock"),
    })
    .expect("walk");

    assert_eq!(found, ["keep.md", "src/main.rs"]);
}

// A symlink is not a regular file or a directory to enter, so a link at the
// root — even to a real file beside it — is not listed.
#[test]
fn skips_symlinks() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "real.md", "");
    symlink(tmp.path().join("real.md"), tmp.path().join("link.md")).expect("symlink");

    let found = survey::files(tmp.path(), |_, _| true).expect("walk");

    assert_eq!(found, ["real.md"]);
}

// A root the walk cannot open is the adapter host's defect, not the
// operator's input.
#[test]
fn missing_root() {
    let error =
        survey::files(Path::new("/no/such/emery-survey-root"), |_, _| true).expect_err("missing");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("reading"), "{error}");
}

// Directories holding at least `floor` files are their own group, in
// directory-name order; smaller directories and the root's own files fold
// into one sorted remainder.
#[test]
fn grain_floor() {
    assert_eq!(
        survey::by_directory(owned(&["a/1.md", "a/2.md", "b/1.md", "root.md"]), 2),
        [owned(&["a/1.md", "a/2.md"]), owned(&["b/1.md", "root.md"])]
    );
}

// When every directory meets the floor, the remainder is only the root's
// own files; when there are none, it is dropped.
#[test]
fn no_empty_remainder() {
    assert_eq!(
        survey::by_directory(owned(&["a/1.md", "a/2.md", "b/1.md"]), 1),
        [owned(&["a/1.md", "a/2.md"]), owned(&["b/1.md"])]
    );
}

// The survey request carries the embedded survey prompt as the system, the
// root lent whole so the model can read what it groups, every candidate
// file listed as the model must name it, the reference tools, `check` set,
// and the `Partition` schema under `survey`.
#[tokio::test]
async fn model_request() {
    let model = Scripted::answering([
        r#"{"groups":[{"name":"orders","files":["routes/orders.ts","services/orders.ts"]}]}"#,
    ]);

    survey(&model, DOCS, &workspace("/lend/code"), 2).await.expect("accepted");

    let seen = model.seen();
    assert_eq!(seen.len(), 1, "one survey turn");
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("SURVEY"));
    assert_eq!(request.workspace.as_deref(), Some("/lend/code"), "the root is lent");
    assert!(request.check, "acceptance is the check");
    assert_eq!(request.tools, ["list_docs", "read_doc"], "the corpus is offered through tools");
    let turn = &request.messages[0];
    assert!(turn.contains("adapter `source:probe` (source key `code`)"), "{turn}");
    assert!(turn.contains("read-only view at `/lend/code`"), "{turn}");
    for file in FILES {
        assert!(turn.contains(&format!("\n- `{file}`")), "`{file}` is offered: {turn}");
    }
    assert!(turn.contains("fewer than 2 files"), "the floor is stated: {turn}");
    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("the survey is steered by schema");
    };
    assert_eq!(name, "survey");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("generated schema parses");
    assert!(schema.pointer("/properties/groups").is_some(), "{schema}");
    let group = schema.pointer("/$defs/Group").expect("Group definition");
    assert!(group.pointer("/properties/name").is_some(), "{group}");
    assert!(group.pointer("/properties/files").is_some(), "{group}");
    model.assert_exhausted();
}

// The accepted groups come back in answer order, each sorted; a group under
// the floor folds, with every file the model left out, into one sorted
// remainder last — so the whole tree is mined however the model grouped it.
#[tokio::test]
async fn model_fold() {
    let model = Scripted::answering([r#"{"groups":[
            {"name":"users","files":["services/users.ts","routes/users.ts"]},
            {"name":"nightly","files":["jobs/nightly.ts"]},
            {"name":"orders","files":["routes/orders.ts","services/orders.ts"]}
        ]}"#]);

    let groups = survey(&model, DOCS, &workspace("/lend/code"), 2).await.expect("accepted");

    assert_eq!(
        groups,
        [
            owned(&["routes/users.ts", "services/users.ts"]),
            owned(&["routes/orders.ts", "services/orders.ts"]),
            owned(&["index.ts", "jobs/nightly.ts"]),
        ]
    );
    model.assert_exhausted();
}

// A partition that names every file in groups meeting the floor has no
// remainder; a floor of one keeps every group.
#[tokio::test]
async fn model_no_remainder() {
    let model = Scripted::answering([r#"{"groups":[
            {"name":"orders","files":["routes/orders.ts","services/orders.ts"]},
            {"name":"users","files":["routes/users.ts","services/users.ts"]},
            {"name":"nightly","files":["jobs/nightly.ts"]},
            {"name":"entry","files":["index.ts"]}
        ]}"#]);

    let groups = survey(&model, DOCS, &workspace("/lend/code"), 1).await.expect("accepted");

    assert_eq!(
        groups,
        [
            owned(&["routes/orders.ts", "services/orders.ts"]),
            owned(&["routes/users.ts", "services/users.ts"]),
            owned(&["jobs/nightly.ts"]),
            owned(&["index.ts"]),
        ]
    );
}

// A candidate the check refuses — a file never offered, a file in two
// groups, a group naming none — goes back as findings and the next candidate
// is checked; the model chooses the grouping, never what exists.
#[tokio::test]
async fn model_corrections() {
    let model = Scripted::answering([
        r#"{"groups":[
            {"name":"orders","files":["routes/orders.ts","routes/orders.ts"]},
            {"name":"ghost","files":["routes/ghost.ts"]},
            {"name":"empty","files":[]}
        ]}"#,
        r#"{"groups":[{"name":"orders","files":["routes/orders.ts","services/orders.ts"]}]}"#,
    ]);

    let groups = survey(&model, DOCS, &workspace("/lend/code"), 2)
        .await
        .expect("the second candidate is a partition");

    assert_eq!(
        groups,
        [
            owned(&["routes/orders.ts", "services/orders.ts"]),
            owned(&["index.ts", "jobs/nightly.ts", "routes/users.ts", "services/users.ts"]),
        ]
    );
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "one rejection, one acceptance");
    let correction = exchanges[0].outcome.as_ref().expect_err("the first candidate is rejected");
    assert!(
        correction.contains("`routes/orders.ts` appears in more than one group"),
        "{correction}"
    );
    assert!(
        correction.contains("`routes/ghost.ts` is not among the files offered"),
        "{correction}"
    );
    assert!(correction.contains("group `empty` names no file"), "{correction}");
    assert_eq!(exchanges[1].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A stray key on the answer is a schema miss, corrected like a finding.
#[tokio::test]
async fn model_stray_key() {
    let model = Scripted::answering([
        r#"{"groups":[{"name":"orders","files":["routes/orders.ts"],"reason":"handler"}]}"#,
        r#"{"groups":[{"name":"orders","files":["routes/orders.ts","services/orders.ts"]}]}"#,
    ]);

    survey(&model, DOCS, &workspace("/lend/code"), 2).await.expect("the second candidate parses");

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the stray key is refused");
    assert!(correction.contains("unknown field"), "{correction}");
}

// When the backend spends its rounds on a rejected partition the last
// findings surface as `bad_request`, as an evidence call's do.
#[tokio::test]
async fn model_rounds_exhausted() {
    let model = Scripted::answering([r#"{"groups":[{"name":"ghost","files":["nope.ts"]}]}"#]);

    let error = survey(&model, DOCS, &workspace("/lend/code"), 2)
        .await
        .expect_err("the only candidate names a file never offered");

    let Error::BadRequest { code, description } = error else {
        panic!("spent rounds are a bad request: {error}");
    };
    assert_eq!(code, "bad_request");
    assert!(description.contains("`nope.ts` is not among the files offered"), "{description}");
    assert_eq!(model.exchanges().len(), 1, "one check, rejected");
}

// A corpus without `prompts/survey.md` is the adapter build's own defect,
// reported before a turn is spent.
#[tokio::test]
async fn model_missing_prompt() {
    let model = Scripted::default();

    let error =
        survey(&model, MUTE, &workspace("/lend/code"), 2).await.expect_err("no prompt to ask with");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`prompts/survey.md` is not embedded"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// An inline value has no tree to survey: the adapter's own defect, as a
// `Within` material over a value is.
#[tokio::test]
async fn model_inline_value() {
    let model = Scripted::default();
    let input = SourceInput {
        key: "code".to_string(),
        content: SourceContent::Value("export const port = 8080;".to_string()),
    };

    let error = survey(&model, DOCS, &input, 2).await.expect_err("no tree to survey");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("not an inline value"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// No files, nothing to partition: no turn is spent and the survey is empty,
// for `extract` to refuse as it refuses any empty survey.
#[tokio::test]
async fn model_no_files() {
    let model = Scripted::default();
    let input = workspace("/lend/code");
    let ctx = Context {
        adapter_id: "source:probe",
        input: &input,
    };

    let groups = survey::by_model(&model, &ctx, DOCS, &[], 2).await.expect("nothing to ask");

    assert!(groups.is_empty());
    assert!(model.seen().is_empty(), "no turn was spent");
}
