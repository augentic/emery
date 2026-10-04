//! Serves the `write_file` tool a build turn writes the lent tree through.
//!
//! [`Writer`] declares the tool and answers each call by writing one file
//! beneath the workspace root, held to [`beneath`]. [`Written`] is what it
//! wrote this turn, which the report gate holds the answered `written` list
//! to beside the tree itself.

use std::collections::BTreeSet;
use std::future::ready;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use emery_adapter::beneath;
use emery_adapter::target::Report;
use omnia_sdk::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

const WRITE_FILE: &str = "write_file";

// The write tool of one build turn: the root it writes beneath, the slice
// its events name, and the files it has written.
pub(super) struct Writer {
    root: PathBuf,
    slice: String,
    written: Written,
}

impl Writer {
    pub(super) fn new(root: &str, slice: &str) -> Self {
        Self {
            root: PathBuf::from(root),
            slice: slice.to_owned(),
            written: Written::default(),
        }
    }

    #[must_use]
    pub(super) fn tool() -> Tool {
        Tool::Function(Function::of::<WriteFile>(
            WRITE_FILE,
            "Write one file beneath `$WORKSPACE`, created or replaced whole, the directories \
             above it created. The path is `/`-separated and relative to `$WORKSPACE`; one \
             outside it, or among the engine's own files, is refused.",
        ))
    }

    #[must_use]
    pub(super) fn written(&self) -> Written {
        self.written.clone()
    }

    // Answers `write_file` here and hands every other call to `rest`.
    pub(super) fn serve(self, mut rest: Tools) -> Tools {
        Box::new(move |call: ToolCall| -> ToolFuture {
            if call.name != WRITE_FILE {
                return rest(call);
            }
            let outcome = self.write(&call);
            tracing::debug!(
                slice = %self.slice,
                tool = WRITE_FILE,
                path = outcome.as_ref().ok().map(|wrote| wrote.file.as_str()),
                bytes = outcome.as_ref().ok().map(|wrote| wrote.bytes),
                error = outcome.as_ref().err().map(String::as_str),
                "responded"
            );
            let response = outcome
                .map(|wrote| json!({ "path": wrote.file, "bytes": wrote.bytes }).to_string());
            Box::pin(ready(response))
        })
    }

    fn write(&self, call: &ToolCall) -> Result<Wrote, String> {
        let WriteFile { path, content } =
            call.arguments().map_err(|err| format!("{WRITE_FILE}: {err}"))?;
        let file = beneath(&path).map_err(|bad| format!("{WRITE_FILE}: `{path}` {bad}"))?;
        let full = self.root.join(&file);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                format!("{WRITE_FILE}: creating the directories above `{file}`: {err}")
            })?;
        }
        std::fs::write(&full, &content)
            .map_err(|err| format!("{WRITE_FILE}: writing `{file}`: {err}"))?;
        self.written.record(file.clone());
        Ok(Wrote {
            file,
            bytes: content.len(),
        })
    }
}

struct Wrote {
    file: String,
    bytes: usize,
}

// The files `write_file` wrote this turn, shared between the tool and the
// gate: the tool handler and the check are both the question's to call, so
// the set is behind a handle each can hold.
#[derive(Clone, Debug, Default)]
pub(super) struct Written(Arc<Mutex<BTreeSet<String>>>);

impl Written {
    fn record(&self, file: String) {
        self.lock().insert(file);
    }

    // The rules the slice cannot hold a report to alone: a `written` path
    // names a regular file under the lent tree, and a file written this turn
    // is listed. A path the grammar refused is left to that finding.
    pub(super) fn findings(&self, root: &Path, report: &Report) -> Vec<String> {
        let mut findings = Vec::new();

        // the listed files, each held to the tree once however spelled
        let mut listed = BTreeSet::new();
        for path in &report.written {
            let Ok(file) = beneath(path) else { continue };
            if listed.insert(file.clone()) && !root.join(&file).is_file() {
                findings
                    .push(format!("- written `{path}` names no regular file under the lent tree"));
            }
        }

        // the files written and left out
        findings.extend(self.lock().difference(&listed).map(|file| {
            format!("- `{WRITE_FILE}` wrote `{file}` this turn, which `written` leaves out")
        }));

        findings
    }

    fn lock(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The file `write_file` writes.
// The `///` lines are the `JsonSchema` descriptions the model reads.
#[derive(Debug, Deserialize, JsonSchema)]
struct WriteFile {
    /// The `/`-separated path of the file, relative to `$WORKSPACE`.
    path: String,
    /// The whole content the file holds afterwards.
    content: String,
}
