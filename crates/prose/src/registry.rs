//! Document registry
//!
//! The runtime view of an embedded corpus: a [`Doc`] is one document with
//! its tree-relative path and body, and the lookup functions find a document
//! by that path. [`crate::registry!`] gives a crate its own `docs` accessor
//! over the table the build step generated.
//!
//! Paths are the stable names prompts and reference tools use to address
//! documents, so a lookup by path is the only interface the registry needs.

/// An embedded reference document.
#[derive(Clone, Copy, Debug)]
pub struct Doc {
    /// Tree-relative path.
    pub path: &'static str,
    /// Markdown body.
    pub body: &'static str,
}

/// Finds the document at `path` by binary search; `docs` must be sorted by
/// path.
#[must_use]
pub fn find<'d>(docs: &'d [Doc], path: &str) -> Option<&'d Doc> {
    docs.binary_search_by(|doc| doc.path.cmp(path)).ok().map(|idx| &docs[idx])
}

/// Returns the body of the document at `path`, or `None` when the build did
/// not embed it — a registry/tree mismatch the caller reports as its own
/// failure, never a silent miss.
#[must_use]
pub fn body(docs: &[Doc], path: &str) -> Option<&'static str> {
    find(docs, path).map(|doc| doc.body)
}

/// Includes the registry the crate's build script generated.
///
/// The build script's `emery_prose::emit` call writes `prose_docs.rs` into
/// `OUT_DIR`: the embedded document table and the
/// `pub fn docs() -> &'static [Doc]` accessor over it. This macro brings
/// [`Doc`] into scope for that file and includes it, so the module it expands
/// in exposes `docs()`.
///
/// ```ignore
/// mod registry {
///     emery_prose::registry!();
/// }
/// ```
#[macro_export]
macro_rules! registry {
    () => {
        use $crate::registry::Doc;

        include!(concat!(env!("OUT_DIR"), "/prose_docs.rs"));
    };
}
