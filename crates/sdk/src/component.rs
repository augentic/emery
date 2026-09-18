//! Exports adapter functions through the `source-adapter` guest interface.
//!
//! [`source_adapter!`](crate::source_adapter) converts WIT inputs into SDK
//! types, supplies the host model through [`Context`](crate::Context), and
//! converts the adapter's result back into WIT records.

#[cfg(target_arch = "wasm32")]
use crate::{AdapterMetadata, Context, Error, Evidence, Model, SourceInput, export};

/// The default model provider supplied to adapter extraction functions.
///
/// [`source_adapter!`](crate::source_adapter) places this provider in each
/// [`Context`](crate::Context). It delegates model requests through Omnia's
/// WebAssembly interface.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Default)]
pub struct Provider;

#[cfg(target_arch = "wasm32")]
impl Model for Provider {}

/// Exports an adapter's metadata and extraction functions as a component.
///
/// The arguments must identify functions with these signatures:
///
/// - `fn() -> AdapterMetadata`
/// - `async fn<P: Model>(&Context<'_, P>) -> Result<Evidence, Error>`
///
/// The macro supplies a [`Context`](crate::Context) containing the imported
/// source input and host model. It then converts the returned evidence or
/// error into the `source-adapter` WIT records.
///
/// Invoke this macro inside a `#[cfg(target_arch = "wasm32")]` module because
/// the export interface exists only on WebAssembly targets. Adapters needing
/// custom guest behaviour may implement `export::Guest` directly.
///
/// # Examples
///
/// ```
/// # use emery_sdk::{Doc, Error, Seam, SourceInput};
/// # pub static PROSE: &[Doc] = &[Doc { path: "extract.md", body: "Extract." }];
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
///         emery_sdk::extract(ctx, super::PROSE, &seams).await
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
                    $crate::component::metadata($metadata)
                }

                async fn extract(
                    id: $crate::export::AdapterId, input: $crate::export::Input,
                ) -> Result<$crate::export::Evidence, $crate::export::Error> {
                    $crate::component::extract($extract, id, input).await
                }
            }
        };
    };
}

/// Converts an adapter's metadata response into the guest record.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn metadata(answer: impl FnOnce() -> AdapterMetadata) -> export::AdapterMetadata {
    answer().into()
}

/// Runs an adapter extraction with converted input and the host model.
///
/// # Errors
///
/// Returns the adapter's error converted to the guest error record.
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
