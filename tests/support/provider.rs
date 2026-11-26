//! Scripted adapter components and a deployment façade for root CLI suites.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use emery_adapter::source::{Backing, Claim, ClaimKind, Evidence, SourceInput, SourceKind};
use emery_adapter::target::{MergeRule, Report, Slice, Verdict};
use futures::executor::block_on;
use omnia::RegistryConfig;
use omnia_sdk::Error;
use omnia_sdk::command::Response;
use omnia_test::host::{Backends, Deployment, Run, Scratch, ScriptedModel, scratch};
use omnia_wasi_blobstore::WasiBlobStore;
use omnia_wasi_keyvalue::WasiKeyValue;
use omnia_wasi_model::WasiModel;
use omnia_wasi_otel::WasiOtel;
use omnia_wasi_vcs::WasiVcs;
use test_programs::script::{
    ARRIVED, CALLS, CombinedScript, EvidenceScript, MOUNT, ReportScript, RuleWire, STATE,
    ScriptCall, ScriptError, SourceScriptState, TargetScriptState, VerdictScript,
};

use super::guest::engine_guest;
use super::storage::Storage;
use super::vcs::VcsScript;

type Recorded = Vec<(String, SourceInput)>;

/// Parallel-dispatch barrier configuration staged for adapter guests.
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

/// Scripted source outcomes and call records.
#[derive(Clone, Debug, Default)]
pub struct SourceScript {
    pub evidence: BTreeMap<String, Result<Evidence, Error>>,
    pub held: BTreeSet<String>,
    pub versions: BTreeMap<String, String>,
    pub kinds: BTreeMap<String, SourceKind>,
    pub calls: Arc<Mutex<Recorded>>,
    pub metadata: Arc<Mutex<Vec<String>>>,
    pub rendezvous: Option<Rendezvous>,
}

impl SourceScript {
    pub fn calls(&self) -> Vec<(String, SourceInput)> {
        self.calls.lock().expect("calls").clone()
    }
}

/// Scripted target outcomes and call records.
#[derive(Clone, Debug, Default)]
pub struct TargetScript {
    pub reports: BTreeMap<String, Result<Report, Error>>,
    pub failures: Arc<Mutex<BTreeMap<String, VecDeque<Error>>>>,
    pub verdicts: Arc<Mutex<VecDeque<Result<Verdict, Error>>>>,
    pub versions: BTreeMap<String, String>,
    pub rules: Vec<MergeRule>,
    pub budget: Option<u32>,
    pub inits: Arc<Mutex<Vec<String>>>,
    pub calls: Arc<Mutex<Vec<(String, Slice, String)>>>,
    pub verifies: Arc<Mutex<Vec<String>>>,
    pub metadata: Arc<Mutex<Vec<String>>>,
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

/// The façade behind every root scenario suite.
pub struct Provider {
    pub model: ScriptedModel,
    pub source: SourceScript,
    pub target: TargetScript,
    pub vcs: VcsScript,
    pub files: super::files::Files,
    pub storage: Storage,
    backends: Backends,
    project: Scratch,
    store: Scratch,
    registry: Scratch,
    guest: PathBuf,
    script_source: PathBuf,
    script_target: PathBuf,
}

impl Provider {
    pub fn answering<T: Into<String>>(answers: impl IntoIterator<Item = T>) -> Self {
        Self::build(ScriptedModel::answering(answers))
    }

    pub fn idle() -> Self {
        Self::answering(Vec::<String>::new())
    }

    /// Reuses staging storage from a prior run with a fresh deployment shell.
    pub fn over(storage: Storage, answers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut provider = Self::answering(answers);
        provider.storage = storage;
        provider
    }

    /// Writes `contents` under the deployment project mount and returns its relative path.
    pub fn write_project(&self, relative: &str, contents: impl AsRef<[u8]>) -> String {
        self.project.write(relative, contents);
        relative.to_owned()
    }

    #[expect(clippy::print_stderr, reason = "A17 records per-invocation wall time for the spike")]
    pub async fn cli(&self, argv: &[&str]) -> Response {
        let started = Instant::now();
        self.stage_script();
        if self.target.budget.is_some()
            && uses_project_build_config(argv)
            && !argv_has_budget_flag(argv)
        {
            self.target.metadata.lock().expect("metadata").push("acme:builder".to_owned());
        }
        self.files.mark_run_start();
        let backends = self.backends.clone().model(self.model.clone()).vcs(self.vcs.clone());
        let captured = self.deployment(argv).captured();
        let run = captured
            .run(backends, |deployment| {
                deployment.host::<WasiModel>()?;
                deployment.host::<WasiKeyValue>()?;
                deployment.host::<WasiBlobStore>()?;
                deployment.host::<WasiOtel>()?;
                deployment.host::<WasiVcs>()?;
                Ok(())
            })
            .await
            .expect("emery runs");
        self.ingest_calls();
        eprintln!(
            "provider cli {:?} {:.3?}",
            argv.first().copied().unwrap_or("emery"),
            started.elapsed()
        );
        response_of(run)
    }
}

impl Provider {
    fn build(model: ScriptedModel) -> Self {
        let project = scratch();
        seed_build_fixtures(project.path());
        let store = scratch();
        let registry = scratch();
        let backends = block_on(Backends::defaults());
        let storage = Storage::from_backends(&backends);
        let mut files = super::files::Files::default();
        files.bind(&project);
        let vcs = VcsScript::default();
        vcs.bind_project(project.path());
        let guest = engine_guest();
        let (script_source, script_target) = adapter_paths();
        publish_adapters(&store, &registry, &script_source, &script_target);
        Self {
            model,
            source: SourceScript::default(),
            target: TargetScript::default(),
            vcs,
            files,
            storage,
            backends,
            project,
            store,
            registry,
            guest,
            script_source,
            script_target,
        }
    }

