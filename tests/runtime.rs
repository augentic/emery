//! End-to-end runtime integration tests for the engine guest, plugin loader, and git VCS.
//!
//! Deploys `EMERY_GUEST` under `omnia_test::host` over scratch mounts, a local
//! package registry, a hermetic git client, and a scripted model.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

use emery_engine::vcs::Message;
use omnia::{Backend as _, CompileOptions, Digest, ExitStatus, RuntimeOptions, StoreCtx};
use omnia_git::{Client, ConnectOptions};
use omnia_test::host::{Backends, Deployment, Run, Scratch, ScriptedModel, scratch};
use omnia_wasi_blobstore::WasiBlobstore;
use omnia_wasi_keyvalue::WasiKeyValue;
use omnia_wasi_model::WasiModel;
use omnia_wasi_otel::WasiOtel;
use omnia_wasi_vcs::WasiVcs;
use serde_json::Value;
use sha2::{Digest as _, Sha256};

// The engine `build.rs` emits, as the binary embeds it; `guest` is what a
// scenario deploys.
const GUEST: &str = env!("EMERY_GUEST");

// The mocks `build.rs` emits, raw wasm in every profile.
const MOCK_SOURCE: &str = env!("EMERY_MOCK_SOURCE");
const MOCK_TARGET: &str = env!("EMERY_MOCK_TARGET");

// Every default in memory, the scripted model answering, git behind `omnia:vcs`.
type Bundle = <Backends<ScriptedModel> as Over<Client>>::Bundle;

// A type alias cannot fill the last parameter of `Backends` without naming
// the eight before it, which are omnia-test's defaults; this projection
// swaps the one slot, as the `vcs` setter does at the value level.
trait Over<N> {
    type Bundle;
}

impl<M, K, B, D, S, V, G, I, O, C, N> Over<N> for Backends<M, K, B, D, S, V, G, I, O, C> {
    type Bundle = Backends<M, K, B, D, S, V, G, I, O, N>;
}

// The references the scenarios name, under a namespace the binary routes
// nowhere; the scratch registry serves it where a scenario routes it.
const SOURCE: &str = "acme:source@0.1.0";
const TARGET: &str = "acme:target@0.1.0";
// The target's repository under the project, named `target` as the adapter is.
const TARGET_REPO: &str = ".emery/vcs/repos/target";

const GREETING: &str = include_str!("../examples/docs/greeting.md");
const EXTRACT_ANSWER: &str = r#"{"claims": [{"kind": "requirement", "id": "greeting.behaviour",
    "path": "docs/greeting.md#L3", "statement": "GET /greeting returns the static string 'hello'."}]}"#;
const SPEC_ANSWER: &str = include_str!("specify/spec-draft.json");
const DESIGN_ANSWER: &str = include_str!("specify/design-draft.json");
const WRITE_GREETING: &str =
    r##"{"files": [{"path": "build/greeting.md", "content": "# Greeting\n"}]}"##;
const REPORT_ANSWER: &str = r#"{"covered": ["REQ-001"], "written": ["build/greeting.md"]}"#;
const VERDICT_ANSWER: &str = r#"{"passed": true, "failures": []}"#;
// A build that writes a private key beside the greeting, and reports both.
const WRITE_LEAK: &str = r##"{"files": [
    {"path": "build/greeting.md", "content": "# Greeting\n"},
    {"path": "build/leak.pem", "content": "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA\n-----END RSA PRIVATE KEY-----\n"}]}"##;
const REPORT_LEAK: &str =
    r#"{"covered": ["REQ-001"], "written": ["build/greeting.md", "build/leak.pem"]}"#;
// A build whose turn wrote nothing through `write_files`.
const REPORT_BARE: &str = r#"{"covered": ["REQ-001"], "written": []}"#;
// A token spelled as a symbolic link's target: the text a commit stores of the link.
const TOKEN: &str = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

// A two-stem journey: a greeting and a farewell, grouped apart, drafted in
// one chunk, and sliced one stem apiece, so the plan is one wave of two.
const EXTRACT_PAIR: &str = r#"{"claims": [
    {"kind": "requirement", "id": "greeting.behaviour", "path": "docs/greeting.md#L3",
     "statement": "GET /greeting returns the static string 'hello'."},
    {"kind": "requirement", "id": "farewell.behaviour", "path": "docs/greeting.md#L7",
     "statement": "GET /farewell returns the static string 'bye'."}]}"#;
const GROUPING_PAIR: &str =
    r#"{"groups": [{"claims": [0], "classes": [[0]]}, {"claims": [1], "classes": [[1]]}]}"#;
const SPEC_PAIR: &str = r#"{"preamble": ["Two surfaces: a greeting and a farewell."],
    "requirements": [
    {"subject": "greeting.behaviour", "scenarios": [{"name": "Greeting requested",
     "given": ["the greeting surface is bound"], "when": "`/greeting` is requested",
     "then": "the response is `hello`"}]},
    {"subject": "farewell.behaviour", "scenarios": [{"name": "Farewell requested",
     "given": ["the farewell surface is bound"], "when": "`/farewell` is requested",
     "then": "the response is `bye`"}]}]}"#;
const SLICING_PAIR: &str = r#"{"preamble": [], "slices": [
    {"name": "greeting", "requirements": ["REQ-001"]},
    {"name": "farewell", "requirements": ["REQ-002"]}]}"#;
// Both slices write `build/index.md`, each with its own line.
const WRITE_INDEX_GREETING: &str =
    r##"{"files": [{"path": "build/index.md", "content": "# Index\n\n- greeting\n"}]}"##;
const WRITE_INDEX_FAREWELL: &str =
    r##"{"files": [{"path": "build/index.md", "content": "# Index\n\n- farewell\n"}]}"##;
const REPORT_INDEX_GREETING: &str = r#"{"covered": ["REQ-001"], "written": ["build/index.md"]}"#;
const REPORT_INDEX_FAREWELL: &str = r#"{"covered": ["REQ-002"], "written": ["build/index.md"]}"#;
const RESOLVED_INDEX: &str = "# Index\n\n- greeting\n- farewell\n";
const MERGE_INDEX: &str = r##"{"files": [{"path": "build/index.md", "content": "# Index\n\n- greeting\n- farewell\n"}]}"##;

// Git under a scratch configuration: a wrapper exporting `GIT_CONFIG_GLOBAL`
// and `GIT_CONFIG_NOSYSTEM` before handing over to the `git` on `PATH`, so
// nothing of the developer's identity, signing, hooks, or default branch
// reaches a scenario; the backend runs it as a deployment's `GIT_BINARY`
// would, and the oracle reads repositories through it.
struct Git {
    root: Scratch,
}

