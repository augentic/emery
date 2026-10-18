#![warn(missing_docs, clippy::missing_errors_doc)]

//! Provides the types and functions an adapter is written with.
//!
//! A source adapter implements [`SourceAdapter`] and exports it with
//! [`export_source!`], surveying a [`SourceInput`] into [`Seam`]s and calling
//! [`extract`](fn@extract) to return validated [`Evidence`]. A target adapter
//! implements [`target::TargetAdapter`] and exports it with [`export_target!`],
//! handling each [`target::Task`] through [`target::build`] and
//! [`target::verify`].
//!
//! Contract types, [`Doc`] / [`prose!`] prompt helpers, [`Error`],
//! [`serde_json`], and [`tracing`] are re-exported so an adapter can depend on
//! this crate alone.
//!
//! # Examples
//!
//! This adapter treats its input as a single mining seam:
//!
//! ```
//! use emery_sdk::{
//!     Context, Doc, Error, Evidence, Metadata, Model, Seam, SourceAdapter, SourceInput,
//!     SourceKind,
//! };
//!
//! const KIND: SourceKind = SourceKind::Intent;
//!
//! pub struct Adapter;
//!
//! impl SourceAdapter for Adapter {
//!     fn metadata() -> Metadata {
//!         Metadata { kind: KIND }
//!     }
//!
//!     async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
//!         let seams = survey(ctx.input)?;
//!         emery_sdk::extract(ctx, PROSE, &seams).await
//!     }
//! }
//!
//! pub static PROSE: &[Doc] = &[Doc {
//!     path: "extract.md",
//!     body: "Extract every requirement the brief states as a `requirement` claim.",
//! }];
//!
//! fn survey(_input: &SourceInput) -> Result<Vec<Seam>, Error> {
//!     Ok(vec![Seam::whole()])
//! }
//!
//! #[cfg(target_arch = "wasm32")]
//! emery_sdk::export_source!(Adapter);
//! # fn main() {}
//! ```
//!
//! # Vocabulary
//!
//! - **Seam**: the portion of a source handled by one model request ([`Seam`]).
//! - **Survey**: the adapter step that divides an input into seams, either by
//!   code alone or through [`survey::seams`] / [`survey::surfaces`].
//! - **Recogniser**: language-specific rules for a parsed [`survey::code::Tree`]
//!   ([`survey::code::Recogniser`]).
//! - **Mine**: to run [`extract`](fn@extract) over a source's seams under
//!   `extract.md` and gate the returned claims.
//! - **Slice**: one plan entry built by [`target::build`] ([`target::Slice`]).
//! - **Report**: the covered requirements and written paths returned by a build
//!   ([`target::Report`]).

mod extract;
mod reference;
pub mod survey;
pub mod target;
pub mod workspace;

#[cfg(target_arch = "wasm32")]
#[doc(inline)]
pub use emery_adapter::source::export;
pub use emery_adapter::source::{
    AdapterMetadata, Anchor, Backing, BadAnchor, Claim, ClaimKind, Evidence, Source, SourceContent,
    SourceInput, SourceKind,
};
pub use emery_adapter::{BadPath, Lexicon, beneath, is_kebab};
pub use emery_prose::{Doc, body, check, find, prose};
pub use omnia_sdk::{Error, Model, bad_gateway, bad_request, not_found, server_error};
/// The JSON crate a [`Claim`]'s `extras` are built from, for an adapter that
/// joins claims of its own to what the model answered.
pub use serde_json;
/// The tracing crate, for an adapter's own events beside this crate's.
pub use tracing;

pub use self::extract::{CONCURRENT, INLINE_BYTES, Seam, extract};

/// The runtime references every source adapter prompt may link.
///
/// `claims.md` defines claims and their gate; `reconciliation.md` explains
/// where accepted claims land. Prompts link these paths without listing them
/// in their own table; pass [`RUNTIME`] to [`check`] as imports. A source's
/// turns offer them through the reference tools beside the adapter's own; a
/// target's turns, which write code and never claims, offer the adapter's
/// alone.
pub static RUNTIME: &[Doc] = prose!["../prose/claims.md", "../prose/reconciliation.md"];

