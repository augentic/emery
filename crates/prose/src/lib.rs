//! Embeds a crate's prompts and reference documents at compile time.
//!
//! Prompts and references ship inside the binaries that use them — the
//! engine's synthesis prose, each adapter's extraction prose — rather than
//! being read from disk at run time. This crate is the shared way to do that:
//! [`include_prose!`] embeds a Markdown tree as a table of [`Doc`]s the way
//! `include_str!` embeds one file, and [`find`] and [`body`] look a document
//! up in that table by path.
//!
//! # Examples
//!
//! A crate embeds the `prose/` tree beside its `src/` and reads a document by
//! path:
//!
//! ```
//! use emery_prose::Doc;
//!
//! static DOCS: &[Doc] = emery_prose::include_prose!("../tests/fixtures");
//!
//! let prompt = emery_prose::body(DOCS, "prompts/extract.md");
//! assert!(prompt.is_some());
//! ```

mod doc;

pub use doc::{Doc, body, find};
#[doc(hidden)]
pub use emery_prose_macros::include_prose as __include_prose;

/// Embeds the Markdown tree at `tree`, relative to the invoking file, as a table of [`Doc`]s.
///
/// The expansion is a `&'static [Doc]` holding every `.md` file beneath
/// `tree`, sorted by tree-relative path, each body embedded as `include_str!`
/// embeds it. Symlinked directories are followed. The build fails at the
/// invocation when the tree is missing or holds no document, when a relative
/// link in any document has no target, or when a symlink cycle is found.
///
/// Cargo tracks each embedded file, so an edit rebuilds the crate; a file
/// added to or removed from the tree is tracked only by a build script that
/// prints `cargo::rerun-if-changed=<tree>`.
///
/// # Examples
///
/// ```
/// use emery_prose::Doc;
///
/// static DOCS: &[Doc] = emery_prose::include_prose!("../tests/fixtures");
///
/// assert_eq!(
///     DOCS.iter().map(|doc| doc.path).collect::<Vec<_>>(),
///     ["prompts/extract.md", "references/ids.md",]
/// );
/// ```
#[macro_export]
macro_rules! include_prose {
    ($tree:literal) => {
        $crate::__include_prose!($crate::Doc, $tree)
    };
}
