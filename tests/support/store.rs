//! Seeds the scripted capabilities with a sealed revision or a staged candidate, as the engine writes one.

use emery_engine::vcs::{REVISION, SPECIFICATION};
use emery_engine::{EntryKind, STAGED_CONTAINER, STAGED_KEY};
use omnia_test::guest::Memory;
use sha2::{Digest as _, Sha256};

use super::FilesScript;

/// Scripts the three documents under `spec/` in the tree every revision working copy reads.
///
/// The scripted `Vcs` cuts every working copy at the head, so the tree reads
/// as the repository's head holding the revision: the shape a label merged
/// into the branch leaves, which needs no pointer and no label. Returns the
/// revision id.
pub fn seed(files: &FilesScript, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    files.lists(REVISION, ".", &[(SPECIFICATION, EntryKind::Directory)]);
    files.lists(
        REVISION,
        SPECIFICATION,
        &[
            ("spec.json", EntryKind::File),
            ("design.json", EntryKind::File),
            ("plan.json", EntryKind::File),
        ],
    );
    for (name, body) in [("spec", spec), ("design", design), ("plan", plan)] {
        files.holds_bytes(REVISION, &document(name), body);
    }
    revision(spec, design, plan)
}

/// Stores the three documents under their content id and stages it as the candidate.
///
/// Returns the candidate's id.
pub fn stage(storage: &Memory, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let id = revision(spec, design, plan);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/spec.json"), spec);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/design.json"), design);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/plan.json"), plan);
    storage.insert_state(STAGED_KEY, id.as_bytes());
    id
}

/// Scripts the revision `from` sealed last into the tree `to` reads, as a repository holding it does.
///
/// Returns the revision id, which is the digest of the written bytes.
pub fn carry(from: &FilesScript, to: &FilesScript) -> String {
    let spec = written(from, "spec");
    let design = written(from, "design");
    let plan = written(from, "plan");
    seed(to, &spec, &design, &plan)
}

/// The bytes last written to `spec/<name>.json` in the revision working copy.
pub fn written(files: &FilesScript, name: &str) -> Vec<u8> {
    let path = document(name);
    let writes = files.writes();
    let found =
        writes.into_iter().rev().find(|(at, written, _)| at == REVISION && *written == path);
    let Some((_, _, body)) = found else { panic!("{name} sealed") };
    body
}

/// The path a revision document lives at in the repository, `spec/<name>.json`.
pub fn document(name: &str) -> String {
    format!("{SPECIFICATION}/{name}.json")
}

/// The engine's content id: SHA-256 over the length-prefixed bodies, spec, design, then plan.
pub fn revision(spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for body in [spec, design, plan] {
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}
