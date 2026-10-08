//! Verifies the adapter boundary under the omnia runtime.
//!
//! Each scenario deploys the engine guest the binary embeds over scratch
//! roots — the project mounted writable as `.`, the package store, and a
//! wasm-pkg `local` registry — under the hosts the engine imports, and
//! drives it over in-memory backends, a scripted model, and git behind
//! `omnia:vcs`, hermetic under a scratch configuration. omnia's real store
//! and loader answer every load, so what a reference resolves to is asserted
//! here and nowhere natively; what a build commits, labels, and pushes is
//! read back with git as the oracle; and the operator's `$HOME` and the
//! network are never touched.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use omnia::{CompileOptions, Digest, ExitStatus, StoreCtx};
use omnia_git::Client;
use omnia_test::host::{Backends, Deployment, Run, Scratch, ScriptedModel, scratch};
use omnia_wasi_blobstore::WasiBlobstore;
use omnia_wasi_keyvalue::WasiKeyValue;
use omnia_wasi_model::WasiModel;
use omnia_wasi_otel::WasiOtel;
use omnia_wasi_vcs::WasiVcs;
use serde_json::Value;

// The engine `build.rs` emits, as the binary embeds it.
const GUEST: &str = env!("EMERY_GUEST");

// The two mocks `build.rs` emits, raw wasm in every profile.
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

const GREETING: &str = include_str!("../examples/docs/greeting.md");
const EXTRACT_ANSWER: &str = r#"{"claims": [{"kind": "requirement", "id": "greeting.behaviour",
    "path": "docs/greeting.md#L3", "statement": "GET /greeting returns the static string 'hello'."}]}"#;
