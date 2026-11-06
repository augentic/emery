//! Seeds the scripted capabilities with a sealed revision or a staged candidate, as the engine writes one.

use emery_engine::vcs::{REVISION, SPECIFICATION};
use emery_engine::{EntryKind, LAYOUTS, STAGED_CONTAINER, STAGED_KEY};
use omnia_test::guest::Memory;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::files::FilesScript;

// Scripts the three documents under `spec/` in the tree every revision working copy reads.
//
// The scripted `Vcs` cuts every working copy at the head, so the tree reads
// as the repository's head holding the revision: the shape a label merged
// into the branch leaves, which needs no pointer and no label. Returns the
// revision id.
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

// Scripts `layouts`, each a file name and its bytes, under `spec/layouts/` in
// the tree every revision working copy reads, beside the seeded documents.
pub fn seed_layouts(files: &FilesScript, layouts: &[(String, Vec<u8>)]) {
    files.lists(
        REVISION,
        SPECIFICATION,
        &[
            ("spec.json", EntryKind::File),
            ("design.json", EntryKind::File),
            ("plan.json", EntryKind::File),
            (LAYOUTS, EntryKind::Directory),
        ],
    );
    let names: Vec<(&str, EntryKind)> =
        layouts.iter().map(|(name, _)| (name.as_str(), EntryKind::File)).collect();
    files.lists(REVISION, &layouts_dir(), &names);
    for (name, body) in layouts {
        files.holds_bytes(REVISION, &layout(name), body);
    }
}

// The files `spec/layouts/` holds in the revision working copy, each name and its bytes.
pub fn sealed_layouts(files: &FilesScript) -> Vec<(String, Vec<u8>)> {
    let listing = files
        .listings
        .lock()
        .expect("listings")
        .get(&(REVISION.to_owned(), layouts_dir()))
        .cloned()
        .unwrap_or_default();
    let contents = files.contents.lock().expect("contents");
    listing
        .into_iter()
        .map(|entry| {
            let body = contents
                .get(&(REVISION.to_owned(), layout(&entry.name)))
                .cloned()
                .flatten()
                .unwrap_or_default();
            (entry.name, body)
        })
        .collect()
}

// `design` with a `## UI / layout` section naming `body` as one layout, and
// the layout's digest; the body's view a placeholder image.
pub fn laid_out(design: &[u8], body: &[u8]) -> (Vec<u8>, String) {
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(body)));
    let mut design: Value = serde_json::from_slice(design).expect("the design is JSON");
    design["sections"].as_array_mut().expect("sections").push(serde_json::json!({
        "kind": "ui-layout",
        "blocks": [{"layout": {
            "digest": digest,
            "media-type": "application/json",
            "size": body.len(),
            "source": "page",
            "views": [{"label": "320px wide", "media-type": "image/svg+xml", "body": "<svg/>"}],
        }}],
    }));
    let mut bytes = serde_json::to_vec_pretty(&design).expect("the design serialises");
    bytes.push(b'\n');
    (bytes, digest)
}

// The path a layout of digest hex `hex` lives at in the repository, `spec/layouts/<hex>`.
pub fn layout(hex: &str) -> String {
    format!("{}/{hex}", layouts_dir())
}

fn layouts_dir() -> String {
    format!("{SPECIFICATION}/{LAYOUTS}")
}

// Stores the three documents under their content id and stages it as the candidate.
//
// The staged slot holds the record the engine writes: the id and the kind.
// Returns the candidate's id.
pub fn stage(storage: &Memory, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let id = revision(spec, design, plan);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/spec.json"), spec);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/design.json"), design);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/plan.json"), plan);
    let record = serde_json::json!({"id": id, "kind": "revision"}).to_string();
    storage.insert_state(STAGED_KEY, record.as_bytes());
    id
}

// Stages the three documents as `emery specify --fresh` does: the record
// marks the candidate `fresh`, so `apply` seals it with no parent over the
// head. Returns the candidate's id.
pub fn stage_fresh(storage: &Memory, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let id = stage(storage, spec, design, plan);
    let record = serde_json::json!({"id": id, "kind": "revision", "fresh": true}).to_string();
    storage.insert_state(STAGED_KEY, record.as_bytes());
    id
}

// Stores the amendment document under its content id and stages it, as `emery amend` does.
//
// The staged slot holds the record the engine writes for an amendment: the
// id, the kind, and the base the document names. Returns the amendment's id.
pub fn stage_amendment(storage: &Memory, amendment: &[u8]) -> String {
    let id = digest(&[amendment]);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/amendment.json"), amendment);
    let document: Value = serde_json::from_slice(amendment).expect("amendment JSON");
    let record = serde_json::json!({
        "id": id,
        "kind": "amendment",
        "base": {
            "revision": document["base"]["revision"],
            "commit": document["base"]["commit"],
        },
    })
    .to_string();
    storage.insert_state(STAGED_KEY, record.as_bytes());
    id
}

// The staged revision candidate's id, read from the record at `STAGED_KEY`.
pub fn staged(storage: &Memory) -> String {
    staged_at(storage, STAGED_KEY)
}

// The staged revision candidate's id at `key`, for a namespaced store.
pub fn staged_at(storage: &Memory, key: &str) -> String {
    let raw = storage.state(key).expect("staged record");
    let record: Value = serde_json::from_slice(&raw).expect("staged record JSON");
    assert_eq!(record["kind"], "revision", "{record}");
    record["id"].as_str().expect("staged record id").to_owned()
}

// Scripts the revision `from` sealed last into the tree `to` reads, as a repository holding it does.
//
// The layouts `from` holds under `spec/layouts/` are carried beside the
// documents. Returns the revision id, which is the digest of the written bytes.
pub fn carry(from: &FilesScript, to: &FilesScript) -> String {
    let spec = written(from, "spec");
    let design = written(from, "design");
    let plan = written(from, "plan");
    let id = seed(to, &spec, &design, &plan);
    let layouts = sealed_layouts(from);
    if !layouts.is_empty() {
        seed_layouts(to, &layouts);
    }
    id
}

// The bytes last written to `spec/<name>.json` in the revision working copy.
pub fn written(files: &FilesScript, name: &str) -> Vec<u8> {
    let path = document(name);
    let writes = files.writes();
    let found =
        writes.into_iter().rev().find(|(at, written, _)| at == REVISION && *written == path);
    let Some((_, _, body)) = found else { panic!("{name} sealed") };
    body
}

// The path a revision document lives at in the repository, `spec/<name>.json`.
pub fn document(name: &str) -> String {
    format!("{SPECIFICATION}/{name}.json")
}

// The engine's content id: SHA-256 over the length-prefixed bodies, spec, design, then plan.
pub fn revision(spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    digest(&[spec, design, plan])
}

fn digest(bodies: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for body in bodies {
        hasher.update((body.len() as u64).to_be_bytes());
        hasher.update(body);
    }
    hex::encode(hasher.finalize())
}
