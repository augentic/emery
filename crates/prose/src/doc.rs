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

/// Returns the tree-relative path of the file at `path`: what follows its `prose/` segment.
///
/// `../prose/prompts/extract.md` and `prose/prompts/extract.md` both yield
/// `prompts/extract.md`. The first `prose/` segment counts, so a document
/// beneath a nested `prose/` keeps that part of its path.
///
/// # Panics
///
/// Panics when no segment of `path` is `prose`. The macro calls this in a
/// `static` initializer, where the panic fails the build at the list.
#[doc(hidden)]
#[must_use]
pub const fn within(path: &'static str) -> &'static str {
    const TREE: &[u8] = b"prose/";
    let bytes = path.as_bytes();
    let mut start = 0;
    while start + TREE.len() <= bytes.len() {
        let at_segment = start == 0 || bytes[start - 1] == b'/';
        if at_segment && names_tree(bytes, start) {
            return path.split_at(start + TREE.len()).1;
        }
        start += 1;
    }
    panic!("a listed document must sit beneath a `prose/` directory");
}

const fn names_tree(bytes: &[u8], at: usize) -> bool {
    const TREE: &[u8] = b"prose/";
    let mut i = 0;
    while i < TREE.len() {
        if bytes[at + i] != TREE[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Returns the document at `path`, if the table embeds one.
///
/// # Examples
///
/// ```
/// use emery_prose::{Doc, find};
///
/// static PROSE: &[Doc] = &[
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
/// assert_eq!(find(PROSE, "references/ids.md").map(|doc| doc.body), Some("# Ids"));
/// assert!(find(PROSE, "references/missing.md").is_none());
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
