#![warn(missing_docs, clippy::missing_errors_doc)]

//! Provides the types and functions an adapter is written with.
//!
//! A source adapter reads a [`SourceInput`] and returns typed [`Evidence`];
//! a target adapter builds a [`target::Slice`] of the plan into a workspace
//! and returns a [`target::Report`]. The crate root holds what every source
//! adapter uses, [`target`] what a target adapter uses, and the other modules
//! what some source adapters use:
//!
//! - [`SourceAdapter`] is what a source adapter implements: the kind of
//!   source it reads and its `extract`. [`export_source!`] exports a type
//!   implementing it as a WebAssembly component, answering the component's
//!   `metadata` through [`metadata`]. [`target::TargetAdapter`] and
//!   [`export_target!`] do the same for a target adapter, with
//!   [`target::metadata`] and [`target::build`] behind them.
//! - [`Context`], [`Seam`], and [`extract`](fn@extract) run extraction over
//!   the boundaries selected by an adapter, at most [`CONCURRENT`] at a time,
//!   laying a seam's files into its turn whole when they fit within
//!   [`INLINE_BYTES`].
//! - [`Doc`], [`prose!`], [`body`], and [`find`] embed and read adapter
//!   guidance; [`RUNTIME`] is the guidance every adapter shares, and
//!   [`check`] holds an adapter's list to its tree.
//! - [`workspace::list`] traverses workspace input under an adapter-defined
//!   filter.
//! - [`survey::seams`] is the shared survey pipeline for an adapter that
//!   parses its source: a [`survey::code::Tree`] read through the adapter's
//!   [`survey::code::Recogniser`] in, the facts laid, one turn put for the
//!   surfaces, the answer held to the tree, and the seams and `type` claims
//!   of a [`survey::Survey`] out. [`survey::surfaces`] beneath it puts the
//!   one turn over the [`survey::Facts`] an adapter renders itself and holds
//!   the [`survey::Inventory`] it answers to the tree.
//! - [`kebab`], [`unique`], [`push_unique`], [`survey::Lines`],
//!   [`survey::resolve`], [`survey::route`], and [`survey::tests`] are what
//!   a survey spells stems, once-each lists, line spans, import targets,
//!   routes, and stated behaviours with, each pure over strings and
//!   iterators and reading no parsed module.
//! - [`survey::code`] is the module an adapter's parser fills and the
//!   lookups every rule reads it through, and the tree over every module
//!   with the lookups that read it whole; [`survey::Dialect`] carries the
//!   names and spellings those lookups read of a language, one `static` per
//!   adapter.
//!
//! Contract types and [`Error`] are re-exported, allowing an adapter to depend
//! on this crate alone: the claim types, [`Anchor`] and [`BadAnchor`] for the
//! `path` grammar a survey spells anchors in, [`is_kebab`] for the stems it
//! derives, and the [`serde_json`] and [`tracing`] crates for claims of its
//! own and events beside this crate's. [`Source`] is among them for a host
//! program that calls an adapter the way the engine does; an adapter
//! implements [`SourceAdapter`], exported through [`export_source!`], and
//! never [`Source`].
//!
//! # Examples
//!
//! This adapter treats its input as a single mining seam:
//!
//! ```
//! use emery_sdk::{Doc, Error, Seam, SourceInput};
//!
//! pub static PROSE: &[Doc] = &[Doc {
//!     path: "extract.md",
//!     body: "Extract every requirement the brief states as a `requirement` claim.",
//! }];
//!
//! /// Returns the input as a single mining seam.
//! pub fn survey(_input: &SourceInput) -> Result<Vec<Seam>, Error> {
//!     Ok(vec![Seam::whole()])
//! }
//!
//! #[cfg(target_arch = "wasm32")]
//! mod guest {
//!     use emery_sdk::{Context, Error, Evidence, Model, SourceAdapter, SourceKind};
//!
//!     struct Adapter;
//!
//!     emery_sdk::export_source!(Adapter);
//!
//!     impl SourceAdapter for Adapter {
//!         const KIND: SourceKind = SourceKind::Intent;
//!
//!         async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
//!             let seams = super::survey(ctx.input)?;
//!             emery_sdk::extract(ctx, super::PROSE, &seams).await
//!         }
//!     }
//! }
//! # fn main() {}
//! ```
//!
//! # Vocabulary
//!
//! - **Seam**: the portion of a source handled by one model request. See
//!   [`Seam`].
//! - **Survey**: the adapter-specific step that divides an input into seams
//!   before extraction. It is a plain function over the input where the
//!   source alone decides the cut, or, where its surfaces are the model's to
//!   name, a parsed tree handed to [`survey::seams`], which puts one turn
//!   and derives the seams from the anchors it accepted; either way the
//!   stems, the closures, and the ids are code's.
//! - **Recogniser**: what an adapter recognises of its own language that
//!   the shared pipeline cannot — the bootstrap, a handler, a mount, what
//!   the code says at an anchor. See [`survey::code::Recogniser`].
//! - **Mine**: to put one seam to the model under the adapter's `extract.md`
//!   and gate its answer. [`extract`](fn@extract) mines every seam of a source.
//! - **Context**: the adapter identifier, source input, and model available to
//!   one extraction call. See [`Context`].
//! - **Lend**: the workspace directory made readable to the model for a seam,
//!   or handed to it to build into for a build, written through the build
//!   turn's `write_files` tool; whether it may be written is the
//!   deployment's grant.
//! - **Finding**: a validation problem returned to the model for correction.
//!   The host limits how many correction rounds are available.
//! - **Slice**: the unit a target adapter builds, one entry of the plan with
//!   its cut of the specification and the design. See [`target::Slice`].
//! - **Report**: what a build answers, the requirements it covered and the
//!   files it wrote, held to its slice by the report gate. See
//!   [`target::Report`].
//! - **Stem**: the first dotted segment of a claim id, `orders` in
//!   `orders.create`. A [`Seam`]'s `stems` hold its `requirement` and
//!   `criterion` ids to them, and the engine slices its plan by stem.
//! - **Anchor**: a claim's `path`, a file and its lines. A [`Seam`]'s
//!   `anchors` are the spans its survey found a behaviour can start at, and
//!   hold every `requirement`'s anchor to one of them.
//!
//! Fallible APIs return [`Error`]. Use [`bad_request!`] when an adapter rejects
//! unusable input.
//!
//! Progress is emitted through `tracing`. Every event names the source and,
//! within a seam, its index, or the slice a build is of. An adapter opens at
//! its guest environment's
//! `RUST_LOG`, which the Omnia runtime sets from the run's one tracing level:
//!
//! - `info` on a bare `emery` run, so this crate's progress reaches stderr.
//! - One step up per `-v` and one step down per `-q`.
//! - The operator's own `RUST_LOG` (`emery_sdk=debug`, `off`) when no flag is
//!   passed.

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
pub use emery_adapter::{BadPath, beneath, is_kebab};
pub use emery_prose::{Doc, body, check, find, prose};
pub use omnia_sdk::{Error, Model, bad_gateway, bad_request, not_found, server_error};
/// The JSON crate a [`Claim`]'s `extras` are built from, for an adapter that
/// joins claims of its own to what the model answered.
pub use serde_json;
/// The tracing crate, for an adapter's own events beside this crate's.
pub use tracing;

