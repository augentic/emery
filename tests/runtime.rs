//! Verifies the adapter boundary under the omnia runtime.
//!
//! Each scenario deploys the engine guest the binary embeds over scratch
//! roots — the project mounted writable as `.`, the package store, and a
//! wasm-pkg `local` registry — under the hosts the engine imports, and
//! drives it over in-memory backends and a scripted model. omnia's real
//! store and loader answer every load, so what a reference resolves to is
//! asserted here and nowhere natively, and the operator's `$HOME` and the
//! network are never touched.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use omnia::{CompileOptions, Digest, ExitStatus, StoreCtx};
use omnia_test::host::{Backends, Deployment, Run, Scratch, ScriptedModel, scratch};
use omnia_wasi_blobstore::WasiBlobstore;
use omnia_wasi_keyvalue::WasiKeyValue;
use omnia_wasi_model::WasiModel;
use omnia_wasi_otel::WasiOtel;
use serde_json::Value;

// The engine `build.rs` emits, as the binary embeds it.
const GUEST: &str = env!("EMERY_GUEST");

// The two mocks `build.rs` emits, raw wasm in every profile.
const MOCK_SOURCE: &str = env!("EMERY_MOCK_SOURCE");
const MOCK_TARGET: &str = env!("EMERY_MOCK_TARGET");

// Every default in memory, the scripted model answering.
type Bundle = Backends<ScriptedModel>;

// The references the scenarios name, under a namespace the binary routes
// nowhere; the scratch registry serves it where a scenario routes it.
const SOURCE: &str = "acme:source@0.1.0";
const TARGET: &str = "acme:target@0.1.0";

const GREETING: &str = include_str!("../examples/docs/greeting.md");
const EXTRACT_ANSWER: &str = r#"{"claims": [{"kind": "requirement", "id": "greeting.behaviour",
    "path": "docs/greeting.md#L3", "statement": "GET /greeting returns the static string 'hello'."}]}"#;
const SPEC_ANSWER: &str = include_str!("specify/spec-draft.json");
const DESIGN_ANSWER: &str = include_str!("specify/design-draft.json");
const WRITE_GREETING: &str = r##"{"path": "build/greeting.md", "content": "# Greeting\n"}"##;
const REPORT_ANSWER: &str = r#"{"covered": ["REQ-001"], "written": ["build/greeting.md"]}"#;

// The three roots a scenario lays: the project the run reads and builds
// into, the store a release is kept in, and the registry one is fetched from.
struct Roots {
    project: Scratch,
    store: Scratch,
    registry: Scratch,
}

impl Roots {
    fn new() -> Self {
        let project = scratch();
        project.write("docs/greeting.md", GREETING);
        Self {
            project,
            store: scratch(),
            registry: scratch(),
        }
    }

    // The engine over these roots, the `acme` namespace routed to the
    // scratch registry.
    fn deployment(&self, args: &[&str]) -> Deployment {
        self.unrouted(args).registries(local_registry_toml(self.registry.path()))
    }

    // The engine over these roots routing no namespace, as the binary routes
    // no `acme`.
    fn unrouted(&self, args: &[&str]) -> Deployment {
        Deployment::new()
            .guest("emery", GUEST)
            .mount(self.project.mount(true))
            .store(self.store.path())
            .args(args.iter().copied())
    }

    // A release copied into the store, as `cp` or `wkg get -o` lays it.
    fn stored(&self, reference: &str, wasm: impl AsRef<Path>) -> PathBuf {
        let target = self.store.path().join(stored_name(reference));
        fs::copy(wasm, &target).expect("copying the release into the store");
        target
    }

    // A release the `local` backend serves, at `<root>/<namespace>/<name>/<version>.wasm`.
    fn published(&self, reference: &str, wasm: impl AsRef<Path>) {
        let (package, version) = reference.split_once('@').expect("a versioned reference");
        let (namespace, name) = package.split_once(':').expect("a namespaced reference");
        let dir = self.registry.path().join(namespace).join(name);
        fs::create_dir_all(&dir).expect("creating the package directory");
        fs::copy(wasm, dir.join(format!("{version}.wasm"))).expect("publishing the release");
    }

