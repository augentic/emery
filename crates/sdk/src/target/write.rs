//! Serves the `write_files` and `edit_files` tools for build and verify turns.

use std::collections::{BTreeMap, BTreeSet};
use std::future::ready;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::{fmt, fs};

use emery_adapter::target::Report;
use emery_adapter::{Standing, beneath, standing, write_whole};
use omnia_sdk::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use schemars::JsonSchema;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::json;

const WRITE_FILES: &str = "write_files";
const EDIT_FILES: &str = "edit_files";

// The write tools of one turn: the root they write beneath, the turn their
// events name, the files they have written, and the files `write_files`
// may not replace or remove.
pub(super) struct Writer {
    root: PathBuf,
    turn: String,
    written: Written,
    guarded: BTreeSet<String>,
}

impl Writer {
    pub(super) fn new(root: &str, turn: &str) -> Self {
        Self {
            root: PathBuf::from(root),
            turn: turn.to_owned(),
            written: Written::default(),
            guarded: BTreeSet::new(),
        }
    }

    // The writer refusing a `write_files` entry or `delete` over `paths`,
    // each spelled as the tree does; a path the rule refuses guards nothing,
    // since no call can name it either.
    pub(super) fn guarding(mut self, paths: &[String]) -> Self {
        self.guarded = paths.iter().filter_map(|path| beneath(path).ok()).collect();
        self
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
                 entry would leave a file empty or under half its size without `shrink` set, \
                 or one naming a file the brief says was scaffolded before this turn. For a \
                 change within a file, use `edit_files`.",
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
    // path, or its removal, meets what the call wrote there rather than what
    // stood before it.
    fn write(&self, call: &ToolCall) -> Result<Done, String> {
        let WriteFiles { files, delete } =
            call.arguments().map_err(|err| format!("{WRITE_FILES}: {err}"))?;
        if files.is_empty() && delete.is_empty() {
            return Err(format!("{WRITE_FILES}: `files` names no file and `delete` no path"));
        }

        // the paths held to the rule, a replacement to the shrink rule, and
        // every path to the guard
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
                    if let Some(why) = self.guards(&file) {
                        refused.push(why);
                    } else if !shrink && let Some(why) = self.shrinks(&file, tree[&file], &content)
                    {
                        refused.push(why);
                    }
                    accepted.push((file, content));
                }
                Err(why) => refused.push(why),
            }
        }
        for path in delete {
            match self.held(&path, &mut tree) {
                Ok(file) => {
                    if let Some(why) = self.guards(&file) {
                        refused.push(why);
                    }
                    removals.push(file);
                }
                Err(why) => refused.push(why),
            }
        }
        if !refused.is_empty() {
            return Err(format!("{WRITE_FILES}: nothing written: {}", refused.join("; ")));
        }

        // the files written in order
        let mut wrote = Vec::with_capacity(accepted.len());
        for (file, content) in accepted {
            write_whole(&self.root, &file, tree[&file], content.as_bytes())
                .map_err(|err| format!("{WRITE_FILES}: writing `{file}`: {err}"))?;
            tree.insert(file.clone(), Standing::File);
            self.written.record(file.clone());
            wrote.push(Wrote {
                file,
                bytes: content.len(),
            });
        }

        // the files removed, one this call wrote among them; one the tree
        // lacks is already gone
        let mut deleted = Vec::with_capacity(removals.len());
        for file in removals {
            if tree[&file] == Standing::File {
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
            if tree[&file] == Standing::Absent {
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
                0 => refused.push(format!(
                    "`{path}`: `find` matches nothing in the file; {}",
                    nearest(content, &find)
                )),
                n => refused.push(format!("`{path}`: `find` matches {n} places in the file")),
            }
        }
        if !refused.is_empty() {
            return Err(format!("{EDIT_FILES}: nothing edited: {}", refused.join("; ")));
        }

        // the edited files written whole
        let mut edited = Vec::with_capacity(contents.len());
        for (file, content) in contents {
            write_whole(&self.root, &file, Standing::File, content.as_bytes())
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
    // it, or the refusal naming it. A link anywhere on the path would carry
    // a write or a removal wherever the mount reaches.
    fn held(&self, path: &str, tree: &mut BTreeMap<String, Standing>) -> Result<String, String> {
        let file = beneath(path).map_err(|bad| format!("`{path}` {bad}"))?;
        if !tree.contains_key(&file) {
            let held = standing(&self.root, &file).map_err(|why| format!("`{path}` {why}"))?;
            tree.insert(file.clone(), held);
        }
        Ok(file)
    }

    // The guard: a file the adapter scaffolded before the turn is changed
    // through `edit_files` alone, since one written whole loses the markers
    // the adapter laid in it.
    fn guards(&self, file: &str) -> Option<String> {
        self.guarded.contains(file).then(|| {
            format!(
                "`{file}` was scaffolded before this turn; change it through `{EDIT_FILES}`, \
                 which keeps what the skeleton holds"
            )
        })
    }

    // The shrink rule: a file the tree holds is not left empty, or under half
    // its size, by an entry that does not say so, since a model that sends a
    // fragment of a file it means to edit loses the rest. A new file, or one
    // already empty, is under no rule.
    fn shrinks(&self, file: &str, held: Standing, content: &str) -> Option<String> {
        if held != Standing::File {
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

fn holds_file(root: &Path, file: &str) -> bool {
    matches!(standing(root, file), Ok(Standing::File))
}

// What one `write_files` call did: the files written and the files removed.
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
#[derive(Debug, JsonSchema)]
struct WriteFiles {
    /// The files to write, in order; a later entry naming an earlier one's
    /// path replaces it.
    #[serde(default)]
    files: Vec<WriteFile>,
    /// The paths of files to remove, each `/`-separated and relative to
    /// `$WORKSPACE`; one the tree lacks is already removed, and one `files`
    /// also names is written, then removed.
    #[serde(default)]
    delete: Vec<String>,
}

// The schema asks for `files` and `delete`; one entry spelled bare at the
// root of the call — `{path, content}`, as a model sends for one file — is
// read as `files` holding it alone, and never beside `files`.
impl<'de> Deserialize<'de> for WriteFiles {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Call;

        impl<'de> Visitor<'de> for Call {
            type Value = WriteFiles;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("`files` and `delete`, or one `{path, content}` entry")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut files = None;
                let mut delete = None;
                let mut bare = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "files" => {
                            if files.is_some() {
                                return Err(de::Error::duplicate_field("files"));
                            }
                            files = Some(map.next_value_seed(Files)?);
                        }
                        "delete" => {
                            if delete.is_some() {
                                return Err(de::Error::duplicate_field("delete"));
                            }
                            delete = Some(map.next_value()?);
                        }
                        "path" | "content" | "shrink" => bare.push((key, map.next_value()?)),
                        other => {
                            return Err(de::Error::unknown_field(
                                other,
                                &["files", "delete", "path", "content", "shrink"],
                            ));
                        }
                    }
                }
                let files = match (files, bare.is_empty()) {
                    (files, true) => files.unwrap_or_default(),
                    (None, false) => vec![single_file(bare)?],
                    (Some(_), false) => {
                        return Err(de::Error::custom(
                            "`files` and a bare `{path, content}` entry cannot both be given; \
                             put every entry under `files`",
                        ));
                    }
                };
                Ok(WriteFiles {
                    files,
                    delete: delete.unwrap_or_default(),
                })
            }
        }

        deserializer.deserialize_map(Call)
    }
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
#[derive(Debug, JsonSchema)]
struct EditFiles {
    /// The edits, in order; a later edit of a file meets it as the earlier
    /// ones left it.
    edits: Vec<Edit>,
}

// The schema asks for `edits`; one edit spelled bare at the root of the
// call — `{path, find, replace}`, as a model sends by analogy with a
// write's one file — is read as `edits` holding it alone, and never beside
// `edits`.
impl<'de> Deserialize<'de> for EditFiles {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Call;

        impl<'de> Visitor<'de> for Call {
            type Value = EditFiles;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("`edits`, or one `{path, find, replace}` edit")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut edits = None;
                let mut bare = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "edits" => {
                            if edits.is_some() {
                                return Err(de::Error::duplicate_field("edits"));
                            }
                            edits = Some(map.next_value_seed(Edits)?);
                        }
                        "path" | "find" | "replace" => bare.push((key, map.next_value()?)),
                        other => {
                            return Err(de::Error::unknown_field(
                                other,
                                &["edits", "path", "find", "replace"],
                            ));
                        }
                    }
                }
                let edits = match (edits, bare.is_empty()) {
                    (Some(edits), true) => edits,
                    (None, true) => return Err(de::Error::missing_field("edits")),
                    (None, false) => vec![single_edit(bare)?],
                    (Some(_), false) => {
                        return Err(de::Error::custom(
                            "`edits` and a bare `{path, find, replace}` edit cannot both be \
                             given; put every edit under `edits`",
                        ));
                    }
                };
                Ok(EditFiles { edits })
            }
        }

        deserializer.deserialize_map(Call)
    }
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
// the order the call spells it; one entry spelled bare — a map with a
// `path` key — is that entry, not files named `path` and `content`; and
// the shape sent as JSON text is the shape.
struct Files;

