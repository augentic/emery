//! Serves the `write_files` tool a build or verify turn writes the lent tree
//! through.
//!
//! [`Writer`] declares the tool and answers each call by writing the files it
//! names beneath the workspace root and removing the paths it deletes, every
//! path held to [`beneath`] and to the tree — no symlink anywhere on it —
//! before any is touched. [`Written`] is what it wrote this turn, which the
//! build fills the answered `written` list from beside the tree itself.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::future::ready;
use std::io::{self, ErrorKind, Write as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use emery_adapter::beneath;
use emery_adapter::target::Report;
use omnia_sdk::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use schemars::JsonSchema;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
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
             `.emery/` or `.git/`, naming a projection, or leading through a symlink, is \
             refused whole and changes nothing.",
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

    // Every path is held to the root and to the tree before the first write,
    // so a call the rule refuses anywhere leaves the tree as it was.
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
            match self.held(&path) {
                Ok((file, held)) => accepted.push((file, held, content)),
                Err(why) => refused.push(why),
            }
        }
        for path in delete {
            match self.held(&path) {
                Ok((file, held)) => removals.push((file, held)),
                Err(why) => refused.push(why),
            }
        }
        if !refused.is_empty() {
            return Err(format!("{WRITE_FILES}: nothing written: {}", refused.join("; ")));
        }

        // the files laid in order
        let mut wrote = Vec::with_capacity(accepted.len());
        for (file, held, content) in accepted {
            if held == Held::Absent
                && let Some((dir, _)) = file.rsplit_once('/')
            {
                make_dirs(&self.root, dir).map_err(|err| {
                    format!("{WRITE_FILES}: creating the directories above `{file}`: {err}")
                })?;
            }
            lay(&self.root.join(&file), held, &content)
                .map_err(|err| format!("{WRITE_FILES}: writing `{file}`: {err}"))?;
            self.written.record(file.clone());
            wrote.push(Wrote {
                file,
                bytes: content.len(),
            });
        }

        // the files removed; one the tree lacks is already gone
        let mut deleted = Vec::with_capacity(removals.len());
        for (file, held) in removals {
            if held == Held::File {
                match fs::remove_file(self.root.join(&file)) {
                    Ok(()) => {}
                    Err(err) if err.kind() == ErrorKind::NotFound => {}
                    Err(err) => return Err(format!("{WRITE_FILES}: removing `{file}`: {err}")),
                }
            }
            self.written.forget(&file);
            deleted.push(file);
        }
        Ok(Done { wrote, deleted })
    }

    // The path held to `beneath` and to the tree, spelled as the tree does,
    // or the refusal naming it.
    fn held(&self, path: &str) -> Result<(String, Held), String> {
        let file = beneath(path).map_err(|bad| format!("`{path}` {bad}"))?;
        let held = Held::at(&self.root, &file).map_err(|why| format!("`{path}` {why}"))?;
        Ok((file, held))
    }
}

// What the tree holds at a root-relative path, read without following a
// link anywhere on it: a regular file, or nothing yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    Absent,
    File,
}

impl Held {
    // The rule `beneath` cannot hold a path to, reading its spelling alone:
    // no component is a symlink, each directory on the way is one, and the
    // file is a regular file or absent. A link would carry a write or a
    // removal wherever the mount reaches. The clause answered follows the
    // path as `BadPath`'s do.
    fn at(root: &Path, file: &str) -> Result<Self, String> {
        let (dirs, _) = file.rsplit_once('/').unwrap_or(("", file));
        let mut prefix = String::with_capacity(file.len());
        for segment in dirs.split('/').filter(|segment| !segment.is_empty()) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(segment);
            let Some(kind) = kind_at(root, &prefix)? else {
                return Ok(Self::Absent);
            };
            if kind.is_symlink() {
                return Err(format!("leads through the symlink `{prefix}`"));
            }
            if !kind.is_dir() {
                return Err(format!("leads through `{prefix}`, which is not a directory"));
            }
        }

