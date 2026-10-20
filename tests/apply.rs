//! Verifies `emery apply` and `emery specify --apply` over scripted storage.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use emery_engine::{CONTAINER, REVISION_KEY, STAGED_CONTAINER, STAGED_KEY};
use omnia_sdk::{BlobStore, CasError, ContainerMetadata, ObjectMetadata, StateStore};
use omnia_test::guest::Memory;
use serde_json::Value;
use support::{Provider, cli, cli_ok, fail, seed, stage};

// The one-source reference `specify` names; the scripted loader lands any
// exact package reference.
const SOURCE: &str = "acme:source@1.0.0";

const SPEC_ANSWER: &str = include_str!("specify/spec-draft.json");
const DESIGN_ANSWER: &str = include_str!("specify/design-draft.json");

const SPEC: &[u8] = include_bytes!("build/spec.json");
const DESIGN: &[u8] = include_bytes!("build/design.json");
const PLAN: &[u8] = include_bytes!("build/plan.json");
const WIDE_PLAN: &[u8] = include_bytes!("build/wide-plan.json");

// A staged candidate becomes the current revision: its documents land in the
// revision container under the same id, the staged slot is cleared and its
// documents pruned, and the output names the sealed revision alone, since a
// first seal displaces nothing.
#[tokio::test]
async fn apply_seals() {
    let provider = Provider::idle();
    let id = stage(&provider.storage, SPEC, DESIGN, PLAN);

    let resp = cli_ok(&provider, &["emery", "apply"]).await;

    assert_eq!(String::from_utf8_lossy(&resp.stdout), format!("sealed revision {id}\n"));
    assert_eq!(current(&provider.storage), id, "the candidate is the current revision");
    for name in ["spec.json", "design.json", "plan.json"] {
        assert!(
            provider.storage.object(CONTAINER, &format!("{id}/{name}")).is_some(),
            "{name} is sealed under the revision's id"
        );
    }
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "the candidate is pruned");

    // the sealed revision is what show and a second apply see
    cli_ok(&provider, &["emery", "show", "plan"]).await;
    fail(&provider, &["emery", "show", "plan", "--staged"], 2, "nothing-staged").await;
    fail(&provider, &["emery", "apply"], 2, "nothing-staged").await;
}

// Sealing over a current revision displaces it: the outgoing revision is
// pruned, the output carries the diff against it, and the envelope names
// the parent and the kind of candidate sealed.
#[tokio::test]
async fn apply_displaces() {
    let provider = Provider::idle();
    let outgoing = seed(&provider.storage, SPEC, DESIGN, PLAN);
    let candidate = stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    assert_ne!(outgoing, candidate);

    let resp = cli_ok(&provider, &["emery", "--format", "json", "apply"]).await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], candidate, "{envelope}");
    assert_eq!(envelope["parent"], outgoing, "{envelope}");
    assert_eq!(envelope["kind"], "revision", "{envelope}");
    assert_eq!(envelope["diff"]["from"], outgoing, "{envelope}");
    assert_eq!(
        envelope["diff"]["plan"]["added"],
        serde_json::json!([{"id": "SLICE-003", "name": "orders"}]),
        "the diff is against the revision displaced: {envelope}"
    );
    assert_eq!(current(&provider.storage), candidate);
    assert!(
        provider.storage.object(CONTAINER, &format!("{outgoing}/plan.json")).is_none(),
        "the displaced revision is pruned"
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");

    // the text render carries the same diff on one line
    let provider = Provider::idle();
    seed(&provider.storage, SPEC, DESIGN, PLAN);
    stage(&provider.storage, SPEC, DESIGN, WIDE_PLAN);
    let resp = cli_ok(&provider, &["emery", "apply"]).await;
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "sealed revision {candidate}\n  diff vs {outgoing}: spec +0 -0 ~0, design +0 -0 ~0, \
             plan +1 -0 ~2 preamble\n"
        )
    );
}

