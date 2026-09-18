//! Verifies mining, concurrency, ordering, retry, and failure aggregation across seams.
//!
//! Each seam produces one model request, with at most four requests pending
//! and a slot reused as soon as any request answers. Seams are dispatched
//! largest first, and claims preserve seam order even when requests complete
//! out of order. A request that fails upstream is put once more. The
//! scenarios also cover pre-request validation, single-seam errors, and
//! aggregation of several failures under the first failure's class.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use emery_sdk::{Backing, Context, Doc, Error, Evidence, Model, Seam, SourceInput};
use omnia_sdk::model::{Error as ModelError, Reply, Request, ToolCall};
use omnia_test::guest::Scripted;
use tokio::sync::Notify;

const PROSE: &[Doc] = &[Doc {
    path: "extract.md",
    body: "SYSTEM",
}];

// How long a scenario that must not deadlock is given before it fails.
const DEADLINE: Duration = Duration::from_secs(5);

// One claim anchored at a note beside the seam's file, so the joined
// document shows which seam each claim came through.
fn note(dir: &str) -> String {
    format!(
        r#"{{"claims":[{{"kind":"decision","path":"{dir}/note.md#L1","backing":{{"path":"{dir}/note.md"}}}}]}}"#
    )
}

// A scripted upstream failure, the class the SDK retries.
fn down(detail: &str) -> Result<Reply, ModelError> {
    Err(ModelError::Backend(detail.to_string()))
}

/// A model with one FIFO answer script per named seam file.
///
/// A turn is matched to the script of the file it names. Each file receives
/// its own responses regardless of polling order. The model records the order
/// seams arrive in and peak concurrency, yielding before every response so all
/// requests started by the SDK can become pending together, and can hold one
/// seam's answer until another seam's turn arrives.
#[derive(Clone, Default)]
struct ByFile {
    scripts: BTreeMap<String, Scripted>,
    arrivals: Arc<Mutex<Vec<String>>>,
    pending: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    hold: Option<Hold>,
}

// One seam's answer withheld until another seam's turn arrives.
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

    // Withholds `held`'s answer until `until`'s turn has arrived.
    fn holding(mut self, held: &str, until: &str) -> Self {
        self.hold = Some(Hold {
            held: held.to_string(),
            until: until.to_string(),
            gate: Arc::default(),
        });
        self
    }

    // The scripted file a turn names, and its script.
    fn script(&self, request: &Request) -> (&str, &Scripted) {
        let turn = request.messages.first().map(|message| message.content.as_str());
        let turn = turn.unwrap_or_default();
        self.scripts.iter().find(|(file, _)| turn.contains(&format!("`{file}`"))).map_or_else(
            || panic!("no script for the turn:\n{turn}"),
            |(file, script)| (file.as_str(), script),
        )
    }

    // The scripted file of each turn, in the order the turns arrived.
    fn arrivals(&self) -> Vec<String> {
        self.arrivals.lock().expect("arrivals").clone()
    }

    // The most completions pending at once.
    fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    // How many turns the seam naming `file` was put.
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
        // One yield lets every buffered seam start; the second lets each
        // record the others before any of them answers.
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

// A slot is reused as soon as any turn answers, not once the earliest
// pending one does: the first seam's answer is withheld until the fifth
// seam's turn arrives, which a fan-out that yielded in seam order — holding
// three answered slots behind the first — would never dispatch; the joined
// document still reads in seam order.
#[tokio::test]
async fn head_of_line() {
    let mut model = ByFile::default().holding("d0/f.md", "d4/f.md");
    for i in 0..=4 {
        model = model.file(&format!("d{i}/f.md"), Scripted::answering([note(&format!("d{i}"))]));
    }
    let seams: Vec<_> = (0..=4).map(|i| files([format!("d{i}/f.md").as_str()])).collect();

    let evidence = tokio::time::timeout(
        DEADLINE,
        extract(&model, &SourceInput::workspace("docs", "./docs"), &seams),
    )
    .await
    .expect("the fifth seam is dispatched while the first is still pending")
    .expect("every seam joins");

    let expected: Vec<String> = (0..=4).map(|i| format!("d{i}/note.md#L1")).collect();
    assert_eq!(paths(&evidence), expected);
    model.assert_exhausted();
}

