//! Scripted adapter components and a deployment façade for root CLI suites.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use emery_adapter::source::{Backing, Claim, ClaimKind, Evidence, SourceInput};
use emery_adapter::target::{Report, Slice, Verdict};
use futures::FutureExt as _;
use futures::executor::block_on;
use omnia::{Digest, RegistryConfig};
use omnia_sdk::Error;
use omnia_sdk::command::Response;
use omnia_test::host::{Backends, Deployment, Run, Scratch, ScriptedModel, scratch};
use omnia_wasi_blobstore::WasiBlobStore;
use omnia_wasi_keyvalue::WasiKeyValue;
use omnia_wasi_model::{
    Answer, Error as ModelError, FutureResult, Limits, ModelClient, Request, ToolHost, WasiModel,
};
use omnia_wasi_otel::WasiOtel;
use omnia_wasi_vcs::WasiVcs;
use test_programs::script::{
    ARRIVED, CALLS_DIR, CombinedScript, EvidenceScript, FAILURES_DIR, MOUNT, ReportScript, STATE,
    ScriptCall, ScriptError, SourceScriptState, TargetScriptState, VERIFIES_DIR, VerdictScript,
    sanitise_id,
};

use super::guest::engine_guest;
use super::prefix::{PrefixedBlobStore, PrefixedKeyValue};
use super::storage::Storage;
use super::vcs::VcsScript;

type Recorded = Vec<(String, SourceInput)>;

#[derive(Clone, Debug)]
enum HostModel {
    Scripted(ScriptedModel),
    Failing(FailingCompletes),
}

impl ModelClient for HostModel {
    fn complete(&self, request: Request, tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        match self {
            Self::Scripted(model) => model.complete(request, tool_host),
            Self::Failing(model) => model.complete(request, tool_host),
        }
    }

    fn limits(&self) -> Limits {
        match self {
            Self::Scripted(model) => ModelClient::limits(model),
            Self::Failing(model) => ModelClient::limits(model),
        }
    }
}

// Refuses either the first `left` completions before delegating to `inner`, or every completion when `left` is `None`.
#[derive(Clone, Debug)]
struct FailingCompletes {
    inner: ScriptedModel,
    left: Option<Arc<Mutex<usize>>>,
    message: String,
}

impl FailingCompletes {
    fn first(count: usize, message: impl Into<String>, inner: ScriptedModel) -> Self {
        Self {
            inner,
            left: Some(Arc::new(Mutex::new(count))),
            message: message.into(),
        }
    }

    fn always(message: impl Into<String>, inner: ScriptedModel) -> Self {
        Self {
            inner,
            left: None,
            message: message.into(),
        }
    }
}

impl ModelClient for FailingCompletes {
    fn complete(&self, request: Request, tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        let Some(left) = &self.left else {
            let message = self.message.clone();
            return async move { Err(ModelError::Unavailable(message).into()) }.boxed();
        };
        let mut left = left.lock().expect("transport failures");
        if *left > 0 {
            *left -= 1;
            let message = self.message.clone();
            return async move { Err(ModelError::Unavailable(message).into()) }.boxed();
        }
        drop(left);
        self.inner.complete(request, tool_host)
    }

