//! Serves the `write_files` tool a build turn writes the lent tree through.
//!
//! [`Writer`] declares the tool and answers each call by writing the files it
//! names beneath the workspace root, every path held to [`beneath`] before
//! any is written. [`Written`] is what it wrote this turn, which the report
//! gate holds the answered `written` list to beside the tree itself.

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

const WRITE_FILES: &str = "write_files";

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
        Tool::Function(Function::of::<WriteFiles>(
            WRITE_FILES,
            "Write one or more files beneath `$WORKSPACE`, each created or replaced whole, the \
             directories above it created. Each path is `/`-separated and relative to \
             `$WORKSPACE`; a call naming a path outside it, under `.emery/` or `.git/`, or \
             naming a projection, is refused whole and writes nothing.",
        ))
    }

    #[must_use]
    pub(super) fn written(&self) -> Written {
        self.written.clone()
    }

    // Answers `write_files` here and hands every other call to `rest`.
    pub(super) fn serve(self, mut rest: Tools) -> Tools {
        Box::new(move |call: ToolCall| -> ToolFuture {
            if call.name != WRITE_FILES {
                return rest(call);
            }
            let outcome = self.write(&call);
            let files = outcome
                .as_ref()
                .ok()
                .map(|wrote| wrote.iter().map(|wrote| wrote.file.as_str()).collect::<Vec<_>>());
            tracing::debug!(
                slice = %self.slice,
                tool = WRITE_FILES,
                files = ?files,
                bytes = outcome
                    .as_ref()
                    .ok()
                    .map(|wrote| wrote.iter().map(|wrote| wrote.bytes).sum::<usize>()),
                error = outcome.as_ref().err().map(String::as_str),
                "responded"
            );
            let response = outcome.map(|wrote| {
                let written = wrote
                    .iter()
                    .map(|wrote| json!({ "path": wrote.file, "bytes": wrote.bytes }))
                    .collect::<Vec<_>>();
                json!({ "written": written }).to_string()
            });
            Box::pin(ready(response))
        })
    }

    // Every path is held to the root before the first write, so a call the
    // rule refuses anywhere leaves the tree as it was.
    fn write(&self, call: &ToolCall) -> Result<Vec<Wrote>, String> {
        let WriteFiles { files } =
            call.arguments().map_err(|err| format!("{WRITE_FILES}: {err}"))?;
        if files.is_empty() {
            return Err(format!("{WRITE_FILES}: `files` names no file"));
        }

        // the paths held to the rule
        let mut accepted = Vec::with_capacity(files.len());
        let mut refused = Vec::new();
        for WriteFile { path, content } in files {
            match beneath(&path) {
                Ok(file) => accepted.push((file, content)),
                Err(bad) => refused.push(format!("`{path}` {bad}")),
            }
        }
        if !refused.is_empty() {
            return Err(format!("{WRITE_FILES}: nothing written: {}", refused.join("; ")));
        }

        // the files laid in order
        let mut wrote = Vec::with_capacity(accepted.len());
        for (file, content) in accepted {
            let full = self.root.join(&file);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    format!("{WRITE_FILES}: creating the directories above `{file}`: {err}")
                })?;
            }
            std::fs::write(&full, &content)
                .map_err(|err| format!("{WRITE_FILES}: writing `{file}`: {err}"))?;
            self.written.record(file.clone());
            wrote.push(Wrote {
                file,
                bytes: content.len(),
            });
        }
        Ok(wrote)
    }
}

struct Wrote {
    file: String,
    bytes: usize,
}

// The files `write_files` wrote this turn, shared between the tool and the
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
            format!("- `{WRITE_FILES}` wrote `{file}` this turn, which `written` leaves out")
        }));

        findings
    }

    fn lock(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The files one `write_files` call writes.
// The `///` lines are the `JsonSchema` descriptions the model reads.
#[derive(Debug, Deserialize, JsonSchema)]
struct WriteFiles {
    /// The files to write, in order; a later entry naming an earlier one's
    /// path replaces it.
    files: Vec<WriteFile>,
}

/// One file of a `write_files` call.
#[derive(Debug, Deserialize, JsonSchema)]
struct WriteFile {
    /// The `/`-separated path of the file, relative to `$WORKSPACE`.
    path: String,
    /// The whole content the file holds afterwards.
    content: String,
}
