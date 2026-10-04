//! Seeds scripted storage with a committed revision, as the engine writes one.

use emery_engine::{CONTAINER, REVISION_KEY};
use omnia_test::guest::Memory;
use sha2::{Digest as _, Sha256};

/// Stores the three documents under their content id and makes it current.
///
/// Returns the revision id.
pub fn seed(storage: &Memory, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let id = revision(spec, design, plan);
    storage.insert_object(CONTAINER, &format!("{id}/spec.json"), spec);
    storage.insert_object(CONTAINER, &format!("{id}/design.json"), design);
    storage.insert_object(CONTAINER, &format!("{id}/plan.json"), plan);
    storage.insert_state(REVISION_KEY, id.as_bytes());
    id
}

// The engine's content id: SHA-256 over the length-prefixed bodies, spec,
// design, then plan.
fn revision(spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for body in [spec, design, plan] {
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}
