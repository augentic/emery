//! The provided `extract`
//!
//! What an adapter gets from `SourceAdapter::extract` without overriding it:
//! the survey's materials mined through `evidence` — at most `IN_FLIGHT`
//! pending, in material order — and joined into one document with each
//! material's anchors re-rooted under what it was lent; the default survey of
//! one bound material, a single `evidence` call whose outcome passes through
//! unchanged; the refusals a survey earns before any model call; and every
//! failed material reported together under the first one's class.

use std::collections::BTreeMap;
use std::future::{Future, ready};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use emery_prose::registry::Doc;
use emery_sdk::model::{Error as ModelError, Reply, Request, ToolCall};
use emery_sdk::{
    Backing, Context, Error, Evidence, Material, Model, SourceAdapter, SourceContent, SourceInput,
    SourceKind,
};
use omnia_test::guest::Scripted;

const DOCS: &[Doc] = &[Doc {
    path: "prompts/extract.md",
    body: "SYSTEM",
}];

// One claim anchored relative to its material's lend, so the joined document
// shows which lend each claim came through.
const NOTE: &str =
    r#"{"claims":[{"kind":"decision","path":"note.md#L1","backing":{"path":"note.md"}}]}"#;

// An adapter that states no survey: the default is the bound input, whole.
struct Plain;

impl SourceAdapter for Plain {
    const KIND: SourceKind = SourceKind::Documentation;
    const SOURCE: &'static str = "probe";

    fn docs() -> &'static [Doc] {
        DOCS
    }
}

// An adapter whose survey is the materials given.
macro_rules! probe {
    ($name:ident, $survey:expr) => {
        struct $name;

        impl SourceAdapter for $name {
            const KIND: SourceKind = SourceKind::Documentation;
            const SOURCE: &'static str = "probe";

            fn docs() -> &'static [Doc] {
                DOCS
            }

            fn survey<P: Model>(
                _model: &P, _ctx: &Context<'_>,
            ) -> impl Future<Output = Result<Vec<Material>, Error>> + Send {
                ready(Ok($survey))
            }
        }
    };
}

probe!(Split, vec![within(["a/x.md"]), within(["b/y.md"]), within(["c/z.md"])]);
probe!(Pair, vec![within(["a/x.md"]), within(["b/y.md"])]);
probe!(Wide, (0..=4).map(|i| within([format!("d{i}/f.md").as_str()])).collect());
probe!(Nothing, Vec::new());
probe!(Blank, vec![within([])]);
probe!(Escape, vec![within(["a/x.md"]), within(["../secret.md"])]);

/// A model routed by the request's workspace lend: one FIFO script per lend,
/// so each material — every `Within` here lends its own directory — answers
/// from its own script whichever order the fan-out polls them in. It also
/// counts the completions pending at once, yielding before each answer so
/// every future the SDK has started is in flight together.
#[derive(Clone, Default)]
struct ByLend {
    scripts: BTreeMap<String, Scripted>,
    pending: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
}

impl ByLend {
    fn lend(mut self, workspace: &str, script: Scripted) -> Self {
        self.scripts.insert(workspace.to_string(), script);
        self
    }