    fn limits(&self) -> Limits {
        ModelClient::limits(&self.inner)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Rendezvous {
    expected: BTreeSet<String>,
}

impl<T: Into<String>> FromIterator<T> for Rendezvous {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            expected: iter.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SourceScript {
    pub evidence: BTreeMap<String, Result<Evidence, Error>>,
    pub held: BTreeSet<String>,
    pub calls: Arc<Mutex<Recorded>>,
    pub rendezvous: Option<Rendezvous>,
}

impl SourceScript {
    pub fn calls(&self) -> Vec<(String, SourceInput)> {
        self.calls.lock().expect("calls").clone()
    }
}

#[derive(Clone, Debug, Default)]
pub struct TargetScript {
    pub reports: BTreeMap<String, Result<Report, Error>>,
    pub failures: Arc<Mutex<BTreeMap<String, VecDeque<Error>>>>,
    pub verdicts: Arc<Mutex<VecDeque<Result<Verdict, Error>>>>,
    pub inits: Arc<Mutex<Vec<String>>>,
    pub calls: Arc<Mutex<Vec<(String, Slice, String)>>>,
    pub verifies: Arc<Mutex<Vec<String>>>,
    pub rendezvous: Option<Rendezvous>,
}

impl TargetScript {
    pub fn inits(&self) -> Vec<String> {
        self.inits.lock().expect("inits").clone()
    }

    pub fn calls(&self) -> Vec<(String, Slice, String)> {
        self.calls.lock().expect("calls").clone()
    }

    pub fn verifies(&self) -> Vec<String> {
        self.verifies.lock().expect("verifies").clone()
    }

    pub fn verdict(&self, verdict: Result<Verdict, Error>) {
        self.verdicts.lock().expect("verdicts").push_back(verdict);
    }

    pub fn fails(&self, id: &str, error: Error) {
        self.failures.lock().expect("failures").entry(id.to_owned()).or_default().push_back(error);
    }
}

pub struct Provider {
    pub model: ScriptedModel,
    model_override: Option<FailingCompletes>,
    pub source: SourceScript,
    pub target: TargetScript,
    pub vcs: VcsScript,
    pub files: super::files::Files,
    pub storage: Storage,
    backends: Backends,
    project: Scratch,
    store: Scratch,
    registry: Scratch,
    store_scope: Option<String>,
}

impl Provider {
    pub fn answering<T: Into<String>>(answers: impl IntoIterator<Item = T>) -> Self {
        Self::build(ScriptedModel::answering(answers))
    }

    pub fn idle() -> Self {
        Self::answering(Vec::<String>::new())
    }

    pub fn over(storage: Storage, answers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut provider = Self::answering(answers);
        provider.storage = storage;
        provider
    }

    pub fn scoped(
        backends: Backends, scope: impl Into<String>,
        answers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let storage = Storage::from_backends(&backends);
        let mut provider = Self::build_on(backends, ScriptedModel::answering(answers));
        provider.storage = storage;
        provider.store_scope = Some(scope.into());
        provider
    }

    #[must_use]
    pub fn failing_model(mut self, count: usize, message: impl Into<String>) -> Self {
        self.model_override = Some(FailingCompletes::first(count, message, self.model.clone()));
        self
    }

    #[must_use]
    pub fn upstream_failing(mut self, message: impl Into<String>) -> Self {
        self.model_override = Some(FailingCompletes::always(message, self.model.clone()));
        self
    }

    pub fn continuing(self, answers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let Self {
            storage,
            backends,
            files,
            vcs,
            source,
            target,
            ..
        } = self;
        let mut next = Self::answering(answers);
        next.storage = storage;
        next.backends = backends;
        super::store::carry(&files, &next.files);
        next.source = source;
        next.target = target;
        next.vcs = vcs;
        next.vcs.bind_project(next.project.path());
        next.model_override = self.model_override;
        next
    }

    pub fn write_project(&self, relative: &str, contents: impl AsRef<[u8]>) -> String {
        self.project.write(relative, contents);
        relative.to_owned()
    }

    pub async fn cli(&self, argv: &[&str]) -> Response {
        self.stage_script();
        self.files.mark_run_start();
        let model = self.model_override.as_ref().map_or_else(
            || HostModel::Scripted(self.model.clone()),
            |failing| HostModel::Failing(failing.clone()),
        );
        let captured = self.deployment(argv).captured();
        let run = if let Some(scope) = &self.store_scope {
            let backends = self
                .backends
                .clone()
                .model(model)
                .vcs(self.vcs.clone())
                .keyvalue(PrefixedKeyValue::new(scope.clone(), self.backends.keyvalue.clone()))
                .blobstore(PrefixedBlobStore::new(scope.clone(), self.backends.blobstore.clone()));
            captured.run(backends, link_stores).await.expect("emery runs")
        } else {
            let backends = self.backends.clone().model(model).vcs(self.vcs.clone());
            captured.run(backends, link_stores).await.expect("emery runs")
        };
        self.files.record_accesses();
        self.ingest_calls();
        self.ingest_target_script();
        response_of(run)
    }
}

impl Provider {
    fn build(model: ScriptedModel) -> Self {
        Self::build_on(block_on(Backends::defaults()), model)
    }

    fn build_on(backends: Backends, model: ScriptedModel) -> Self {
        let project = scratch();
        seed_build_fixtures(project.path());
        let store = scratch();
        let registry = scratch();
        let storage = Storage::from_backends(&backends);
        let mut files = super::files::Files::default();
        files.bind(&project);
        let vcs = VcsScript::default();
        vcs.bind_project(project.path());
        vcs.bind_files(files.clone());
        publish_adapters(
            &store,
            &registry,
            Path::new(test_programs::ADAPTER_SCRIPT_SOURCE),
            Path::new(test_programs::ADAPTER_SCRIPT_TARGET),
        );
        Self {
            model,
            model_override: None,
            source: SourceScript::default(),
            target: TargetScript::default(),
            vcs,
            files,
            storage,
            backends,
            project,
            store,
            registry,
            store_scope: None,
        }
    }

    fn deployment(&self, argv: &[&str]) -> Deployment {
        let (command_name, command_args) = deployment_command(argv);
        Deployment::new()
            .guest("emery", engine_guest())
            .command(command_name)
            .mount(self.project.mount(true))
            .store(self.store.path())
            .registries(local_registry(self.registry.path()))
            .args(command_args.iter().copied())
    }

    fn stage_script(&self) {
        let script = CombinedScript {
            source: self.host_source_state(),
            target: self.host_target_state(),
        };
        let dir = self.project.path().join(MOUNT);
        for subdir in [ARRIVED, CALLS_DIR, FAILURES_DIR, VERIFIES_DIR] {
            let sub = dir.join(subdir);
            let _ = fs::remove_dir_all(&sub);
            fs::create_dir_all(&sub).expect("script subdir");
        }
        fs::write(dir.join(STATE), serde_json::to_vec_pretty(&script).expect("script JSON"))
            .expect("script state");
    }

    fn host_source_state(&self) -> SourceScriptState {
        SourceScriptState {
            evidence: self
                .source
                .evidence
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        value.as_ref().map_or_else(
                            |err| EvidenceScript::Err(ScriptError::from(err)),
                            |evidence| EvidenceScript::Ok(evidence.clone()),
                        ),
                    )
                })
                .collect(),
            held: self.source.held.clone(),
            rendezvous: self.source.rendezvous.as_ref().map(|r| {
                test_programs::script::RendezvousScript {
                    expected: r.expected.clone(),
                }
            }),
        }
    }

