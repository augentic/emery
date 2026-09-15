//! Scripts every capability of a provider and drives the command façade over it.
//!
//! The model, source, plugin loading, and storage capabilities are each a
//! scripted double, and each impl delegates to the field named for it — the
//! shape a production provider's single backend has. The runner drives the
//! command façade over the provider in-process.
//!
//! Scripting rather than mocking means each scenario states exactly the turns
//! it will consume, and a scenario that consumes more or fewer fails, so the
//! suites cannot silently stop exercising a path.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use emery_adapter::is_kebab;
use emery_adapter::source::{
    AdapterMetadata, Backing, Claim, ClaimKind, Evidence, Source, SourceInput, SourceKind,
};
use omnia_guest::api::command::Response;
use omnia_guest::plugins::{self, Digest, PluginRef};
use omnia_guest::{
    BlobStore, CasError, ContainerMetadata, Error, Model, ObjectMetadata, Plugins, StateStore,
    model,
};
use omnia_test::guest::{Memory, Scripted, ScriptedLoader};
use serde_json::Value;
use tokio::sync::Barrier;

const GREETING: &str = "GET /greeting returns the static string 'hello'.";

// How long a held extract waits for the other sources before the double
// gives up: long enough for an in-process run, short enough that a
// serialising engine fails the scenario instead of hanging it.
const RENDEZVOUS: Duration = Duration::from_secs(1);

/// Dispatched `(adapter id, input)` pairs, in call order.
type Recorded = Vec<(String, SourceInput)>;

/// A scripted `Source` with a record of every dispatch.
///
/// Evidence is scripted per key; the minimum `emery` version and the kind of
/// source per adapter. An unscripted key answers the greeting requirement; an
/// unscripted adapter reads documentation; a scripted failure is the
/// classified error the WIT bindings' lift would have produced.
#[derive(Clone, Debug, Default)]
pub struct SourceScript {
    /// Extract outcomes keyed by source key.
    pub evidence: BTreeMap<String, Result<Evidence, Error>>,
    /// Minimum `emery` versions keyed by adapter id — the reference itself.
    pub versions: BTreeMap<String, String>,
    /// Kinds of source keyed by adapter id; an unscripted adapter reads
    /// documentation.
    pub kinds: BTreeMap<String, SourceKind>,
    /// Every extract dispatch, recorded for call assertions.
    pub calls: Arc<Mutex<Recorded>>,
    /// Every metadata dispatch, by adapter id, in call order.
    pub metadata: Arc<Mutex<Vec<String>>>,
    /// When set, every extract is held until each expected source has been
    /// requested, so a scenario can prove the engine runs its sources
    /// together.
    pub rendezvous: Option<Rendezvous>,
}

/// A meeting point for the sources of one run: no extract resolves until
/// every expected source has asked to extract. An engine that extracts its
/// sources one at a time never gets there, so the wait is bounded and the
/// failure names the sources that never arrived.
#[derive(Clone, Debug)]
pub struct Rendezvous {
    expected: BTreeSet<String>,
    arrived: Arc<Mutex<BTreeSet<String>>>,
    barrier: Arc<Barrier>,
}

impl<T: Into<String>> FromIterator<T> for Rendezvous {
    /// Builds the rendezvous over the source keys of one run.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let expected: BTreeSet<String> = iter.into_iter().map(Into::into).collect();
        Self {
            barrier: Arc::new(Barrier::new(expected.len())),
            arrived: Arc::default(),
            expected,
        }
    }
}

impl Rendezvous {
    // Holds `key` until every expected source has arrived, or fails the
    // scenario once the bound elapses.
    async fn wait(&self, key: &str) {
        self.arrived.lock().expect("arrived").insert(key.to_string());
        if tokio::time::timeout(RENDEZVOUS, self.barrier.wait()).await.is_err() {
            let missing: Vec<String> =
                self.expected.difference(&self.arrived.lock().expect("arrived")).cloned().collect();
            panic!(
                "source `{key}` waited {RENDEZVOUS:?} for {missing:?}, which never asked to \
                 extract: the engine is extracting its sources one at a time"
            );
        }
    }
}

