//! Revision trees and working-copy fixtures on the project scratch mount.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use emery_engine::EntryKind;
use emery_engine::vcs::SPECIFICATION;
use omnia_test::host::Scratch;

type BytesSnapshot = BTreeMap<String, Vec<u8>>;

const REVISION_DIR: &str = ".emery/vcs/revision";
const INTEGRATION: &str = "./.emery/vcs/integration";

type Preserved = BTreeMap<String, BTreeMap<String, Vec<u8>>>;

/// Tracks revision-tree and working-copy changes across a run.
#[derive(Clone, Debug, Default)]
pub struct Files {
    project: Option<PathBuf>,
    before_revision: Arc<Mutex<BytesSnapshot>>,
    before_project: Arc<Mutex<BytesSnapshot>>,
    preserved: Arc<Mutex<Preserved>>,
}

impl Files {
    /// Binds this helper to `project` for subsequent staging.
    pub fn bind(&mut self, project: &Scratch) {
        self.project = Some(project.path().to_path_buf());
    }

    /// Scripts `spec/` at the revision working copy with the three documents.
    pub fn seed(&self, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
        for (name, body) in [("spec", spec), ("design", design), ("plan", plan)] {
            self.write_revision(&document(name), body);
        }
        super::store::revision(spec, design, plan)
    }

    /// Records project bytes before the next CLI run.
    pub fn mark_run_start(&self) {
        self.before_revision.lock().expect("before revision").clone_from(&self.snapshot_revision());
        self.before_project.lock().expect("before project").clone_from(&self.snapshot_project());
    }

    /// Returns paths under the revision tree that changed since [`mark_run_start`].
    pub fn writes(&self) -> Vec<(String, String)> {
        let before = self.before_revision.lock().expect("before revision");
        let after = self.snapshot_revision();
        after
            .into_iter()
            .filter(|(path, bytes)| before.get(path) != Some(bytes))
            .map(|(path, _)| (format!("./{REVISION_DIR}"), path))
            .collect()
    }

    /// Returns working-copy writes since [`mark_run_start`]: the copy, the path, and the bytes.
    pub fn worktree_writes(&self) -> Vec<(String, String, Vec<u8>)> {
        let before = self.before_project.lock().expect("before project");
        let after = self.snapshot_project();
        after
            .into_iter()
            .filter(|(path, bytes)| before.get(path) != Some(bytes))
            .filter(|(path, _)| !path.starts_with(REVISION_DIR))
            .filter_map(|(path, bytes)| {
                working_copy_at(&path).map(|at| {
                    let path = strip_working_path(&path, &at);
                    (at, path, bytes)
                })
            })
            .collect()
    }

    /// Removes `path` beneath working copy `at` when it exists.
    pub fn remove(&self, at: &str, path: &str) {
        if let Some(paths) = self.preserved.lock().expect("preserved").get_mut(at) {
            paths.remove(path);
        }
        let file = self.resolve(at, path);
        let _ = std::fs::remove_file(&file);
    }

    /// Drops a working copy tree beneath the project mount.
    pub fn purge(&self, at: &str) {
        let root = self.project.as_ref().expect("project bound");
        let at = at.strip_prefix("./").unwrap_or(at);
        let dir = root.join(at);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Forgets held files for a working copy so the next cut does not replant them.
    pub fn clear_preserved(&self, at: &str) {
        let stripped = at.strip_prefix("./").unwrap_or(at);
        let mut keys = vec![at.to_owned(), stripped.to_owned()];
        if !at.starts_with("./") {
            keys.push(format!("./{stripped}"));
        }
        let mut preserved = self.preserved.lock().expect("preserved");
        for key in keys {
            preserved.remove(&key);
        }
        drop(preserved);
    }

    /// Lays text at `path` beneath working copy `at` on the project mount.
    pub fn holds(&self, at: &str, path: &str, content: &str) {
        self.holds_bytes(at, path, content.as_bytes());
    }

    /// Lays bytes at `path` beneath working copy `at` on the project mount.
    pub fn holds_bytes(&self, at: &str, path: &str, content: &[u8]) {
        self.preserved
            .lock()
            .expect("preserved")
            .entry(at.to_owned())
            .or_default()
            .insert(path.to_owned(), content.to_vec());
        self.ensure_working_copy(at);
        let file = self.resolve(at, path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).expect("mkdir working copy path");
        }
        std::fs::write(file, content).expect("write working copy file");
    }

