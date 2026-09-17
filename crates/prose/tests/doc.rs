//! Verifies exact path lookup in an embedded document table.
//!
//! Existing documents retain their complete bodies. Missing paths return
//! `None` rather than panicking.

use emery_prose::{Doc, body, find};

// A table written by hand, as a probe's is; `prose!` expands to the same shape.
static DOCS: &[Doc] = &[
    Doc {
        path: "prompts/build.md",
        body: "# build",
    },
    Doc {
        path: "prompts/guidance.md",
        body: "# guidance",
    },
    Doc {
        path: "references/verifier.md",
        body: "# verifier",
    },
];

#[test]
fn lookup() {
    assert_eq!(find(DOCS, "prompts/guidance.md").map(|doc| doc.body), Some("# guidance"));
    assert_eq!(find(DOCS, "references/verifier.md").map(|doc| doc.body), Some("# verifier"));
    assert!(find(DOCS, "prompts/missing.md").is_none());

    assert_eq!(body(DOCS, "prompts/build.md"), Some("# build"));
    assert_eq!(body(DOCS, "prompts/missing.md"), None);
}