/// The scripted provider behind every root scenario.
#[derive(Debug)]
pub struct Provider<S = Memory> {
    /// FIFO-scripted model answers.
    pub model: Scripted,
    /// The scripted `Source`.
    pub source: SourceScript,
    /// The scripted `Plugins` loader: an unscripted package resolves to
    /// the fixed `digest("ab")`.
    pub plugins: ScriptedLoader,
    /// The scripted storage pair.
    pub storage: Arc<S>,
}

impl Provider<Memory> {
    /// Creates a provider answering `answers` over fresh in-memory storage.
    pub fn answering<T: Into<String>>(answers: impl IntoIterator<Item = T>) -> Self {
        Self::over(Arc::new(Memory::default()), answers)
    }

    /// Creates a provider whose model is never dispatched.
    pub fn idle() -> Self {
        Self::answering(Vec::<String>::new())
    }
}

impl<S> Provider<S> {
    /// Creates a provider answering `answers` over `storage`.
    pub fn over<T: Into<String>>(storage: Arc<S>, answers: impl IntoIterator<Item = T>) -> Self {
        Self {
            model: Scripted::answering(answers),
            source: SourceScript::default(),
            plugins: ScriptedLoader::default().defaulting(digest("ab")),
            storage,
        }
    }

    // Mirrors host-mediated dispatch: a bare name is a guest the deployment
    // declares; any other id is routable only once the loader has landed it.
    // A source call before its load is the engine's ordering defect, and it
    // fails here rather than only under the real runtime.
    fn routable(&self, id: &str) {
        let loaded = self.plugins.loads().iter().any(|plugin| plugin.package == id);
        assert!(is_kebab(id) || loaded, "`{id}` was dispatched before its load");
    }
}

impl<S> Clone for Provider<S> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            source: self.source.clone(),
            plugins: self.plugins.clone(),
            storage: Arc::clone(&self.storage),
        }
    }
}

// Several capabilities share method names (`get`, `put`, `delete`), so every
// delegation is a fully qualified trait call.

impl<S: Send + Sync + 'static> Model for Provider<S> {
    fn complete(
        &self, request: model::Request,
    ) -> impl Future<Output = Result<model::Reply, model::Error>> + Send {
        Model::complete(&self.model, request)
    }

    fn complete_with<H, F>(
        &self, request: model::Request, handler: H,
    ) -> impl Future<Output = Result<model::Reply, model::Error>> + Send
    where
        H: FnMut(model::ToolCall) -> F + Send,
        F: Future<Output = Result<String, String>> + Send,
    {
        Model::complete_with(&self.model, request, handler)
    }
}

impl<S: Send + Sync + 'static> Plugins for Provider<S> {
    fn load(
        &self, plugin: &PluginRef,
    ) -> impl Future<Output = Result<plugins::Plugin, plugins::Error>> + Send {
        Plugins::load(&self.plugins, plugin)
    }
}

impl<S: StateStore + Send + Sync + 'static> StateStore for Provider<S> {
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        StateStore::get(&self.storage, key)
    }

    fn set(
        &self, key: &str, value: &[u8], ttl_secs: Option<u64>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        StateStore::set(&self.storage, key, value, ttl_secs)
    }

    fn delete(&self, key: &str) -> impl Future<Output = Result<()>> + Send {
        StateStore::delete(&self.storage, key)
    }

    fn cas(
        &self, key: &str, expected: Option<&[u8]>, value: &[u8],
    ) -> impl Future<Output = Result<(), CasError>> + Send {
        StateStore::cas(&self.storage, key, expected, value)
    }

    fn increment(&self, key: &str, delta: i64) -> impl Future<Output = Result<i64>> + Send {
        StateStore::increment(&self.storage, key, delta)
    }
}