impl<'de> DeserializeSeed<'de> for Files {
    type Value = Vec<WriteFile>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Files {
    type Value = Vec<WriteFile>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "a list of `{path, content}` entries, one such entry, or a map from path to content",
        )
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut files = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(file) = seq.next_element()? {
            files.push(file);
        }
        Ok(files)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        let entries = entries(map)?;
        if entries.iter().any(|(key, _)| key == "path") {
            return single_file(entries).map(|file| vec![file]);
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

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        spelled(text, self)
    }
}

// The schema asks for a sequence of `{path, find, replace}`; one edit
// spelled bare — a map in place of the list — is that edit; and the shape
// sent as JSON text is the shape.
struct Edits;

impl<'de> DeserializeSeed<'de> for Edits {
    type Value = Vec<Edit>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Edits {
    type Value = Vec<Edit>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a list of `{path, find, replace}` edits, or one such edit")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut edits = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(edit) = seq.next_element()? {
            edits.push(edit);
        }
        Ok(edits)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        single_edit(entries(map)?).map(|edit| vec![edit])
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Self::Value, E> {
        spelled(text, self)
    }
}

// A model that serialises the list before it sends it puts the shape in a
// string; the string is read as the JSON it spells, through the same seed,
// and one that spells no JSON is refused as the seed expects.
fn spelled<'de, S, E>(text: &str, seed: S) -> Result<<S as DeserializeSeed<'de>>::Value, E>
where
    S: DeserializeSeed<'de> + Visitor<'de>,
    E: de::Error,
{
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(err) => {
            return Err(E::custom(format!(
                "a string that is not JSON ({err}), where {} is expected",
                &seed as &dyn de::Expected
            )));
        }
    };
    DeserializeSeed::deserialize(seed, value).map_err(E::custom)
}

