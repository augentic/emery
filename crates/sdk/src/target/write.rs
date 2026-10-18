//! Serves the `write_files` and `edit_files` tools a build or verify turn
//! writes the lent tree through.
//!
//! [`Writer`] declares the two tools and answers each call beneath the
//! workspace root: `write_files` lays the files it names whole and removes
//! the paths it deletes; `edit_files` replaces the one place each `find`
//! matches within a file the tree holds. Every path is held to [`beneath`]
//! and to the tree — no symlink anywhere on it — before any is touched, and
//! a write that would empty a file or cut it below half its size is refused
//! unless the entry says so. [`Written`] is what the turn wrote or edited,
//! which the build fills the answered `written` list from beside the tree
//! itself.

use std::collections::{BTreeMap, BTreeSet};
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
const EDIT_FILES: &str = "edit_files";

// The write tools of one turn: the root they write beneath, the turn their
// events name, and the files they have written.
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
    pub(super) fn tools() -> [Tool; 2] {
        [
            Tool::Function(Function::of::<WriteFiles>(
                WRITE_FILES,
                "Write one or more files beneath `$WORKSPACE`, each created or replaced whole, \
                 the directories above it created, and remove the files `delete` names. Each \
                 path is `/`-separated and relative to `$WORKSPACE`; a call naming a path \
                 outside it, under `.emery/` or `.git/`, naming a projection, or leading \
                 through a symlink, is refused whole and changes nothing, as is one whose \
                 entry would leave a file empty or under half its size without `shrink` set. \
                 For a change within a file, use `edit_files`.",
            )),
            Tool::Function(Function::of::<EditFiles>(
                EDIT_FILES,
                "Edit within one or more files beneath `$WORKSPACE`: each edit replaces the one \
                 place `find` matches in the file at `path` with `replace`, and the file is \
                 otherwise as it was. Every edit is held before any is applied: a `find` \
                 matching nowhere or more than once, a path the tree holds no file at, or a \
                 path `write_files` would refuse, refuses the call whole and changes nothing. \
                 To create or replace a file whole, use `write_files`.",
            )),
        ]
    }

    #[must_use]
    pub(super) fn written(&self) -> Written {
        self.written.clone()
    }

    // Answers `write_files` and `edit_files` here and hands every other call
    // to `rest`.
    pub(super) fn serve(self, mut rest: Tools) -> Tools {
        Box::new(move |call: ToolCall| -> ToolFuture {
            let response = match call.name.as_str() {
                WRITE_FILES => {
                    let outcome = self.write(&call);
                    tracing::debug!(
                        turn = %self.turn,
                        tool = WRITE_FILES,
                        wrote = ?outcome.as_ref().ok().map(|done| done.wrote.as_slice()),
                        deleted = ?outcome.as_ref().ok().map(|done| done.deleted.as_slice()),
                        error = outcome.as_ref().err().map(String::as_str),
                        "responded"
                    );
                    outcome.map(|done| {
                        let mut response = json!({ "written": json_of(&done.wrote) });
                        if !done.deleted.is_empty() {
                            response["deleted"] = json!(done.deleted);
                        }
                        response.to_string()
                    })
                }
                EDIT_FILES => {
                    let outcome = self.edit(&call);
                    tracing::debug!(
                        turn = %self.turn,
                        tool = EDIT_FILES,
                        edited = ?outcome.as_ref().ok().map(Vec::as_slice),
                        error = outcome.as_ref().err().map(String::as_str),
                        "responded"
                    );
                    outcome.map(|edited| json!({ "edited": json_of(&edited) }).to_string())
                }
                _ => return rest(call),
            };
            Box::pin(ready(response))
        })
    }

    // Every path is held to the root and to the tree before the first write,
    // and every entry replacing a held file to the shrink rule, so a call
    // refused anywhere leaves the tree as it was. What the tree holds at each
    // is read once and moved with every write, so a later entry naming a
    // path, or its removal, meets what the call laid there rather than what
    // stood before it.
    fn write(&self, call: &ToolCall) -> Result<Done, String> {
        let WriteFiles { files, delete } =
            call.arguments().map_err(|err| format!("{WRITE_FILES}: {err}"))?;
        if files.is_empty() && delete.is_empty() {
            return Err(format!("{WRITE_FILES}: `files` names no file and `delete` no path"));
        }

        // the paths held to the rule, and a replacement to the shrink rule
        let mut tree = BTreeMap::new();
        let mut accepted = Vec::with_capacity(files.len());
        let mut removals = Vec::with_capacity(delete.len());
        let mut refused = Vec::new();
        for WriteFile {
            path,
            content,
            shrink,
        } in files
        {
            match self.held(&path, &mut tree) {
                Ok(file) => {
                    if !shrink && let Some(why) = self.shrinks(&file, tree[&file], &content) {
                        refused.push(why);
                    }
                    accepted.push((file, content));
                }
                Err(why) => refused.push(why),
            }
        }
        for path in delete {
            match self.held(&path, &mut tree) {
                Ok(file) => removals.push(file),
                Err(why) => refused.push(why),
            }
        }
        if !refused.is_empty() {
            return Err(format!("{WRITE_FILES}: nothing written: {}", refused.join("; ")));
        }

        // the files laid in order
        let mut wrote = Vec::with_capacity(accepted.len());
        for (file, content) in accepted {
            let held = tree[&file];
            if held == Held::Absent
                && let Some((dir, _)) = file.rsplit_once('/')
            {
                make_dirs(&self.root, dir).map_err(|err| {
                    format!("{WRITE_FILES}: creating the directories above `{file}`: {err}")
                })?;
            }
            lay(&self.root.join(&file), held, &content)
                .map_err(|err| format!("{WRITE_FILES}: writing `{file}`: {err}"))?;
            tree.insert(file.clone(), Held::File);
            self.written.record(file.clone());
            wrote.push(Wrote {
                file,
                bytes: content.len(),
            });
        }

        // the files removed, one this call laid among them; one the tree
        // lacks is already gone
        let mut deleted = Vec::with_capacity(removals.len());
        for file in removals {
            if tree[&file] == Held::File {
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

    // Every path is held to the rule and to a file the tree holds, and every
    // `find` to one match in the file as the call's earlier edits leave it,
    // before any file is written, so a call refused anywhere leaves the tree
    // as it was; a file edited more than once is written once.
    fn edit(&self, call: &ToolCall) -> Result<Vec<Wrote>, String> {
        let EditFiles { edits } = call.arguments().map_err(|err| format!("{EDIT_FILES}: {err}"))?;
        if edits.is_empty() {
            return Err(format!("{EDIT_FILES}: `edits` names no edit"));
        }

        // the edits applied in memory, in order
        let mut tree = BTreeMap::new();
        let mut contents: BTreeMap<String, String> = BTreeMap::new();
        let mut refused = Vec::new();
        for Edit { path, find, replace } in edits {
            let file = match self.held(&path, &mut tree) {
                Ok(file) => file,
                Err(why) => {
                    refused.push(why);
                    continue;
                }
            };
            if tree[&file] == Held::Absent {
                refused.push(format!("`{path}` names no file in the tree"));
                continue;
            }
            if find.is_empty() {
                refused.push(format!("`{path}`: `find` is empty"));
                continue;
            }
            let content = match contents.entry(file.clone()) {
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::btree_map::Entry::Vacant(entry) => {
                    match fs::read_to_string(self.root.join(&file)) {
                        Ok(content) => entry.insert(content),
                        Err(err) => {
                            refused.push(format!("`{path}` cannot be read: {err}"));
                            continue;
                        }
                    }
                }
            };
            match content.matches(find.as_str()).count() {
                1 => *content = content.replacen(find.as_str(), &replace, 1),
                0 => refused.push(format!("`{path}`: `find` matches nothing in the file")),
                n => refused.push(format!("`{path}`: `find` matches {n} places in the file")),
            }
        }
        if !refused.is_empty() {
            return Err(format!("{EDIT_FILES}: nothing edited: {}", refused.join("; ")));
        }

        // the edited files written whole
        let mut edited = Vec::with_capacity(contents.len());
        for (file, content) in contents {
            lay(&self.root.join(&file), Held::File, &content)
                .map_err(|err| format!("{EDIT_FILES}: writing `{file}`: {err}"))?;
            self.written.record(file.clone());
            edited.push(Wrote {
                file,
                bytes: content.len(),
            });
        }
        Ok(edited)
    }

    // The path held to `beneath` and to the tree, spelled as the tree does,
    // what the tree holds there noted in `tree` the first time the call names
    // it, or the refusal naming it.
    fn held(&self, path: &str, tree: &mut BTreeMap<String, Held>) -> Result<String, String> {
        let file = beneath(path).map_err(|bad| format!("`{path}` {bad}"))?;
        if !tree.contains_key(&file) {
            let held = Held::at(&self.root, &file).map_err(|why| format!("`{path}` {why}"))?;
            tree.insert(file.clone(), held);
        }
        Ok(file)
    }

    // The shrink rule: a file the tree holds is not left empty, or under half
    // its size, by an entry that does not say so, since a model that sends a
    // fragment of a file it means to edit loses the rest. A new file, or one
    // already empty, is under no rule.
    fn shrinks(&self, file: &str, held: Held, content: &str) -> Option<String> {
        if held != Held::File {
            return None;
        }
        let current = fs::metadata(self.root.join(file)).ok()?.len();
        let after = content.len() as u64;
        (current > 0 && (after == 0 || after * 2 < current)).then(|| {
            format!(
                "`{file}` would shrink from {current} to {after} bytes; a file is replaced \
                 whole, so send the whole file, use `{EDIT_FILES}` for a change within it, or \
                 set `shrink: true` if that is meant"
            )
        })
    }
}

fn json_of(wrote: &[Wrote]) -> Vec<serde_json::Value> {
    wrote.iter().map(|wrote| json!({ "path": wrote.file, "bytes": wrote.bytes })).collect()
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

// What one `write_files` call did: the files laid and the files removed.
struct Done {
    wrote: Vec<Wrote>,
    deleted: Vec<String>,
}

struct Wrote {
    file: String,
    bytes: usize,
}

// Logged as `"path"=bytes`, so a reader of the turn's events has each
// file's size beside its path.
impl fmt::Debug for Wrote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}={}", self.file, self.bytes)
    }
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
    /// `$WORKSPACE`; one the tree lacks is already removed, and one `files`
    /// also names is written, then removed.
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
    /// Set when the file is meant to become empty or less than half its
    /// current size; without it, such a write is refused, since it is
    /// usually a fragment sent in place of the whole file.
    #[serde(default)]
    shrink: bool,
}