// The documents a turn's system prompt is built from: the adapter's prompt,
// and the claim rules within `RUNTIME` that every mining turn carries after
// `extract.md`.
const EXTRACT: &str = "extract.md";
const CLAIMS: &str = "claims.md";
// The prompt of a survey by model, for the adapter that puts one.
const SURVEY: &str = "survey.md";
// The prompts of a target adapter's build turn and its verify turn.
const BUILD: &str = "build.md";
const VERIFY: &str = "verify.md";

/// Exports a [`SourceAdapter`] as the `source-adapter` world of a component.
///
/// The argument is a type implementing [`SourceAdapter`]. The component's
/// `metadata` answers [`metadata`](fn@metadata) over the type's
/// [`Metadata::kind`]. Its `extract` builds a [`Context`] from the
/// imported source input and the host model, puts it to the type's
/// [`extract`](SourceAdapter::extract), and converts the returned evidence or
/// error into the WIT records.
///
/// Invoke this macro under `#[cfg(target_arch = "wasm32")]`, since the export
/// interface exists only on WebAssembly targets; the type and its impl need
/// no gate, so a native test can call them as the component does. Adapters
/// needing custom guest behaviour may implement `export::Guest` directly.
#[macro_export]
macro_rules! export_source {
    ($adapter:ty $(,)?) => {
        const _: () = {
            struct Exported;
    $crate::export::export!(Exported with_types_in $crate::export);

            impl $crate::export::Guest for Exported {
                fn metadata(_id: $crate::export::AdapterId) -> $crate::export::AdapterMetadata {
                    $crate::export::AdapterMetadata::from($crate::metadata(
                        <$adapter as $crate::SourceAdapter>::metadata().kind,
                    ))
                }

                async fn extract(
                    id: $crate::export::AdapterId, input: $crate::export::Input,
                ) -> Result<$crate::export::Evidence, $crate::export::Error> {
                    $crate::call::<$adapter>(id, input).await
                }
            }
        };
    };
}

/// Exports a [`target::TargetAdapter`] as the `target-adapter` world of a component.
///
/// The argument is a type implementing [`target::TargetAdapter`]. The
/// component's `metadata` answers [`target::metadata`] over the type's
/// [`rules`](target::Metadata::rules). Its `run` lifts the imported task
/// into a [`target::Task`] over the workspace root and the host model — a
/// [`target::InitContext`], a [`target::BuildContext`], or a
/// [`target::VerifyContext`], the last two carrying the type's
/// [`grants`](target::Metadata::grants) — puts it to the type's
/// [`run`](target::TargetAdapter::run), holds the outcome to the task's
/// kind, and converts it or the error into the WIT records.
///
/// Invoke this macro under `#[cfg(target_arch = "wasm32")]`, since the export
/// interface exists only on WebAssembly targets; the type and its impl need
/// no gate, so a native test can call them as the component does. Adapters
/// needing custom guest behaviour may implement `target::export::Guest`
/// directly.
#[macro_export]
macro_rules! export_target {
    ($adapter:ty $(,)?) => {
        const _: () = {
            struct Exported;
    $crate::target::export::export!(Exported with_types_in $crate::target::export);

            impl $crate::target::export::Guest for Exported {
                fn metadata(
                    _id: $crate::target::export::AdapterId,
                ) -> $crate::target::export::TargetMetadata {
                    $crate::target::export::TargetMetadata::from($crate::target::metadata(
                        <$adapter as $crate::target::TargetAdapter>::metadata().rules,
                    ))
                }

                async fn run(
                    id: $crate::target::export::AdapterId, workspace: String,
                    task: $crate::target::export::Task,
                ) -> Result<$crate::target::export::Outcome, $crate::target::export::Error>
                {
                    $crate::target::call::<$adapter>(id, workspace, task).await
                }
            }
        };
    };
}

/// The host model a turn is put to on WebAssembly.
///
/// [`export_source!`] and [`export_target!`] bind it into every context
/// they build; the empty [`Model`] impl delegates each request through
/// Omnia's WASI interface.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Default)]
pub struct Provider;

#[cfg(target_arch = "wasm32")]
impl Model for Provider {}

/// The `extract` arm of [`export_source!`], reached through the macro alone.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
#[omnia_wasi_otel::instrument(name = "source_adapter_extract")]
pub async fn call<A: SourceAdapter>(
    id: export::AdapterId, input: export::Input,
) -> Result<export::Evidence, export::Error> {
    let input = SourceInput::from(input);
    let ctx = Context {
        adapter_id: &id,
        input: &input,
        model: &Provider,
    };
    Ok(A::extract(&ctx).await?.into())
}