        let Some(kind) = kind_at(root, file)? else {
            return Ok(Self::Absent);
        };
        if kind.is_symlink() {
            Err("is a symlink".to_owned())
        } else if kind.is_dir() {
            Err("names a directory".to_owned())
        } else if kind.is_file() {
            Ok(Self::File)
        } else {
            Err("is not a regular file".to_owned())
        }
    }
}

fn holds_file(root: &Path, file: &str) -> bool {
    matches!(Held::at(root, file), Ok(Held::File))
}

// The kind of entry at a root-relative path, the link itself where one
// stands there, or none where the tree holds nothing.
fn kind_at(root: &Path, path: &str) -> Result<Option<fs::FileType>, String> {
    match root.join(path).symlink_metadata() {
        Ok(meta) => Ok(Some(meta.file_type())),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("cannot be read at `{path}`: {err}")),
    }
}

// Makes the directories of a root-relative path one at a time, so a link
// standing where one is to go is refused rather than followed as
// `create_dir_all` would follow it.
fn make_dirs(root: &Path, dirs: &str) -> io::Result<()> {
    let mut full = root.to_path_buf();
    for segment in dirs.split('/') {
        full.push(segment);
        match fs::create_dir(&full) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                if !full.symlink_metadata()?.file_type().is_dir() {
                    return Err(io::Error::other(format!(
                        "`{}` is not a directory",
                        full.strip_prefix(root).unwrap_or(&full).display()
                    )));
                }
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

// Lays one file whole. A file the walk found absent is created and never
// followed, so a link raced into its place fails the open; one it found is
// truncated in place, keeping its mode — `std` on `wasm32` has no no-follow
// open, so that open alone trusts the walk.
fn lay(full: &Path, held: Held, content: &str) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true);
    match held {
        Held::Absent => options.create_new(true),
        Held::File => options.truncate(true),
    };
    options.open(full)?.write_all(content.as_bytes())
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
    // names a regular file under the lent tree, reached through no link. A
    // path the grammar refused is left to that finding.
    pub(super) fn findings(root: &Path, report: &Report) -> Vec<String> {
        let mut findings = Vec::new();
        let mut listed = BTreeSet::new();
        for path in &report.written {
            let Ok(file) = beneath(path) else { continue };
            if listed.insert(file.clone()) && !holds_file(root, &file) {
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
        files.extend(self.lock().iter().filter(|file| holds_file(root, file)).cloned());
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
// the order the call spells it; and one entry spelled bare — a map with a
// `path` key — is that entry, not files named `path` and `content`.
fn files<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<WriteFile>, D::Error> {
    struct Files;

    impl<'de> Visitor<'de> for Files {
        type Value = Vec<WriteFile>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(
                "a list of `{path, content}` entries, one such entry, or a map from path to \
                 content",
            )
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut files = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(file) = seq.next_element()? {
                files.push(file);
            }
            Ok(files)
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut entries: Vec<(String, String)> =
                Vec::with_capacity(map.size_hint().unwrap_or(0));
            while let Some(entry) = map.next_entry()? {
                entries.push(entry);
            }
            if entries.iter().any(|(key, _)| key == "path") {
                return single(entries).map(|file| vec![file]);
            }
            Ok(entries.into_iter().map(|(path, content)| WriteFile { path, content }).collect())
        }
    }

    deserializer.deserialize_any(Files)
}

// The one entry a bare `{path, content}` spells, held to those two keys as
// the derived reading of the struct would hold a listed one.
fn single<E: de::Error>(entries: Vec<(String, String)>) -> Result<WriteFile, E> {
    let (mut path, mut content) = (None, None);
    for (key, value) in entries {
        match key.as_str() {
            "path" => path = Some(value),
            "content" => content = Some(value),
            other => return Err(E::unknown_field(other, &["path", "content"])),
        }
    }
    Ok(WriteFile {
        path: path.ok_or_else(|| E::missing_field("path"))?,
        content: content.ok_or_else(|| E::missing_field("content"))?,
    })
}