    /// Rewrites every path recorded for `at` after a working copy was cut again.
    pub fn replant(&self, at: &str) {
        let Some(paths) = self.preserved.lock().expect("preserved").get(at).cloned() else {
            return;
        };
        self.ensure_working_copy(at);
        for (path, content) in paths {
            let file = self.resolve(at, &path);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent).expect("mkdir working copy path");
            }
            std::fs::write(file, content).expect("replant working copy file");
        }
    }

    /// Lays directory entries beneath `at` so [`list`](emery_engine::files::list) sees them.
    pub fn lists(&self, at: &str, path: &str, entries: &[(&str, EntryKind)]) {
        self.ensure_working_copy(at);
        let base = self.resolve(at, path);
        std::fs::create_dir_all(&base).expect("mkdir for listing");
        for (name, kind) in entries {
            let entry = base.join(name);
            match kind {
                EntryKind::Directory => {
                    std::fs::create_dir_all(&entry).expect("mkdir entry");
                }
                EntryKind::File => {
                    if let Some(parent) = entry.parent() {
                        std::fs::create_dir_all(parent).expect("mkdir file parent");
                    }
                    if !entry.exists() {
                        std::fs::write(&entry, b"").expect("touch file");
                    }
                }
                EntryKind::Link => {
                    if let Some(parent) = entry.parent() {
                        std::fs::create_dir_all(parent).expect("mkdir link parent");
                    }
                    #[cfg(unix)]
                    {
                        let _ = std::fs::remove_file(&entry);
                        std::os::unix::fs::symlink("orders.rs", &entry).expect("symlink");
                    }
                    #[cfg(not(unix))]
                    {
                        std::fs::write(&entry, b"orders.rs").expect("link stand-in");
                    }
                }
            }
        }
    }

    /// Reads `path` relative to a working-copy directory beneath the project mount.
    pub fn read_at(&self, working_copy: &str, path: &str) -> Vec<u8> {
        let file = self.resolve(working_copy, path);
        std::fs::read(&file)
            .unwrap_or_else(|err| panic!("read `{path}` under `{working_copy}`: {err}"))
    }

    fn resolve(&self, at: &str, path: &str) -> PathBuf {
        let root = self.project.as_ref().expect("project bound");
        let at = at.strip_prefix("./").unwrap_or(at);
        root.join(at).join(path)
    }

    fn ensure_working_copy(&self, at: &str) {
        let root = self.project.as_ref().expect("project bound");
        let at = at.strip_prefix("./").unwrap_or(at);
        std::fs::create_dir_all(root.join(at)).expect("mkdir working copy");
    }

    fn write_revision(&self, path: &str, body: &[u8]) {
        let root = self.project.as_ref().expect("project bound");
        let file = root.join(REVISION_DIR).join(path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).expect("mkdir revision tree");
        }
        std::fs::write(file, body).expect("write revision file");
    }

    fn snapshot_revision(&self) -> BytesSnapshot {
        let root = self.project.as_ref().expect("project bound").join(REVISION_DIR);
        let mut snap = BytesSnapshot::new();
        walk_snapshot(&root, &root, &mut snap);
        snap
    }

    fn snapshot_project(&self) -> BytesSnapshot {
        let root = self.project.as_ref().expect("project bound");
        let mut snap = BytesSnapshot::new();
        walk_snapshot(root, root, &mut snap);
        snap
    }
}

fn walk_snapshot(base: &Path, at: &Path, snap: &mut BytesSnapshot) {
    if !at.exists() {
        return;
    }
    let meta = std::fs::symlink_metadata(at).expect("metadata");
    if meta.file_type().is_file() {
        let rel = at.strip_prefix(base).expect("under project").display().to_string();
        let bytes = std::fs::read(at).expect("read file");
        snap.insert(rel, bytes);
        return;
    }
    if meta.file_type().is_dir() {
        for entry in std::fs::read_dir(at).expect("read dir") {
            let entry = entry.expect("dir entry");
            walk_snapshot(base, &entry.path(), snap);
        }
    }
}

fn working_copy_at(rel: &str) -> Option<String> {
    if rel == ".emery/vcs/integration" || rel.starts_with(".emery/vcs/integration/") {
        return Some(INTEGRATION.to_owned());
    }
    if let Some(suffix) = rel.strip_prefix(".emery/vcs/worktrees/") {
        let slice = suffix.split('/').next()?;
        return Some(format!("./.emery/vcs/worktrees/{slice}"));
    }
    None
}

fn strip_working_path(rel: &str, at: &str) -> String {
    let prefix = at.strip_prefix("./").unwrap_or(at);
    rel.strip_prefix(prefix).and_then(|rest| rest.strip_prefix('/')).unwrap_or(rel).to_owned()
}

/// Path of a revision document relative to the revision working copy root.
pub fn document(name: &str) -> String {
    format!("{SPECIFICATION}/{name}.json")
}

/// Bytes last written to `spec/<name>.json` in the revision working copy.
pub fn written(files: &Files, name: &str) -> Vec<u8> {
    let root = files.project.as_ref().expect("project bound");
    let path = root.join(REVISION_DIR).join(document(name));
    std::fs::read(path).unwrap_or_else(|err| panic!("{name} sealed: {err}"))
}

/// Scripts the revision `from` sealed into the tree `to` reads.
pub fn carry(from: &Files, to: &Files) -> String {
    let spec = written(from, "spec");
    let design = written(from, "design");
    let plan = written(from, "plan");
    to.seed(&spec, &design, &plan)
}