impl Git {
    fn new() -> Self {
        let root = scratch();
        root.write(
            "gitconfig",
            "[user]\n\tname = Emery Test\n\temail = test@emery.invalid\n\
             [init]\n\tdefaultBranch = main\n\
             [commit]\n\tgpgsign = false\n\
             [core]\n\thooksPath = /dev/null\n",
        );
        let wrapper = root.path().join("git");
        fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nexport GIT_CONFIG_GLOBAL='{}' GIT_CONFIG_NOSYSTEM=1\nexec git \"$@\"\n",
                root.path().join("gitconfig").display()
            ),
        )
        .expect("writing the git wrapper");
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))
            .expect("marking the wrapper executable");
        Self { root }
    }

    // The backends over this git, the model set per run.
    async fn backends(&self) -> Bundle {
        let client = Client::connect_with(ConnectOptions {
            binary: self.binary().to_str().expect("scratch path is UTF-8").to_owned(),
        })
        .await
        .expect("connecting over the hermetic git");
        Backends::defaults().await.vcs(client).model(ScriptedModel::default())
    }

    // The oracle: one git over the wrapper, held to success, its stdout trimmed.
    fn run(&self, at: &Path, args: &[&str]) -> String {
        let output = Command::new(self.root.path().join("git"))
            .arg("-C")
            .arg(at)
            .args(args)
            .output()
            .expect("running the oracle git");
        assert!(
            output.status.success(),
            "git {args:?} at {}: {}",
            at.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    // Seals everything under `at` as one commit, initialising the repository first.
    fn seal(&self, at: &Path, message: &str) -> String {
        if !at.join(".git").exists() {
            self.run(at, &["init", "--quiet"]);
        }
        self.run(at, &["add", "--all"]);
        self.run(at, &["commit", "--quiet", "--allow-empty", "--message", message]);
        self.run(at, &["rev-parse", "HEAD"])
    }

    // A repository under `name` holding `files` at one commit, tagged `tag`,
    // and the `file://` URL a clone reaches it by.
    fn published(&self, name: &str, files: &[(&str, &str)], tag: &str) -> (PathBuf, String) {
        let at = self.root.path().join(name);
        for (relative, contents) in files {
            let path = at.join(relative);
            fs::create_dir_all(path.parent().expect("a parent")).expect("creating the tree");
            fs::write(path, contents).expect("writing the tree");
        }
        self.seal(&at, "initial");
        self.run(&at, &["tag", tag]);
        let url = format!("file://{}", at.display());
        (at, url)
    }

    // A bare repository under `name` whose `main` holds `files`, and its URL.
    fn bare(&self, name: &str, files: &[(&str, &str)]) -> (PathBuf, String) {
        let at = self.root.path().join(name);
        fs::create_dir_all(&at).expect("creating the bare repository");
        self.run(&at, &["init", "--quiet", "--bare"]);
        let url = format!("file://{}", at.display());
        let (seed, _) = self.published(&format!("{name}-seed"), files, "seed");
        self.run(&seed, &["push", "--quiet", &url, "HEAD:refs/heads/main"]);
        (at, url)
    }

    // Advances the `main` of the repository at `url` by one commit writing
    // `files`, through a clone of its own, and returns that commit.
    fn advanced(&self, name: &str, url: &str, files: &[(&str, &str)]) -> String {
        let work = self.root.path().join(name);
        self.run(self.root.path(), &["clone", "--quiet", url, name]);
        for (relative, contents) in files {
            fs::write(work.join(relative), contents).expect("writing the tree");
        }
        let commit = self.seal(&work, "next");
        self.run(&work, &["push", "--quiet", url, "HEAD:refs/heads/main"]);
        commit
    }

    // A commit written behind the engine's back: `HEAD`'s tree under
    // `message`, over `parent`, or a root with none, so its history is
    // whatever a scenario claims rather than what a build sealed.
    fn forged(&self, at: &Path, parent: Option<&str>, message: &str) -> String {
        let tree = self.run(at, &["rev-parse", "HEAD:"]);
        let mut args = vec!["commit-tree", tree.as_str(), "-m", message];
        if let Some(parent) = parent {
            args.extend(["-p", parent]);
        }
        self.run(at, &args)
    }

    // The wrapper itself, for a scenario that runs git where `run` cannot reach.
    fn binary(&self) -> PathBuf {
        self.root.path().join("git")
    }

    // The branches a repository holds, names in order, spelled the same
    // whether or not a tag shares one.
    fn branches(&self, at: &Path) -> Vec<String> {
        self.run(at, &["for-each-ref", "--format=%(refname)", "refs/heads/"])
            .lines()
            .map(|name| name.strip_prefix("refs/heads/").unwrap_or(name).to_owned())
            .collect()
    }

    // The commit a fresh build's first wave stands on beneath the adapter's
    // `Initialise`: `head`'s grandparent along the first parent, its parent
    // asserted to be that commit.
    fn beneath_initialise(&self, at: &Path, head: &str) -> String {
        assert_eq!(
            self.run(at, &["log", "-1", "--format=%s", &format!("{head}~1")]),
            "Initialise",
            "a fresh build seals the adapter's layout before its first wave"
        );
        self.run(at, &["rev-parse", &format!("{head}~2")])
    }
}

// The three roots a scenario creates — the project the run reads and builds
// into, the store a release is kept in, and the registry one is fetched
// from — and the git behind the runs over them.
struct Roots {
    project: Scratch,
    store: Scratch,
    registry: Scratch,
    git: Git,
}

impl Roots {
    fn new() -> Self {
        let project = scratch();
        project.write("docs/greeting.md", GREETING);
        Self {
            project,
            store: scratch(),
            registry: scratch(),
            git: Git::new(),
        }
    }

    async fn backends(&self) -> Bundle {
        self.git.backends().await
    }

    // The project configured to build into its own repository: an `emery.toml`
    // whose `[target]` names the mock and `repository = "."`, which a run
    // naming no adapter reads.
    fn targeted(&self) {
        self.project
            .write("emery.toml", format!("[target]\nadapter = \"{TARGET}\"\nrepository = \".\"\n"));
    }

    // The project as a repository sealed at one commit, configured to build
    // into it, which a build starts from.
    fn sealed(&self) -> String {
        self.targeted();
        self.git.seal(self.project.path(), "project")
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
            .guest("emery", guest())
            .mount(self.project.mount(true))
            .store(self.store.path())
            .args(args.iter().copied())
    }

    // A release copied into the store, as `cp` or `wkg get -o` writes it.
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

// The engine guest compiled for this host once per build, under
// `CARGO_TARGET_TMPDIR` keyed by its bytes, so every process nextest runs
// deploys one artifact: the first to take the lock compiles it under the
// deployment's own compile options, the rest wait and read it. A guest
// `build.rs` already precompiled is deployed as it is.
fn guest() -> PathBuf {
    let bytes = fs::read(GUEST).expect("reading the engine guest");
    if !bytes.starts_with(b"\0asm") {
        return PathBuf::from(GUEST);
    }
    let key = hex::encode(&Sha256::digest(&bytes)[..8]);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let compiled = dir.join(format!("emery-{key}.cwasm"));
    if compiled.exists() {
        return compiled;
    }

    // compile once, under the lock
    fs::create_dir_all(dir).expect("creating the target tmpdir");
    let lock = fs::File::create(dir.join(format!("emery-{key}.lock"))).expect("creating the lock");
    lock.lock().expect("locking the artifact");
    if !compiled.exists() {
        let options = RuntimeOptions::load_env().expect("the runtime options").compile_options();
        let staged = dir.join(format!("emery-{key}.cwasm.tmp"));
        omnia::compile::compile(Path::new(GUEST), Some(staged.clone()), None, &options)
            .expect("compiling the engine guest");
        fs::rename(&staged, &compiled).expect("placing the artifact");
    }
    compiled
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
async fn emery(deployment: Deployment, backends: &Bundle, model: &ScriptedModel) -> Run {
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
    deployment.host::<WasiVcs, Bundle>()?;
    Ok(())
}

// The revision id a `specify`, `apply`, or `build` run reports on its first line.
fn revision_of(run: &Run) -> String {
    let first = run.stdout.lines().next().unwrap_or_default();
    first
        .strip_prefix("staged candidate ")
        .or_else(|| first.strip_prefix("sealed revision "))
        .or_else(|| first.strip_prefix("built revision "))
        .unwrap_or_else(|| panic!("no revision line: {}", run.stdout))
        .to_owned()
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

// The build script over the one-slice plan: the mock target's build turn,
// writing the greeting through `write_files`, then its verify turn over the
// integrated tree.
fn building() -> ScriptedModel {
    ScriptedModel::answering([REPORT_ANSWER, VERDICT_ANSWER])
        .calling(0, [("write_files", WRITE_GREETING)])
}

// A build turn that leaves `copy` as `shell` left it and reports no file
// written: what the model's shell leaves in a working copy, which no scripted
// tool writes, is written by host code as the turn is put.
fn shelled(copy: PathBuf, shell: impl Fn(&Path) + Send + Sync + 'static) -> ScriptedModel {
    ScriptedModel::default().then(move || {
        shell(&copy);
        REPORT_BARE.to_owned()
    })
}

// The one-slice journey up to its build: both mocks stored, the project
// specified with its revision sealed, and the checkout sealed. Returns the
// roots, the backends, and the base.
async fn specified() -> (Roots, Bundle, String) {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await);
    model.assert_exhausted();
    let base = roots.sealed();
    (roots, backends, base)
}

// The journey through its first build into the project: both mocks stored,
// the project specified and sealed, the plan built and labelled. Returns the
// roots, the backends, the revision, and the base the build started from.
async fn built() -> (Roots, Bundle, String, String) {
    let (roots, backends, base) = specified().await;
    let model = building();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;
    assert_ok(&run);
    model.assert_exhausted();
    let revision = revision_of(&run);
    (roots, backends, revision, base)
}

// --- the store ---

// A release the store holds is the adapter: nothing is fetched, the file is
// read, and the run proceeds through every turn to a staged candidate, which
// `show --staged` reads and `apply` seals as the revision `show` reads.
#[tokio::test]
async fn specify_stored() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    let backends = roots.backends().await;
    let model = specifying();

    let run = emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await;

    assert_ok(&run);
    assert!(run.stdout.starts_with("staged candidate "), "{}", run.stdout);
    assert!(run.stdout.ends_with("nothing sealed; `emery apply` seals it\n"), "{}", run.stdout);
    model.assert_exhausted();
    assert_eq!(roots.stored_files(), [stored_name(SOURCE)], "the store is read, never written");
    let candidate = revision_of(&run);

    // the candidate is what `show --staged` reads; nothing is sealed yet
    let idle = ScriptedModel::default();
    let shown = emery(roots.deployment(&["show", "spec", "--staged"]), &backends, &idle).await;
    assert_ok(&shown);
    assert!(shown.stdout.contains("### Requirement: greeting.behaviour"), "{}", shown.stdout);
    let run =
        emery(roots.deployment(&["--format", "json", "show", "spec"]), &backends, &idle).await;
    let envelope = refused(&run, 2, "spec-not-generated");
    assert_message(&envelope, &format!("candidate `{candidate}` is staged and not sealed"));

    // apply seals it as the revision the next verb reads
    let run = emery(roots.deployment(&["apply"]), &backends, &idle).await;
    assert_ok(&run);
    assert_eq!(run.stdout, format!("sealed revision {candidate}\n"));
    let shown = emery(roots.deployment(&["show", "spec"]), &backends, &idle).await;
    assert_ok(&shown);
    assert!(shown.stdout.contains("### Requirement: greeting.behaviour"), "{}", shown.stdout);
    let run = emery(
        roots.deployment(&["--format", "json", "show", "spec", "--staged"]),
        &backends,
        &idle,
    )
    .await;
    refused(&run, 2, "nothing-staged");
}

// A stored target named `repository = "."` initialises the integration
// working copy with its layout, sealed as `Initialise` over the project's
// sealed head, builds the committed plan's one slice through its turn's
// `write_files` into a working copy of its own cut from that commit,
// verifies the integrated tree through a second turn, and the label
// `emery/<revision>` holds one merge commit over the initialised tree whose
// message carries the trailers a later run reads; the checkout itself is
// untouched.
#[tokio::test]
async fn build_merged_labelled() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await);
    model.assert_exhausted();
    let base = roots.sealed();

    let model = building();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    let lent = model.lent();
    let [build, verify] = lent.as_slice() else {
        panic!("a build turn, then a verify turn: {lent:?}");
    };
    assert!(
        build.as_deref().is_some_and(|at| at.ends_with(".emery/vcs/worktrees/SLICE-001")),
        "the build turn is lent the slice's working copy: {lent:?}"
    );
    assert!(
        verify.as_deref().is_some_and(|at| at.ends_with(".emery/vcs/integration")),
        "the verify turn is lent the integration working copy: {lent:?}"
    );
    let revision = revision_of(&run);
    let label = format!("emery/{revision}");
    let project = roots.project.path();
    let head = roots.git.run(project, &["rev-parse", &label]);
    let initialised = roots.git.run(project, &["rev-parse", &format!("{label}~1")]);
    assert!(
        run.stdout.contains(&format!(
            "\n  base {base}\n  initialised {}, 1 file\n  wave 1: 1 slice verified at {}\n    \
             SLICE-001 greeting: covered 1/1, written 1 file, merged {head}\n",
            &initialised[..8],
            &head[..8]
        )),
        "{}",
        run.stdout
    );
    assert!(run.stdout.ends_with(&format!("  labelled {label} at {head}\n")), "{}", run.stdout);

    // the label holds one merge over the initialised tree, which stands
    // over the base, bringing in the slice's own commit, which was built
    // over the initialised tree too
    assert_eq!(roots.git.run(project, &["log", "-1", "--format=%s", &initialised]), "Initialise");
    assert_eq!(roots.git.run(project, &["rev-parse", &format!("{initialised}~1")]), base);
    assert_eq!(
        roots.git.run(project, &["diff", "--name-only", &base, &initialised]),
        "build/README.md",
        "the initialisation seals what the adapter's layout scaffolded"
    );
    let side = roots.git.run(project, &["rev-parse", &format!("{label}^2")]);
    assert_eq!(roots.git.run(project, &["rev-parse", &format!("{side}~1")]), initialised);
    assert_eq!(
        roots.git.run(project, &["show", &format!("{label}:build/greeting.md")]),
        "# Greeting",
        "the target wrote the working copy through the turn"
    );
    assert!(
        roots
            .git
            .run(project, &["show", &format!("{initialised}:build/README.md")])
            .starts_with("# Build\n"),
        "the target's layout, scaffolded by its initialisation before any slice"
    );
    assert_eq!(
        roots
            .git
            .run(project, &["log", "--first-parent", "--format=%s", &format!("{base}..{label}")]),
        "SLICE-001 greeting\nInitialise",
        "the initialisation, then one merge per slice along the first-parent chain"
    );
    let message = roots.git.run(project, &["log", "-1", "--format=%B", &label]);
    assert!(message.starts_with("SLICE-001 greeting\n\n"), "{message}");
    for trailer in [
        "Slice: SLICE-001".to_owned(),
        format!("Revision: {revision}"),
        "Requirements: REQ-001".to_owned(),
        "Covered: REQ-001".to_owned(),
        format!("Adapter: {TARGET}"),
        format!("Base: {initialised}"),
        "Wave: 1".to_owned(),
    ] {
        assert!(message.contains(&trailer), "`{trailer}` in: {message}");
    }
    assert_eq!(
        roots.git.run(project, &["log", "-1", "--format=%B", &side]),
        message,
        "the slice's commit and its merge carry one message"
    );

    // the checkout is as the operator left it, and the working copies are gone
    assert_eq!(roots.git.run(project, &["rev-parse", "HEAD"]), base);
    assert!(roots.project.read("build/greeting.md").is_none(), "nothing lands in the checkout");
    assert!(!project.join(".emery/vcs/integration").exists(), "the working copy is removed");
    assert!(!project.join(".emery/vcs/worktrees/SLICE-001").exists(), "the slice's too");
    let listed = roots.git.run(project, &["worktree", "list", "--porcelain"]);
    assert_eq!(listed.matches("worktree ").count(), 1, "{listed}");
    assert_eq!(roots.git.branches(project), [label, "main".to_owned()]);
}

