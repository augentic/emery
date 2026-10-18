//! Verifies the SDK's embedded runtime reference documents.

use std::path::Path;

use emery_sdk::{RUNTIME, check};

// Every runtime reference is a root: an adapter's prompt links whichever it
// needs, so none is reached through another by rule.
#[test]
fn runtime() {
    let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("prose");
    let roots: Vec<&str> = RUNTIME.iter().map(|doc| doc.path).collect();
    let findings = check(RUNTIME, &tree, &roots, &[]);
    assert!(findings.is_empty(), "{}", findings.join("\n"));
}
