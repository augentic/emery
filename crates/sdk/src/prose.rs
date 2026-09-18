//! Provides the shared runtime references and corpus validation for adapters.
//!
//! [`RUNTIME`] is the SDK's own document table: the references every adapter
//! prompt may link under the `emery/` prefix. The reference tools answer a
//! model's `read_doc` from an adapter's table first and then from [`RUNTIME`],
//! and [`check`] accepts links into it.

use std::path::Path;

use emery_prose::Doc;
pub use emery_prose::{body, find};

/// The runtime references every adapter prompt may link under `emery/`.
///
/// - `emery/claims.md` — the claim `id` grammar, `path` anchors, the skip
///   roots, and the fail-closed gate.
/// - `emery/reconciliation.md` — the `specify` pipeline and where extracted
///   claims land in it.
///
/// A prompt links them as it links the adapter's own references
/// (`../emery/claims.md` from `prompts/extract.md`), and the model reads them
/// through `read_doc` beside the adapter's table. An adapter never lists them:
/// a table holding a document at one of these paths fails [`check`].
pub static RUNTIME: &[Doc] = emery_prose::prose!["emery/claims.md", "emery/reconciliation.md"];

/// Returns inconsistencies between an adapter's `docs`, its tree, and its prompts.
///
/// The findings are the prose crate's, with [`RUNTIME`] as the imports: every
/// Markdown file beneath `root` is listed once in `docs`, every relative link
/// names a listed document or a runtime reference, every path in `prompts`
/// is listed, every other listed document is reached from a prompt through
/// those links, and no listed document shadows a runtime reference. Use it in
/// a native test beside the adapter's [`prose!`](crate::prose!) invocation.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use emery_sdk::Doc;
///
/// static PROSE: &[Doc] = &[Doc {
///     path: "prompts/extract.md",
///     body: "Ids follow [claims.md](../emery/claims.md).",
/// }];
///
/// # let dir = tempfile::tempdir()?;
/// # std::fs::create_dir(dir.path().join("prompts"))?;
/// # std::fs::write(dir.path().join("prompts/extract.md"), PROSE[0].body)?;
/// # let tree = dir.path();
/// let findings = emery_sdk::prose::check(PROSE, tree, &["prompts/extract.md"]);
/// assert!(findings.is_empty(), "{}", findings.join("\n"));
/// # Ok::<(), std::io::Error>(())
/// ```
#[must_use]
pub fn check(docs: &[Doc], root: &Path, prompts: &[&str]) -> Vec<String> {
    emery_prose::check(docs, root, prompts, RUNTIME)
}