    // Every file the store holds, by name.
    fn stored_files(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.store.path())
            .expect("reading the store")
            .map(|entry| entry.expect("a store entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

// The file name a reference is kept under: `<namespace>_<name>@<version>.wasm`.
fn stored_name(reference: &str) -> String {
    let (package, version) = reference.split_once('@').expect("a versioned reference");
    format!("{}@{version}.wasm", package.replace(':', "_"))
}

// Routes the `acme` namespace to a wasm-pkg `local` backend at `root`.
fn local_registry_toml(root: &Path) -> String {
    format!(
        "[namespace_registries]\nacme = \"registry.test\"\n\n[registry.\"registry.test\"]\n\
         default = \"local\"\n\n[registry.\"registry.test\".local]\nroot = {:?}\n",
        root.display().to_string()
    )
}

// What `omnia compile` writes for `wasm`, under the scratch registry root so
// it is removed with the roots.
fn precompiled(roots: &Roots, wasm: &str) -> PathBuf {
    let target = roots.registry.path().join("precompiled.cwasm");
    omnia::compile::compile(
        Path::new(wasm),
        Some(target.clone()),
        None,
        &CompileOptions::default(),
    )
    .expect("compiling the mock");
    target
}

// One run of the engine over `backends` with `model` answering.
async fn emery(deployment: Deployment, backends: &Backends, model: &ScriptedModel) -> Run {
    deployment
        .captured()
        .run(backends.clone().model(model.clone()), link)
        .await
        .expect("the deployment assembles and the command runs")
}

// The hosts the engine imports; the guest loader is assembly's.
fn link(deployment: &mut omnia::Deployment<StoreCtx<Bundle>>) -> anyhow::Result<()> {
    deployment.host::<WasiModel, Bundle>()?;
    deployment.host::<WasiKeyValue, Bundle>()?;
    deployment.host::<WasiBlobstore, Bundle>()?;
    deployment.host::<WasiOtel, Bundle>()?;
    Ok(())
}

fn assert_ok(run: &Run) {
    assert_eq!(run.status, ExitStatus::SUCCESS, "stdout: {}\nstderr: {}", run.stdout, run.stderr);
}

// The JSON envelope a refused run writes to stderr, with its exit and code asserted.
fn refused(run: &Run, exit: i32, code: &str) -> Value {
    assert_eq!(run.status.code(), exit, "{code}: stdout: {}\nstderr: {}", run.stdout, run.stderr);
    assert!(run.stdout.is_empty(), "{code}: a failure writes nothing to stdout: {}", run.stdout);
    let start = run
        .stderr
        .find('{')
        .unwrap_or_else(|| panic!("{code}: no envelope on stderr: {}", run.stderr));
    let end = run.stderr.rfind('}').expect("the envelope closes");
    let envelope: Value =
        serde_json::from_str(&run.stderr[start..=end]).expect("one JSON envelope");
    assert_eq!(envelope["error"], code, "{envelope}");
    assert_eq!(envelope["exit-code"], exit, "{envelope}");
    envelope
}

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

// The specify script: the mock source's one extract turn, then the engine's
// spec and design drafts; one stem plans with no turn spent.
fn specifying() -> ScriptedModel {
    ScriptedModel::answering([EXTRACT_ANSWER, SPEC_ANSWER, DESIGN_ANSWER])
}

// --- the store ---

// A release the store holds is the adapter: nothing is fetched, the file is
// read, and the run proceeds through every turn to a revision.
#[tokio::test]
async fn specify_stored() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    let backends = Backends::defaults().await;
    let model = specifying();

    let run = emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await;

    assert_ok(&run);
    assert!(run.stdout.starts_with("committed revision "), "{}", run.stdout);
    model.assert_exhausted();
    assert_eq!(roots.stored_files(), [stored_name(SOURCE)], "the store is read, never written");

    // the revision is the one the next verb reads
    let shown =
        emery(roots.deployment(&["show", "spec"]), &backends, &ScriptedModel::default()).await;
    assert_ok(&shown);
    assert!(shown.stdout.contains("### Requirement: greeting.behaviour"), "{}", shown.stdout);
}

// A stored target builds every slice of the committed plan into the project
// tree through its turn's `write_file`, and the tree is the output.
#[tokio::test]
async fn build_stored() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = Backends::defaults().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await);
    model.assert_exhausted();

