use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use emery_sdk::{Backing, Context, Doc, Error, Evidence, Model, Seam, SourceContent, SourceInput};
use omnia_sdk::model::{Error as ModelError, Reply, Request, ToolCall};
use omnia_test::guest::Scripted;
use tokio::sync::Notify;

const PROSE: &[Doc] = &[Doc {
    path: "extract.md",
    body: "SYSTEM",
}];

const DEADLINE: Duration = Duration::from_secs(5);

// Anchored at the seam's file, so the joined document shows which seam each
// claim came through.
fn note(file: &str) -> String {
    format!(
        r#"{{"claims":[{{"kind":"decision","path":"{file}#L1","backing":{{"path":"{file}"}}}}]}}"#
    )
}

// A tree holding every seam file with one line, lent as the source `docs`.
fn workspace(files: &[&str]) -> (tempfile::TempDir, SourceInput) {
    let tmp = tempfile::tempdir().expect("tempdir");
    for file in files {
        let path = tmp.path().join(file);
        let parent = path.parent().expect("a parent");
        std::fs::create_dir_all(parent).expect("mkdir");
        std::fs::write(path, "the line\n").expect("write");
    }
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let input = SourceInput::workspace("docs", root);
    (tmp, input)
}

fn anchors<const N: usize>(files: [&str; N]) -> Vec<String> {
    files.iter().map(|file| format!("{file}#L1")).collect()
}

// The class the SDK retries.
fn down(detail: &str) -> Result<Reply, ModelError> {
    Err(ModelError::Backend(detail.to_string()))
}

// One FIFO script per seam file, matched by the file the turn names, so each
// file gets its own answers whatever the polling order. Records arrival order
// and peak concurrency, and can hold one seam's answer until another's turn arrives.
#[derive(Clone, Default)]
struct ByFile {
    scripts: BTreeMap<String, Scripted>,
    arrivals: Arc<Mutex<Vec<String>>>,
    pending: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    hold: Option<Hold>,
}

#[derive(Clone)]
struct Hold {
    held: String,
    until: String,
    gate: Arc<Notify>,
}

impl ByFile {
    fn file(mut self, file: &str, script: Scripted) -> Self {
        self.scripts.insert(file.to_string(), script);
        self
    }

    fn holding(mut self, held: &str, until: &str) -> Self {
        self.hold = Some(Hold {
            held: held.to_string(),
            until: until.to_string(),
            gate: Arc::default(),
        });
        self
    }

    fn script(&self, request: &Request) -> (&str, &Scripted) {
        let turn = request.messages.first().map(|message| message.content.as_str());
        let turn = turn.unwrap_or_default();
        self.scripts.iter().find(|(file, _)| turn.contains(&format!("`{file}`"))).map_or_else(
            || panic!("no script for the turn:\n{turn}"),
            |(file, script)| (file.as_str(), script),
        )
    }

    fn arrivals(&self) -> Vec<String> {
        self.arrivals.lock().expect("arrivals").clone()
    }

    fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    fn turns(&self, file: &str) -> usize {
        self.scripts[file].seen().len()
    }

    fn assert_exhausted(&self) {
        for script in self.scripts.values() {
            script.assert_exhausted();
        }
    }
}

impl Model for ByFile {
    fn complete(&self, request: Request) -> impl Future<Output = Result<Reply, ModelError>> + Send {
        self.script(&request).1.complete(request)
    }

