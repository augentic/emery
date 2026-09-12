//! `SourceAdapter` contract
//!
//! A minimal adapter implemented against the trait, driven natively over a
//! scripted model. It shows the trait is complete enough to implement and
//! exercise without a wasm build, which is the promise adapter authors' own
//! test suites depend on.

use emery_adapter::types::{AdapterMetadata, Context, Evidence, SourceInput};
use emery_adapter::{Error, EvidenceTurn, Model, SourceAdapter, evidence};
use emery_prose::registry::Doc;
use omnia_test::guest::Scripted;

const DOCS: &[Doc] = &[Doc {
    path: "prompts/extract.md",
    body: "EXTRACT",
}];

// The SDK's own version is the default `emery-version` pin.
const PIN: Option<&str> = Some(env!("CARGO_PKG_VERSION"));

struct Probe;

impl SourceAdapter for Probe {
    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        let turn = EvidenceTurn::prepared("probe", input.key.clone());
        evidence(model, ctx, input, "SYSTEM", turn).await
    }
}

#[tokio::test]
async fn source_dispatch() {
    let model = Scripted::answering([
        r#"{"authority":"documentation","claims":[{"kind":"requirement","id":"one.claim","statement":"One."}]}"#,
    ]);
    let ctx = Context {
        adapter_id: "source:probe",
        docs: DOCS,
        lend: Some("."),
    };

    let evidence = Probe::extract(&model, &ctx, &SourceInput::value("main", ""))
        .await
        .expect("scripted extract succeeds");
    assert_eq!(evidence.claims.len(), 1);
    assert_eq!(evidence.claims[0].id.as_deref(), Some("one.claim"));

    assert_eq!(
        <Probe as SourceAdapter>::metadata(),
        AdapterMetadata {
            emery_version: PIN.map(str::to_string),
        }
    );
    assert_eq!(<Probe as SourceAdapter>::docs()[0].path, "prompts/extract.md");
}
