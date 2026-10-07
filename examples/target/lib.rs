//! Demonstrates a minimal Emery target adapter.
//!
//! The adapter builds each slice of the plan into the lent project tree
//! through the SDK's one gated turn. It shows the two parts of a target
//! adapter:
//!
//! - An embedded build prompt.
//! - A WebAssembly guest: an `emery_sdk::target::TargetAdapter` exported
//!   with `emery_sdk::target_adapter!`.

use emery_sdk::Doc;

#[cfg(target_arch = "wasm32")]
mod guest {
    use emery_sdk::target::{Context, Report, TargetAdapter};
    use emery_sdk::{Error, Model};

    struct Adapter;

    emery_sdk::target_adapter!(Adapter);

    impl TargetAdapter for Adapter {
        async fn build<P: Model>(ctx: &Context<'_, P>) -> Result<Report, Error> {
            emery_sdk::target::build(ctx, super::PROSE).await
        }
    }
}

// The crate is a `cdylib` whose one caller is the `wasm32` guest above.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(dead_code, reason = "read by the guest module alone")
)]
static PROSE: &[Doc] = emery_sdk::prose!["prose/build.md"];
