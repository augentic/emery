use tracing::level_filters::LevelFilter;

use crate::{Context, Error, Evidence, Model, SourceInput, TRACING, export, level};

/// The default model provider supplied to adapter extraction functions.
///
/// It delegates model requests through Omnia's WebAssembly interface.
#[derive(Clone, Copy, Debug, Default)]
pub struct Provider;

impl Model for Provider {}

#[doc(hidden)]
pub async fn call(
    extract: impl AsyncFnOnce(&Context<'_, Provider>) -> Result<Evidence, Error>, adapter: &str,
    id: export::AdapterId, input: export::Input,
) -> Result<export::Evidence, export::Error> {
    // The scope starts at `error`; reload before `run` opens its instrumented span.
    omnia_wasi_otel::scope(move || async move {
        reload(adapter);
        run(extract, id, input).await
    })
    .await
}

fn reload(adapter: &str) {
    let level = omnia_wasi_otel::baggage().get(TRACING).map_or(LevelFilter::INFO, |value| {
        let value = value.as_str();
        value.parse().unwrap_or_else(|err| {
            eprintln!("tracing level `{value}` not recognised, opening at info: {err}");
            LevelFilter::INFO
        })
    });

    if let Err(error) = omnia_wasi_otel::set_filter(&level::directives(level, adapter)) {
        eprintln!("tracing filter not reloaded: {error:#}");
    }
}

#[omnia_wasi_otel::instrument(name = "source_adapter_extract")]
async fn run(
    extract: impl AsyncFnOnce(&Context<'_, Provider>) -> Result<Evidence, Error>,
    id: export::AdapterId, input: export::Input,
) -> Result<export::Evidence, export::Error> {
    let input = SourceInput::from(input);
    let ctx = Context {
        adapter_id: &id,
        input: &input,
        model: &Provider,
    };
    Ok(extract(&ctx).await?.into())
}
