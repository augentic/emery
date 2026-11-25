//! Defines source-adapter inputs, outputs, and the host import surface.
//!
//! [`SourceInput`] identifies the source to read. An adapter returns
//! [`Evidence`], whose claims are validated by [`Evidence::findings`].
//! The engine guest invokes loaded adapters through [`metadata`] and
//! [`extract`].
//!
//! On WebAssembly targets, `export` contains the guest interface implemented
//! by an adapter.

#[cfg(target_arch = "wasm32")]
pub(crate) mod bindings;
mod capability;
mod evidence;

#[cfg(target_arch = "wasm32")]
pub use bindings::export;
pub use capability::{SourceContent, SourceInput, SourceMetadata};
pub use evidence::{
    Anchor, Backing, BadAnchor, CLAIM_ID_REGEX, Claim, ClaimKind, Evidence, SourceKind,
};
use omnia_sdk::Error;

/// Returns the metadata the adapter registered as `id` declares.
///
/// # Panics
///
/// Panics when called off `wasm32`, where the host import is not linked.
#[must_use]
pub fn metadata(id: &str) -> SourceMetadata {
    #[cfg(target_arch = "wasm32")]
    {
        bindings::metadata(id)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
        unreachable!("source adapters are invoked only from wasm32 guests")
    }
}

/// Extracts evidence from `input` using the adapter registered as `id`.
///
/// An input refusal is [`omnia_sdk::ErrorKind::BadRequest`]; any other adapter failure is
/// [`omnia_sdk::ErrorKind::BadGateway`].
///
/// # Errors
///
/// Returns [`Error`] when the adapter refuses the call or the answer fails
/// validation.
///
/// # Panics
///
/// Panics when called off `wasm32`, where the host import is not linked.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::unused_async,
        reason = "native builds keep the wasm32 signature until the host links the import"
    )
)]
pub async fn extract(id: &str, input: &SourceInput) -> Result<Evidence, Error> {
    #[cfg(target_arch = "wasm32")]
    {
        bindings::extract(id, input).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (id, input);
        unreachable!("source adapters are invoked only from wasm32 guests")
    }
}
