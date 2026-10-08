//! Scripts every capability of a provider and drives the command façade over it.
//!
//! Model, source, target, storage, and version-control capabilities use
//! strict scripts. Each scenario must consume exactly the expected
//! operations, so an unexercised or unexpected path fails immediately. The
//! plugin capability is a constant: every load lands, so the synthesis
//! suites assert nothing of a load and the adapter boundary is the runtime
//! suite's.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use emery_adapter::source::{
    AdapterMetadata, Backing, Claim, ClaimKind, Evidence, Source, SourceInput, SourceKind,
};
use emery_adapter::target::{Report, Slice, Target, TargetMetadata};
use emery_engine::Axis;
use omnia_sdk::api::command::Response;
use omnia_sdk::plugins::{self, Digest, Location, Plugin};
use omnia_sdk::{
    BlobStore, CasError, ContainerMetadata, Error, Model, ObjectMetadata, Plugins, StateStore, Vcs,
    model, vcs,
};
use omnia_test::guest::{Memory, Scripted};
use serde_json::Value;
use tokio::sync::Barrier;

use super::vcs::VcsScript;

const GREETING: &str = "GET /greeting returns the static string 'hello'.";

// Long enough for an in-process run, short enough that a serialising engine
// fails the scenario instead of hanging it.
const RENDEZVOUS: Duration = Duration::from_secs(1);

type Recorded = Vec<(String, SourceInput)>;

/// A scripted `Source` with a record of every dispatch.
///
/// Evidence is scripted per source name, the minimum `emery` version and the
/// kind of source per adapter.
///
/// - An unscripted source answers the greeting requirement.
/// - An unscripted adapter reads documentation.
/// - A scripted failure is the classified error the WIT bindings' lift would
///   have produced.
/// - A held source never answers at all.
#[derive(Clone, Debug, Default)]
pub struct SourceScript {
    /// Extract outcomes keyed by source name.
    pub evidence: BTreeMap<String, Result<Evidence, Error>>,
    /// Source names whose extract never resolves, so a scenario can prove the
    /// engine does not wait for them.
    pub held: BTreeSet<String>,
    /// Minimum `emery` versions keyed by adapter id, the guest name each
    /// load registers.
    pub versions: BTreeMap<String, String>,
    /// Kinds of source by adapter id; an unscripted adapter reads
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

impl SourceScript {
    /// Returns every extract dispatch so far: the adapter id and its input, in dispatch order.
    pub fn calls(&self) -> Vec<(String, SourceInput)> {
        self.calls.lock().expect("calls").clone()
    }
}

/// A barrier that holds extraction until every expected source has arrived.
///
/// The bounded wait reports missing sources, allowing serial extraction to
/// fail clearly instead of deadlocking the suite.
#[derive(Clone, Debug)]
pub struct Rendezvous {
    expected: BTreeSet<String>,
    arrived: Arc<Mutex<BTreeSet<String>>>,
    barrier: Arc<Barrier>,
}

impl<T: Into<String>> FromIterator<T> for Rendezvous {
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

/// A scripted `Target` with a record of every dispatch.
///
/// Reports are scripted per slice id, the minimum `emery` version per adapter.
///
/// - An unscripted slice reports every requirement covered and one file
///   written, `src/<name>.rs`.
/// - A scripted failure is the classified error the WIT bindings' lift would
///   have produced.
#[derive(Clone, Debug, Default)]
pub struct TargetScript {
    /// Build outcomes keyed by slice id.
    pub reports: BTreeMap<String, Result<Report, Error>>,
    /// Minimum `emery` versions keyed by adapter id, the guest name each
    /// load registers.
    pub versions: BTreeMap<String, String>,
    /// Every build dispatch, recorded in dispatch order: the adapter id, the
    /// slice, and the workspace root.
    pub calls: Arc<Mutex<Vec<(String, Slice, String)>>>,
    /// Every metadata dispatch, by adapter id, in call order.
    pub metadata: Arc<Mutex<Vec<String>>>,
}

impl TargetScript {
    /// Returns every build dispatch so far: the adapter id, the slice, and the
    /// workspace root, in dispatch order.
    pub fn calls(&self) -> Vec<(String, Slice, String)> {
        self.calls.lock().expect("calls").clone()
    }
}

/// The scripted provider behind every root scenario.
#[derive(Debug)]
pub struct Provider<S = Memory> {
    /// FIFO-scripted model answers.
    pub model: Scripted,
    /// The scripted `Source`.
    pub source: SourceScript,
    /// The scripted `Target`.
    pub target: TargetScript,
    /// The scripted `Vcs`.
    pub vcs: VcsScript,
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
            target: TargetScript::default(),
            vcs: VcsScript::default(),
            storage,
        }
    }
}

impl<S> Clone for Provider<S> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            source: self.source.clone(),
            target: self.target.clone(),
            vcs: self.vcs.clone(),
            storage: Arc::clone(&self.storage),
        }
    }
}