// Seams are dispatched largest first — a `Files` seam by its file count, a
// seam of no known size before them — so the small seams fill the slots the
// large ones leave; the joined document still reads in seam order.
#[tokio::test]
async fn largest_first() {
    let model = ByFile::default()
        .file("a/1.md", Scripted::answering([note("a")]))
        .file("n/1.md", Scripted::answering([note("n")]))
        .file("b/1.md", Scripted::answering([note("b")]))
        .file("c/1.md", Scripted::answering([note("c")]));
    let seams = [
        files(["a/1.md"]),
        Seam::Note("Mine the surface entered at `n/1.md`.".to_string()),
        files(["b/1.md", "b/2.md", "b/3.md"]),
        files(["c/1.md", "c/2.md"]),
    ];

    let evidence = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect("every seam joins");

    assert_eq!(
        model.arrivals(),
        ["n/1.md", "b/1.md", "c/1.md", "a/1.md"],
        "the unsized seam first, then the `Files` seams by descending count"
    );
    assert_eq!(
        paths(&evidence),
        ["a/note.md#L1", "n/note.md#L1", "b/note.md#L1", "c/note.md#L1"],
        "the document reads in seam order"
    );
    model.assert_exhausted();
}

// A seam whose turn fails upstream is put once more, and the second turn's
// answer joins as if the first had never failed; a seam that answered is
// not put again.
#[tokio::test]
async fn retried() {
    let model = ByFile::default().file("a/x.md", Scripted::answering([note("a")])).file(
        "b/y.md",
        Scripted::new([
            down("down"),
            Ok(Reply {
                answer: note("b"),
                usage: None,
            }),
        ]),
    );
    let seams = [files(["a/x.md"]), files(["b/y.md"])];

    let evidence = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect("the retried seam joins");

    assert_eq!(paths(&evidence), ["a/note.md#L1", "b/note.md#L1"]);
    assert_eq!(model.turns("b/y.md"), 2, "the failed seam was put twice");
    assert_eq!(model.turns("a/x.md"), 1, "the answered seam once");
    model.assert_exhausted();
}

// The retry is one: a seam that fails upstream again fails the source, and
// the report names the second failure — the retry's outcome stands in for
// the first.
#[tokio::test]
async fn retry_spent() {
    let model = ByFile::default()
        .file("a/x.md", Scripted::answering([note("a")]))
        .file("b/y.md", Scripted::new([down("down"), down("still down")]));
    let seams = [files(["a/x.md"]), files(["b/y.md"])];

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect_err("the retry failed too");

    assert_eq!(error.code(), "bad_gateway");
    assert_eq!(
        error.description(),
        "`docs`: 1 of 2 seams failed:\n- seam 1: backend failure: still down"
    );
    assert_eq!(model.turns("b/y.md"), 2, "the seam was put exactly twice");
    model.assert_exhausted();
}

// A refusal is never retried: a request the host refuses, or a candidate the
// gate rejects until the rounds are spent, is the seam's answer as it
// stands — more turns would not change it — so each is put once.
#[tokio::test]
async fn refusal_not_retried() {
    let model = ByFile::default()
        .file(
            "a/x.md",
            Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]),
        )
        .file("b/y.md", Scripted::answering([note("b")]))
        .file("c/z.md", Scripted::answering([r#"{"claims":[{"kind":"requirement","id":"c"}]}"#]));
    let seams = [files(["a/x.md"]), files(["b/y.md"]), files(["c/z.md"])];

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &seams)
        .await
        .expect_err("two seams are refused");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().starts_with("`docs`: 2 of 3 seams failed:\n"), "{error}");
    assert_eq!(model.turns("a/x.md"), 1, "a refused request is put once");
    assert_eq!(model.turns("c/z.md"), 1, "spent rounds are put once");
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

// One seam of two fails, its retry too: the fan-out waits for both, then
// the source fails under that seam's class, naming it by index and no other.
#[tokio::test]
async fn one_seam_fails() {
    let model = ByFile::default()
        .file("a/x.md", Scripted::answering([note("a")]))
        .file("b/y.md", Scripted::new([down("down"), down("down")]));
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

// Two seams of three fail differently — one refused, one down twice: both
// are reported, in seam order, and the first one's class carries.
#[tokio::test]
async fn two_seams_fail() {
    let model = ByFile::default()
        .file(
            "a/x.md",
            Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]),
        )
        .file("b/y.md", Scripted::answering([note("b")]))
        .file("c/z.md", Scripted::new([down("down"), down("down")]));
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

// A lone seam is a single turn and its retry: its failure is the source's
// exactly as the model reported it, with no seam report around it.
#[tokio::test]
async fn single_seam_passthrough() {
    let model = Scripted::new([down("down"), down("still down")]);

    let error = extract(&model, &SourceInput::workspace("docs", "./docs"), &[Seam::Whole])
        .await
        .expect_err("the one seam failed twice");

    assert_eq!(error.code(), "bad_gateway");
    assert_eq!(error.description(), "backend failure: still down");
    assert_eq!(model.seen().len(), 2, "the one seam was put twice");
    model.assert_exhausted();
}
