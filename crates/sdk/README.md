# emery-sdk

The SDK an [Emery](https://github.com/augentic/emery) source adapter is written with.

A source adapter is a WebAssembly component that reads one kind of source — an operator's brief, a documentation tree, a codebase — and returns typed claims for Emery to reconcile into a specification. This crate is the adapter's one dependency: the `source_adapter!` export, `extract` (one gated model turn per seam, joined into one document), `workspace::list` and `survey::surfaces` for dividing a source into seams, and the embedded prose an adapter's prompts are read from.

```rust
#[cfg(target_arch = "wasm32")]
mod guest {
    use emery_sdk::{AdapterMetadata, Context, Error, Evidence, Model, SourceKind};

    emery_sdk::source_adapter!(metadata, extract);

    fn metadata() -> AdapterMetadata {
        emery_sdk::metadata(SourceKind::Documentation)
    }

    async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
        let seams = crate::survey::survey(ctx.input)?;
        emery_sdk::extract(ctx, crate::PROSE, &seams).await
    }
}
```

The adapter writes `survey`, which chooses the seams; the SDK owns every model turn, the claim gate, and the component boundary.

- API documentation: [docs.rs/emery-sdk](https://docs.rs/emery-sdk), built for `wasm32-wasip2` so the `export` module is present
- Authoring guide: [emery-adapters/docs/authoring.md](https://github.com/augentic/emery-adapters/blob/main/docs/authoring.md)
- First-party adapters: [augentic/emery-adapters](https://github.com/augentic/emery-adapters)

Licensed under MIT or Apache-2.0, at your option.