// Several capabilities share method names, so every delegation is a fully
// qualified trait call.

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

// Every load lands as the guest its location names, exporting both axes, at
// a fixed digest: what a load resolves to is the runtime suite's to assert.
impl<S: Send + Sync + 'static> Plugins for Provider<S> {
    fn load(
        &self, from: &Location, _digest: Option<&Digest>,
    ) -> impl Future<Output = Result<Plugin, plugins::Error>> + Send {
        let plugin = Plugin::new(from.name(), digest("ab"), Axis::INTERFACES);
        async move { Ok(plugin) }
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
        // record the dispatch before the future is polled, so `calls` is dispatch order
        self.source.calls.lock().expect("calls").push((id.to_string(), input.clone()));
        let outcome = self
            .source
            .evidence
            .get(&input.name)
            .cloned()
            .unwrap_or_else(|| Ok(evidence(vec![requirement("greeting.behaviour", GREETING)])));
        let rendezvous = self.source.rendezvous.clone();
        let held = self.source.held.contains(&input.name);
        let name = input.name.clone();
        async move {
            if let Some(rendezvous) = rendezvous {
                rendezvous.wait(&name).await;
            }
            if held {
                std::future::pending::<()>().await;
            }
            outcome
        }
    }

    fn metadata(&self, id: &str) -> AdapterMetadata {
        self.source.metadata.lock().expect("metadata").push(id.to_string());
        AdapterMetadata {
            emery_version: self.source.versions.get(id).cloned(),
            kind: self.source.kinds.get(id).copied().unwrap_or(SourceKind::Documentation),
        }
    }
}

impl<S: Send + Sync + 'static> Target for Provider<S> {
    fn build(
        &self, id: &str, slice: &Slice, workspace: &str,
    ) -> impl Future<Output = Result<Report, Error>> + Send {
        self.target.calls.lock().expect("calls").push((
            id.to_string(),
            slice.clone(),
            workspace.to_string(),
        ));
        let outcome = self.target.reports.get(&slice.id).cloned().unwrap_or_else(|| {
            Ok(Report {
                covered: slice.requirements.clone(),
                written: vec![format!("src/{}.rs", slice.name)],
            })
        });
        async move { outcome }
    }

    fn metadata(&self, id: &str) -> TargetMetadata {
        self.target.metadata.lock().expect("metadata").push(id.to_string());
        TargetMetadata {
            emery_version: self.target.versions.get(id).cloned(),
        }
    }
}

impl<S: Send + Sync + 'static> Vcs for Provider<S> {
    fn resolve(
        &self, repo: &str, revision: &str,
    ) -> impl Future<Output = Result<String, vcs::Error>> + Send {
        Vcs::resolve(&self.vcs, repo, revision)
    }

    fn head(&self, at: &str) -> impl Future<Output = Result<String, vcs::Error>> + Send {
        Vcs::head(&self.vcs, at)
    }

    fn commit(
        &self, at: &str, message: &str,
    ) -> impl Future<Output = Result<Option<String>, vcs::Error>> + Send {
        Vcs::commit(&self.vcs, at, message)
    }

    fn merge(
        &self, at: &str, revision: &str, message: &str, policy: &[vcs::Rule],
    ) -> impl Future<Output = Result<vcs::Merged, vcs::Error>> + Send {
        Vcs::merge(&self.vcs, at, revision, message, policy)
    }

    fn init(&self, at: &str) -> impl Future<Output = Result<(), vcs::Error>> + Send {
        Vcs::init(&self.vcs, at)
    }

    fn add(
        &self, repo: &str, at: &str, revision: &str,
    ) -> impl Future<Output = Result<(), vcs::Error>> + Send {
        Vcs::add(&self.vcs, repo, at, revision)
    }

    fn remove(&self, at: &str) -> impl Future<Output = Result<(), vcs::Error>> + Send {
        Vcs::remove(&self.vcs, at)
    }

    fn pending(
        &self, at: &str,
    ) -> impl Future<Output = Result<Vec<vcs::Change>, vcs::Error>> + Send {
        Vcs::pending(&self.vcs, at)
    }

    fn clone_repo(
        &self, url: &str, at: &str, options: vcs::CloneOptions,
    ) -> impl Future<Output = Result<(), vcs::Error>> + Send {
        Vcs::clone_repo(&self.vcs, url, at, options)
    }

    fn fetch(
        &self, repo: &str, remote: &str,
    ) -> impl Future<Output = Result<(), vcs::Error>> + Send {
        Vcs::fetch(&self.vcs, repo, remote)
    }

    fn label(
        &self, repo: &str, name: &str, revision: &str,
    ) -> impl Future<Output = Result<(), vcs::Error>> + Send {
        Vcs::label(&self.vcs, repo, name, revision)
    }

    fn push(
        &self, repo: &str, remote: &str, label: &str,
    ) -> impl Future<Output = Result<(), vcs::Error>> + Send {
        Vcs::push(&self.vcs, repo, remote, label)
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