// A second run over a labelled revision resumes from the label: the slice
// its history records is not built again, so no turn is put, and the label
// stands where the first run left it.
#[tokio::test]
async fn build_resumed() {
    let (roots, backends, revision, base) = built().await;
    let label = format!("emery/{revision}");
    let project = roots.project.path();
    let head = roots.git.run(project, &["rev-parse", &label]);

    let model = ScriptedModel::default();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    assert!(model.lent().is_empty(), "no turn is put: {:?}", model.lent());
    assert_eq!(
        run.stdout,
        format!(
            "built revision {revision}\n  plan: 1 slice in 1 wave, widest 1\n  base {base}\n  \
             resumed: SLICE-001\n  labelled {label} at {head}\n"
        )
    );
    assert_eq!(roots.git.run(project, &["rev-parse", &label]), head, "the label stands");
    assert_eq!(roots.git.run(project, &["rev-parse", "HEAD"]), base);
    assert!(!project.join(".emery/vcs/integration").exists(), "the working copy is removed");
    assert_eq!(roots.git.branches(project), [label, "main".to_owned()]);
}

// A run whose wave the adapter does not verify leaves the slice merged in
// the integration working copy and sets no label; the next run reads the
// slice from that copy's history, puts no build turn, verifies the wave
// from where the copy stands, and labels it there.
#[tokio::test]
async fn build_resumed_staged() {
    let (roots, backends, base) = specified().await;
    let project = roots.project.path();
    let model = ScriptedModel::answering([
        REPORT_ANSWER,
        r#"{"passed": false, "failures": ["the greeting is not yet signed"]}"#,
    ])
    .calling(0, [("write_files", WRITE_GREETING)]);

    let run = emery(roots.deployment(&["--format", "json", "build"]), &backends, &model).await;

    let envelope = refused(&run, 1, "verify-failed");
    assert_message(
        &envelope,
        "SLICE-001 merged in it stays committed in `./.emery/vcs/integration`",
    );
    model.assert_exhausted();
    let integration = project.join(".emery/vcs/integration");
    let staged = roots.git.run(&integration, &["rev-parse", "HEAD"]);
    assert_eq!(
        roots
            .git
            .run(project, &["log", "--first-parent", "--format=%s", &format!("{base}..{staged}")]),
        "SLICE-001 greeting\nInitialise",
        "the slice merged over the initialised tree"
    );
    assert_eq!(roots.git.branches(project), ["main"], "no label is set");

    let model = ScriptedModel::answering([VERDICT_ANSWER]);
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    let lent = model.lent();
    let [Some(verify)] = lent.as_slice() else {
        panic!("one verify turn over the integration working copy, no build: {lent:?}");
    };
    assert!(verify.ends_with(".emery/vcs/integration"), "{lent:?}");
    let revision = revision_of(&run);
    let label = format!("emery/{revision}");
    assert_eq!(
        run.stdout,
        format!(
            "built revision {revision}\n  plan: 1 slice in 1 wave, widest 1\n  base {base}\n  \
             resumed: SLICE-001\n  wave 1: 1 slice verified at {}\n    SLICE-001: staged by an \
             earlier run, not built again\n  labelled {label} at {staged}\n",
            &staged[..8]
        )
    );
    assert_eq!(
        roots.git.run(project, &["rev-parse", &label]),
        staged,
        "the label is set where the first run left the slice merged"
    );
    assert_eq!(roots.git.run(project, &["rev-parse", "HEAD"]), base);
    assert!(!integration.exists(), "the working copy is removed once every slice is merged");
    let listed = roots.git.run(project, &["worktree", "list", "--porcelain"]);
    assert_eq!(listed.matches("worktree ").count(), 1, "{listed}");
    assert_eq!(roots.git.branches(project), [label, "main".to_owned()]);
}

