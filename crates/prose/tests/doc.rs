//! Asserts what callers can rely on from an embedded corpus.
//!
//! A document is found by its tree-relative path, its body comes back intact,
//! and asking for a path the build did not embed is `None` — the caller
//! reports the mismatch as its own failure, so it is never a silent miss and
//! never a panic.

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
