//! Drives a minimal `SourceAdapter` natively over a scripted model.
//!
//! The trait is complete enough to implement and exercise without a wasm
//! build — the promise adapter authors' own test suites depend on — and its
//! provided members answer from the adapter's own declarations: the
//! `emery-version` pin and the kind of source in `metadata`, the extraction
//! prompt, the survey's seam.

use std::future::{Future, ready};

use emery_sdk::{
    AdapterMetadata, Context, Doc, Error, Model, Seam, SourceAdapter, SourceContent, SourceInput,
    SourceKind,
};
use omnia_test::guest::Scripted;

const DOCS: &[Doc] = &[Doc {
    path: "prompts/extract.md",
    body: "EXTRACT",
}];

// The SDK's own version is the default `emery-version` pin.
const PIN: Option<&str> = Some(env!("CARGO_PKG_VERSION"));

struct Probe;

impl SourceAdapter for Probe {
    const KIND: SourceKind = SourceKind::Documentation;

    fn docs() -> &'static [Doc] {
        DOCS
    }

    // Mechanical: the key is the note, with no model turn.
    fn survey<P: Model>(
        _model: &P, ctx: &Context<'_>,
    ) -> impl Future<Output = Result<Vec<Seam>, Error>> + Send {
        ready(Ok(vec![Seam::Note(ctx.input.key.clone())]))
    }
}

// An adapter whose corpus lacks the extraction prompt; its survey is the
// default.
struct Mute;

impl SourceAdapter for Mute {
    const KIND: SourceKind = SourceKind::Behaviour;

    fn docs() -> &'static [Doc] {
        &[]
    }
}

#[tokio::test]
async fn source_dispatch() {
    let model = Scripted::answering([
        r#"{"claims":[{"kind":"requirement","id":"one.claim","statement":"One."}]}"#,
    ]);
    let input = SourceInput {
        key: "main".to_string(),
        content: SourceContent::Value(String::new()),
    };
    let ctx = Context {
        adapter_id: "source:probe",
        input: &input,
    };

    let evidence = Probe::extract(&model, &ctx).await.expect("scripted extract succeeds");
    assert_eq!(evidence.claims.len(), 1);
    assert_eq!(evidence.claims[0].id.as_deref(), Some("one.claim"));
    let request = &model.seen()[0];
    assert_eq!(
        request.system.as_deref(),
        Some("EXTRACT"),
        "the embedded `prompts/extract.md` is the system prompt"
    );
    assert!(
        request.messages[0].contains("\n\nmain\n\n"),
        "the survey's note is the turn's seam: {}",
        request.messages[0]
    );

    // The kind is the adapter's constant, reported where the engine reads it
    // before any extract.
    assert_eq!(
        Probe::metadata(),
        AdapterMetadata {
            emery_version: PIN.map(str::to_string),
            kind: Probe::KIND,
        }
    );
    assert_eq!(Probe::docs()[0].path, "prompts/extract.md");
}

// A corpus without `prompts/extract.md` is the adapter build's own defect,
// reported before a model call is spent.
#[tokio::test]
async fn missing_prompt() {
    let model = Scripted::answering(Vec::<String>::new());
    let input = SourceInput {
        key: "main".to_string(),
        content: SourceContent::Workspace(".".to_string()),
    };
    let ctx = Context {
        adapter_id: "source:mute",
        input: &input,
    };

    let error = Mute::extract(&model, &ctx).await.expect_err("no prompt to ask with");
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`prompts/extract.md` is not embedded"), "{error}");
    assert!(model.seen().is_empty(), "nothing was asked");
}