// Two slices of one wave write one file: the second's merge conflicts at it,
// so both sides are read from the working copies through the engine's own
// file capability and put to the model, whose resolution is written into
// the slice's copy, sealed over the slice's commit, and merged again with
// the path taken whole; the label holds one merge per slice along the
// first-parent chain and the resolved file, and the slice's merge brings in
// the resolution over the slice's own commit.
#[tokio::test]
async fn build_conflict_resolved() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = ScriptedModel::answering([
        EXTRACT_PAIR,
        GROUPING_PAIR,
        SPEC_PAIR,
        DESIGN_ANSWER,
        SLICING_PAIR,
    ]);
    assert_ok(&emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await);
    model.assert_exhausted();
    let base = roots.sealed();

    // one build at a time, so the turns arrive in slice order
    let model = ScriptedModel::answering([
        REPORT_INDEX_GREETING,
        REPORT_INDEX_FAREWELL,
        MERGE_INDEX,
        VERDICT_ANSWER,
    ])
    .calling(0, [("write_files", WRITE_INDEX_GREETING)])
    .calling(1, [("write_files", WRITE_INDEX_FAREWELL)]);
    let run = emery(roots.deployment(&["build", "--jobs", "1"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    let lent = model.lent();
    let [Some(first), Some(second), None, Some(verify)] = lent.as_slice() else {
        panic!("two build turns, the merge turn lending nothing, then the verify: {lent:?}");
    };
    assert!(first.ends_with(".emery/vcs/worktrees/SLICE-001"), "{lent:?}");
    assert!(second.ends_with(".emery/vcs/worktrees/SLICE-002"), "{lent:?}");
    assert!(verify.ends_with(".emery/vcs/integration"), "{lent:?}");
    let merge = &model.seen()[2];
    assert!(
        merge.messages[0].contains(
            "## `build/index.md`\n\n### Integrated tree\n\n```\n# Index\n\n- \
         greeting\n```\n\n### Slice\n\n```\n# Index\n\n- farewell\n```\n"
        ),
        "both sides are read from the working copies: {}",
        merge.messages[0]
    );
    assert!(
        merge.messages[0].contains("# Merged before it in the wave\n\n## Slice: greeting\n"),
        "{}",
        merge.messages[0]
    );
    let revision = revision_of(&run);
    let label = format!("emery/{revision}");
    let project = roots.project.path();
    let head = roots.git.run(project, &["rev-parse", &label]);
    assert!(
        run.stdout.contains(&format!(
            "    SLICE-002 farewell: covered 1/1, written 1 file, merged {head}, resolved \
             (build/index.md)\n"
        )),
        "{}",
        run.stdout
    );

    // one merge per slice along the first-parent chain over the initialised
    // tree, the resolved file at the label
    assert_eq!(
        roots
            .git
            .run(project, &["log", "--first-parent", "--format=%s", &format!("{base}..{label}")]),
        "SLICE-002 farewell\nSLICE-001 greeting\nInitialise"
    );
    let initialised = roots.git.run(project, &["rev-parse", &format!("{label}~2")]);
    assert_eq!(
        roots.git.run(project, &["show", &format!("{label}:build/index.md")]),
        RESOLVED_INDEX.trim_end()
    );
    assert_eq!(
        roots.git.run(project, &["show", &format!("{label}~1:build/index.md")]),
        "# Index\n\n- greeting",
        "the first slice's merge holds its own side"
    );

    // the second slice's merge brings in the resolution sealed over its commit
    let resolution = roots.git.run(project, &["rev-parse", &format!("{label}^2")]);
    assert_eq!(
        roots.git.run(project, &["log", "-1", "--format=%s", &resolution]),
        "SLICE-002 farewell resolved at build/index.md"
    );
    let commit = roots.git.run(project, &["rev-parse", &format!("{resolution}~1")]);
    assert_eq!(
        roots.git.run(project, &["log", "-1", "--format=%s", &commit]),
        "SLICE-002 farewell"
    );
    assert_eq!(roots.git.run(project, &["rev-parse", &format!("{commit}~1")]), initialised);
    assert_eq!(
        roots.git.run(project, &["show", &format!("{commit}:build/index.md")]),
        "# Index\n\n- farewell",
        "the slice's own commit holds what its build wrote"
    );
    assert_eq!(
        roots.git.run(project, &["diff", "--name-only", &commit, &resolution]),
        "build/index.md",
        "the resolution changes the resolved path alone"
    );
    assert!(!project.join(".emery/vcs/integration").exists(), "the working copy is removed");
    assert!(!project.join(".emery/vcs/worktrees/SLICE-002").exists(), "the slice's too");
    assert_eq!(roots.git.branches(project), [label, "main".to_owned()]);
}

// A credential in what the build's turn wrote is read from the slice's
// working copy through the engine's own file capability, and refuses the
// slice before anything is sealed: no label is set, no verify turn is put,
// and the copy stays under `worktrees/` with the file as it was written.
#[tokio::test]
async fn build_secret_refused() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await);
    model.assert_exhausted();
    let base = roots.sealed();

    let model = ScriptedModel::answering([REPORT_LEAK]).calling(0, [("write_files", WRITE_LEAK)]);
    let run = emery(roots.deployment(&["--format", "json", "build"]), &backends, &model).await;

    let envelope = refused(&run, 1, "secret-detected");
    model.assert_exhausted();
    assert_message(
        &envelope,
        "slice `SLICE-001` (greeting) failed in wave 1: what the build wrote carries a credential, \
         so none of it is sealed:\n- build/leak.pem: a PEM private key block",
    );
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains(".emery/vcs/worktrees/<slice>")),
        "{envelope}"
    );
    let project = roots.project.path();
    assert_eq!(roots.git.branches(project), ["main"], "no label is set");
    assert_eq!(roots.git.run(project, &["rev-parse", "HEAD"]), base);
    let copy = project.join(".emery/vcs/worktrees/SLICE-001");
    assert!(copy.join("build/leak.pem").exists(), "the copy stays as it was written");
    assert_eq!(
        roots.git.run(&copy, &["status", "--porcelain", "--untracked-files=all"]),
        "?? build/greeting.md\n?? build/leak.pem",
        "nothing of the change set is sealed"
    );
    assert!(project.join(".emery/vcs/integration").exists(), "left for inspection");
    assert!(roots.project.read("build/leak.pem").is_none(), "nothing lands in the checkout");
}

