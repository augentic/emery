# emery-sdk

The SDK an [Emery](https://github.com/augentic/emery) source adapter is written with.

A source adapter is a WebAssembly component that reads one kind of source — an operator's brief, a documentation tree, a codebase — and returns typed claims for Emery to reconcile into a specification. This crate is the adapter's one dependency: the `SourceAdapter` trait and the `export_source!` export, `extract` (one gated model turn per seam, joined into one document), `workspace::list` for dividing a source into seams, and the embedded prose an adapter's prompts are read from.

```rust
#[cfg(target_arch = "wasm32")]
mod guest {
    use emery_sdk::{Context, Error, Evidence, Model, SourceAdapter, SourceKind};

    struct Adapter;

    emery_sdk::export_source!(Adapter);

    impl SourceAdapter for Adapter {
        const KIND: SourceKind = SourceKind::Documentation;

        async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
            let seams = crate::survey::survey(ctx.input)?;
            emery_sdk::extract(ctx, crate::PROSE, &seams).await
        }
    }
}
```

The adapter writes `survey`, which chooses the seams — from the source alone, or through the shared pipeline for a parsed tree: the adapter reads its workspace into a `survey::code::Tree` through the `survey::code::Recogniser` it implements for what its language alone decides (the bootstrap, a handler, a mount, what the code says at an anchor), and `survey::seams` lays the facts, has the model name the surfaces in one turn, holds the answer to the tree, derives the stems, ids, and closures from the accepted anchors, and cuts the seams. The SDK owns every model turn, the claim gate, and the component boundary. What a survey derives by code — stems, line spans, import targets, route discriminators, the behaviours a tree's own tests state — it spells with `kebab` and `survey::{Lines, resolve, route, tests}`, each pure over strings. A survey that parses its source fills a `survey::code::Module` per file and reads it through the lookups there, each reading the language's names and spellings from the adapter's `survey::Dialect`.

- API documentation: [docs.rs/emery-sdk](https://docs.rs/emery-sdk), built for `wasm32-wasip2` so the `export` module is present
- First-party adapters, and the shape a new one takes: [augentic/emery-adapters](https://github.com/augentic/emery-adapters)

Licensed under MIT or Apache-2.0, at your option.
