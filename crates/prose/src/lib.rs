//! Embeds a crate's prompts and reference documents at compile time.
//!
//! Prompts and references ship inside the binaries that use them — the
//! engine's synthesis prose, each adapter's extraction prose — rather than
//! being read from disk at run time. This crate is the shared way to do that:
//! [`prose!`] embeds the listed documents of a Markdown tree as a table of
//! [`Doc`]s the way `include_str!` embeds one file, [`find`] and [`body`]
//! look a document up in that table by path, and [`check`] is the test that
//! holds the table to the tree — every document listed, every link answered.
//!
//! # Examples
//!
//! A crate lists the documents of the `prose/` tree beside its `src/` and
//! reads one by path:
//!
//! ```
//! use emery_prose::Doc;
//!
//! static DOCS: &[Doc] =
//!     emery_prose::prose!("../tests/fixtures", ["prompts/extract.md", "references/ids.md"]);
//!
//! let prompt = emery_prose::body(DOCS, "prompts/extract.md");
//! assert!(prompt.is_some());
//! ```

mod check;
mod doc;

pub use self::check::check;
pub use self::doc::{Doc, body, find};

/// Embeds the listed documents of the Markdown tree at `root`, relative to the invoking file, as a table of [`Doc`]s.
///
/// The expansion is a `&'static [Doc]`, one entry per listed path in the
/// order listed: its `path` is the tree-relative path as written and its
/// body is the file at `root/path` as `include_str!` embeds it, so the two
/// cannot disagree and an edit to any listed file rebuilds the crate. A
/// listed document the tree does not hold fails the build at the invocation.
/// A document the tree holds but the list does not is simply not embedded;
/// [`check`] is the test that finds one.
///
/// # Examples
///
/// ```
/// use emery_prose::Doc;
///
/// static DOCS: &[Doc] =
///     emery_prose::prose!("../tests/fixtures", ["prompts/extract.md", "references/ids.md"]);
///
/// assert_eq!(
///     DOCS.iter().map(|doc| doc.path).collect::<Vec<_>>(),
///     ["prompts/extract.md", "references/ids.md"]
/// );
/// assert_eq!(DOCS[1].body, include_str!("../tests/fixtures/references/ids.md"));
/// ```
#[macro_export]
macro_rules! prose {
    ($root:literal, [$($path:literal),+ $(,)?]) => {
        &[$($crate::Doc {
            path: $path,
            body: ::core::include_str!(::core::concat!($root, "/", $path)),
        }),+]
    };
}