    fn deployment(&self, argv: &[&str]) -> Deployment {
        let (command_name, command_args) = deployment_command(argv);
        Deployment::new()
            .guest("emery", self.guest.clone())
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
        fs::create_dir_all(&dir).expect("script dir");
        fs::write(dir.join(STATE), serde_json::to_vec_pretty(&script).expect("script JSON"))
            .expect("script state");
        let _ = fs::remove_file(dir.join(CALLS));
        let _ = fs::remove_dir_all(dir.join(ARRIVED));
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
                            |err| EvidenceScript::Err(ScriptError::from_adapter(err)),
                            |evidence| EvidenceScript::Ok(evidence.clone()),
                        ),
                    )
                })
                .collect(),
            held: self.source.held.clone(),
            versions: self.source.versions.clone(),
            kinds: self.source.kinds.clone(),
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
                            |err| ReportScript::Err(ScriptError::from_adapter(err)),
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
                .map(|(key, errs)| {
                    (key.clone(), errs.iter().map(ScriptError::from_adapter).collect())
                })
                .collect(),
            verdicts: self
                .target
                .verdicts
                .lock()
                .expect("verdicts")
                .iter()
                .map(|value| {
                    value.as_ref().map_or_else(
                        |err| VerdictScript::Err(ScriptError::from_adapter(err)),
                        |verdict| VerdictScript::Ok(verdict.clone()),
                    )
                })
                .collect(),
            versions: self.target.versions.clone(),
            rules: self.target.rules.iter().map(RuleWire::from).collect(),
            budget: self.target.budget,
            rendezvous: self.target.rendezvous.as_ref().map(|r| {
                test_programs::script::RendezvousScript {
                    expected: r.expected.clone(),
                }
            }),
        }
    }

    fn ingest_calls(&self) {
        let path = self.project.path().join(MOUNT).join(CALLS);
        let Ok(text) = fs::read_to_string(path) else {
            return;
        };
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let call: ScriptCall =
                serde_json::from_str(line).unwrap_or_else(|err| panic!("call line: {err}: {line}"));
            match call {
                ScriptCall::SourceMetadata { id } => {
                    self.source.metadata.lock().expect("metadata").push(id);
                }
                ScriptCall::SourceExtract { id, input } => {
                    self.source.calls.lock().expect("calls").push((id, input));
                }
                ScriptCall::TargetMetadata { id } => {
                    self.target.metadata.lock().expect("metadata").push(id);
                }
                ScriptCall::TargetInit { workspace, .. } => {
                    self.target.inits.lock().expect("inits").push(workspace);
                }
                ScriptCall::TargetBuild {
                    id,
                    slice_id,
                    workspace,
                } => {
                    self.target.calls.lock().expect("calls").push((
                        id,
                        Slice {
                            id: slice_id,
                            part: String::new(),
                            name: String::new(),
                            index: 1,
                            count: 1,
                            base: String::new(),
                            requirements: Vec::new(),
                            types: Vec::new(),
                            brief: Vec::new(),
                            spec: String::new(),
                            design: String::new(),
                            plan: String::new(),
                        },
                        workspace,
                    ));
                }
                ScriptCall::TargetVerify { workspace, .. } => {
                    self.target.verifies.lock().expect("verifies").push(workspace);
                }
            }
        }
    }
}

/// Runs one CLI invocation, returning the raw response.
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

pub const fn evidence(claims: Vec<Claim>) -> Evidence {
    Evidence { claims }
}

fn response_of(run: Run) -> Response {
    Response {
        stdout: run.stdout.into_bytes(),
        stderr: run.stderr.into_bytes(),
        exit: run.status.code().try_into().unwrap_or(1),
    }
}

fn adapter_paths() -> (PathBuf, PathBuf) {
    (
        PathBuf::from(test_programs::ADAPTER_SCRIPT_SOURCE),
        PathBuf::from(test_programs::ADAPTER_SCRIPT_TARGET),
    )
}

fn publish_adapters(store: &Scratch, registry: &Scratch, source: &Path, target: &Path) {
    let root = store.path();
    for (reference, wasm) in [
        ("acme:source@1.0.0", source),
        ("acme:source@0.1.0", source),
        ("acme:builder@1.0.0", target),
        ("acme:builder@2.1.0", target),
        ("acme:target@0.1.0", target),
    ] {
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

fn argv_has_budget_flag(argv: &[&str]) -> bool {
    let (_, command_args) = deployment_command(argv);
    command_args.iter().any(|arg| *arg == "--budget" || *arg == "-b")
}

fn uses_project_build_config(argv: &[&str]) -> bool {
    let (_, command_args) = deployment_command(argv);
    matches!(
        command_args.windows(2).find(|window| window[0] == "--config").map(|window| window[1]),
        Some("tests/build/project.toml") | None
    )
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
        "[namespace_registries]\nacme = \"registry.test\"\n\n[registry.\"registry.test\"]\n\
         default = \"local\"\n\n[registry.\"registry.test\".local]\nroot = {:?}\n",
        root.display()
    )
    .into()
}