// A symbolic link the build's shell planted in the slice's working copy is read
// as the target it names, the text a commit would store of it, never as what
// it leads to: a token spelled there refuses the slice as a file holding it
// would, though the link leads nowhere, and the link stays in the copy.
#[tokio::test]
async fn build_secret_in_link() {
    let (roots, backends, base) = specified().await;
    let project = roots.project.path();
    let copy = project.join(".emery/vcs/worktrees/SLICE-001");

    let model = shelled(copy.clone(), |copy| {
        fs::create_dir_all(copy.join("src")).expect("creating the directory");
        symlink(TOKEN, copy.join("src/hook")).expect("planting the link");
    });
    let run = emery(roots.deployment(&["--format", "json", "build"]), &backends, &model).await;

    let envelope = refused(&run, 1, "secret-detected");
    assert_eq!(model.seen().len(), 1, "the build turn alone: no verify turn is put");
    assert_message(
        &envelope,
        "slice `SLICE-001` (greeting) failed in wave 1: what the build wrote carries a credential, \
         so none of it is sealed:\n- src/hook: a GitHub token",
    );
    assert_eq!(roots.git.branches(project), ["main"], "no label is set");
    assert_eq!(roots.git.run(project, &["rev-parse", "HEAD"]), base);
    assert_eq!(
        fs::read_link(copy.join("src/hook")).expect("the link stays").to_str(),
        Some(TOKEN),
        "the copy stays as it was left"
    );
    assert_eq!(
        roots.git.run(&copy, &["status", "--porcelain", "--untracked-files=all"]),
        "?? src/hook",
        "nothing of the change set is sealed"
    );
}

