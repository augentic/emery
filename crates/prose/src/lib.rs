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
//! Embed selected files and read one by path:
//!
//! ```
//! use emery_prose::Doc;
//!
//! static PROSE: &[Doc] =
//!     emery_prose::prose!("../tests/fixtures", ["prompts/extract.md", "references/ids.md"]);
//!
//! let prompt = emery_prose::body(PROSE, "prompts/extract.md");
//! assert!(prompt.is_some());
//! ```

mod check;
mod doc;

pub use self::check::check;
pub use self::doc::{Doc, body, find};

/// Embeds selected Markdown files as a static table of [`Doc`] values.
///
/// `root` is relative to the source file invoking the macro. Each listed path
/// is relative to that root and becomes one table entry, preserving the order
/// written in the invocation.
///
/// File bodies are included at compile time, like `include_str!`. A missing
/// listed file therefore fails the build. Files that exist under `root` but
/// are not listed are omitted; use [`check`] in a native test to detect them.
///
/// # Examples
///
/// ```
/// use emery_prose::Doc;
///
/// static PROSE: &[Doc] =
///     emery_prose::prose!("../tests/fixtures", ["prompts/extract.md", "references/ids.md"]);
///
/// assert_eq!(
///     PROSE.iter().map(|doc| doc.path).collect::<Vec<_>>(),
///     ["prompts/extract.md", "references/ids.md"]
/// );
/// assert_eq!(PROSE[1].body, include_str!("../tests/fixtures/references/ids.md"));
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