pub use self::extract::{CONCURRENT, INLINE_BYTES, Seam, extract};

/// The runtime references every adapter prompt may link.
///
/// `claims.md` defines claims and their gate; `reconciliation.md` explains
/// where accepted claims land. Prompts link these paths without listing them
/// in their own table; pass [`RUNTIME`] to [`check`] as imports.
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
/// [`KIND`](SourceAdapter::KIND). Its `extract` builds a [`Context`] from the
/// imported source input and the host model, puts it to the type's
/// [`extract`](SourceAdapter::extract), and converts the returned evidence or
/// error into the WIT records.
///
/// Invoke this macro inside a `#[cfg(target_arch = "wasm32")]` module because
/// the export interface exists only on WebAssembly targets. Adapters needing
/// custom guest behaviour may implement `export::Guest` directly.
#[macro_export]
macro_rules! export_source {
    ($adapter:ty $(,)?) => {
        const _: () = {
            struct Exported;
    $crate::export::export!(Exported with_types_in $crate::export);

            impl $crate::export::Guest for Exported {
                fn metadata(_id: $crate::export::AdapterId) -> $crate::export::AdapterMetadata {
                    $crate::export::AdapterMetadata::from($crate::metadata(
                        <$adapter as $crate::SourceAdapter>::KIND,
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
/// [`MERGE_RULES`](target::TargetAdapter::MERGE_RULES). Its `build` builds a
/// [`target::Context`] from the imported slice, the workspace root, and the
/// host model, puts it to the type's [`build`](target::TargetAdapter::build),
/// and converts the returned report or error into the WIT records; its
/// `verify` does the same over a [`target::VerifyContext`] and the type's
/// [`verify`](target::TargetAdapter::verify).
///
/// Invoke this macro inside a `#[cfg(target_arch = "wasm32")]` module because
/// the export interface exists only on WebAssembly targets. Adapters needing
/// custom guest behaviour may implement `target::export::Guest` directly.
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
                        <$adapter as $crate::target::TargetAdapter>::MERGE_RULES,
                    ))
                }

                async fn build(
                    id: $crate::target::export::AdapterId, slice: $crate::target::export::Slice,
                    workspace: String,
                ) -> Result<$crate::target::export::Report, $crate::target::export::Error> {
                    $crate::target::call::<$adapter>(id, slice, workspace).await
                }

                async fn verify(
                    id: $crate::target::export::AdapterId, workspace: String,
                ) -> Result<$crate::target::export::Verdict, $crate::target::export::Error>
                {
                    $crate::target::verify_call::<$adapter>(id, workspace).await
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
/// adapter's [`KIND`](SourceAdapter::KIND). The `emery-version` pin is this
/// SDK's own version, identifying the contract the adapter compiled against.
/// Build an [`AdapterMetadata`] directly only when the adapter must loosen or
/// tighten that pin.
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

/// A source adapter: the kind of source it reads and how it extracts one.
///
/// Implement it on a unit struct and hand that type to [`export_source!`],
/// which exports the `source-adapter` world over it.
pub trait SourceAdapter {
    /// The kind of source the adapter reads, which ranks its evidence against
    /// other sources'.
    const KIND: SourceKind;

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