// A changed path that is neither a file nor a link — a repository the
// build's shell initialised inside the copy, which git lists by its
// directory — is nothing the gate can read and nothing it passes: the run
// ends as the read failed, nothing is sealed, and no label is set.
#[tokio::test]
async fn build_change_unreadable() {
    let (roots, backends, base) = specified().await;
    let project = roots.project.path();
    let copy = project.join(".emery/vcs/worktrees/SLICE-001");

    let git = roots.git.binary();
    let model = shelled(copy.clone(), move |copy| {
        let nested = copy.join("vendor");
        fs::create_dir_all(&nested).expect("creating the directory");
        let status = Command::new(&git)
            .arg("-C")
            .arg(&nested)
            .args(["init", "--quiet"])
            .status()
            .expect("running git");
        assert!(status.success(), "initialising the nested repository");
    });
    let run = emery(roots.deployment(&["--format", "json", "build"]), &backends, &model).await;

    let envelope = refused(&run, 3, "server_error");
    assert_eq!(model.seen().len(), 1, "the build turn alone: no verify turn is put");
    assert_message(
        &envelope,
        "slice `SLICE-001` (greeting) failed in wave 1: reading `vendor/` under \
         `./.emery/vcs/worktrees/SLICE-001`: ",
    );
    assert_eq!(roots.git.branches(project), ["main"], "no label is set");
    assert_eq!(roots.git.run(project, &["rev-parse", "HEAD"]), base);
    assert!(copy.join("vendor/.git").is_dir(), "the copy stays as it was left");
    assert_eq!(
        roots.git.run(&copy, &["status", "--porcelain", "--untracked-files=all"]),
        "?? vendor/",
        "nothing of the change set is sealed"
    );
}

// A checkout with a change the operator has not committed is no base: the
// path is named, the engine's own `.emery/` is not, and nothing is built.
#[tokio::test]
async fn build_dirty_refused() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await);
    model.assert_exhausted();
    roots.sealed();
    roots.project.write("notes.md", "later\n");
    roots.project.write(".emery/scratch.txt", "the engine's own\n");

    let model = ScriptedModel::default();
    let run = emery(roots.deployment(&["--format", "json", "build"]), &backends, &model).await;

    let envelope = refused(&run, 1, "base-not-sealed");
    assert_message(&envelope, "the project checkout holds pending changes: notes.md");
    assert!(!envelope["message"].as_str().unwrap_or("").contains(".emery"), "{envelope}");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("commit or stash")),
        "{envelope}"
    );
    assert_eq!(roots.git.branches(roots.project.path()), ["main"], "no label is set");
    model.assert_exhausted();

    // a project that is no repository at all is told what a build into it needs
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await);
    roots.targeted();
    let run = emery(
        roots.deployment(&["--format", "json", "build"]),
        &backends,
        &ScriptedModel::default(),
    )
    .await;
    let envelope = refused(&run, 1, "repository-required");
    assert_message(&envelope, "the project directory is not a repository");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("drop the key")),
        "{envelope}"
    );
}

// A `[target] repository` builds into a clone of it kept under the target's
// name, from the default branch as the origin holds it: the clone gains
// `emery/<revision>` over `main`, the origin is never written, and the
// project directory need not be a repository at all. The next run fetches,
// so a `main` the origin moved is the new base and the label moves over it.
#[tokio::test]
async fn build_brownfield() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let (origin, url) = roots.git.bare("shop.git", &[("README.md", "# Shop\n")]);
    let main = roots.git.run(&origin, &["rev-parse", "main"]);
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\n\n\
             [target]\nadapter = \"{TARGET}\"\nrepository = \"{url}\"\n"
        ),
    );
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", "--apply"]), &backends, &model).await);
    model.assert_exhausted();

    let model = building();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    let label = format!("emery/{}", revision_of(&run));
    assert!(
        run.stdout.contains(&format!("\n  base {main}\n  repository ./{TARGET_REPO}\n")),
        "{}",
        run.stdout
    );
    assert!(!run.stdout.contains("pushed"), "{}", run.stdout);

    // the clone holds the label over main: the initialisation and one merge
    // along the first-parent chain, the slice's commit beneath it; the
    // origin holds main alone
    let clone = roots.project.path().join(TARGET_REPO);
    assert_eq!(roots.git.branches(&clone), [label.clone(), "main".to_owned()]);
    assert_eq!(roots.git.beneath_initialise(&clone, &label), main);
    assert_eq!(
        roots
            .git
            .run(&clone, &["rev-list", "--count", "--first-parent", &format!("main..{label}")]),
        "2"
    );
    assert_eq!(roots.git.run(&clone, &["rev-list", "--count", &format!("main..{label}")]), "3");
    assert_eq!(
        roots.git.run(&clone, &["show", &format!("{label}:build/greeting.md")]),
        "# Greeting"
    );
    assert_eq!(roots.git.run(&clone, &["show", &format!("{label}:README.md")]), "# Shop");
    assert_eq!(roots.git.branches(&origin), ["main"], "nothing is pushed");
    assert_eq!(roots.git.run(&origin, &["rev-parse", "main"]), main);

    // the clone is kept for the next run, the working copy is not
    let repos = roots.project.path().join(".emery/vcs/repos");
    let clones: Vec<PathBuf> = fs::read_dir(&repos)
        .expect("the clones")
        .map(|entry| entry.expect("a clone").path())
        .collect();
    assert_eq!(clones, std::slice::from_ref(&clone));
    assert!(!roots.project.path().join(".emery/vcs/integration").exists());
    assert!(roots.project.read("build/greeting.md").is_none(), "nothing lands in the project");
    assert!(!roots.project.path().join(".git").exists(), "the project is no repository");

    // the origin's main moves on; the next run fetches and builds over it
    let moved = roots.git.advanced("shop-next", &url, &[("README.md", "# Shop, next\n")]);
    assert_eq!(roots.git.run(&origin, &["rev-parse", "main"]), moved);
    let model = building();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;
    assert_ok(&run);
    model.assert_exhausted();
    assert!(run.stdout.contains(&format!("\n  base {moved}\n")), "{}", run.stdout);
    assert!(!run.stdout.contains("resumed:"), "{}", run.stdout);
    assert_eq!(roots.git.beneath_initialise(&clone, &label), moved);
    assert_eq!(roots.git.run(&clone, &["show", &format!("{label}:README.md")]), "# Shop, next");
    assert_eq!(roots.git.branches(&origin), ["main"], "still nothing is pushed");
}