// Sealing a candidate identical to the current revision keeps the id and
// prunes nothing, since the displaced documents are the sealed ones.
#[tokio::test]
async fn apply_same_revision() {
    let provider = Provider::idle();
    let id = seed(&provider.storage, SPEC, DESIGN, PLAN);
    assert_eq!(stage(&provider.storage, SPEC, DESIGN, PLAN), id);

    let resp = cli_ok(&provider, &["emery", "apply"]).await;

    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!("sealed revision {id}\n  diff vs {id}: none (byte-stable)\n")
    );
    assert_eq!(current(&provider.storage), id);
    assert!(provider.storage.object(CONTAINER, &format!("{id}/spec.json")).is_some());
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
}

// With nothing staged the verb refuses typed, writes nothing, and the hint
// names `emery specify` as the way to stage a candidate.
#[tokio::test]
async fn apply_nothing_staged() {
    let provider = Provider::idle();
    let envelope = fail(&provider, &["emery", "apply"], 2, "nothing-staged").await;
    assert_eq!(envelope["message"], "no candidate is staged", "{envelope}");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("emery specify")),
        "the hint names the way out: {envelope}"
    );

    // a current revision is no candidate
    seed(&provider.storage, SPEC, DESIGN, PLAN);
    fail(&provider, &["emery", "apply"], 2, "nothing-staged").await;
}

// `specify --apply` stages the candidate and seals it in one run: the
// output carries both lines, the envelope says the candidate is no longer
// staged and names what was sealed, and the slots stand as after `apply`.
#[tokio::test]
async fn specify_apply() {
    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

    let resp = cli_ok(&provider, &["emery", "specify", SOURCE, "--apply"]).await;

    let id = current(&provider.storage);
    assert_eq!(
        String::from_utf8_lossy(&resp.stdout),
        format!(
            "staged candidate {id}\n  plan: 1 slice in 1 wave, widest 1\n  sealed revision {id}\n"
        )
    );
    assert!(provider.storage.state(STAGED_KEY).is_none(), "the staged slot is cleared");
    assert!(provider.storage.objects(STAGED_CONTAINER).is_empty(), "the candidate is pruned");
    assert!(provider.storage.object(CONTAINER, &format!("{id}/spec.json")).is_some());
    provider.model.assert_exhausted();

    // the envelope under a re-run that seals the same bytes
    let provider = Provider::over(Arc::clone(&provider.storage), [SPEC_ANSWER, DESIGN_ANSWER]);
    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", SOURCE, "--apply"]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(envelope["revision"], id, "{envelope}");
    assert_eq!(envelope["staged"], false, "{envelope}");
    assert_eq!(envelope["applied"]["revision"], id, "{envelope}");
    assert_eq!(envelope["applied"]["parent"], id, "{envelope}");
    assert_eq!(envelope["applied"]["kind"], "revision", "{envelope}");
    assert_eq!(envelope["diff"]["from"], id, "the diff is against the revision displaced");
    assert_eq!(current(&provider.storage), id);
    provider.model.assert_exhausted();
}

// The stage lands before the seal is attempted: a seal storage refuses
// fails the run after the candidate is staged, so `emery apply` can seal it
// once storage answers, and nothing of the current revision moves.
#[tokio::test]
async fn specify_apply_seal_refused() {
    let storage = Arc::new(Refusing::new(REVISION_KEY));
    let outgoing = seed(&storage.memory, SPEC, DESIGN, PLAN);
    let provider = Provider::over(Arc::clone(&storage), [SPEC_ANSWER, DESIGN_ANSWER]);

    let resp = cli(&provider, &["emery", "--format", "json", "specify", SOURCE, "--apply"]).await;

    assert_eq!(resp.exit, 3, "{}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], "server_error", "{envelope}");
    assert!(
        envelope["message"]
            .as_str()
            .is_some_and(|message| message.contains("swapping the revision id")),
        "{envelope}"
    );
    let candidate =
        String::from_utf8(storage.memory.state(STAGED_KEY).expect("staged")).expect("utf-8 id");
    assert_ne!(candidate, outgoing);
    assert!(
        storage.memory.object(STAGED_CONTAINER, &format!("{candidate}/spec.json")).is_some(),
        "the candidate stands staged"
    );
    assert_eq!(current(&storage.memory), outgoing, "the current revision stands");
    assert!(
        storage.memory.object(CONTAINER, &format!("{outgoing}/spec.json")).is_some(),
        "nothing of the current revision is pruned"
    );
    provider.model.assert_exhausted();

    // once storage answers, apply seals what the run left staged
    let provider = Provider::over(Arc::new(storage.memory.clone()), Vec::<String>::new());
    cli_ok(&provider, &["emery", "apply"]).await;
    assert_eq!(current(&storage.memory), candidate);
}

// `--apply` with no source is refused as a bare `specify` is, before any
// load, and nothing is staged or sealed.
#[tokio::test]
async fn specify_apply_no_source() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::env::set_current_dir(dir.path()).expect("enter empty project");
    let provider = Provider::idle();

    fail(&provider, &["emery", "specify", "--apply"], 1, "specify-source-required").await;
    assert!(provider.source.metadata.lock().expect("metadata").is_empty(), "nothing loads");
    assert!(provider.storage.state(STAGED_KEY).is_none());
    assert!(provider.storage.state(REVISION_KEY).is_none());
}

