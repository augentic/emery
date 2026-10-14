//! Demonstrates a minimal Emery target adapter.
//!
//! The adapter builds each slice of the plan into the lent working copy
//! through the SDK's one gated turn, verifies each integrated wave through
//! another, and declares how two slices' writes to one file merge. It shows
//! the parts of a target adapter:
//!
//! - An embedded build prompt and an embedded verify prompt.
//! - A seed: the one file every tree of its layout starts from, laid before
//!   a slice's turn when the tree lacks it.
//! - A merge rule: both slices' lines kept when two write one `index.md`.
//! - A base: the commits the adapter builds over, any by default, a fresh
//!   one when `EMERY_MOCK_FRESH` was set as the adapter compiled.
//! - A WebAssembly guest: an `emery_sdk::target::TargetAdapter` exported
//!   with `emery_sdk::export_target!`.

use emery_sdk::Doc;
use emery_sdk::target::Seed;

#[cfg(target_arch = "wasm32")]
mod guest {
    use std::borrow::Cow;

    use emery_sdk::target::{
        Base, Context, MergeRule, MergeStrategy, Report, TargetAdapter, Verdict, VerifyContext,
    };
    use emery_sdk::{Error, Model};

    struct Adapter;

    emery_sdk::export_target!(Adapter);

    impl TargetAdapter for Adapter {
        // one source, two components: the runtime suite builds the mock once
        // over any base and once over a fresh one
        const BASE: Base = match option_env!("EMERY_MOCK_FRESH") {
            Some(_) => Base::Fresh,
            None => Base::Any,
        };
        // two slices that list requirements in one index keep both lists
        const MERGE_RULES: &'static [MergeRule] = &[MergeRule {
            paths: Cow::Borrowed("build/*/index.md"),
            strategy: MergeStrategy::Union,
        }];

        async fn build<P: Model>(ctx: &Context<'_, P>) -> Result<Report, Error> {
            emery_sdk::target::build(ctx, super::PROSE, super::SEED).await
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

// The layout's one root file, which the first slice to build into a tree
// lacking it lays, and every later slice finds.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "read by the guest module alone")
)]
static SEED: &[Seed] = &[Seed::new(
    "build/README.md",
    "# Build\n\nOne directory per slice, each with an `index.md` naming the requirements it \
     implements and one file per requirement.\n",
)];
