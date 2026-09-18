//! Verifies the SDK's runtime references against their tree.
//!
//! The table is held to `crates/sdk/prose/` the way an adapter's is held to
//! its own tree, with no imports of its own.

use std::path::Path;

use emery_sdk::prose::{RUNTIME, check};

// Every runtime reference is a root: an adapter's prompt links whichever it
// needs, so none is reached through another by rule, and every link between
// them stays inside the table.
#[test]
fn runtime() {
    let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("prose");
    let roots: Vec<&str> = RUNTIME.iter().map(|doc| doc.path).collect();
    let findings = check(RUNTIME, &tree, &roots, &[]);
    assert!(findings.is_empty(), "{}", findings.join("\n"));
}
