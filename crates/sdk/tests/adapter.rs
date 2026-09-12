//! `SourceAdapter` contract
//!
//! A minimal adapter implemented against the trait, driven natively over a
//! scripted model. It shows the trait is complete enough to implement and
//! exercise without a wasm build — the promise adapter authors' own test
//! suites depend on — and that its provided members answer from the
//! adapter's own declarations: the `emery-version` pin, the extraction prompt.

use emery_prose::registry::Doc;
use emery_sdk::{
    AdapterMetadata, Context, Error, Evidence, Material, Model, SourceAdapter, SourceContent,
    SourceInput,
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
    const SOURCE: &'static str = "probe";

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(model: &P, ctx: &Context<'_>) -> Result<Evidence, Error> {
        Self::evidence(model, ctx, Material::Prepared(ctx.input.key.clone())).await
    }
}

// An adapter whose corpus lacks the extraction prompt.
struct Mute;

impl SourceAdapter for Mute {
    const SOURCE: &'static str = "mute";

    fn docs() -> &'static [Doc] {
        &[]
    }

    async fn extract<P: Model>(model: &P, ctx: &Context<'_>) -> Result<Evidence, Error> {
        Self::evidence(model, ctx, Material::Bound).await
    }
}

#[tokio::test]
async fn source_dispatch() {
    let model = Scripted::answering([
        r#"{"authority":"documentation","claims":[{"kind":"requirement","id":"one.claim","statement":"One."}]}"#,
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
    assert_eq!(
        model.seen()[0].system.as_deref(),
        Some("EXTRACT"),
        "the embedded `prompts/extract.md` is the system prompt"
    );

    assert_eq!(
        Probe::metadata(),
        AdapterMetadata {
            emery_version: PIN.map(str::to_string),
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
