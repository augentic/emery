//! Embeds a crate's prompts and reference documents at build time.
//!
//! Prompts and references ship inside the binaries that use them — the
//! engine's synthesis prose, each adapter's extraction prose — rather than
//! being read from disk at run time. This crate is the shared way to do that:
//! [`emit`] walks a Markdown tree from a build script and generates a document
//! table, [`include_prose!`] includes that table where the crate wants its
//! `docs()`, and [`find`] and [`body`] look a [`Doc`] up in it by path.
//!
//! [`emit`] sits behind the `emit` feature, so a build script enables it and a
//! shipped guest never carries the walker.
//!
//! # Examples
//!
//! The build script's `main` embeds the crate's `prose/` tree:
//!
//! ```no_run
//! // build.rs, with `emery-prose = { features = ["emit"] }` as a build-dependency.
//! emery_prose::emit("prose");
//! ```
//!
//! The crate then includes the generated table and reads a document by path:
//!
//! ```ignore
//! // `ignore`: the included file exists only under the crate's own build script.
//! mod prose {
//!     emery_prose::include_prose!();
//! }
//!
//! let prompt = emery_prose::body(prose::docs(), "prompts/extract.md");
//! ```

mod doc;
#[cfg(feature = "emit")]
mod emit;

pub use doc::{Doc, body, find};
#[cfg(feature = "emit")]
pub use emit::emit;

/// Includes the document table the crate's build script generated.
///
/// The build script's [`emit`] call writes `prose_docs.rs` into `OUT_DIR`.
/// This macro brings [`Doc`] into scope and includes that file, so the module
/// it expands in exposes `pub fn docs() -> &'static [Doc]` over the embedded
/// documents.
///
/// # Examples
///
/// ```ignore
/// // `ignore`: the included file exists only under the crate's own build script.
/// mod prose {
///     emery_prose::include_prose!();
/// }
///
/// let prompt = emery_prose::body(prose::docs(), "prompts/extract.md");
/// ```
#[macro_export]
macro_rules! include_prose {
    () => {
        use $crate::Doc;

        include!(concat!(env!("OUT_DIR"), "/prose_docs.rs"));
    };
}
