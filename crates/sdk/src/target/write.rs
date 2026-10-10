//! Serves the `write_files` tool a build or verify turn writes the lent tree
//! through.
//!
//! [`Writer`] declares the tool and answers each call by writing the files it
//! names beneath the workspace root and removing the paths it deletes, every
//! path held to [`beneath`] before any is touched. [`Written`] is what it
//! wrote this turn, which the build fills the answered `written` list from
//! beside the tree itself.

use std::collections::BTreeSet;
use std::fmt;
use std::future::ready;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use emery_adapter::beneath;
use emery_adapter::target::Report;
use omnia_sdk::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use schemars::JsonSchema;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::json;

const WRITE_FILES: &str = "write_files";

// The write tool of one turn: the root it writes beneath, the turn its
// events name, and the files it has written.
pub(super) struct Writer {
    root: PathBuf,
    turn: String,
    written: Written,
}

impl Writer {
    pub(super) fn new(root: &str, turn: &str) -> Self {
        Self {
            root: PathBuf::from(root),
            turn: turn.to_owned(),
            written: Written::default(),
        }
    }

    #[must_use]
    pub(super) fn tool() -> Tool {
        Tool::Function(Function::of::<WriteFiles>(
            WRITE_FILES,
            "Write one or more files beneath `$WORKSPACE`, each created or replaced whole, the \
             directories above it created, and remove the files `delete` names. Each path is \
             `/`-separated and relative to `$WORKSPACE`; a call naming a path outside it, under \
             `.emery/` or `.git/`, or naming a projection, is refused whole and changes nothing.",
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
            tracing::debug!(
                turn = %self.turn,
                tool = WRITE_FILES,
                files = ?outcome.as_ref().ok().map(|done| done.files()),
                deleted = ?outcome.as_ref().ok().map(|done| done.deleted.as_slice()),
                bytes = outcome.as_ref().ok().map(Done::bytes),
                error = outcome.as_ref().err().map(String::as_str),
                "responded"
            );
            let response = outcome.map(|done| {
                let written = done
                    .wrote
                    .iter()
                    .map(|wrote| json!({ "path": wrote.file, "bytes": wrote.bytes }))
                    .collect::<Vec<_>>();
                let mut response = json!({ "written": written });
                if !done.deleted.is_empty() {
                    response["deleted"] = json!(done.deleted);
                }
                response.to_string()
            });
            Box::pin(ready(response))
        })
    }

    // Every path is held to the root before the first write, so a call the
    // rule refuses anywhere leaves the tree as it was.
    fn write(&self, call: &ToolCall) -> Result<Done, String> {
        let WriteFiles { files, delete } =
            call.arguments().map_err(|err| format!("{WRITE_FILES}: {err}"))?;
        if files.is_empty() && delete.is_empty() {
            return Err(format!("{WRITE_FILES}: `files` names no file and `delete` no path"));
        }

        // the paths held to the rule
        let mut accepted = Vec::with_capacity(files.len());
        let mut removals = Vec::with_capacity(delete.len());
        let mut refused = Vec::new();
        for WriteFile { path, content } in files {
            match beneath(&path) {
                Ok(file) => accepted.push((file, content)),
                Err(bad) => refused.push(format!("`{path}` {bad}")),
            }
        }
        for path in delete {
            match beneath(&path) {
                Ok(file) => removals.push(file),
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

        // the files removed; one the tree lacks is already gone
        let mut deleted = Vec::with_capacity(removals.len());
        for file in removals {
            match std::fs::remove_file(self.root.join(&file)) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(format!("{WRITE_FILES}: removing `{file}`: {err}")),
            }
            self.written.forget(&file);
            deleted.push(file);
        }
        Ok(Done { wrote, deleted })
    }
}

// What one call did: the files laid and the files removed.
struct Done {
    wrote: Vec<Wrote>,
    deleted: Vec<String>,
}

impl Done {
    fn files(&self) -> Vec<&str> {
        self.wrote.iter().map(|wrote| wrote.file.as_str()).collect()
    }

    fn bytes(&self) -> usize {
        self.wrote.iter().map(|wrote| wrote.bytes).sum()
    }
}

struct Wrote {
    file: String,
    bytes: usize,
}

// The files `write_files` wrote this turn and has not removed since, shared
// between the tool and the gate: the tool handler and the check are both the
// question's to call, so the set is behind a handle each can hold.
#[derive(Clone, Debug, Default)]
pub(super) struct Written(Arc<Mutex<BTreeSet<String>>>);

impl Written {
    fn record(&self, file: String) {
        self.lock().insert(file);
    }

    fn forget(&self, file: &str) {
        self.lock().remove(file);
    }

    // The rule the slice cannot hold a report to alone: a `written` path
    // names a regular file under the lent tree. A path the grammar refused
    // is left to that finding.
    pub(super) fn findings(&self, root: &Path, report: &Report) -> Vec<String> {
        let mut findings = Vec::new();
        let mut listed = BTreeSet::new();
        for path in &report.written {
            let Ok(file) = beneath(path) else { continue };
            if listed.insert(file.clone()) && !root.join(&file).is_file() {
                findings
                    .push(format!("- written `{path}` names no regular file under the lent tree"));
            }
        }
        findings
    }

    // The report's `written` filled in: every path it lists, spelled as the
    // tree does, and every file written this turn the tree still holds,
    // each once and sorted — so a file the model wrote and left unlisted is
    // reported all the same.
    pub(super) fn fill(&self, root: &Path, report: &mut Report) {
        let mut files: BTreeSet<String> =
            report.written.iter().filter_map(|path| beneath(path).ok()).collect();
        files.extend(self.lock().iter().filter(|file| root.join(file).is_file()).cloned());
        report.written = files.into_iter().collect();
    }

    fn lock(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The files one `write_files` call writes and removes.
// The `///` lines are the `JsonSchema` descriptions the model reads.
#[derive(Debug, Deserialize, JsonSchema)]
struct WriteFiles {
    /// The files to write, in order; a later entry naming an earlier one's
    /// path replaces it.
    #[serde(default, deserialize_with = "files")]
    files: Vec<WriteFile>,
    /// The paths of files to remove, each `/`-separated and relative to
    /// `$WORKSPACE`; one the tree lacks is already removed.
    #[serde(default)]
    delete: Vec<String>,
}

/// One file of a `write_files` call.
#[derive(Debug, Deserialize, JsonSchema)]
struct WriteFile {
    /// The `/`-separated path of the file, relative to `$WORKSPACE`.
    path: String,
    /// The whole content the file holds afterwards.
    content: String,
}

// The schema asks for a sequence of `{path, content}`; a map from path to
// content, which a model reaches for as readily, is read all the same, in
// the order the call spells it.
fn files<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<WriteFile>, D::Error> {
    struct Files;

    impl<'de> Visitor<'de> for Files {
        type Value = Vec<WriteFile>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a list of `{path, content}` entries or a map from path to content")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut files = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(file) = seq.next_element()? {
                files.push(file);
            }
            Ok(files)
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut files = Vec::with_capacity(map.size_hint().unwrap_or(0));
            while let Some((path, content)) = map.next_entry()? {
                files.push(WriteFile { path, content });
            }
            Ok(files)
        }
    }

    deserializer.deserialize_any(Files)
}
