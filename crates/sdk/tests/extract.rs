//! Verifies mining, concurrency, ordering, and failure aggregation across seams.
//!
//! Each seam produces one model request, with at most four requests pending.
//! Claims preserve seam order even when requests complete out of order. The
//! scenarios also cover pre-request validation, single-seam errors, and
//! aggregation of several failures under the first failure's class.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use emery_sdk::{Backing, Context, Doc, Error, Evidence, Model, Seam, SourceInput};
use omnia_sdk::model::{Error as ModelError, Reply, Request, ToolCall};
use omnia_test::guest::Scripted;

const PROSE: &[Doc] = &[Doc {
    path: "extract.md",
    body: "SYSTEM",
}];

// One claim anchored at a note beside the seam's file, so the joined
// document shows which seam each claim came through.
fn note(dir: &str) -> String {
    format!(
        r#"{{"claims":[{{"kind":"decision","path":"{dir}/note.md#L1","backing":{{"path":"{dir}/note.md"}}}}]}}"#
    )
}

/// A model with one FIFO answer script per named seam file.
///
/// Each file receives its own responses regardless of polling order. The
/// model also records peak concurrency, yielding before every response so all
/// requests started by the SDK can become pending together.
#[derive(Clone, Default)]
struct ByFile {
    scripts: BTreeMap<String, Scripted>,
    pending: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl ByFile {
    fn file(mut self, file: &str, script: Scripted) -> Self {
        self.scripts.insert(file.to_string(), script);
        self
    }

    fn script(&self, request: &Request) -> &Scripted {
        let turn = request.messages.first().map(|message| message.content.as_str());
        let turn = turn.unwrap_or_default();
        self.scripts
            .iter()
            .find(|(file, _)| turn.contains(&format!("\n- `{file}`")))
            .map_or_else(|| panic!("no script for the turn:\n{turn}"), |(_, script)| script)
    }

    // The most completions pending at once.
    fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    fn assert_exhausted(&self) {
        for script in self.scripts.values() {
            script.assert_exhausted();
        }
    }
}

impl Model for ByFile {
    fn complete(&self, request: Request) -> impl Future<Output = Result<Reply, ModelError>> + Send {
        self.script(&request).complete(request)
    }