/// Returns the `metadata` answer for an adapter reading `kind` sources.
///
/// [`export_source!`] answers the component's `metadata` with it over the
/// adapter's [`Metadata::kind`]. The `emery-version` pin is this SDK's own
/// version, identifying the contract the adapter compiled against. Build an
/// [`AdapterMetadata`] directly only when the adapter must loosen or tighten
/// that pin.
#[must_use]
pub fn metadata(kind: SourceKind) -> AdapterMetadata {
    AdapterMetadata {
        emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        kind,
    }
}

/// Returns `text` spelled as lowercase kebab-case, or `None` when nothing is left.
///
/// Camel humps split, anything that is not a letter or digit becomes a
/// hyphen, and runs of hyphens collapse. The result passes [`is_kebab`].
///
/// # Examples
///
/// ```
/// use emery_sdk::kebab;
///
/// assert_eq!(kebab("OrdersAPI"), Some("orders-api".to_owned()));
/// assert_eq!(kebab("HTTPServer"), Some("http-server".to_owned()));
/// assert_eq!(kebab("get_user__by id"), Some("get-user-by-id".to_owned()));
/// assert_eq!(kebab("--"), None);
/// ```
#[must_use]
pub fn kebab(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len() + 4);
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && i > 0 {
                let prev = chars[i - 1];
                let next = chars.get(i + 1).copied();
                let hump = prev.is_ascii_lowercase()
                    || prev.is_ascii_digit()
                    || (prev.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase()));
                if hump && !out.ends_with('-') && !out.is_empty() {
                    out.push('-');
                }
            }
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    is_kebab(trimmed).then(|| trimmed.to_owned())
}

/// Pushes `item` onto `into` unless an equal item is already there.
pub fn push_unique<T: PartialEq>(into: &mut Vec<T>, item: T) {
    if !into.contains(&item) {
        into.push(item);
    }
}

/// Returns `items` with each equal item kept once, in first-occurrence order.
///
/// # Examples
///
/// ```
/// use emery_sdk::unique;
///
/// assert_eq!(unique(["b", "a", "b", "c", "a"]), ["b", "a", "c"]);
/// ```
#[must_use]
pub fn unique<T: PartialEq>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut list = Vec::new();
    for item in items {
        push_unique(&mut list, item);
    }
    list
}

/// The adapter addressed, its input, and the model available to one extraction call.
///
/// [`extract`](fn@extract) takes it, so every turn of a call is put to the
/// one model the host bound.
#[derive(Debug)]
pub struct Context<'a, P> {
    /// The identifier used to address the adapter.
    pub adapter_id: &'a str,
    /// The [`SourceInput`] identifying the source and its content.
    pub input: &'a SourceInput,
    /// The [`Model`] used for extraction requests.
    pub model: &'a P,
}

/// What a source adapter declares about itself: the kind of source it reads.
///
/// [`SourceAdapter::metadata`] answers it from the adapter's compiled-in
/// constants, and [`export_source!`] answers the component's `metadata`
/// over its [`kind`](Self::kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metadata {
    /// The kind of source the adapter reads, which ranks its evidence against
    /// other sources'.
    pub kind: SourceKind,
}

/// A source adapter: the kind of source it reads and how it extracts one.
///
/// Implement it on a unit struct and hand that type to [`export_source!`],
/// which exports the `source-adapter` world over it.
pub trait SourceAdapter {
    /// Returns what the adapter declares about itself: the kind of source it
    /// reads.
    ///
    /// Answered from compiled-in constants, with no model call and no
    /// filesystem access.
    #[must_use]
    fn metadata() -> Metadata;

    /// Extracts the evidence of the source `ctx` carries.
    ///
    /// Every turn goes through [`extract`](fn@extract), over the seams the
    /// adapter's survey chose.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] when the adapter refuses its input, and
    /// what [`extract`](fn@extract) returns otherwise.
    fn extract<P: Model>(ctx: &Context<'_, P>) -> impl Future<Output = Result<Evidence, Error>>;
}

// A missing document is a build's own defect, the adapter's for its prompt and
// the SDK's for a runtime reference, reported before a turn is spent.
fn prompt(docs: &[Doc], path: &str) -> Result<&'static str, Error> {
    body(docs, path).ok_or_else(|| server_error!("`{path}` is not embedded"))
}
