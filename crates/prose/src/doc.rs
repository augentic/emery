//! One embedded document, and the lookups over a table of them.
//!
//! A [`Doc`] is one embedded document: its tree-relative path and its body.
//! [`find`] and [`body`] look one up in a table sorted by path. Paths are the
//! stable names prompts and reference tools address documents by, so lookup
//! by path is the whole interface.

/// One embedded document: its tree-relative path and its Markdown body.
#[derive(Clone, Copy, Debug)]
pub struct Doc {
    /// The path relative to the embedded tree, such as `prompts/extract.md`.
    pub path: &'static str,
    /// The document's Markdown body.
    pub body: &'static str,
}

/// Returns the document at `path`, if the table embeds one.
///
/// `docs` must be sorted by path, as a generated table is.
///
/// # Examples
///
/// ```
/// use emery_prose::{Doc, find};
///
/// static DOCS: &[Doc] = &[
///     Doc {
///         path: "prompts/extract.md",
///         body: "Extract every claim.",
///     },
///     Doc {
///         path: "references/ids.md",
///         body: "# Ids",
///     },
/// ];
///
/// assert_eq!(find(DOCS, "references/ids.md").map(|doc| doc.body), Some("# Ids"));
/// assert!(find(DOCS, "references/missing.md").is_none());
/// ```
#[must_use]
pub fn find<'d>(docs: &'d [Doc], path: &str) -> Option<&'d Doc> {
    docs.binary_search_by(|doc| doc.path.cmp(path)).ok().map(|idx| &docs[idx])
}

/// Returns the body of the document at `path`, if the table embeds one.
///
/// `None` means the build did not embed the document: a mismatch between the
/// table and the tree, which the caller reports as its own failure rather than
/// treating as a missing page.
#[must_use]
pub fn body(docs: &[Doc], path: &str) -> Option<&'static str> {
    find(docs, path).map(|doc| doc.body)
}