/// The edits one `edit_files` call makes within files the tree holds.
// The `///` lines are the `JsonSchema` descriptions the model reads.
#[derive(Debug, Deserialize, JsonSchema)]
struct EditFiles {
    /// The edits, in order; a later edit of a file meets it as the earlier
    /// ones left it.
    edits: Vec<Edit>,
}

/// One edit: the one place `find` matches in a file, replaced.
#[derive(Debug, Deserialize, JsonSchema)]
struct Edit {
    /// The `/`-separated path of the file, relative to `$WORKSPACE`, which
    /// the tree must hold.
    path: String,
    /// The text to replace, which must occur exactly once in the file.
    find: String,
    /// The text put in its place; empty removes the match.
    replace: String,
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
            let mut entries: Vec<(String, serde_json::Value)> =
                Vec::with_capacity(map.size_hint().unwrap_or(0));
            while let Some(entry) = map.next_entry()? {
                entries.push(entry);
            }
            if entries.iter().any(|(key, _)| key == "path") {
                return single(entries).map(|file| vec![file]);
            }
            entries
                .into_iter()
                .map(|(path, content)| {
                    let content = text(content, &path)?;
                    Ok(WriteFile {
                        path,
                        content,
                        shrink: false,
                    })
                })
                .collect()
        }
    }

    deserializer.deserialize_any(Files)
}

