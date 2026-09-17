//! Verifies model-assisted discovery of caller-facing surfaces.
//!
//! The scenarios cover request construction, result ordering, entry-path
//! normalisation, and validation against both the workspace and adapter
//! filter. They also cover empty inventories, pre-request failures, correction
//! rounds, and model error classification.

use std::fs;
use std::path::Path;

use emery_sdk::survey::{self, Surface};
use emery_sdk::workspace::Entry;
use emery_sdk::{Context, Doc, Error, SourceInput};
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

// A source tree with production modules, a directory and a file the suite's
// `keep` refuses, and the engine's own files beside them.
const FILES: &[&str] = &[
    ".omnia/store.json",
    "index.ts",
    "jobs/nightly.ts",
    "routes/orders.ts",
    "routes/users.ts",
    "services/orders.ts",
    "spec.md",
    "types/index.d.ts",
];

// The suite's policy, stated as an adapter states one: no `services/`
// directory, no declaration file.
fn keep(entry: Entry<'_>) -> bool {
    match entry {
        Entry::Dir(path) => path != "services",
        Entry::File(_) => !entry.name().ends_with(".d.ts"),
    }
}

async fn survey(
    model: &Scripted, docs: &'static [Doc], input: &SourceInput,
) -> Result<Vec<Surface>, Error> {
    let ctx = Context {
        adapter_id: "source:probe",
        input,
        model,
    };
    survey::surfaces(&ctx, docs, keep).await
}

fn surface(name: &str, entry: &str) -> Surface {
    Surface {
        name: name.to_string(),
        entry: entry.to_string(),
    }
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(path, body).expect("write");
}

// The scratch root as the engine lends one: a string.
fn utf8(root: &Path) -> &str {
    root.to_str().expect("a UTF-8 scratch root")
}

// An empty file at each relative path; the root as the engine lends it.
fn tree<'a>(root: &'a Path, files: &[&str]) -> &'a str {
    for file in files {
        write(root, file, "");
    }
    utf8(root)
}

// The survey request carries the embedded survey prompt as the system, the
// root lent whole so the model reads the tree itself — no file is listed, so
// the turn does not grow with the estate — the reference tools, `check`
// set, and the `Inventory` schema under `survey`.
#[tokio::test]
async fn model_request() {
    let model = Scripted::answering([
        r#"{"surfaces":[{"name":"POST /orders","entry":"routes/orders.ts"}]}"#,
    ]);
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), FILES);

    survey(&model, DOCS, &SourceInput::workspace("code", root)).await.expect("accepted");

    let seen = model.seen();
    assert_eq!(seen.len(), 1, "one survey turn");
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("SURVEY"));
    assert_eq!(request.workspace.as_deref(), Some(root), "the root is lent");
    assert!(request.check, "acceptance is the check");
    assert_eq!(request.tools, ["list_docs", "read_doc"], "the corpus is offered through tools");
    let turn = &request.messages[0];
    assert!(turn.contains("adapter `source:probe` (source key `code`)"), "{turn}");
    assert!(turn.contains(&format!("read-only view at `{root}`")), "{turn}");
    assert!(turn.contains("relative to `$SOURCE_DIR`"), "{turn}");
    assert!(!turn.contains("\n- `"), "no file is listed: {turn}");
    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("the survey is steered by schema");
    };
    assert_eq!(name, "survey");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("generated schema parses");
    assert!(schema.pointer("/properties/surfaces").is_some(), "{schema}");
    let surface = schema.pointer("/$defs/Surface").expect("Surface definition");
    assert!(surface.pointer("/properties/name").is_some(), "{surface}");
    assert!(surface.pointer("/properties/entry").is_some(), "{surface}");
    model.assert_exhausted();
}

// The surfaces come back in answer order, as many as the model found: a
// module may be the entry of several, and a module no surface enters — a
// service, the bootstrap — is no surface, so the tree is mined from its
// boundary and nothing is grouped or folded.
#[tokio::test]
async fn model_surfaces() {
    let model = Scripted::answering([r#"{"surfaces":[
            {"name":"POST /users","entry":"routes/users.ts"},
            {"name":"nightly reconciliation job","entry":"jobs/nightly.ts"},
            {"name":"GET /users/:id","entry":"routes/users.ts"},
            {"name":"POST /orders","entry":"routes/orders.ts"}
        ]}"#]);
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), FILES);

    let surfaces =
        survey(&model, DOCS, &SourceInput::workspace("code", root)).await.expect("accepted");

    assert_eq!(
        surfaces,
        [
            surface("POST /users", "routes/users.ts"),
            surface("nightly reconciliation job", "jobs/nightly.ts"),
            surface("GET /users/:id", "routes/users.ts"),
            surface("POST /orders", "routes/orders.ts"),
        ]
    );
    model.assert_exhausted();
}

// An entry is held to the tree, not to a listing: a `./` prefix and a
// doubled separator are dropped, and the surface comes back with the path
// as a claim's anchor would cite it.
#[tokio::test]
async fn model_normalised() {
    let model = Scripted::answering([
        r#"{"surfaces":[{"name":"POST /orders","entry":"./routes//orders.ts"}]}"#,
    ]);
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), FILES);

    let surfaces =
        survey(&model, DOCS, &SourceInput::workspace("code", root)).await.expect("accepted");

    assert_eq!(surfaces, [surface("POST /orders", "routes/orders.ts")]);
    assert_eq!(model.exchanges().len(), 1, "one turn, accepted");
    model.assert_exhausted();
}

