//! Embeds a crate's prompts and reference documents at build time.
//!
//! Prompts and references ship inside the binaries that use them — the
//! engine's synthesis prose, each adapter's extraction prose — rather than
//! being read from disk at run time. This crate is the shared way to do that:
//! [`emit`] walks a Markdown tree from a build script and generates a document
//! table, and [`mod@registry`] looks documents up in it by path.
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
//! mod registry {
//!     emery_prose::registry!();
//! }
//!
//! let prompt = emery_prose::registry::body(registry::docs(), "prompts/extract.md");
//! ```

pub mod registry;

#[cfg(feature = "emit")]
mod emit;

#[cfg(feature = "emit")]
pub use emit::emit;
