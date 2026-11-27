//! Revision trees and working-copy fixtures on the project scratch mount.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use emery_engine::EntryKind;
use emery_engine::vcs::SPECIFICATION;
use omnia_test::host::Scratch;

type BytesSnapshot = BTreeMap<String, Vec<u8>>;

const REVISION_DIR: &str = ".emery/vcs/revision";
const INTEGRATION: &str = "./.emery/vcs/integration";

type Preserved = BTreeMap<String, BTreeMap<String, Vec<u8>>>;

#[derive(Clone, Debug, Default)]
pub struct Files {
    project: Option<PathBuf>,
    before_revision: Arc<Mutex<BytesSnapshot>>,
    before_project: Arc<Mutex<BytesSnapshot>>,
    preserved: Arc<Mutex<Preserved>>,
    access_before: Arc<Mutex<HashMap<String, SystemTime>>>,
    read_order: Arc<Mutex<Vec<(String, String)>>>,
}

impl Files {
    pub fn bind(&mut self, project: &Scratch) {
        self.project = Some(project.path().to_path_buf());
    }

    pub fn seed(&self, spec: &[u8], design: &[u8], plan: &[u8]) -> String {
        for (name, body) in [("spec", spec), ("design", design), ("plan", plan)] {
            self.write_revision(&document(name), body);
        }
        super::store::revision(spec, design, plan)
    }

    pub fn mark_run_start(&self) {
        self.before_revision.lock().expect("before revision").clone_from(&self.snapshot_revision());
        self.before_project.lock().expect("before project").clone_from(&self.snapshot_project());
        self.read_order.lock().expect("reads").clear();
        self.access_before.lock().expect("access before").clone_from(&self.access_times());
    }

    pub fn record_accesses(&self) {
        let before = self.access_before.lock().expect("access before").clone();
        let after = self.access_times();
        let paths: BTreeSet<String> = after.keys().chain(before.keys()).cloned().collect();
        let root = self.project.as_ref().expect("project bound");
        let mut read_batch = Vec::new();
        for rel in paths {
            let Some(after_time) = after.get(&rel) else {
                continue;
            };
            if before.get(&rel) == Some(after_time) || rel.contains("/.git/") {
                continue;
            }
            let Some((at, path)) = working_copy_path(&rel) else {
                continue;
            };
            if path.is_empty() {
                continue;
            }
            let full = root.join(rel.strip_prefix("./").unwrap_or(&rel));
            if !full.is_dir() {
                read_batch.push((at, path));
            }
        }
        *self.read_order.lock().expect("reads") = read_batch;
    }

    pub fn accessed_reads(&self) -> Vec<(String, String)> {
        self.read_order.lock().expect("reads").clone()
    }

    pub fn writes(&self) -> Vec<(String, String)> {
        let before = self.before_revision.lock().expect("before revision");
        let after = self.snapshot_revision();
        after
            .into_iter()
            .filter(|(path, bytes)| before.get(path) != Some(bytes))
            .map(|(path, _)| (format!("./{REVISION_DIR}"), path))
            .collect()
    }

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

    // Lays a directory at `path` so a credential read answers as neither file nor link.
    pub fn unreadable(&self, at: &str, path: &str) {
        self.remove(at, path);
        self.ensure_working_copy(at);
        let dir = self.resolve(at, path);
        std::fs::create_dir_all(&dir).expect("unreadable directory");
    }

    pub fn remove(&self, at: &str, path: &str) {
        if let Some(paths) = self.preserved.lock().expect("preserved").get_mut(at) {
            paths.remove(path);
        }
        let file = self.resolve(at, path);
        let _ = std::fs::remove_file(&file);
    }

    pub fn purge(&self, at: &str) {
        let root = self.project.as_ref().expect("project bound");
        let at = at.strip_prefix("./").unwrap_or(at);
        let dir = root.join(at);
        let _ = std::fs::remove_dir_all(&dir);
    }

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

    pub fn holds(&self, at: &str, path: &str, content: &str) {
        self.holds_bytes(at, path, content.as_bytes());
    }

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

    pub fn read_at(&self, working_copy: &str, path: &str) -> Vec<u8> {
        let file = self.resolve(working_copy, path);
        std::fs::read(&file)
            .unwrap_or_else(|err| panic!("read `{path}` under `{working_copy}`: {err}"))
    }

    pub fn written(&self, name: &str) -> Vec<u8> {
        let root = self.project.as_ref().expect("project bound");
        let path = root.join(REVISION_DIR).join(document(name));
        std::fs::read(path).unwrap_or_else(|err| panic!("{name} sealed: {err}"))
    }

    pub fn carry(&self, to: &Self) -> String {
        let spec = self.written("spec");
        let design = self.written("design");
        let plan = self.written("plan");
        to.seed(&spec, &design, &plan)
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

    fn access_times(&self) -> HashMap<String, SystemTime> {
        let root = self.project.as_ref().expect("project bound");
        let mut out = HashMap::new();
        walk_access_times(root, root, &mut out);
        out
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

fn walk_access_times(base: &Path, at: &Path, out: &mut HashMap<String, SystemTime>) {
    if !at.exists() {
        return;
    }
    let meta = std::fs::symlink_metadata(at).expect("metadata");
    let rel = at.strip_prefix(base).expect("under project").display().to_string();
    if let Ok(accessed) = meta.accessed() {
        out.insert(rel, accessed);
    }
    if meta.file_type().is_dir() {
        for entry in std::fs::read_dir(at).expect("read dir") {
            let entry = entry.expect("dir entry");
            walk_access_times(base, &entry.path(), out);
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

fn working_copy_path(rel: &str) -> Option<(String, String)> {
    let at = working_copy_at(rel)?;
    let path = strip_working_path(rel, &at);
    Some((at, path))
}

fn strip_working_path(rel: &str, at: &str) -> String {
    let prefix = at.strip_prefix("./").unwrap_or(at);
    rel.strip_prefix(prefix).and_then(|rest| rest.strip_prefix('/')).unwrap_or(rel).to_owned()
}

pub fn document(name: &str) -> String {
    format!("{SPECIFICATION}/{name}.json")
}