// A build naming no repository, as one naming its adapter on the command
// line does, builds into one the engine creates under the target's name: a
// root commit `Initialise` holding the adapter's layout and a `.gitignore`
// of the engine's own directory, the slice merged over it and labelled, the
// project directory never a repository. The next run finds the repository
// and resumes from its label.
#[tokio::test]
async fn build_laid() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await);
    model.assert_exhausted();

    let model = building();
    let run = emery(roots.deployment(&["build", TARGET]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    let revision = revision_of(&run);
    let label = format!("emery/{revision}");
    let repo = roots.project.path().join(TARGET_REPO);
    let root = roots.git.run(&repo, &["rev-list", "--max-parents=0", &label]);
    assert!(
        run.stdout.contains(&format!("\n  base {root}\n  repository ./{TARGET_REPO}\n")),
        "{}",
        run.stdout
    );
    let head = roots.git.run(&repo, &["rev-parse", &label]);
    assert!(run.stdout.ends_with(&format!("  labelled {label} at {head}\n")), "{}", run.stdout);
    assert!(!run.stdout.contains("initialised "), "the root commit is the layout: {}", run.stdout);

    // the root commit is the adapter's layout beside the engine's ignore
    // file, and the one the slice was built over, initialised again nowhere
    assert_eq!(roots.git.run(&repo, &["log", "-1", "--format=%s", &root]), "Initialise");
    assert_eq!(roots.git.run(&repo, &["ls-tree", "--name-only", &root]), ".gitignore\nbuild");
    assert_eq!(roots.git.run(&repo, &["show", &format!("{root}:.gitignore")]), ".emery/");
    assert!(
        roots
            .git
            .run(&repo, &["show", &format!("{root}:build/README.md")])
            .starts_with("# Build\n"),
        "the adapter scaffolded its layout into the root commit"
    );
    assert_eq!(roots.git.run(&repo, &["rev-parse", &format!("{label}~1")]), root);
    assert_eq!(
        roots.git.run(&repo, &["show", &format!("{label}:build/greeting.md")]),
        "# Greeting"
    );
    assert_eq!(roots.git.branches(&repo), [label.clone(), "main".to_owned()]);
    assert_eq!(
        roots.git.run(&repo, &["rev-parse", "HEAD"]),
        root,
        "the checkout stands at the root"
    );
    assert!(!roots.project.path().join(".git").exists(), "the project is no repository");
    assert!(roots.project.read("build/greeting.md").is_none(), "nothing lands in the project");
    assert!(!roots.project.path().join(".emery/vcs/integration").exists());

    // the next run reuses the repository and resumes from the label
    let model = ScriptedModel::default();
    let run = emery(roots.deployment(&["build", TARGET]), &backends, &model).await;
    assert_ok(&run);
    model.assert_exhausted();
    assert!(model.lent().is_empty(), "no turn is put: {:?}", model.lent());
    assert_eq!(
        run.stdout,
        format!(
            "built revision {revision}\n  plan: 1 slice in 1 wave, widest 1\n  base {root}\n  \
             repository ./{TARGET_REPO}\n  resumed: SLICE-001\n  labelled {label} at {head}\n"
        )
    );
    assert_eq!(roots.git.run(&repo, &["rev-list", "--max-parents=0", &label]), root);
    assert_eq!(roots.git.run(&repo, &["rev-parse", &label]), head, "the label stands");
}

// The message a build seals `SLICE-001` under for `revision` over `base`,
// as a forger would copy it.
fn forged_message(revision: &str, base: &str) -> String {
    Message {
        slice: "SLICE-001".parse().expect("a slice id"),
        name: "greeting".to_owned(),
        revision: revision.to_owned(),
        requirements: vec!["REQ-001".parse().expect("a requirement id")],
        covered: vec!["REQ-001".parse().expect("a requirement id")],
        adapter: TARGET.to_owned(),
        base: base.to_owned(),
        wave: 1,
    }
    .to_string()
}

// A tag spelled `emery/<revision>`, which git resolves the bare name to
// before the branch, is no label: the run reads none, builds every slice,
// and sets the branch, while the tag stands where it was.
#[tokio::test]
async fn build_resumed_shadowed() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    let run = emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await;
    assert_ok(&run);
    model.assert_exhausted();
    let revision = revision_of(&run);
    let label = format!("emery/{revision}");
    let project = roots.project.path();
    let base = roots.sealed();
    let forged = roots.git.forged(project, Some(&base), &forged_message(&revision, &base));
    roots.git.run(project, &["tag", &label, &forged]);

    let model = building();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    assert_eq!(model.lent().len(), 2, "the slice is built and verified: {:?}", model.lent());
    assert!(!run.stdout.contains("resumed:"), "{}", run.stdout);
    let head = roots.git.run(project, &["rev-parse", &format!("refs/heads/{label}")]);
    assert!(run.stdout.ends_with(&format!("  labelled {label} at {head}\n")), "{}", run.stdout);
    assert_ne!(head, forged, "the branch is the build's");
    assert_eq!(roots.git.beneath_initialise(project, &head), base);
    assert_eq!(roots.git.run(project, &["rev-parse", &format!("refs/tags/{label}")]), forged);
    assert_eq!(
        roots.git.run(project, &["rev-parse", &label]),
        forged,
        "the bare name is the tag's, which the run never read"
    );
    assert_eq!(roots.git.branches(project), [label, "main".to_owned()]);
}

// A label moved onto a history that does not descend from the base records
// nothing for this base: the run builds every slice from the base and the
// label moves back over it.
#[tokio::test]
async fn build_resumed_off_base() {
    let (roots, backends, revision, base) = built().await;
    let label = format!("emery/{revision}");
    let project = roots.project.path();
    let forged = roots.git.forged(project, None, &forged_message(&revision, &base));
    roots.git.run(project, &["branch", "--force", &label, &forged]);

    let model = building();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    assert_eq!(model.lent().len(), 2, "the slice is built again: {:?}", model.lent());
    assert!(run.stdout.contains(&format!("\n  base {base}\n")), "{}", run.stdout);
    assert!(!run.stdout.contains("resumed:"), "{}", run.stdout);
    let head = roots.git.run(project, &["rev-parse", &format!("refs/heads/{label}")]);
    assert!(run.stdout.ends_with(&format!("  labelled {label} at {head}\n")), "{}", run.stdout);
    assert_ne!(head, forged);
    assert_eq!(roots.git.beneath_initialise(project, &head), base);
    assert_eq!(
        roots.git.run(project, &["branch", "--contains", &forged, "--format=%(refname:short)"]),
        "",
        "the forged history is reached from no branch"
    );
    assert_eq!(roots.git.run(project, &["rev-parse", "HEAD"]), base, "the checkout is untouched");
}

// A tag a remote holds under the label's spelling arrives with the clone
// and is no label there either: the build is fresh, the branch is set
// beside the tag in the clone, and the tag stands.
#[tokio::test]
async fn build_brownfield_shadowed() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let (origin, url) = roots.git.bare("shop.git", &[("README.md", "# Shop\n")]);
    let main = roots.git.run(&origin, &["rev-parse", "main"]);
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\n\n\
             [target]\nadapter = \"{TARGET}\"\nrepository = \"{url}\"\n"
        ),
    );
    let backends = roots.backends().await;
    let model = specifying();
    let run = emery(roots.deployment(&["specify", "--apply"]), &backends, &model).await;
    assert_ok(&run);
    model.assert_exhausted();
    let revision = revision_of(&run);
    let label = format!("emery/{revision}");
    let forged = roots.git.forged(&origin, Some(&main), &forged_message(&revision, &main));
    roots.git.run(&origin, &["tag", &label, &forged]);

    let model = building();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    assert_eq!(model.lent().len(), 2, "the slice is built and verified: {:?}", model.lent());
    assert!(!run.stdout.contains("resumed:"), "{}", run.stdout);
    let clone = roots.project.path().join(TARGET_REPO);
    let head = roots.git.run(&clone, &["rev-parse", &format!("refs/heads/{label}")]);
    assert!(run.stdout.ends_with(&format!("  labelled {label} at {head}\n")), "{}", run.stdout);
    assert_ne!(head, forged, "the branch is the build's");
    assert_eq!(roots.git.beneath_initialise(&clone, &head), main);
    assert_eq!(roots.git.run(&clone, &["rev-parse", &format!("refs/tags/{label}")]), forged);
    assert_eq!(roots.git.branches(&clone), [label, "main".to_owned()]);
    assert_eq!(roots.git.branches(&origin), ["main"], "nothing is pushed");
}

