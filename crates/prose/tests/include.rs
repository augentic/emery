//! Asserts what `include_prose!` embeds from a tree on disk.
//!
//! The tree is named relative to the invoking file, every `.md` beneath it
//! is one `Doc` under its tree-relative path, the table is sorted so the
//! lookups' binary search holds, and each body is the file verbatim.

use emery_prose::{Doc, body};

static DOCS: &[Doc] = emery_prose::include_prose!("fixtures");

#[test]
fn tree() {
    let paths: Vec<&str> = DOCS.iter().map(|doc| doc.path).collect();
    assert_eq!(paths, ["prompts/extract.md", "references/ids.md"]);

    assert_eq!(body(DOCS, "references/ids.md"), Some(include_str!("fixtures/references/ids.md")));
    assert_eq!(body(DOCS, "prompts/extract.md"), Some(include_str!("fixtures/prompts/extract.md")));
}
