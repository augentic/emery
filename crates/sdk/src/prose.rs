//! Provides the embedded-prose lookups and the runtime references every adapter shares.
//!
//! [`RUNTIME`] is the SDK's own document table: the references an adapter
//! prompt may link without listing them. The reference tools answer a
//! model's `read_doc` from an adapter's table first and then from [`RUNTIME`];
//! [`check`] accepts links into it when given it as the imports.
//!
//! # Examples
//!
//! Hold an adapter's table to its tree, with the runtime references as the
//! documents a link may name without the tree holding them:
//!
//! ```
//! use std::path::Path;
//!
//! use emery_sdk::Doc;
//! use emery_sdk::prose::{RUNTIME, check};
//!
//! static PROSE: &[Doc] = &[Doc {
//!     path: "extract.md",
//!     body: "Ids follow [claims.md](claims.md).",
//! }];
//!
//! # let dir = tempfile::tempdir()?;
//! # std::fs::write(dir.path().join("extract.md"), PROSE[0].body)?;
//! # let tree = dir.path();
//! let findings = check(PROSE, tree, &["extract.md"], RUNTIME);
//! assert!(findings.is_empty(), "{}", findings.join("\n"));
//! # Ok::<(), std::io::Error>(())
//! ```

use emery_prose::Doc;
pub use emery_prose::{body, check, find};

/// The runtime references every adapter prompt may link.
///
/// - `claims.md` — the claim `id` grammar, `path` anchors, the skip roots,
///   and the fail-closed gate.
/// - `reconciliation.md` — the `specify` pipeline and where extracted claims
///   land in it.
///
/// A prompt links them as it links the adapter's own references
/// (`claims.md` from `extract.md`), and the model reads them through
/// `read_doc` beside the adapter's table. An adapter never lists them: pass
/// this table to [`check`] as the imports, and a listed document at one of
/// these paths is a finding.
pub static RUNTIME: &[Doc] =
    emery_prose::prose!["../prose/claims.md", "../prose/reconciliation.md"];