// The one entry a bare `{path, content, shrink}` spells, held to those keys
// as the derived reading of the struct would hold a listed one.
fn single<E: de::Error>(entries: Vec<(String, serde_json::Value)>) -> Result<WriteFile, E> {
    let (mut path, mut content, mut shrink) = (None, None, false);
    for (key, value) in entries {
        match key.as_str() {
            "path" => path = Some(text(value, "path")?),
            "content" => content = Some(text(value, "content")?),
            "shrink" => {
                shrink = value
                    .as_bool()
                    .ok_or_else(|| E::custom("`shrink` holds a non-boolean, not true or false"))?;
            }
            other => return Err(E::unknown_field(other, &["path", "content", "shrink"])),
        }
    }
    Ok(WriteFile {
        path: path.ok_or_else(|| E::missing_field("path"))?,
        content: content.ok_or_else(|| E::missing_field("content"))?,
        shrink,
    })
}

fn text<E: de::Error>(value: serde_json::Value, at: &str) -> Result<String, E> {
    match value {
        serde_json::Value::String(text) => Ok(text),
        serde_json::Value::Null => Err(E::custom(format!("`{at}` holds null, not a string"))),
        serde_json::Value::Bool(_) => {
            Err(E::custom(format!("`{at}` holds a boolean, not a string")))
        }
        serde_json::Value::Number(_) => {
            Err(E::custom(format!("`{at}` holds a number, not a string")))
        }
        serde_json::Value::Array(_) => Err(E::custom(format!("`{at}` holds a list, not a string"))),
        serde_json::Value::Object(_) => Err(E::custom(format!("`{at}` holds a map, not a string"))),
    }
}