    fn script(&self, request: &Request) -> &Scripted {
        let lend = request.workspace.as_deref().unwrap_or_default();
        self.scripts.get(lend).unwrap_or_else(|| panic!("no script for the lend `{lend}`"))
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

impl Model for ByLend {
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
        // One yield lets every buffered material start; the second lets each
        // record the others before any of them answers.
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;

        let outcome = self.script(&request).complete_with(request, handler).await;
        self.pending.fetch_sub(1, Ordering::SeqCst);
        outcome
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

fn within<const N: usize>(paths: [&str; N]) -> Material {
    Material::Within(paths.into_iter().map(str::to_string).collect())
}

async fn extract<A: SourceAdapter, M: Model>(
    model: &M, input: &SourceInput,
) -> Result<Evidence, Error> {
    let ctx = Context {
        adapter_id: "probe",
        input,
    };
    A::extract(model, &ctx).await
}

// Each claim's `path` anchor, in document order.
fn paths(evidence: &Evidence) -> Vec<&str> {
    evidence.claims.iter().map(|claim| claim.path.as_deref().unwrap_or_default()).collect()
}

// An adapter that states no survey mines the bound input whole in one model
// turn — the request `evidence` builds for `Material::Bound` — and returns
// its claims with anchors as answered.
#[tokio::test]
async fn default_survey() {
    let model = Scripted::answering([NOTE]);

    let evidence = extract::<Plain, _>(&model, &workspace("./docs")).await.expect("one bound turn");

    assert_eq!(paths(&evidence), ["note.md#L1"]);
    assert_eq!(evidence.claims[0].backing, Some(Backing::Path("note.md".to_string())));
    let seen = model.seen();
    assert_eq!(seen.len(), 1, "one material, one turn");
    assert_eq!(seen[0].workspace.as_deref(), Some("./docs"), "the root is lent");
    let user = &seen[0].messages[0];
    assert!(
        user.contains(
            "`$SOURCE_DIR` is the read-only view at `./docs` — the probe source tree the prompt \
             walks."
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// Three `Within` materials, each lending its own directory, run through three
// model turns and join as one document: claims in material order, each `path`
// anchor and path backing re-rooted under the material's lend, and each turn
// listing its files relative to what it was lent.
#[tokio::test]
async fn three_materials() {
    let model = ByLend::default()
        .lend("./docs/a", Scripted::answering([NOTE]))
        .lend("./docs/b", Scripted::answering([NOTE]))
        .lend("./docs/c", Scripted::answering([NOTE]));

    let evidence =
        extract::<Split, _>(&model, &workspace("./docs")).await.expect("three materials join");

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
    for (lend, file) in [("./docs/a", "x.md"), ("./docs/b", "y.md"), ("./docs/c", "z.md")] {
        let seen = model.scripts[lend].seen();
        assert_eq!(seen.len(), 1, "one turn per material");
        assert_eq!(seen[0].workspace.as_deref(), Some(lend));
        let user = &seen[0].messages[0];
        assert!(user.contains(&format!("read-only view at `{lend}`")), "{user}");
        assert!(user.contains(&format!("nothing else:\n\n- `{file}`\n\n")), "{user}");
    }
    model.assert_exhausted();
}

// A survey wider than 4 holds exactly 4 completions
// pending at once — the fan-out is neither serial nor unbounded — and the
// joined document still reads in material order.
#[tokio::test]
async fn concurrent() {
    let mut model = ByLend::default();
    for i in 0..=4 {
        model = model.lend(&format!("./docs/d{i}"), Scripted::answering([NOTE]));
    }

    let evidence =
        extract::<Wide, _>(&model, &workspace("./docs")).await.expect("every material joins");

    assert_eq!(model.peak(), 4, "5 materials hold at most 4 completions pending");

    let expected: Vec<String> = (0..=4).map(|i| format!("d{i}/note.md#L1")).collect();
    assert_eq!(paths(&evidence), expected);

    model.assert_exhausted();
}

// An empty survey is refused before any model call: the input had nothing to
// mine.
#[tokio::test]
async fn empty_survey() {
    let model = Scripted::default();

    let error =
        extract::<Nothing, _>(&model, &workspace("./docs")).await.expect_err("nothing to mine");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("nothing to mine"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// Every material's lend is checked before the first model call, so a `Within`
// path that escapes the root refuses the source with no turn spent — even
// when an earlier material was sound.
#[tokio::test]
async fn escaping_path() {
    let model = Scripted::default();

    let error =
        extract::<Escape, _>(&model, &workspace("./docs")).await.expect_err("a path escapes");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("`../secret.md` escapes the source root"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// A `Within` material naming no file has nothing to mine; refused as the
// input's, before any model call.
#[tokio::test]
async fn empty_within() {
    let model = Scripted::default();

    let error = extract::<Blank, _>(&model, &workspace("./docs")).await.expect_err("no file");

    assert_eq!(error.code(), "bad_request");
    assert!(error.description().contains("names no file"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// `Within` over an inline value is the adapter's own defect — there is no
// tree to lend — so the class is the adapter's, not the operator's.
#[tokio::test]
async fn within_value() {
    let model = Scripted::default();

    let error = extract::<Split, _>(&model, &value("Ship it.")).await.expect_err("no tree to lend");

    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("not an inline value"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// One material of two fails: the fan-out waits for both, then the source
// fails under that material's class, naming it by index and no other.
#[tokio::test]
async fn one_material_fails() {
    let model = ByLend::default()
        .lend("./docs/a", Scripted::answering([NOTE]))
        .lend("./docs/b", Scripted::new([Err(ModelError::Backend("down".to_string()))]));

    let error =
        extract::<Pair, _>(&model, &workspace("./docs")).await.expect_err("one material failed");

    assert_eq!(error.code(), "bad_gateway");
    assert_eq!(
        error.description(),
        "`docs`: 1 of 2 materials failed:\n- material 1: backend failure: down"
    );
    model.assert_exhausted();
}

// Two materials of three fail differently: both are reported, in material
// order, and the first one's class carries.
#[tokio::test]
async fn two_materials_fail() {
    let model = ByLend::default()
        .lend(
            "./docs/a",
            Scripted::new([Err(ModelError::InvalidRequest("no such model".to_string()))]),
        )
        .lend("./docs/b", Scripted::answering([NOTE]))
        .lend("./docs/c", Scripted::new([Err(ModelError::Backend("down".to_string()))]));

    let error =
        extract::<Split, _>(&model, &workspace("./docs")).await.expect_err("two materials failed");

    assert_eq!(error.code(), "bad_request", "the first failure's class");
    assert_eq!(
        error.description(),
        "`docs`: 2 of 3 materials failed:\n- material 0: invalid request: no such model\n- \
         material 2: backend failure: down"
    );
    model.assert_exhausted();
}

// A survey of one is a single `evidence` call: its failure is the source's
// exactly as `evidence` reported it, with no material report around it.
#[tokio::test]
async fn single_material_passthrough() {
    let model = Scripted::new([Err(ModelError::Backend("down".to_string()))]);

    let error = extract::<Plain, _>(&model, &workspace("./docs"))
        .await
        .expect_err("the one material failed");

    assert_eq!(error.code(), "bad_gateway");
    assert_eq!(error.description(), "backend failure: down");
    model.assert_exhausted();
}