fn current(storage: &Memory) -> String {
    String::from_utf8(storage.state(REVISION_KEY).expect("current")).expect("utf-8 revision id")
}

// A `Memory` whose compare-and-swap refuses one key, so a scenario can
// fail the seal and nothing else.
#[derive(Clone, Debug)]
struct Refusing {
    memory: Memory,
    key: &'static str,
}

impl Refusing {
    fn new(key: &'static str) -> Self {
        Self {
            memory: Memory::default(),
            key,
        }
    }
}

impl StateStore for Refusing {
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        StateStore::get(&self.memory, key)
    }

    fn set(
        &self, key: &str, value: &[u8], ttl_secs: Option<u64>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        StateStore::set(&self.memory, key, value, ttl_secs)
    }

    fn delete(&self, key: &str) -> impl Future<Output = Result<()>> + Send {
        StateStore::delete(&self.memory, key)
    }

    async fn cas(&self, key: &str, expected: Option<&[u8]>, value: &[u8]) -> Result<(), CasError> {
        if key == self.key {
            return Err(CasError::Store(format!("storage refused `{key}`")));
        }
        StateStore::cas(&self.memory, key, expected, value).await
    }

    fn increment(&self, key: &str, delta: i64) -> impl Future<Output = Result<i64>> + Send {
        StateStore::increment(&self.memory, key, delta)
    }
}

impl BlobStore for Refusing {
    fn get(
        &self, container: &str, name: &str,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        BlobStore::get(&self.memory, container, name)
    }

    fn put(
        &self, container: &str, name: &str, data: &[u8],
    ) -> impl Future<Output = Result<()>> + Send {
        BlobStore::put(&self.memory, container, name, data)
    }

    fn delete(&self, container: &str, name: &str) -> impl Future<Output = Result<()>> + Send {
        BlobStore::delete(&self.memory, container, name)
    }

    fn list(&self, container: &str) -> impl Future<Output = Result<Vec<String>>> + Send {
        BlobStore::list(&self.memory, container)
    }

    fn get_range(
        &self, container: &str, name: &str, start: u64, end: u64,
    ) -> impl Future<Output = Result<Vec<u8>>> + Send {
        BlobStore::get_range(&self.memory, container, name, start, end)
    }

    fn object_info(
        &self, container: &str, name: &str,
    ) -> impl Future<Output = Result<ObjectMetadata>> + Send {
        BlobStore::object_info(&self.memory, container, name)
    }

    fn create_container(&self, name: &str) -> impl Future<Output = Result<()>> + Send {
        BlobStore::create_container(&self.memory, name)
    }

    fn delete_container(&self, name: &str) -> impl Future<Output = Result<()>> + Send {
        BlobStore::delete_container(&self.memory, name)
    }

    fn container_exists(&self, name: &str) -> impl Future<Output = Result<bool>> + Send {
        BlobStore::container_exists(&self.memory, name)
    }

    fn container_info(
        &self, container: &str,
    ) -> impl Future<Output = Result<ContainerMetadata>> + Send {
        BlobStore::container_info(&self.memory, container)
    }
}