// A map's entries in the order the call spells them, which a
// `serde_json::Value` would not keep.
fn entries<'de, A: MapAccess<'de>>(
    mut map: A,
) -> Result<Vec<(String, serde_json::Value)>, A::Error> {
    let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
    while let Some(entry) = map.next_entry()? {
        entries.push(entry);
    }
    Ok(entries)
}

// The one edit a bare `{path, find, replace}` spells, held to those keys as
// the derived reading of the struct would hold a listed one.
fn single_edit<E: de::Error>(entries: Vec<(String, serde_json::Value)>) -> Result<Edit, E> {
    let (mut path, mut find, mut replace) = (None, None, None);
    for (key, value) in entries {
        match key.as_str() {
            "path" => path = Some(text(value, "path")?),
            "find" => find = Some(text(value, "find")?),
            "replace" => replace = Some(text(value, "replace")?),
            other => return Err(E::unknown_field(other, &["path", "find", "replace"])),
        }
    }
    Ok(Edit {
        path: path.ok_or_else(|| E::missing_field("path"))?,
        find: find.ok_or_else(|| E::missing_field("find"))?,
        replace: replace.ok_or_else(|| E::missing_field("replace"))?,
    })
}

// The one entry a bare `{path, content, shrink}` spells, held to those keys
// as the derived reading of the struct would hold a listed one.
fn single_file<E: de::Error>(entries: Vec<(String, serde_json::Value)>) -> Result<WriteFile, E> {
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

// What the file holds nearest a `find` that matched nothing: the line its
// first line is found on and the one after it, where the two usually part,
// or the file's opening line and length when no line holds it, so the
// refusal carries the text to copy.
fn nearest(content: &str, find: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let Some(opening) = lines.first() else {
        return "the file is empty".to_owned();
    };
    let first = find.lines().map(str::trim).find(|line| !line.is_empty());
    let Some(index) = first.and_then(|first| lines.iter().position(|line| line.contains(first)))
    else {
        return if total == 1 {
            format!("no line holds its first line; the file's one line is `{opening}`")
        } else {
            format!(
                "no line holds its first line; the file has {total} lines and opens with \
                 `{opening}`"
            )
        };
    };
    let (line, number) = (lines[index], index + 1);
    let Some(next) = lines.get(index + 1) else {
        return format!("its first line is on line {number} of {total}, `{line}`, the last");
    };
    format!(
        "its first line is on line {number} of {total}, `{line}`, and line {} is `{next}`",
        number + 1
    )
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
