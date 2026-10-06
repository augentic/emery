//! Lays a config file under the project root the guest's preopen answers for.
//!
//! The CLI reads `emery.toml` through the filesystem, so a scenario naming
//! one writes it beneath the project root and hands the CLI the
//! project-relative path.

use std::fs;
use std::path::Path;

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// A scratch directory under the project root, removed with the value.
pub struct Scratch {
    project: tempfile::TempDir,
}

impl Scratch {
    /// Creates a scratch directory under the project root.
    pub fn new() -> Self {
        Self {
            project: tempfile::TempDir::new_in(PROJECT_ROOT).expect("project tempdir"),
        }
    }

    /// Writes an `emery.toml` beneath the scratch and returns its project-relative path.
    pub fn config(&self, body: &str) -> String {
        let path = self.project.path().join("emery.toml");
        fs::write(&path, body).unwrap_or_else(|err| panic!("write emery.toml: {err}"));
        path.strip_prefix(Path::new(PROJECT_ROOT))
            .expect("path under the project root")
            .to_str()
            .expect("utf-8 path")
            .to_string()
    }
}
