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

/// Generates the `docs` accessor over the build-time `DOCS` table.
///
/// ```ignore
/// mod registry {
///     emery_prose::registry!();
/// }
/// ```
#[macro_export]
macro_rules! registry {
    () => {
        pub use $crate::registry::Doc;

        include!(concat!(env!("OUT_DIR"), "/prose_docs.rs"));

        /// Returns every embedded document, sorted by tree-relative path.
        #[must_use]
        pub fn docs() -> &'static [Doc] {
            DOCS
        }
    };
}