impl<S: BlobStore + Send + Sync + 'static> BlobStore for Provider<S> {
    fn get(
        &self, container: &str, name: &str,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        BlobStore::get(&self.storage, container, name)
    }

    fn put(
        &self, container: &str, name: &str, data: &[u8],
    ) -> impl Future<Output = Result<()>> + Send {
        BlobStore::put(&self.storage, container, name, data)
    }

    fn delete(&self, container: &str, name: &str) -> impl Future<Output = Result<()>> + Send {
        BlobStore::delete(&self.storage, container, name)
    }

    fn list(&self, container: &str) -> impl Future<Output = Result<Vec<String>>> + Send {
        BlobStore::list(&self.storage, container)
    }

    fn get_range(
        &self, container: &str, name: &str, start: u64, end: u64,
    ) -> impl Future<Output = Result<Vec<u8>>> + Send {
        BlobStore::get_range(&self.storage, container, name, start, end)
    }

    fn object_info(
        &self, container: &str, name: &str,
    ) -> impl Future<Output = Result<ObjectMetadata>> + Send {
        BlobStore::object_info(&self.storage, container, name)
    }

    fn create_container(&self, name: &str) -> impl Future<Output = Result<()>> + Send {
        BlobStore::create_container(&self.storage, name)
    }

    fn delete_container(&self, name: &str) -> impl Future<Output = Result<()>> + Send {
        BlobStore::delete_container(&self.storage, name)
    }

    fn container_exists(&self, name: &str) -> impl Future<Output = Result<bool>> + Send {
        BlobStore::container_exists(&self.storage, name)
    }

    fn container_info(
        &self, container: &str,
    ) -> impl Future<Output = Result<ContainerMetadata>> + Send {
        BlobStore::container_info(&self.storage, container)
    }
}

impl<S: Send + Sync + 'static> Source for Provider<S> {
    fn extract(
        &self, id: &str, input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        self.routable(id);
        // The dispatch is recorded and the outcome chosen before the future
        // is polled, so `calls` is dispatch order whatever resolves first.
        self.source.calls.lock().expect("calls").push((id.to_string(), input.clone()));
        let outcome = self
            .source
            .evidence
            .get(&input.key)
            .cloned()
            .unwrap_or_else(|| Ok(evidence(vec![requirement("greeting.behaviour", GREETING)])));
        let rendezvous = self.source.rendezvous.clone();
        let key = input.key.clone();
        async move {
            if let Some(rendezvous) = rendezvous {
                rendezvous.wait(&key).await;
            }
            outcome
        }
    }

    fn metadata(&self, id: &str) -> AdapterMetadata {
        self.routable(id);
        self.source.metadata.lock().expect("metadata").push(id.to_string());
        AdapterMetadata {
            emery_version: self.source.versions.get(id).cloned(),
            kind: self.source.kinds.get(id).copied().unwrap_or(SourceKind::Documentation),
        }
    }
}

/// Runs one CLI invocation in-process, returning the raw response.
pub async fn cli<S>(provider: &Provider<S>, argv: &[&str]) -> Response
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    emery_cli::run(provider.clone(), argv.iter().copied()).await
}

/// Builds a full-length `sha256:` digest from one repeated hex pair.
pub fn digest(pair: &str) -> Digest {
    format!("sha256:{}", pair.repeat(32)).parse().expect("a valid digest")
}

/// Builds a claim of `kind` carrying one required extra.
pub fn claim(kind: ClaimKind, id: &str, extra: (&str, &str)) -> Claim {
    let mut extras = serde_json::Map::new();
    extras.insert(extra.0.to_string(), Value::String(extra.1.to_string()));
    Claim {
        kind,
        id: Some(id.to_string()),
        path: None,
        synopsis: None,
        backing: Some(Backing::Payload(extra.1.to_string())),
        extras,
    }
}

/// Builds a requirement claim carrying its required `statement` extra.
pub fn requirement(id: &str, statement: &str) -> Claim {
    claim(ClaimKind::Requirement, id, ("statement", statement))
}

/// Builds an evidence document over `claims`.
pub const fn evidence(claims: Vec<Claim>) -> Evidence {
    Evidence { claims }
}