// A branch a remote holds under the label, built elsewhere, is no label in
// the clone, which holds it as the remote's: the build is fresh, the local
// label the build's, the remote's standing untouched; the next run resumes
// from the local label.
#[tokio::test]
async fn build_brownfield_remote_label() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let (origin, url) = roots.git.bare("shop.git", &[("README.md", "# Shop\n")]);
    let main = roots.git.run(&origin, &["rev-parse", "main"]);
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\n\n\
             [target]\nadapter = \"{TARGET}\"\nrepository = \"{url}\"\n"
        ),
    );
    let backends = roots.backends().await;
    let model = specifying();
    let run = emery(roots.deployment(&["specify", "--apply"]), &backends, &model).await;
    assert_ok(&run);
    model.assert_exhausted();
    let revision = revision_of(&run);
    let label = format!("emery/{revision}");
    let forged = roots.git.forged(&origin, Some(&main), &forged_message(&revision, &main));
    roots.git.run(&origin, &["branch", &label, &forged]);

    let model = building();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    assert_eq!(model.lent().len(), 2, "the slice is built and verified: {:?}", model.lent());
    assert!(!run.stdout.contains("resumed:"), "{}", run.stdout);
    let clone = roots.project.path().join(TARGET_REPO);
    let head = roots.git.run(&clone, &["rev-parse", &format!("refs/heads/{label}")]);
    assert_ne!(head, forged, "the local label is the build's");
    assert_eq!(roots.git.beneath_initialise(&clone, &head), main);
    assert_eq!(
        roots.git.run(&clone, &["rev-parse", &format!("refs/remotes/origin/{label}")]),
        forged,
        "the remote's arrived with the clone and was never read as the label"
    );
    assert_eq!(
        roots.git.run(&origin, &["rev-parse", &format!("refs/heads/{label}")]),
        forged,
        "the remote's label stands"
    );

    // the next run resumes from the local label and builds nothing
    let model = ScriptedModel::default();
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;
    assert_ok(&run);
    model.assert_exhausted();
    assert!(model.lent().is_empty(), "no turn is put: {:?}", model.lent());
    assert!(run.stdout.contains("\n  resumed: SLICE-001\n"), "{}", run.stdout);
    assert_eq!(roots.git.run(&clone, &["rev-parse", &format!("refs/heads/{label}")]), head);
    assert_eq!(roots.git.run(&origin, &["rev-parse", &format!("refs/heads/{label}")]), forged);
}

// A `[[source]]` naming a repository is read in a working copy of it at the
// revision, which the mock source's turn is lent; the clone is kept under
// the project and the working copy removed once the source answered.
#[tokio::test]
async fn specify_repository() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    let (upstream, url) = roots.git.published("upstream", &[("docs/greeting.md", GREETING)], "v1");
    let tagged = roots.git.run(&upstream, &["rev-parse", "v1^{commit}"]);
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\nrepository = \"{url}\"\n\
             revision = \"v1\"\n"
        ),
    );
    let backends = roots.backends().await;
    let model = specifying();

    let run = emery(roots.deployment(&["specify"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    assert!(
        run.stdout.contains(&format!("\n  docs read from {url} at v1: {tagged}\n")),
        "{}",
        run.stdout
    );
    let repos = roots.project.path().join(".emery/vcs/repos");
    let clones: Vec<PathBuf> = fs::read_dir(&repos)
        .expect("the clones")
        .map(|entry| entry.expect("a clone").path())
        .collect();
    let [clone] = clones.as_slice() else { panic!("one clone: {clones:?}") };
    assert_eq!(roots.git.run(clone, &["rev-parse", "v1^{commit}"]), tagged, "the clone is kept");
    assert!(!roots.project.path().join(".emery/vcs/sources").join("docs").exists());
    let listed = roots.git.run(clone, &["worktree", "list", "--porcelain"]);
    assert_eq!(listed.matches("worktree ").count(), 1, "the working copy is gone: {listed}");

    // the extract turn was lent the working copy, not the project
    let lent = model.lent();
    let extract = lent.first().cloned().flatten().expect("the extract turn lends a workspace");
    assert!(extract.ends_with(".emery/vcs/sources/docs"), "{lent:?}");
}

// A release the store lacks is fetched through the registry its namespace
// is routed to and kept under its reference's name; the next run reads the
// file with the registry gone.
#[tokio::test]
async fn specify_fetched() {
    let roots = Roots::new();
    roots.published(SOURCE, MOCK_SOURCE);
    let backends = roots.backends().await;
    let model = specifying();

    let run = emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await;

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
    let backends = roots.backends().await;
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
    let backends = roots.backends().await;
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
    let backends = roots.backends().await;
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
    let backends = roots.backends().await;
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
    let backends = roots.backends().await;
    let model = ScriptedModel::default();

    let run = emery(roots.deployment(&["--format", "json", "specify"]), &backends, &model).await;

    let envelope = refused(&run, 1, "refused");
    assert_message(&envelope, &format!("adapter `{SOURCE}`: "));
    assert_message(&envelope, &format!("not its declared digest {pin}"));
    model.assert_exhausted();
}

// A command-line adapter takes the place of the file's entry of its name,
// and the `digest` that pinned the file's release goes with it: a pin the
// stored bytes miss refuses nothing once the reference it pinned is replaced.
#[tokio::test]
async fn pin_layered() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let pin = Digest::of(b"other bytes");
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nadapter = \"{SOURCE}\"\ndigest = \"{pin}\"\npath = \"docs\"\n\n\
             [target]\nadapter = \"{TARGET}\"\ndigest = \"{pin}\"\nrepository = \".\"\n"
        ),
    );
    let backends = roots.backends().await;

    let model = specifying();
    let run =
        emery(roots.deployment(&["specify", SOURCE, "--config", "--apply"]), &backends, &model)
            .await;
    assert_ok(&run);
    model.assert_exhausted();

    roots.git.seal(roots.project.path(), "project");
    let model = building();
    let run = emery(roots.deployment(&["build", TARGET, "--config"]), &backends, &model).await;
    assert_ok(&run);
    model.assert_exhausted();
}

// A pre-compiled artifact copied into the store is refused however it hashes:
// the store is raw wasm alone.
#[tokio::test]
async fn precompiled_in_store() {
    let roots = Roots::new();
    let cwasm = precompiled(&roots, MOCK_SOURCE);
    roots.stored(SOURCE, &cwasm);
    let backends = roots.backends().await;
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
    roots.stored("acme:source@0.1.0-dev", MOCK_SOURCE);
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"acme:source@0.1.0-dev\"\n"
        ),
    );
    let backends = roots.backends().await;
    let model = ScriptedModel::default();

    let run = emery(roots.deployment(&["--format", "json", "specify"]), &backends, &model).await;

    let envelope = refused(&run, 1, "bad_request");
    assert_message(&envelope, "adapters `acme:source@0.1.0-dev` and `acme:source@0.1.0`");
    assert_message(&envelope, "would both register as `acme:source`");
    model.assert_exhausted();
}

// A source adapter named as the target is refused on its exports before any
// slice is dispatched or a turn spent.
#[tokio::test]
async fn build_source_only() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE, "--apply"]), &backends, &model).await);
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
    let backends = roots.backends().await;

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
