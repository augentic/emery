//! Lays files under the two roots the guest's preopens answer for.
//!
//! A config file is project-relative, a component is relative to the
//! adapters root the runtime mounts apart from the project, so each write
//! answers with the path the CLI is handed.

use std::fs;
use std::path::{Path, PathBuf};

use emery_engine::ADAPTERS;

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn adapters_root() -> PathBuf {
    Path::new(PROJECT_ROOT).join(ADAPTERS)
}

/// A scratch directory under each root, removed with the value.
pub struct Scratch {
    project: tempfile::TempDir,
    adapters: tempfile::TempDir,
}

impl Scratch {
    /// Creates a scratch directory under the project root and one under the adapters root.
    pub fn new() -> Self {
        let adapters = adapters_root();
        fs::create_dir_all(&adapters).expect("adapters root");
        Self {
            project: tempfile::TempDir::new_in(PROJECT_ROOT).expect("project tempdir"),
            adapters: tempfile::TempDir::new_in(adapters).expect("adapters tempdir"),
        }
    }

    /// Writes `name` beneath the adapters scratch and returns its path relative to the adapters root.
    pub fn write(&self, name: &str, body: impl AsRef<[u8]>) -> String {
        Self::write_under(self.adapters.path(), &adapters_root(), name, body)
    }

    /// Writes an `emery.toml` beneath the project scratch and returns its project-relative path.
    pub fn config(&self, body: &str) -> String {
        Self::write_under(self.project.path(), Path::new(PROJECT_ROOT), "emery.toml", body)
    }

    /// Writes the component `<stem>.wasm` and returns its path relative to the adapters root.
    ///
    /// The loader is scripted, so the component only has to exist as a
    /// `.wasm` file; its stem is the guest name the load registers.
    pub fn component(&self, stem: &str) -> String {
        self.write(&format!("{stem}.wasm"), b"\0asm-stub")
    }

    /// Removes `name` from beneath the adapters scratch.
    pub fn remove(&self, name: &str) {
        fs::remove_file(self.adapters.path().join(name))
            .unwrap_or_else(|err| panic!("remove {name}: {err}"));
    }

    fn write_under(root: &Path, base: &Path, name: &str, body: impl AsRef<[u8]>) -> String {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|err| panic!("mkdir for {name}: {err}"));
        }
        fs::write(&path, body).unwrap_or_else(|err| panic!("write {name}: {err}"));
        path.strip_prefix(base)
            .expect("path under its root")
            .to_str()
            .expect("utf-8 path")
            .to_string()
    }
}