    let model =
        ScriptedModel::answering([REPORT_ANSWER]).calling(0, [("write_file", WRITE_GREETING)]);
    let run = emery(roots.deployment(&["build", TARGET]), &backends, &model).await;

    assert_ok(&run);
    assert!(run.stdout.starts_with("built revision "), "{}", run.stdout);
    assert!(
        run.stdout.contains("SLICE-001 greeting: covered 1/1, written 1 file"),
        "{}",
        run.stdout
    );
    model.assert_exhausted();
    assert_eq!(
        roots.project.read("build/greeting.md"),
        Some(b"# Greeting\n".to_vec()),
        "the target wrote the tree through the turn"
    );
}

// A release the store lacks is fetched through the registry its namespace
// is routed to and kept under its reference's name; the next run reads the
// file with the registry gone.
#[tokio::test]
async fn specify_fetched() {
    let roots = Roots::new();
    roots.published(SOURCE, MOCK_SOURCE);
    let backends = Backends::defaults().await;
    let model = specifying();

    let run = emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    assert_eq!(roots.stored_files(), [stored_name(SOURCE)], "the fetched release is kept");
    assert_eq!(
        roots.store.read(&stored_name(SOURCE)),
        Some(fs::read(MOCK_SOURCE).expect("the mock source")),
        "the bytes are kept whole"
    );

    // the registry gone, the stored file answers
    fs::remove_dir_all(roots.registry.path().join("acme")).expect("unpublishing the release");
    let model = specifying();
    let run = emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await;
    assert_ok(&run);
    assert!(run.stdout.contains("none (byte-stable)"), "{}", run.stdout);
    model.assert_exhausted();
}

// A registry serving a pre-compiled artifact is refused: a package admits
// raw wasm alone, and nothing is kept.
#[tokio::test]
async fn precompiled_fetched() {
    let roots = Roots::new();
    let cwasm = precompiled(&roots, MOCK_SOURCE);
    roots.published(SOURCE, &cwasm);
    let backends = Backends::defaults().await;
    let model = ScriptedModel::default();

    let run =
        emery(roots.deployment(&["--format", "json", "specify", SOURCE]), &backends, &model).await;

    let envelope = refused(&run, 1, "refused");
    assert_message(&envelope, &format!("adapter `{SOURCE}`: "));
    assert_message(&envelope, "raw wasm alone");
    assert!(roots.stored_files().is_empty(), "nothing pre-compiled is kept");
    model.assert_exhausted();
}

// A namespace the binary routes nowhere, absent from the store, is refused
// before any fetch: the message names the namespace and the store, and the
// hint the `wkg get` that fills it.
#[tokio::test]
async fn specify_unrouted() {
    let roots = Roots::new();
    let backends = Backends::defaults().await;
    let model = ScriptedModel::default();

    let run = emery(
        roots.unrouted(&["--format", "json", "specify", "acme:nope@1.0.0"]),
        &backends,
        &model,
    )
    .await;

    let envelope = refused(&run, 1, "refused");
    assert_message(&envelope, "adapter `acme:nope@1.0.0`: no registry routes `acme:nope@1.0.0`");
    assert_message(&envelope, "the `acme` namespace");
    assert_message(
        &envelope,
        &format!("the store `{}` holds no `acme_nope@1.0.0.wasm`", roots.store.path().display()),
    );
    let hint = envelope["hint"].as_str().unwrap_or("");
    assert!(hint.contains("wkg get <reference> -o ~/.emery/adapters/"), "{envelope}");
    assert!(roots.stored_files().is_empty(), "nothing is kept");
    model.assert_exhausted();
}

// A file in the store not named as a reference is never read: the reference
// resolves against the names the store spells, and nothing else.
#[tokio::test]
async fn file_off_reference() {
    let roots = Roots::new();
    fs::copy(MOCK_SOURCE, roots.store.path().join("source.wasm")).expect("copying the mock");
    let backends = Backends::defaults().await;
    let model = ScriptedModel::default();

    let run =
        emery(roots.unrouted(&["--format", "json", "specify", SOURCE]), &backends, &model).await;

    let envelope = refused(&run, 1, "refused");
    assert_message(&envelope, &format!("holds no `{}`", stored_name(SOURCE)));
    assert_eq!(roots.stored_files(), ["source.wasm"], "the store is left as it was");
    model.assert_exhausted();
}

