//! The scripted provider
//!
//! A provider whose every capability — model, source, plugin loading,
//! storage — is a scripted double, and the runner that drives the command
//! façade over it in-process. Each capability impl delegates to the field
//! named for it, the shape a production provider's single backend has.
//!
//! Scripting rather than mocking means each scenario states exactly the turns
//! it will consume, and a scenario that consumes more or fewer fails, so the
//! suites cannot silently stop exercising a path.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use emery_source::{
    AdapterMetadata, Authority, Backing, Claim, ClaimKind, Evidence, Source, SourceInput,
};
use omnia_guest::api::command::Response;
use omnia_guest::plugins::{self, Digest, PluginRef};
use omnia_guest::{
    BlobStore, CasError, ContainerMetadata, Error, Model, ObjectMetadata, Plugins, StateStore,
    model,
};
use omnia_test::guest::{Memory, Scripted, ScriptedLoader};
use serde_json::Value;

const GREETING: &str = "GET /greeting returns the static string 'hello'.";

/// Dispatched `(adapter id, input)` pairs, in call order.
type Recorded = Vec<(String, SourceInput)>;

/// Scripted `Source`: per-key evidence, per-adapter minimum `emery`
/// versions, and a record of every dispatch. An unscripted key answers
/// the greeting requirement as documentation evidence; a scripted failure
/// is the classified error the WIT bindings lift would have produced.
#[derive(Clone, Debug, Default)]
pub struct SourceScript {
    /// Extract outcomes keyed by source key.
    pub evidence: BTreeMap<String, Result<Evidence, Error>>,
    /// Minimum `emery` versions keyed by adapter name.
    pub versions: BTreeMap<String, String>,
    /// Every extract dispatch, recorded for call assertions.
    pub calls: Arc<Mutex<Recorded>>,
}

/// The scripted provider behind every root scenario.
#[derive(Debug)]
pub struct Provider<S = Memory> {
    /// FIFO-scripted model answers.
    pub model: Scripted,
    /// The scripted `Source`.
    pub source: SourceScript,
    /// The scripted `Plugins` loader: an unscripted, unpinned package
    /// resolves to the fixed `digest("ab")`; a pin that disagrees with a
    /// scripted digest refuses `refused`, mirroring the host's
    /// verify-before-validate step.
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
        self.source.calls.lock().expect("calls").push((id.to_string(), input.clone()));
        let outcome = self.source.evidence.get(&input.key).cloned().unwrap_or_else(|| {
            Ok(evidence(
                Authority::Documentation,
                vec![requirement("greeting.behaviour", GREETING)],
            ))
        });
        std::future::ready(outcome)
    }

    fn metadata(&self, id: &str) -> AdapterMetadata {
        // Routed ids are `source:<name>` or a package reference
        // (`<namespace>:<name>@<version>`); versions key on the name.
        let name = id.split_once('@').map_or(id, |(stem, _)| stem);
        let name = name.rsplit_once(':').map_or(name, |(_, stem)| stem);
        AdapterMetadata {
            emery_version: self.source.versions.get(name).cloned(),
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
pub const fn evidence(authority: Authority, claims: Vec<Claim>) -> Evidence {
    Evidence { authority, claims }
}