    async fn complete_with<H, F>(&self, request: Request, handler: H) -> Result<Reply, ModelError>
    where
        H: FnMut(ToolCall) -> F + Send,
        F: Future<Output = Result<String, String>> + Send,
    {
        let (file, script) = self.script(&request);
        self.arrivals.lock().expect("arrivals").push(file.to_string());
        let pending = self.pending.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(pending, Ordering::SeqCst);

        // let every buffered seam start, then record the others, before any answers
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        if let Some(hold) = &self.hold {
            if file == hold.until {
                hold.gate.notify_one();
            }
            if file == hold.held {
                hold.gate.notified().await;
            }
        }

        let outcome = script.complete_with(request, handler).await;
        self.pending.fetch_sub(1, Ordering::SeqCst);
        outcome
    }
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

fn paths(evidence: &Evidence) -> Vec<&str> {
    evidence.claims.iter().map(|claim| claim.path.as_deref().unwrap_or_default()).collect()
}

#[tokio::test]
async fn three_seams() {
    let (_tmp, input) = workspace(&["a/x.md", "b/y.md", "c/z.md"]);
    let model = ByFile::default()
        .file("a/x.md", Scripted::answering([note("a/x.md")]))
        .file("b/y.md", Scripted::answering([note("b/y.md")]))
        .file("c/z.md", Scripted::answering([note("c/z.md")]));
    let seams = [Seam::files(["a/x.md"]), Seam::files(["b/y.md"]), Seam::files(["c/z.md"])];

    let evidence = extract(&model, &input, &seams).await.expect("three seams join");

    assert_eq!(paths(&evidence), anchors(["a/x.md", "b/y.md", "c/z.md"]));
    let backings: Vec<_> = evidence.claims.iter().map(|claim| claim.backing.clone()).collect();
    assert_eq!(
        backings,
        [
            Some(Backing::Path("a/x.md".to_string())),
            Some(Backing::Path("b/y.md".to_string())),
            Some(Backing::Path("c/z.md".to_string())),
        ]
    );
    let SourceContent::Workspace(root) = &input.content else { panic!("a workspace") };
    for file in ["a/x.md", "b/y.md", "c/z.md"] {
        let seen = model.scripts[file].seen();
        assert_eq!(seen.len(), 1, "one turn per seam");
        assert_eq!(
            seen[0].workspace.as_deref(),
            Some(root.as_str()),
            "every seam is lent the root"
        );
        let user = &seen[0].messages[0];
        assert!(user.contains("`$SOURCE_DIR` is the bound source tree, lent read-only:"), "{user}");
        assert!(
            user.contains(&format!("### `{file}` (1 line)\n\n```\n1|the line\n```\n\n")),
            "the one file is laid out: {user}"
        );
    }
    model.assert_exhausted();
}

// The fan-out is neither serial nor unbounded.
#[tokio::test]
async fn concurrent() {
    let named: Vec<String> = (0..=4).map(|i| format!("d{i}/f.md")).collect();
    let named: Vec<&str> = named.iter().map(String::as_str).collect();
    let (_tmp, input) = workspace(&named);
    let mut model = ByFile::default();
    for file in &named {
        model = model.file(file, Scripted::answering([note(file)]));
    }
    let seams: Vec<_> = named.iter().map(|file| Seam::files([*file])).collect();

    let evidence = extract(&model, &input, &seams).await.expect("every seam joins");

    assert_eq!(model.peak(), 4, "5 seams hold at most 4 completions pending");
    let expected: Vec<String> = named.iter().map(|file| format!("{file}#L1")).collect();
    assert_eq!(paths(&evidence), expected);
    model.assert_exhausted();
}

// The first seam's answer is withheld until the fifth seam's turn arrives, which
// a fan-out yielding in seam order would never dispatch.
#[tokio::test]
async fn head_of_line() {
    let named: Vec<String> = (0..=4).map(|i| format!("d{i}/f.md")).collect();
    let named: Vec<&str> = named.iter().map(String::as_str).collect();
    let (_tmp, input) = workspace(&named);
    let mut model = ByFile::default().holding("d0/f.md", "d4/f.md");
    for file in &named {
        model = model.file(file, Scripted::answering([note(file)]));
    }
    let seams: Vec<_> = named.iter().map(|file| Seam::files([*file])).collect();

    let evidence = tokio::time::timeout(DEADLINE, extract(&model, &input, &seams))
        .await
        .expect("the fifth seam is dispatched while the first is still pending")
        .expect("every seam joins");

    let expected: Vec<String> = named.iter().map(|file| format!("{file}#L1")).collect();
    assert_eq!(paths(&evidence), expected);
    model.assert_exhausted();
}

// Small seams fill the slots the large ones leave.
#[tokio::test]
async fn largest_first() {
    let (_tmp, input) =
        workspace(&["a/1.md", "n/1.md", "b/1.md", "b/2.md", "b/3.md", "c/1.md", "c/2.md"]);
    let model = ByFile::default()
        .file("a/1.md", Scripted::answering([note("a/1.md")]))
        .file("n/1.md", Scripted::answering([note("n/1.md")]))
        .file("b/1.md", Scripted::answering([note("b/1.md")]))
        .file("c/1.md", Scripted::answering([note("c/1.md")]));
    let seams = [
        Seam::files(["a/1.md"]),
        Seam::note("Mine the surface entered at `n/1.md`."),
        Seam::files(["b/1.md", "b/2.md", "b/3.md"]),
        Seam::files(["c/1.md", "c/2.md"]),
    ];

    let evidence = extract(&model, &input, &seams).await.expect("every seam joins");

    assert_eq!(
        model.arrivals(),
        ["n/1.md", "b/1.md", "c/1.md", "a/1.md"],
        "the unsized seam first, then the seams naming files by descending count"
    );
    assert_eq!(
        paths(&evidence),
        anchors(["a/1.md", "n/1.md", "b/1.md", "c/1.md"]),
        "the document reads in seam order"
    );
    model.assert_exhausted();
}

#[tokio::test]
async fn retried() {
    let (_tmp, input) = workspace(&["a/x.md", "b/y.md"]);
    let model = ByFile::default().file("a/x.md", Scripted::answering([note("a/x.md")])).file(
        "b/y.md",
        Scripted::new([
            down("down"),
            Ok(Reply {
                answer: note("b/y.md"),
                usage: None,
            }),
        ]),
    );
    let seams = [Seam::files(["a/x.md"]), Seam::files(["b/y.md"])];

    let evidence = extract(&model, &input, &seams).await.expect("the retried seam joins");

    assert_eq!(paths(&evidence), anchors(["a/x.md", "b/y.md"]));
    assert_eq!(model.turns("b/y.md"), 2, "the failed seam was put twice");
    assert_eq!(model.turns("a/x.md"), 1, "the answered seam once");
    model.assert_exhausted();
}

// The fan-out waits for both seams before failing under the one's class, and
// the retry's outcome stands in for the first failure.
#[tokio::test]
async fn retry_spent() {
    let (_tmp, input) = workspace(&["a/x.md", "b/y.md"]);
    let model = ByFile::default()
        .file("a/x.md", Scripted::answering([note("a/x.md")]))
        .file("b/y.md", Scripted::new([down("down"), down("still down")]));
    let seams = [Seam::files(["a/x.md"]), Seam::files(["b/y.md"])];

    let error = extract(&model, &input, &seams).await.expect_err("the retry failed too");

    assert_eq!(error.code(), "bad_gateway");
    assert_eq!(
        error.description(),
        "`docs`: 1 of 2 seams failed:\n- seam 1: backend failure: still down"
    );
    assert_eq!(model.turns("b/y.md"), 2, "the seam was put exactly twice");
    model.assert_exhausted();
}

// A refused request and spent rounds are the seam's answer as it stands; more
// turns would not change it.
#[tokio::test]
async fn refusal_not_retried() {
    let (_tmp, input) = workspace(&["a/x.md", "b/y.md", "c/z.md"]);
    let model = ByFile::default()
        .file(
            "a/x.md",
            Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]),
        )
        .file("b/y.md", Scripted::answering([note("b/y.md")]))
        .file("c/z.md", Scripted::answering([r#"{"claims":[{"kind":"requirement","id":"c"}]}"#]));
    let seams = [Seam::files(["a/x.md"]), Seam::files(["b/y.md"]), Seam::files(["c/z.md"])];

    let error = extract(&model, &input, &seams).await.expect_err("two seams are refused");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().starts_with("`docs`: 2 of 3 seams failed:\n"), "{error}");
    assert_eq!(model.turns("a/x.md"), 1, "a refused request is put once");
    assert_eq!(model.turns("c/z.md"), 1, "spent rounds are put once");
    model.assert_exhausted();
}

#[tokio::test]
async fn no_seams() {
    let model = Scripted::default();
    let input = SourceInput::workspace("docs", "./docs");

    let error = extract(&model, &input, &[]).await.expect_err("nothing to extract");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("nothing to extract"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// Every seam's lend is checked before the first model call, even when an
// earlier seam was sound.
#[tokio::test]
async fn escaping_path() {
    let model = Scripted::default();
    let input = SourceInput::workspace("docs", "./docs");
    let seams = [Seam::files(["a/x.md"]), Seam::files(["../secret.md"])];

    let error = extract(&model, &input, &seams).await.expect_err("a path escapes");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("`../secret.md` escapes the source root"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

#[tokio::test]
async fn dot_path() {
    let model = Scripted::default();
    let input = SourceInput::workspace("docs", "./docs");

    let error = extract(&model, &input, &[Seam::files(["./"])]).await.expect_err("no file");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("names no file"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// No tree to lend is the adapter's own defect, so the class is the adapter's.
#[tokio::test]
async fn files_value() {
    let model = Scripted::default();

    let error =
        extract(&model, &SourceInput::value("brief", "Ship it."), &[Seam::files(["a/x.md"])])
            .await
            .expect_err("no tree to lend");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("inline value with no tree to lend"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

#[tokio::test]
async fn two_seams_fail() {
    let (_tmp, input) = workspace(&["a/x.md", "b/y.md", "c/z.md"]);
    let model = ByFile::default()
        .file(
            "a/x.md",
            Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]),
        )
        .file("b/y.md", Scripted::answering([note("b/y.md")]))
        .file("c/z.md", Scripted::new([down("down"), down("down")]));
    let seams = [Seam::files(["a/x.md"]), Seam::files(["b/y.md"]), Seam::files(["c/z.md"])];

    let error = extract(&model, &input, &seams).await.expect_err("two seams failed");

    assert_eq!(error.code(), "bad_request", "the first failure's class");
    assert_eq!(
        error.description(),
        "`docs`: 2 of 3 seams failed:\n- seam 0: invalid request: no such model\n- \
         seam 2: backend failure: down"
    );
    model.assert_exhausted();
}

// A lone seam's failure is the source's as the model reported it, with no seam
// report around it.
#[tokio::test]
async fn single_seam_passthrough() {
    let model = Scripted::new([down("down"), down("still down")]);
    let input = SourceInput::workspace("docs", "./docs");

    let error =
        extract(&model, &input, &[Seam::whole()]).await.expect_err("the one seam failed twice");

    assert_eq!(error.code(), "bad_gateway");
    assert_eq!(error.description(), "backend failure: still down");
    assert_eq!(model.seen().len(), 2, "the one seam was put twice");
    model.assert_exhausted();
}