// --- the pin ---

// A `digest` the stored file hashes to rides the load and the run proceeds.
#[tokio::test]
async fn pin_held() {
    let roots = Roots::new();
    let stored = roots.stored(SOURCE, MOCK_SOURCE);
    let pin = Digest::of(&fs::read(stored).expect("the stored release"));
    roots.project.write(
        "emery.toml",
        format!("[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\ndigest = \"{pin}\"\n"),
    );
    let backends = Backends::defaults().await;
    let model = specifying();

    let run = emery(roots.deployment(&["specify"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
}

// A `digest` the stored file misses is refused before any turn: the pin is
// checked on the bytes that answered, stored or fetched.
#[tokio::test]
async fn pin_mismatch() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    let pin = Digest::of(b"other bytes");
    roots.project.write(
        "emery.toml",
        format!("[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\ndigest = \"{pin}\"\n"),
    );
    let backends = Backends::defaults().await;
    let model = ScriptedModel::default();

    let run = emery(roots.deployment(&["--format", "json", "specify"]), &backends, &model).await;

    let envelope = refused(&run, 1, "refused");
    assert_message(&envelope, &format!("adapter `{SOURCE}`: "));
    assert_message(&envelope, &format!("not its declared digest {pin}"));
    model.assert_exhausted();
}

// A pre-compiled artifact copied into the store is refused however it hashes:
// the store is raw wasm alone.
#[tokio::test]
async fn precompiled_in_store() {
    let roots = Roots::new();
    let cwasm = precompiled(&roots, MOCK_SOURCE);
    roots.stored(SOURCE, &cwasm);
    let backends = Backends::defaults().await;
    let model = ScriptedModel::default();

    let run =
        emery(roots.deployment(&["--format", "json", "specify", SOURCE]), &backends, &model).await;

    let envelope = refused(&run, 1, "refused");
    assert_message(&envelope, "raw wasm alone");
    model.assert_exhausted();
}

// --- the grammar at the boundary ---

// Two versions of one package would register as one guest, so the list is
// refused before either is read from the store.
#[tokio::test]
async fn two_versions_one_run() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored("acme:source@0.2.0-dev", MOCK_SOURCE);
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"acme:source@0.2.0-dev\"\n"
        ),
    );
    let backends = Backends::defaults().await;
    let model = ScriptedModel::default();

    let run = emery(roots.deployment(&["--format", "json", "specify"]), &backends, &model).await;

    let envelope = refused(&run, 1, "bad_request");
    assert_message(&envelope, "adapters `acme:source@0.1.0` and `acme:source@0.2.0-dev`");
    assert_message(&envelope, "would both register as `acme:source`");
    model.assert_exhausted();
}

// A source adapter named as the target is refused on its exports before any
// slice is dispatched or a turn spent.
#[tokio::test]
async fn build_source_only() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    let backends = Backends::defaults().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await);
    model.assert_exhausted();

    let model = ScriptedModel::default();
    let run =
        emery(roots.deployment(&["--format", "json", "build", SOURCE]), &backends, &model).await;

    let envelope = refused(&run, 1, "bad_request");
    assert_message(
        &envelope,
        &format!(
            "adapter `{SOURCE}` is not a target adapter: it exports no `emery:adapter/target@0.1.0`"
        ),
    );
    assert!(roots.project.read("build/greeting.md").is_none(), "nothing is built");
    model.assert_exhausted();
}

// A store beneath the writable project mount is refused at assembly, present
// or absent: a guest could rewrite what the deployment loads.
#[tokio::test]
async fn store_beneath_writable() {
    let roots = Roots::new();
    let beneath = roots.project.path().join("adapters");
    let backends = Backends::defaults().await;

    for present in [false, true] {
        if present {
            fs::create_dir_all(&beneath).expect("creating the store beneath the project");
        }
        let error = roots
            .unrouted(&["specify", SOURCE])
            .store(&beneath)
            .captured()
            .run(backends.clone().model(ScriptedModel::default()), link)
            .await
            .expect_err("the store beneath the project is refused at assembly");
        let message = format!("{error:#}");
        assert!(
            message.contains("lies beneath the writable mount `.`"),
            "present {present}: {message}"
        );
    }
}
