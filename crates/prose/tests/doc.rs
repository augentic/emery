//! Verifies exact path lookup in an embedded document table.
//!
//! Existing documents retain their complete bodies. Missing paths return
//! `None` rather than panicking.

use emery_prose::{Doc, body, find};

// A table written by hand, as a probe's is; `prose!` expands to the same shape.
static PROSE: &[Doc] = &[
    Doc {
        path: "build.md",
        body: "# build",
    },
    Doc {
        path: "guidance.md",
        body: "# guidance",
    },
    Doc {
        path: "references/verifier.md",
        body: "# verifier",
    },
];

#[test]
fn lookup() {
    assert_eq!(find(PROSE, "guidance.md").map(|doc| doc.body), Some("# guidance"));
    assert_eq!(find(PROSE, "references/verifier.md").map(|doc| doc.body), Some("# verifier"));
    assert!(find(PROSE, "missing.md").is_none());

    assert_eq!(body(PROSE, "build.md"), Some("# build"));
    assert_eq!(body(PROSE, "missing.md"), None);
}
