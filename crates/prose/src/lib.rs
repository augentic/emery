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
//!     emery_prose::prose!("tests/fixtures", ["prompts/extract.md", "references/ids.md"]);
//!
//! let prompt = emery_prose::body(PROSE, "prompts/extract.md");
//! assert!(prompt.is_some());
//! ```

mod check;
mod doc;

pub use self::check::check;
pub use self::doc::{Doc, body, find};

/// Embeds the listed Markdown files of a crate's tree as a static table of [`Doc`] values.
///
/// `prose![..]` reads the crate's `prose/` directory, beside its `Cargo.toml`,
/// which is where a prompt corpus lives:
///
/// ```text
/// pub static PROSE: &[Doc] = emery_prose::prose!["prompts/extract.md", "references/ids.md"];
/// ```
///
/// `prose!(root, [..])` reads another directory of the crate, `root` named
/// relative to its `Cargo.toml`. In either form each listed path is relative
/// to that tree and becomes one table entry, preserving the order written in
/// the invocation.
///
/// File bodies are included at compile time, like `include_str!`. A missing
/// listed file therefore fails the build. Files that exist under the tree but
/// are not listed are omitted; use [`check`] in a native test to detect them.
///
/// # Examples
///
/// This crate embeds no corpus of its own, so its fixtures are named by
/// their directory:
///
/// ```
/// use emery_prose::Doc;
///
/// static PROSE: &[Doc] =
///     emery_prose::prose!("tests/fixtures", ["prompts/extract.md", "references/ids.md"]);
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
            body: ::core::include_str!(::core::concat!(
                ::core::env!("CARGO_MANIFEST_DIR"), "/", $root, "/", $path
            )),
        }),+]
    };
    ($($path:literal),+ $(,)?) => {
        $crate::prose!("prose", [$($path),+])
    };
}
