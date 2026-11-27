//! Seeds staging and revision trees for CLI scenarios.

use emery_engine::{STAGED_CONTAINER, STAGED_KEY};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use super::files::Files;
use super::storage::Storage;

pub fn seed(files: &Files, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    files.seed(spec, design, plan)
}

pub async fn stage(storage: &Storage, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let id = revision(spec, design, plan);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/spec.json"), spec).await;
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/design.json"), design).await;
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/plan.json"), plan).await;
    let record = serde_json::json!({"id": id, "kind": "revision"}).to_string();
    storage.insert_state(STAGED_KEY, record.as_bytes()).await;
    id
}

pub async fn stage_fresh(storage: &Storage, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
    let id = stage(storage, spec, design, plan).await;
    let record = serde_json::json!({"id": id, "kind": "revision", "fresh": true}).to_string();
    storage.insert_state(STAGED_KEY, record.as_bytes()).await;
    id
}

pub async fn stage_amendment(storage: &Storage, amendment: &[u8]) -> String {
    let id = digest(&[amendment]);
    storage.insert_object(STAGED_CONTAINER, &format!("{id}/amendment.json"), amendment).await;
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
    storage.insert_state(STAGED_KEY, record.as_bytes()).await;
    id
}

pub fn carry(from: &Files, to: &Files) -> String {
    from.carry(to)
}

pub fn written(files: &Files, name: &str) -> Vec<u8> {
    files.written(name)
}

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