    async fn complete_with<H, F>(&self, request: Request, handler: H) -> Result<Reply, ModelError>
    where
        H: FnMut(ToolCall) -> F + Send,
        F: Future<Output = Result<String, String>> + Send,
    {
        let pending = self.pending.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(pending, Ordering::SeqCst);
        // One yield lets every buffered seam start; the second lets each
        // record the others before any of them answers.
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let outcome = self.script(&request).complete_with(request, handler).await;
        self.pending.fetch_sub(1, Ordering::SeqCst);
        outcome
    }
}

fn files<const N: usize>(paths: [&str; N]) -> Seam {
    Seam::Files(paths.into_iter().map(str::to_string).collect())
}

async fn extract<M: Model>(
    model: &M, input: &SourceInput, seams: &[Seam],
) -> Result<Evidence, Error> {
    let ctx = Context {
        adapter_id: "probe",
        input,
        model,
    };
    emery_sdk::extract(&ctx, PROSE, seams).await
}

// Each claim's `path` anchor, in document order.
fn paths(evidence: &Evidence) -> Vec<&str> {
    evidence.claims.iter().map(|claim| claim.path.as_deref().unwrap_or_default()).collect()
}

// The bound input, whole, is one model turn — the one `turn.rs` asserts for
// `Seam::Whole` — whose claims come back with anchors as answered.
#[tokio::test]
async fn whole() {
    let model = Scripted::answering([note("a")]);

    let evidence = extract(&model, &SourceInput::workspace("docs", "./docs"), &[Seam::Whole])
        .await
        .expect("one bound turn");

    assert_eq!(paths(&evidence), ["a/note.md#L1"]);
    assert_eq!(evidence.claims[0].backing, Some(Backing::Path("a/note.md".to_string())));
    let seen = model.seen();
    assert_eq!(seen.len(), 1, "one seam, one turn");
    assert_eq!(seen[0].workspace.as_deref(), Some("./docs"), "the root is lent");
    let user = &seen[0].messages[0];
    assert!(
        user.contains(
            "`$SOURCE_DIR` is the read-only view at `./docs` — the source tree the prompt walks."
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// Three `Files` seams run through three model turns, each lent the root and
// listing its own file relative to it, and join as one document: claims in
// seam order, every anchor and path backing as the model answered it.
#[tokio::test]
async fn three_seams() {
    let model = ByFile::default()
        .file("a/x.md", Scripted::answering([note("a")]))
        .file("b/y.md", Scripted::answering([note("b")]))
        .file("c/z.md", Scripted::answering([note("c")]));
    let seams = [files(["a/x.md"]), files(["b/y.md"]), files(["c/z.md"])];

    let evidence = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect("three seams join");

    assert_eq!(paths(&evidence), ["a/note.md#L1", "b/note.md#L1", "c/note.md#L1"]);
    let backings: Vec<_> = evidence.claims.iter().map(|claim| claim.backing.clone()).collect();
    assert_eq!(
        backings,
        [
            Some(Backing::Path("a/note.md".to_string())),
            Some(Backing::Path("b/note.md".to_string())),
            Some(Backing::Path("c/note.md".to_string())),
        ]
    );
    for file in ["a/x.md", "b/y.md", "c/z.md"] {
        let seen = model.scripts[file].seen();
        assert_eq!(seen.len(), 1, "one turn per seam");
        assert_eq!(seen[0].workspace.as_deref(), Some("./docs"), "every seam is lent the root");
        let user = &seen[0].messages[0];
        assert!(user.contains("read-only view at `./docs` — the source tree."), "{user}");
        assert!(user.contains(&format!("nothing else:\n\n- `{file}`\n\n")), "{user}");
    }
    model.assert_exhausted();
}

// Five seams hold exactly four completions pending at once — the fan-out is
// neither serial nor unbounded — and the joined document still reads in
// seam order.
#[tokio::test]
async fn concurrent() {
    let mut model = ByFile::default();
    for i in 0..=4 {
        model = model.file(&format!("d{i}/f.md"), Scripted::answering([note(&format!("d{i}"))]));
    }
    let seams: Vec<_> = (0..=4).map(|i| files([format!("d{i}/f.md").as_str()])).collect();

    let evidence = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect("every seam joins");

    assert_eq!(model.peak(), 4, "5 seams hold at most 4 completions pending");
    let expected: Vec<String> = (0..=4).map(|i| format!("d{i}/note.md#L1")).collect();
    assert_eq!(paths(&evidence), expected);
    model.assert_exhausted();
}

// No seam is refused before any model call: the input had nothing to extract.
#[tokio::test]
async fn no_seams() {
    let model = Scripted::default();

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &[])
        .await
        .expect_err("nothing to extract");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("nothing to extract"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// Every seam's lend is checked before the first model call, so a `Files`
// path that escapes the root refuses the source with no turn spent — even
// when an earlier seam was sound.
#[tokio::test]
async fn escaping_path() {
    let model = Scripted::default();
    let seams = [files(["a/x.md"]), files(["../secret.md"])];

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect_err("a path escapes");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("`../secret.md` escapes the source root"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// A `Files` seam naming no file has nothing to extract; refused as the
// input's, before any model call.
#[tokio::test]
async fn empty_files() {
    let model = Scripted::default();

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &[files([])])
        .await
        .expect_err("no file");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("names no file"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// `Files` over an inline value is the adapter's own defect — there is no
// tree to lend — so the class is the adapter's, not the operator's.
#[tokio::test]
async fn files_value() {
    let model = Scripted::default();

    let error = extract(&model, &SourceInput::value("brief", "Ship it."), &[files(["a/x.md"])])
        .await
        .expect_err("no tree to lend");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("not an inline value"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// One seam of two fails: the fan-out waits for both, then the source
// fails under that seam's class, naming it by index and no other.
#[tokio::test]
async fn one_seam_fails() {
    let model = ByFile::default()
        .file("a/x.md", Scripted::answering([note("a")]))
        .file("b/y.md", Scripted::new([Err(ModelError::Backend("down".to_string()))]));
    let seams = [files(["a/x.md"]), files(["b/y.md"])];

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect_err("one seam failed");

    assert_eq!(error.code(), "bad_gateway");
    assert_eq!(
        error.description(),
        "`docs`: 1 of 2 seams failed:\n- seam 1: backend failure: down"
    );
    model.assert_exhausted();
}

// Two seams of three fail differently: both are reported, in seam order,
// and the first one's class carries.
#[tokio::test]
async fn two_seams_fail() {
    let model = ByFile::default()
        .file(
            "a/x.md",
            Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]),
        )
        .file("b/y.md", Scripted::answering([note("b")]))
        .file("c/z.md", Scripted::new([Err(ModelError::Backend("down".to_string()))]));
    let seams = [files(["a/x.md"]), files(["b/y.md"]), files(["c/z.md"])];

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect_err("two seams failed");

    assert_eq!(error.code(), "bad_request", "the first failure's class");
    assert_eq!(
        error.description(),
        "`docs`: 2 of 3 seams failed:\n- seam 0: invalid request: no such model\n- \
         seam 2: backend failure: down"
    );
    model.assert_exhausted();
}

// A lone seam is a single turn: its failure is the source's exactly as the
// model reported it, with no seam report around it.
#[tokio::test]
async fn single_seam_passthrough() {
    let model = Scripted::new([Err(ModelError::Backend("down".to_string()))]);

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &[Seam::Whole])
        .await
        .expect_err("the one seam failed");

    assert_eq!(error.code(), "bad_gateway");
    assert_eq!(error.description(), "backend failure: down");
    model.assert_exhausted();
}
