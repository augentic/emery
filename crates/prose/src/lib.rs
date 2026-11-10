#![warn(missing_docs, clippy::missing_errors_doc)]

//! Embeds Markdown prompts and reference documents into Rust binaries.
//!
//! [`prose!`] creates a static table of [`Doc`] values from files selected at
//! compile time. [`find`] and [`body`] retrieve documents by their
//! tree-relative paths.
//!
//! [`check`] validates an embedded table against its source tree and prompt
//! graph. It is intended for native tests, where the original files are
//! available.
//!
//! # Examples
//!
//! Embed selected files and read one by its tree-relative path:
//!
//! ```
//! use emery_prose::Doc;
//!
//! static PROSE: &[Doc] =
//!     emery_prose::prose!["../tests/prose/extract.md", "../tests/prose/references/ids.md"];
//!
//! let prompt = emery_prose::body(PROSE, "extract.md");
//! assert!(prompt.is_some());
//! ```

mod check;

pub use self::check::check;

/// An embedded Markdown document.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Doc {
    /// The path relative to the document tree, such as `references/ids.md`.
    pub path: &'static str,
    /// The document's Markdown body.
    pub body: &'static str,
}

impl Doc {
    /// Creates an embedded document with the given tree-relative `path` and `body`.
    #[must_use]
    pub const fn new(path: &'static str, body: &'static str) -> Self {
        Self { path, body }
    }
}

/// Returns the document at `path`, if the table embeds one.
#[must_use]
pub fn find<'d>(docs: &'d [Doc], path: &str) -> Option<&'d Doc> {
    docs.iter().find(|doc| doc.path == path)
}

/// Returns the body of the document at `path`, if the table embeds one.
#[must_use]
pub fn body(docs: &[Doc], path: &str) -> Option<&'static str> {
    find(docs, path).map(|doc| doc.body)
}

/// Embeds the listed Markdown files as a static table of [`Doc`] values.
///
/// Each path names a file relative to the invoking source file, the way
/// `include_str!` does, and the file must sit beneath a `prose/` directory:
/// its table path is what follows that segment, so
/// `"../prose/references/ids.md"` is embedded as `references/ids.md` and
/// `"../prose/extract.md"` as `extract.md`. Entries keep the order written
/// in the invocation.
///
/// File bodies are included at compile time, like `include_str!`. A missing
/// listed file, or one outside a `prose/` directory, therefore fails the
/// build. Files that exist under the tree but are not listed are omitted; use
/// [`check`] in a native test to detect them.
///
/// # Examples
///
/// ```
/// use emery_prose::Doc;
///
/// static PROSE: &[Doc] =
///     emery_prose::prose!["../tests/prose/extract.md", "../tests/prose/references/ids.md"];
///
/// assert_eq!(
///     PROSE.iter().map(|doc| doc.path).collect::<Vec<_>>(),
///     ["extract.md", "references/ids.md"]
/// );
/// assert_eq!(PROSE[1].body, include_str!("../tests/prose/references/ids.md"));
/// ```
#[macro_export]
macro_rules! prose {
    ($($path:literal),+ $(,)?) => {
        &[$($crate::Doc::new(
            $crate::within($path),
            ::core::include_str!($path),
        )),+]
    };
}

#[doc(hidden)]
#[must_use]
pub const fn within(path: &'static str) -> &'static str {
    let bytes = path.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if (i == 0 || bytes[i - 1] == b'/')
            && matches!(bytes.split_at(i).1, [b'p', b'r', b'o', b's', b'e', b'/', b, ..] if *b != b'/')
        {
            return path.split_at(i + "prose/".len()).1;
        }
        i += 1;
    }
    panic!("a listed document must sit beneath a `prose/` directory");
}
