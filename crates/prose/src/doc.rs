//! Defines embedded documents and path-based lookup functions.
//!
//! Document paths are stable, tree-relative identifiers. Both [`find`] and
//! [`body`] compare them exactly.

/// An embedded Markdown document.
#[derive(Clone, Copy, Debug)]
pub struct Doc {
    /// The path relative to the document tree, such as `prompts/extract.md`.
    pub path: &'static str,
    /// The document's Markdown body.
    pub body: &'static str,
}

/// Returns the document at `path`, if the table embeds one.
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
    docs.iter().find(|doc| doc.path == path)
}

/// Returns the body of the document at `path`, if the table embeds one.
///
/// # Examples
///
/// ```
/// use emery_prose::{Doc, body};
///
/// let docs = [Doc {
///     path: "prompts/extract.md",
///     body: "Extract every claim.",
/// }];
///
/// assert_eq!(body(&docs, "prompts/extract.md"), Some("Extract every claim."));
/// assert_eq!(body(&docs, "prompts/missing.md"), None);
/// ```
#[must_use]
pub fn body(docs: &[Doc], path: &str) -> Option<&'static str> {
    find(docs, path).map(|doc| doc.body)
}
