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
mod doc;

pub use self::check::check;
#[doc(hidden)]
pub use self::doc::within;
pub use self::doc::{Doc, body, find};

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
        &[$($crate::Doc {
            path: $crate::within($path),
            body: ::core::include_str!($path),
        }),+]
    };
}