    fn host_target_state(&self) -> TargetScriptState {
        TargetScriptState {
            reports: self
                .target
                .reports
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        value.as_ref().map_or_else(
                            |err| ReportScript::Err(ScriptError::from(err)),
                            |report| ReportScript::Ok(report.clone()),
                        ),
                    )
                })
                .collect(),
            failures: self
                .target
                .failures
                .lock()
                .expect("failures")
                .iter()
                .map(|(key, errs)| (key.clone(), errs.iter().map(ScriptError::from).collect()))
                .collect(),
            verdicts: self
                .target
                .verdicts
                .lock()
                .expect("verdicts")
                .iter()
                .map(|value| {
                    value.as_ref().map_or_else(
                        |err| VerdictScript::Err(ScriptError::from(err)),
                        |verdict| VerdictScript::Ok(verdict.clone()),
                    )
                })
                .collect(),
            rendezvous: self.target.rendezvous.as_ref().map(|r| {
                test_programs::script::RendezvousScript {
                    expected: r.expected.clone(),
                }
            }),
        }
    }

    pub fn clear_target_failures(&self, slice_id: &str) {
        self.target.failures.lock().expect("failures").remove(slice_id);
    }

    fn ingest_target_script(&self) {
        let base = self.project.path().join(MOUNT).join(FAILURES_DIR);
        let mut failures = self.target.failures.lock().expect("failures");
        for (slice_id, queue) in failures.iter_mut() {
            let dir = base.join(sanitise_id(slice_id));
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            let attempts = entries
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry.path().extension().and_then(|ext| ext.to_str()) == Some("slot")
                })
                .count();
            let consumed = attempts.min(queue.len());
            drop(queue.drain(..consumed));
        }
        failures.retain(|_, queue| !queue.is_empty());
    }

    fn ingest_calls(&self) {
        let dir = self.project.path().join(MOUNT).join(CALLS_DIR);
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect();
        files.sort();
        for file in files {
            let bytes =
                fs::read(&file).unwrap_or_else(|err| panic!("reading {}: {err}", file.display()));
            let call: ScriptCall = serde_json::from_slice(&bytes)
                .unwrap_or_else(|err| panic!("call JSON {}: {err}", file.display()));
            match call {
                ScriptCall::SourceExtract { id, input } => {
                    self.source.calls.lock().expect("calls").push((id, input));
                }
                ScriptCall::TargetInit { workspace, .. } => {
                    self.target.inits.lock().expect("inits").push(workspace);
                }
                ScriptCall::TargetBuild { id, workspace, slice } => {
                    self.target.calls.lock().expect("calls").push((id, *slice, workspace));
                }
                ScriptCall::TargetVerify { workspace, .. } => {
                    self.target.verifies.lock().expect("verifies").push(workspace);
                    let _ = self.target.verdicts.lock().expect("verdicts").pop_front();
                }
            }
        }
    }
}

pub async fn cli(provider: &Provider, argv: &[&str]) -> Response {
    provider.cli(argv).await
}

pub fn digest(pair: &str) -> String {
    format!("sha256:{}", pair.repeat(32))
}

pub fn claim(kind: ClaimKind, id: &str, (key, value): (&str, &str)) -> Claim {
    Claim::new(kind)
        .with_id(id)
        .with_backing(Backing::Payload(value.to_string()))
        .with_extra(key, value)
}

pub fn requirement(id: &str, statement: &str) -> Claim {
    claim(ClaimKind::Requirement, id, ("statement", statement))
}

