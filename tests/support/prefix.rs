//! Project-scoped keyvalue and blobstore views over one shared in-memory backend.

use std::fmt::Debug;
use std::sync::Arc;

use omnia::futures::FutureExt;
use omnia_wasi_blobstore::{BlobStoreClient, Container};
use omnia_wasi_keyvalue::{
    Bucket, Entry, FutureResult, KeyValueClient, Lifetime, Precondition, Revision,
};

#[derive(Clone, Debug)]
pub struct PrefixedKeyValue<K> {
    prefix: String,
    inner: K,
}

impl<K> PrefixedKeyValue<K> {
    pub fn new(prefix: impl Into<String>, inner: K) -> Self {
        Self {
            prefix: prefix.into(),
            inner,
        }
    }

    fn scoped(&self, key: &str) -> String {
        format!("{}/{key}", self.prefix)
    }
}

impl<K: KeyValueClient + Clone + Debug + Send + Sync + 'static> KeyValueClient
    for PrefixedKeyValue<K>
{
    fn open_bucket(&self, identifier: String) -> FutureResult<Arc<dyn Bucket>> {
        let inner = self.inner.clone();
        let prefix = self.prefix.clone();
        async move {
            let bucket = inner.open_bucket(identifier).await?;
            Ok(Arc::new(ScopedBucket {
                prefix,
                inner: bucket,
            }) as Arc<dyn Bucket>)
        }
        .boxed()
    }
}

#[derive(Debug)]
struct ScopedBucket {
    prefix: String,
    inner: Arc<dyn Bucket>,
}

impl ScopedBucket {
    fn scoped(&self, key: &str) -> String {
        format!("{}/{key}", self.prefix)
    }
}

impl Bucket for ScopedBucket {
    fn get(&self, key: String) -> FutureResult<Option<Entry>> {
        let inner = Arc::clone(&self.inner);
        let key = self.scoped(&key);
        async move { inner.get(key).await }.boxed()
    }

    fn set(
        &self, key: String, value: Vec<u8>, lifetime: Lifetime, when: Option<Precondition>,
    ) -> FutureResult<std::result::Result<Revision, Option<Entry>>> {
        let inner = Arc::clone(&self.inner);
        let key = self.scoped(&key);
        async move { inner.set(key, value, lifetime, when).await }.boxed()
    }

    fn delete(
        &self, key: String, when: Option<Revision>,
    ) -> FutureResult<std::result::Result<(), Option<Entry>>> {
        let inner = Arc::clone(&self.inner);
        let key = self.scoped(&key);
        async move { inner.delete(key, when).await }.boxed()
    }

    fn exists(&self, key: String) -> FutureResult<bool> {
        let inner = Arc::clone(&self.inner);
        let key = self.scoped(&key);
        async move { inner.exists(key).await }.boxed()
    }

    fn keys(&self) -> FutureResult<Vec<String>> {
        let inner = Arc::clone(&self.inner);
        let prefix = format!("{}/", self.prefix);
        async move {
            let keys = inner.keys().await?;
            Ok(keys
                .into_iter()
                .filter_map(|key| key.strip_prefix(&prefix).map(str::to_owned))
                .collect())
        }
        .boxed()
    }

    fn increment(
        &self, key: String, delta: i64, ttl: Option<std::time::Duration>,
    ) -> FutureResult<i64> {
        let inner = Arc::clone(&self.inner);
        let key = self.scoped(&key);
        async move { inner.increment(key, delta, ttl).await }.boxed()
    }
}

#[derive(Clone, Debug)]
pub struct PrefixedBlobStore<B> {
    prefix: String,
    inner: B,
}

impl<B> PrefixedBlobStore<B> {
    pub fn new(prefix: impl Into<String>, inner: B) -> Self {
        Self {
            prefix: prefix.into(),
            inner,
        }
    }

    fn scoped(&self, name: &str) -> String {
        format!("{}/{name}", self.prefix)
    }
}

impl<B: BlobStoreClient + Clone + Debug + Send + Sync + 'static> BlobStoreClient
    for PrefixedBlobStore<B>
{
    fn create_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
        let inner = self.inner.clone();
        let name = self.scoped(&name);
        async move { inner.create_container(name).await }.boxed()
    }

    fn get_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
        let inner = self.inner.clone();
        let name = self.scoped(&name);
        async move { inner.get_container(name).await }.boxed()
    }

    fn delete_container(&self, name: String) -> FutureResult<()> {
        let inner = self.inner.clone();
        let name = self.scoped(&name);
        async move { inner.delete_container(name).await }.boxed()
    }

    fn container_exists(&self, name: String) -> FutureResult<bool> {
        let inner = self.inner.clone();
        let name = self.scoped(&name);
        async move { inner.container_exists(name).await }.boxed()
    }
}
