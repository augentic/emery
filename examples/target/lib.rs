//! Demonstrates a minimal Emery target adapter.
//!
//! The adapter builds each slice of the plan into the lent working copy
//! through the SDK's one gated turn, verifies each integrated wave through
//! another, and declares how two slices' writes to one file merge. It shows
//! the parts of a target adapter:
//!
//! - An embedded build prompt and an embedded verify prompt.
//! - A merge rule: both slices' lines kept when two write one `index.md`.
//! - A WebAssembly guest: an `emery_sdk::target::TargetAdapter` exported
//!   with `emery_sdk::target_adapter!`.

use emery_sdk::Doc;

#[cfg(target_arch = "wasm32")]
mod guest {
    use std::borrow::Cow;

    use emery_sdk::target::{
        Context, MergeRule, MergeStrategy, Report, TargetAdapter, Verdict, VerifyContext,
    };
    use emery_sdk::{Error, Model};

    struct Adapter;

    emery_sdk::target_adapter!(Adapter);

    impl TargetAdapter for Adapter {
        // two slices that list requirements in one index keep both lists
        const MERGE_RULES: &'static [MergeRule] = &[MergeRule {
            paths: Cow::Borrowed("build/*/index.md"),
            strategy: MergeStrategy::Union,
        }];

        async fn build<P: Model>(ctx: &Context<'_, P>) -> Result<Report, Error> {
            emery_sdk::target::build(ctx, super::PROSE).await
        }

        async fn verify<P: Model>(ctx: &VerifyContext<'_, P>) -> Result<Verdict, Error> {
            emery_sdk::target::verify(ctx, super::PROSE).await
        }
    }
}

// The crate is a `cdylib` whose one caller is the `wasm32` guest above.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "read by the guest module alone")
)]
static PROSE: &[Doc] = emery_sdk::prose!["prose/build.md", "prose/verify.md"];