// A candidate the check refuses goes back as findings and the next candidate
// is checked: a nameless surface, one name listed twice, an entry at no
// file, at a directory, at a module the adapter's `keep` refuses — by its
// directory or by itself — at one of the engine's own files, or escaping
// the root. The model finds the boundary; the tree and the adapter say what
// a module is.
#[tokio::test]
async fn model_corrections() {
    let model = Scripted::answering([
        r#"{"surfaces":[
            {"name":"POST /orders","entry":"routes/orders.ts"},
            {"name":"POST /orders","entry":"index.ts"},
            {"name":"","entry":"jobs/nightly.ts"},
            {"name":"GET /ghosts","entry":"routes/ghost.ts"},
            {"name":"routes","entry":"routes"},
            {"name":"order service","entry":"services/orders.ts"},
            {"name":"types","entry":"types/index.d.ts"},
            {"name":"spec","entry":"spec.md"},
            {"name":"outside","entry":"../x.ts"}
        ]}"#,
        r#"{"surfaces":[{"name":"POST /orders","entry":"routes/orders.ts"}]}"#,
    ]);
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), FILES);

    let surfaces = survey(&model, DOCS, &SourceInput::workspace("code", root))
        .await
        .expect("the second candidate is an inventory");

    assert_eq!(surfaces, [surface("POST /orders", "routes/orders.ts")]);
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "one rejection, one acceptance");
    let correction = exchanges[0].outcome.as_ref().expect_err("the first candidate is rejected");
    for finding in [
        "surface `POST /orders` is listed twice",
        "the surface entered at `jobs/nightly.ts` has no name",
        "no file at `routes/ghost.ts`",
        "no file at `routes`",
        "`services/orders.ts` is not a module this adapter mines",
        "`types/index.d.ts` is not a module this adapter mines",
        "`spec.md` is not a module this adapter mines",
        "`../x.ts` escapes the source root",
    ] {
        assert!(correction.contains(finding), "{finding}: {correction}");
    }
    assert!(!correction.contains("`index.ts`"), "the second `POST /orders` is a sound entry");
    assert_eq!(exchanges[1].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A stray key on the answer is a schema miss, corrected like a finding.
#[tokio::test]
async fn model_stray_key() {
    let model = Scripted::answering([
        r#"{"surfaces":[{"name":"POST /orders","entry":"routes/orders.ts","files":["services/orders.ts"]}]}"#,
        r#"{"surfaces":[{"name":"POST /orders","entry":"routes/orders.ts"}]}"#,
    ]);
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), FILES);

    survey(&model, DOCS, &SourceInput::workspace("code", root))
        .await
        .expect("the second candidate parses");

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the stray key is refused");
    assert!(correction.contains("unknown field"), "{correction}");
}

// When the backend spends its rounds on a rejected inventory the last
// findings surface as `bad_request`, as an evidence call's do.
#[tokio::test]
async fn model_rounds_exhausted() {
    let model = Scripted::answering([r#"{"surfaces":[{"name":"GET /ghosts","entry":"nope.ts"}]}"#]);
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), FILES);

    let error = survey(&model, DOCS, &SourceInput::workspace("code", root))
        .await
        .expect_err("the only candidate is entered at no file");

    let Error::BadRequest { code, description } = error else {
        panic!("spent rounds are a bad request: {error}");
    };
    assert_eq!(code, "bad_request");
    assert!(description.contains("no file at `nope.ts`"), "{description}");
    assert_eq!(model.exchanges().len(), 1, "one check, rejected");
}

// A corpus without `prompts/survey.md` is the adapter build's own defect,
// reported before a turn is spent.
#[tokio::test]
async fn model_missing_prompt() {
    let model = Scripted::default();
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), FILES);

    let error = survey(&model, MUTE, &SourceInput::workspace("code", root))
        .await
        .expect_err("no prompt to ask with");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`prompts/survey.md` is not embedded"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// An inline value has no tree to survey: asking is the adapter's own
// defect, reported before a turn is spent, as a `Files` seam over a value
// is.
#[tokio::test]
async fn model_inline_value() {
    let model = Scripted::default();
    let input = SourceInput::value("code", "export const x = 1;");

    let error = survey(&model, DOCS, &input).await.expect_err("no tree to survey");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("not an inline value"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// An empty inventory is an answer, not a finding: the model read the tree
// — here one with no file at all, which costs the turn like any other —
// and found no boundary. It is accepted as it stands, and what a source with
// no surface means is the adapter's to decide.
#[tokio::test]
async fn model_no_surfaces() {
    let model = Scripted::answering([r#"{"surfaces":[]}"#]);
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tree(tmp.path(), &[]);

    let surfaces = survey(&model, DOCS, &SourceInput::workspace("code", root))
        .await
        .expect("an empty inventory is accepted");

    assert!(surfaces.is_empty());
    assert_eq!(model.exchanges().len(), 1, "one turn, accepted");
    model.assert_exhausted();
}
