//! The guest side of the `source-adapter` world: the one export an adapter makes.
//!
//! [`source_adapter!`](crate::source_adapter) binds an adapter's two plain
//! fns — its `metadata` answer and its `extract` — as the world's exports.
//! The lift of the WIT input and the host's model onto a
//! [`Context`](crate::Context), and the lowering of the outcome onto the
//! world's `evidence` and `error`, happen here, so an adapter's own code
//! names the contract types alone and no backend at all.

#[cfg(target_arch = "wasm32")]
use crate::{AdapterMetadata, Context, Error, Evidence, Model, SourceInput, export};

/// The host's model on omnia's WASI defaults: the one capability an adapter's call carries.
///
/// The lift behind [`source_adapter!`](crate::source_adapter) puts it in the
/// [`Context`](crate::Context) of every call, so an adapter written over the
/// macro never names it; a guest written by hand builds its `Context` with
/// `model: &Provider`, or with a provider of its own. It is the unit struct
/// with the empty [`Model`] impl every omnia guest would otherwise declare.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Default)]
pub struct Provider;

#[cfg(target_arch = "wasm32")]
impl Model for Provider {}

/// Exports the `source-adapter` world over an adapter's `metadata` and `extract` fns.
///
/// The two paths are the world's two exports, in the WIT's order. The first
/// names a plain `fn() -> AdapterMetadata` — [`metadata`](crate::metadata)
/// for the kind of source the adapter reads. The second names an
/// `async fn<P: Model>(&Context<'_, P>) -> Result<Evidence, Error>`: the
/// adapter's own survey for the [seams](crate#vocabulary), then
/// [`mine`](crate::mine) over them, and nothing else. The macro implements
/// the world's `Guest` on a private type and invokes the bindings' `export!`
/// for it; the WIT input and the host's model, `Provider`, are lifted onto
/// a [`Context`](crate::Context) before the adapter's `extract` is called,
/// and its outcome is lowered onto the WIT `evidence` and `error` after, so
/// neither fn names a binding or a backend. A fn of another shape is refused
/// where the macro names it.
///
/// The expansion rides the `export` module, which exists on `wasm32` alone,
/// so the macro is invoked inside the guest's `#[cfg(target_arch = "wasm32")]`
/// module; a guest with needs of its own implements `export::Guest` by hand
/// instead.
///
/// # Examples
///
/// ```
/// # use emery_sdk::{Doc, Error, Seam, SourceInput};
/// # pub static DOCS: &[Doc] = &[Doc { path: "prompts/extract.md", body: "Extract." }];
/// # pub fn survey(_input: &SourceInput) -> Result<Vec<Seam>, Error> {
/// #     Ok(vec![Seam::Whole])
/// # }
/// #[cfg(target_arch = "wasm32")]
/// mod guest {
///     use emery_sdk::{AdapterMetadata, Context, Error, Evidence, Model, SourceKind};
///
///     emery_sdk::source_adapter!(metadata, extract);
///
///     fn metadata() -> AdapterMetadata {
///         emery_sdk::metadata(SourceKind::Documentation)
///     }
///
///     async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
///         let seams = super::survey(ctx.input)?;
///         emery_sdk::mine(ctx, super::DOCS, &seams).await
///     }
/// }
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! source_adapter {
    ($metadata:path, $extract:path $(,)?) => {
        const _: () = {
            struct Adapter;
    $crate::export::export!(Adapter with_types_in $crate::export);

            impl $crate::export::Guest for Adapter {
                fn metadata(_id: $crate::export::AdapterId) -> $crate::export::AdapterMetadata {
                    $crate::guest::metadata($metadata)
                }

                async fn extract(
                    id: $crate::export::AdapterId, input: $crate::export::Input,
                ) -> Result<$crate::export::Evidence, $crate::export::Error> {
                    $crate::guest::extract($extract, id, input).await
                }
            }
        };
    };
}

/// Lowers the adapter's `metadata` answer onto the world's record.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn metadata(answer: impl FnOnce() -> AdapterMetadata) -> export::AdapterMetadata {
    answer().into()
}

/// Lifts the WIT input and the host's model onto a [`Context`], runs the adapter's `extract`, and lowers the outcome.
///
/// # Errors
///
/// Whatever the adapter's `extract` returns, lowered onto the WIT `error`.
#[cfg(target_arch = "wasm32")]
pub async fn extract(
    answer: impl AsyncFnOnce(&Context<'_, Provider>) -> Result<Evidence, Error>,
    id: export::AdapterId, input: export::Input,
) -> Result<export::Evidence, export::Error> {
    let input = SourceInput::from(input);
    let ctx = Context {
        adapter_id: &id,
        input: &input,
        model: &Provider,
    };
    Ok(answer(&ctx).await?.into())
}