const SPEC_ANSWER: &str = include_str!("specify/spec-draft.json");
const DESIGN_ANSWER: &str = include_str!("specify/design-draft.json");
const WRITE_GREETING: &str = r##"{"path": "build/greeting.md", "content": "# Greeting\n"}"##;
const REPORT_ANSWER: &str = r#"{"covered": ["REQ-001"], "written": ["build/greeting.md"]}"#;

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
        let client = Client::connect(self.root.path().join("git"))
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

    // The branches a repository holds, short names in order.
    fn branches(&self, at: &Path) -> Vec<String> {
        self.run(at, &["for-each-ref", "--format=%(refname:short)", "refs/heads/"])
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

// The three roots a scenario lays — the project the run reads and builds
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

    // The project as a repository sealed at one commit, which a build starts from.
    fn sealed(&self) -> String {
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

// The revision id a `specify` or `build` run reports on its first line.
fn revision_of(run: &Run) -> String {
    let first = run.stdout.lines().next().unwrap_or_default();
    first
        .strip_prefix("committed revision ")
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

// --- the store ---

// A release the store holds is the adapter: nothing is fetched, the file is
// read, and the run proceeds through every turn to a revision.
#[tokio::test]
async fn specify_stored() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    let backends = roots.backends().await;
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

// A stored target builds every slice of the committed plan through its
// turn's `write_file` into a working copy cut from the project's sealed
// head; each slice is one commit under the label `emery/<revision>`, read
// back through git, and the checkout itself is untouched.
#[tokio::test]
async fn build_labelled() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await);
    model.assert_exhausted();
    let base = roots.sealed();

    let model =
        ScriptedModel::answering([REPORT_ANSWER]).calling(0, [("write_file", WRITE_GREETING)]);
    let run = emery(roots.deployment(&["build", TARGET]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    let lent = model.lent();
    let build = lent.first().cloned().flatten().expect("the build turn lends a workspace");
    assert!(build.ends_with(".emery/vcs/integration"), "{lent:?}");
    let revision = revision_of(&run);
    let label = format!("emery/{revision}");
    let project = roots.project.path();
    let head = roots.git.run(project, &["rev-parse", &label]);
    assert!(run.stdout.contains(&format!("\n  base {base}\n")), "{}", run.stdout);
    assert!(
        run.stdout.contains(&format!(
            "SLICE-001 greeting: covered 1/1, written 1 file, committed {head}\n"
        )),
        "{}",
        run.stdout
    );
    assert!(run.stdout.ends_with(&format!("  labelled {label} at {head}\n")), "{}", run.stdout);

    // the label holds one commit over the base, carrying what the slice wrote
    assert_eq!(roots.git.run(project, &["rev-parse", &format!("{label}~1")]), base);
    assert_eq!(
        roots.git.run(project, &["show", &format!("{label}:build/greeting.md")]),
        "# Greeting",
        "the target wrote the working copy through the turn"
    );
    let message = roots.git.run(project, &["log", "-1", "--format=%B", &label]);
    assert!(message.starts_with("SLICE-001 greeting\n\n"), "{message}");
    for trailer in [
        format!("Revision: {revision}"),
        "Requirements: REQ-001".to_owned(),
        "Covered: REQ-001".to_owned(),
        format!("Adapter: {TARGET}"),
        format!("Base: {base}"),
    ] {
        assert!(message.contains(&trailer), "`{trailer}` in: {message}");
    }

    // the checkout is as the operator left it, and the working copy is gone
    assert_eq!(roots.git.run(project, &["rev-parse", "HEAD"]), base);
    assert!(roots.project.read("build/greeting.md").is_none(), "nothing lands in the checkout");
    assert!(!project.join(".emery/vcs/integration").exists(), "the working copy is removed");
    let listed = roots.git.run(project, &["worktree", "list", "--porcelain"]);
    assert_eq!(listed.matches("worktree ").count(), 1, "{listed}");
    assert_eq!(roots.git.branches(project), [label, "main".to_owned()]);
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
    assert_ok(&emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await);
    model.assert_exhausted();
    roots.sealed();
    roots.project.write("notes.md", "later\n");
    roots.project.write(".emery/scratch.txt", "the engine's own\n");

    let model = ScriptedModel::default();
    let run =
        emery(roots.deployment(&["--format", "json", "build", TARGET]), &backends, &model).await;

    let envelope = refused(&run, 1, "base-not-sealed");
    assert_message(&envelope, "the project checkout holds pending changes: notes.md");
    assert!(!envelope["message"].as_str().unwrap_or("").contains(".emery"), "{envelope}");
    assert!(
        envelope["hint"].as_str().is_some_and(|hint| hint.contains("commit or stash")),
        "{envelope}"
    );
    assert_eq!(roots.git.branches(roots.project.path()), ["main"], "no label is set");
    model.assert_exhausted();

    // a project that is no repository at all is told what a build needs
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify", SOURCE]), &backends, &model).await);
    let run = emery(
        roots.deployment(&["--format", "json", "build", TARGET]),
        &backends,
        &ScriptedModel::default(),
    )
    .await;
    let envelope = refused(&run, 1, "repository-required");
    assert_message(&envelope, "the project directory is not a repository");
}

// A `[target] repository` builds into a clone of it from its branch, and the
// `remote` takes the label: the origin gains `emery/<revision>` over its
// `main`, which it keeps, and the project directory need not be a
// repository at all.
#[tokio::test]
async fn build_brownfield_pushed() {
    let roots = Roots::new();
    roots.stored(SOURCE, MOCK_SOURCE);
    roots.stored(TARGET, MOCK_TARGET);
    let (origin, url) = roots.git.bare("shop.git", &[("README.md", "# Shop\n")]);
    let main = roots.git.run(&origin, &["rev-parse", "main"]);
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\n\n\
             [target]\nadapter = \"{TARGET}\"\nrepository = \"{url}\"\nbranch = \"main\"\n\
             remote = \"origin\"\n"
        ),
    );
    let backends = roots.backends().await;
    let model = specifying();
    assert_ok(&emery(roots.deployment(&["specify"]), &backends, &model).await);
    model.assert_exhausted();

    let model =
        ScriptedModel::answering([REPORT_ANSWER]).calling(0, [("write_file", WRITE_GREETING)]);
    let run = emery(roots.deployment(&["build"]), &backends, &model).await;

    assert_ok(&run);
    model.assert_exhausted();
    let label = format!("emery/{}", revision_of(&run));
    assert!(run.stdout.contains(&format!("\n  base {main}\n")), "{}", run.stdout);
    assert!(run.stdout.ends_with("  pushed to origin\n"), "{}", run.stdout);

    // the origin holds the label over its main, which stands where it was
    assert_eq!(roots.git.branches(&origin), [label.clone(), "main".to_owned()]);
    assert_eq!(roots.git.run(&origin, &["rev-parse", "main"]), main);
    assert_eq!(roots.git.run(&origin, &["rev-parse", &format!("{label}~1")]), main);
    assert_eq!(
        roots.git.run(&origin, &["show", &format!("{label}:build/greeting.md")]),
        "# Greeting"
    );
    assert_eq!(roots.git.run(&origin, &["show", &format!("{label}:README.md")]), "# Shop");

    // the clone is kept for the next run, the working copy is not
    let repos = roots.project.path().join(".emery/vcs/repos");
    let clones: Vec<PathBuf> = fs::read_dir(&repos)
        .expect("the clones")
        .map(|entry| entry.expect("a clone").path())
        .collect();
    assert_eq!(clones.len(), 1, "{clones:?}");
    assert!(!roots.project.path().join(".emery/vcs/integration").exists());
    assert!(roots.project.read("build/greeting.md").is_none(), "nothing lands in the project");
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
    roots.stored("acme:source@0.2.0-dev", MOCK_SOURCE);
    roots.project.write(
        "emery.toml",
        format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"acme:source@0.2.0-dev\"\n"
        ),
    );
    let backends = roots.backends().await;
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
    let backends = roots.backends().await;
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
