//! Revision trees on the project scratch mount.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use emery_engine::vcs::SPECIFICATION;
use omnia_test::host::Scratch;

type BTreeMapSnapshot = BTreeMap<String, Vec<u8>>;

const REVISION_DIR: &str = ".emery/vcs/revision";

/// Tracks whether the revision tree changed across a run.
#[derive(Clone, Debug, Default)]
pub struct Files {
    project: Option<PathBuf>,
    before: Arc<Mutex<BTreeMapSnapshot>>,
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

    /// Records what exists under the revision tree before the next CLI run.
    pub fn mark_run_start(&self) {
        self.before.lock().expect("before").clone_from(&self.snapshot_revision());
    }

    /// Returns paths under the revision tree that changed since [`mark_run_start`].
    pub fn writes(&self) -> Vec<(String, String)> {
        let before = self.before.lock().expect("before");
        let after = self.snapshot_revision();
        after
            .into_iter()
            .filter(|(path, bytes)| before.get(path) != Some(bytes))
            .map(|(path, _)| (format!("./{REVISION_DIR}"), path))
            .collect()
    }

    fn write_revision(&self, path: &str, body: &[u8]) {
        let root = self.project.as_ref().expect("project bound");
        let file = root.join(REVISION_DIR).join(path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).expect("mkdir revision tree");
        }
        std::fs::write(file, body).expect("write revision file");
    }

    fn snapshot_revision(&self) -> BTreeMapSnapshot {
        let root = self.project.as_ref().expect("project bound").join(REVISION_DIR);
        let mut snap = BTreeMapSnapshot::new();
        walk_snapshot(&root, &root, &mut snap);
        snap
    }

    /// Reads `path` relative to a working-copy directory beneath the project mount.
    pub fn read_at(&self, working_copy: &str, path: &str) -> Vec<u8> {
        let root = self.project.as_ref().expect("project bound");
        let at = working_copy.strip_prefix("./").unwrap_or(working_copy);
        let file = root.join(at).join(path);
        std::fs::read(&file)
            .unwrap_or_else(|err| panic!("read `{path}` under `{working_copy}`: {err}"))
    }
}

fn walk_snapshot(base: &Path, at: &Path, snap: &mut BTreeMapSnapshot) {
    if !at.exists() {
        return;
    }
    if at.is_file() {
        let rel = at.strip_prefix(base).expect("under revision").display().to_string();
        let bytes = std::fs::read(at).expect("read revision file");
        snap.insert(rel, bytes);
        return;
    }
    for entry in std::fs::read_dir(at).expect("read revision dir") {
        let entry = entry.expect("revision entry");
        walk_snapshot(base, &entry.path(), snap);
    }
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
