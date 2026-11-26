//! In-memory staging through the same keyvalue and blobstore handles the engine uses.

use std::collections::BTreeMap;
use std::sync::Arc;

use emery_engine::{STAGED_CONTAINER, STAGED_KEY};
use omnia_test::host::{Backends, STATE_BUCKET};
use omnia_wasi_blobstore::{BlobStoreClient, BlobStoreDefault};
use omnia_wasi_keyvalue::{KeyValueClient, KeyValueDefault, Lifetime};
use serde_json::Value;

/// Snapshot of staged state and blob objects for equality assertions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Key-value records in the default bucket.
    pub state: BTreeMap<String, Vec<u8>>,
    /// Object bytes keyed by `container/name`.
    pub objects: BTreeMap<String, Vec<u8>>,
}

/// Host-side handles into the in-memory stores behind a deployment.
#[derive(Clone, Debug)]
pub struct Storage {
    keyvalue: Arc<KeyValueDefault>,
    blobstore: Arc<BlobStoreDefault>,
}

impl Storage {
    /// Captures the keyvalue and blobstore from a [`Backends`] bundle.
    #[must_use]
    pub fn from_backends(backends: &Backends) -> Self {
        Self {
            keyvalue: Arc::new(backends.keyvalue.clone()),
            blobstore: Arc::new(backends.blobstore.clone()),
        }
    }

    /// Returns handles for linking a deployment.
    #[must_use]
    pub fn backends(&self) -> (Arc<KeyValueDefault>, Arc<BlobStoreDefault>) {
        (Arc::clone(&self.keyvalue), Arc::clone(&self.blobstore))
    }

    /// Writes a value to the default bucket.
    pub async fn insert_state(&self, key: &str, value: &[u8]) {
        let bucket = self.open_bucket().await;
        bucket
            .set(key.to_owned(), value.to_vec(), Lifetime::Keep(None), None)
            .await
            .expect("write state")
            .expect("written");
    }

    /// Writes an object into a container, creating the container when needed.
    pub async fn insert_object(&self, container: &str, name: &str, value: &[u8]) {
        let store = Arc::clone(&self.blobstore);
        let container =
            store.create_container(container.to_owned()).await.expect("create container");
        container.write_data(name.to_owned(), value.to_vec().into()).await.expect("write object");
    }

    /// Reads a state key.
    pub async fn state(&self, key: &str) -> Option<Vec<u8>> {
        let bucket = self.open_bucket().await;
        bucket.get(key.to_owned()).await.expect("read state").map(|entry| entry.value)
    }

    /// Names of every object in a container, or an empty list when the container is absent.
    pub async fn list_objects(&self, container: &str) -> Vec<String> {
        let store = Arc::clone(&self.blobstore);
        if !store.container_exists(container.to_owned()).await.unwrap_or(false) {
            return Vec::new();
        }
        let container = store.get_container(container.to_owned()).await.expect("open container");
        let page = container.list_objects(String::new(), None).await.expect("list objects");
        page.objects.into_iter().map(|object| object.name).collect()
    }

    /// Reads an object when it exists.
    pub async fn object(&self, container: &str, name: &str) -> Option<Vec<u8>> {
        let store = Arc::clone(&self.blobstore);
        if !store.container_exists(container.to_owned()).await.unwrap_or(false) {
            return None;
        }
        let container = store.get_container(container.to_owned()).await.ok()?;
        container
            .get_data(name.to_owned(), 0..u64::MAX)
            .await
            .ok()
            .flatten()
            .map(|bytes| bytes.to_vec())
    }

    /// Records every state key and staged object.
    pub async fn snapshot(&self) -> Snapshot {
        let mut snap = Snapshot::default();
        snap.state.insert(STAGED_KEY.to_owned(), self.state(STAGED_KEY).await.unwrap_or_default());
        if self.blobstore.container_exists(STAGED_CONTAINER.to_owned()).await.unwrap_or(false) {
            let container = self
                .blobstore
                .get_container(STAGED_CONTAINER.to_owned())
                .await
                .expect("staged container");
            let page =
                container.list_objects(String::new(), None).await.expect("list staged objects");
            for object in page.objects {
                if let Some(bytes) =
                    container.get_data(object.name.clone(), 0..u64::MAX).await.ok().flatten()
                {
                    snap.objects
                        .insert(format!("{STAGED_CONTAINER}/{}", object.name), bytes.to_vec());
                }
            }
        }
        snap
    }

    async fn open_bucket(&self) -> Arc<dyn omnia_wasi_keyvalue::Bucket> {
        self.keyvalue.open_bucket(STATE_BUCKET.to_owned()).await.expect("open default bucket")
    }
}

/// Reads the staged revision candidate's id from the record at `STAGED_KEY`.
pub async fn staged(storage: &Storage) -> String {
    staged_at(storage, STAGED_KEY).await
}

/// Reads the staged revision candidate's id at `key`.
pub async fn staged_at(storage: &Storage, key: &str) -> String {
    let raw = storage.state(key).await.expect("staged record");
    let record: Value = serde_json::from_slice(&raw).expect("staged record JSON");
    assert_eq!(record["kind"], "revision", "{record}");
    record["id"].as_str().expect("staged record id").to_owned()
}