pub fn retire(id: &str, statement: &str) -> Claim {
    claim(ClaimKind::Retire, id, ("statement", statement))
}

pub const fn evidence(claims: Vec<Claim>) -> Evidence {
    Evidence { claims }
}

fn link_stores<
    B: omnia::Provides<WasiModel>
        + omnia::Provides<WasiKeyValue>
        + omnia::Provides<WasiBlobStore>
        + omnia::Provides<WasiOtel>
        + omnia::Provides<WasiVcs>,
>(
    deployment: &mut omnia::Deployment<B>,
) -> anyhow::Result<()> {
    deployment.host::<WasiModel>()?;
    deployment.host::<WasiKeyValue>()?;
    deployment.host::<WasiBlobStore>()?;
    deployment.host::<WasiOtel>()?;
    deployment.host::<WasiVcs>()?;
    Ok(())
}

fn response_of(run: Run) -> Response {
    Response {
        stdout: run.stdout.into_bytes(),
        stderr: run.stderr.into_bytes(),
        exit: run.status.code().try_into().unwrap_or(1),
    }
}

pub fn builder_digest() -> String {
    let bytes =
        fs::read(test_programs::ADAPTER_SCRIPT_TARGET).expect("the scripted target adapter");
    Digest::of(&bytes).to_string()
}

fn publish_adapters(store: &Scratch, registry: &Scratch, source: &Path, target: &Path) {
    let root = store.path();
    for reference in [
        "acme:source@1.0.0",
        "acme:source@0.1.0",
        "acme:docs@1.0.0",
        "acme:docsv99@1.0.0",
        "acme:docs@1.2.0",
        "acme:wiki@1.0.0",
        "acme:wiki-live@1.0.0",
        "acme:code@1.0.0",
        "acme:api@1.0.0",
        "acme:intent@1.0.0",
        "acme:local@1.0.0",
        "acme:builder@1.0.0",
        "acme:builder@2.1.0",
        "acme:builderv100@2.1.0",
        "acme:builderb4@2.1.0",
        "acme:builderb10@2.1.0",
        "acme:target@0.1.0",
        "emery:documentation@1.2.0",
        "emery:intent@1.0.0",
        "emery:demo@1.2.0",
        "emery:docs@1.2.0",
        "acme:ledger@2.1.0",
    ] {
        let wasm = if reference.starts_with("acme:builder") || reference == "acme:target@0.1.0" {
            target
        } else {
            source
        };
        copy_pkg(root, reference, wasm);
        publish_registry(registry.path(), reference, wasm);
    }
}

fn publish_registry(root: &Path, reference: &str, wasm: &Path) {
    let (package, version) = reference.split_once('@').expect("versioned reference");
    let (namespace, name) = package.split_once(':').expect("namespaced reference");
    let dir = root.join(namespace).join(name);
    fs::create_dir_all(&dir).expect("registry package dir");
    fs::copy(wasm, dir.join(format!("{version}.wasm"))).expect("publish to registry");
}

fn copy_pkg(store: &Path, reference: &str, wasm: &Path) {
    let destination = store.join(stored_name(reference));
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("package dir");
    }
    fs::copy(wasm, &destination).expect("publish adapter");
}

fn stored_name(reference: &str) -> String {
    let (package, version) = reference.split_once('@').expect("versioned reference");
    format!("{}@{version}.wasm", package.replace(':', "_"))
}

fn seed_build_fixtures(root: &Path) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/build");
    let destination = root.join("tests/build");
    copy_tree(&source, &destination);
}

fn copy_tree(source: &Path, destination: &Path) {
    if source.is_dir() {
        fs::create_dir_all(destination).expect("fixture dir");
        for entry in fs::read_dir(source).expect("read fixtures") {
            let entry = entry.expect("fixture entry");
            let name = entry.file_name();
            copy_tree(&entry.path(), &destination.join(name));
        }
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("fixture parent");
        }
        fs::copy(source, destination).expect("copy fixture");
    }
}

fn deployment_command<'a>(argv: &'a [&'a str]) -> (String, Vec<&'a str>) {
    match argv {
        [] => ("emery".to_owned(), Vec::new()),
        ["emery", rest @ ..] => ("emery".to_owned(), rest.to_vec()),
        [cmd, rest @ ..] if cmd.contains(':') => (cmd.to_string(), rest.to_vec()),
        all => ("emery".to_owned(), all.to_vec()),
    }
}

fn local_registry(root: &Path) -> RegistryConfig {
    format!(
        "[namespace_registries]\nacme = \"registry.test\"\nemery = \"registry.test\"\n\n\
         [registry.\"registry.test\"]\ndefault = \"local\"\n\n[registry.\"registry.test\".local]\n\
         root = {:?}\n",
        root.display()
    )
    .into()
}
