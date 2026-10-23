//! Serves the `write_files`, `edit_files`, and `restore_files` tools for
//! build and verify turns.

mod place;
mod schema;

use std::collections::{BTreeMap, BTreeSet};
use std::future::ready;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::{fmt, fs};

use emery_adapter::target::Report;
use emery_adapter::{Standing, beneath, standing, write_whole};
use omnia_sdk::model::{Function, Tool, ToolCall, ToolFuture, Tools};
use omnia_sdk::{Error, server_error};

use self::place::{Miss, Placed, nearest, place};
use self::schema::{
    Edit, EditFiles, EditResponse, RestoreFiles, RestoreResponse, WriteFile, WriteFiles,
    WriteResponse, json_of,
};
use super::{Guarded, Marker};

const WRITE_FILES: &str = "write_files";
const EDIT_FILES: &str = "edit_files";
const RESTORE_FILES: &str = "restore_files";

// The most dropped tokens a refusal names, so a skeleton wiped whole comes
// back with what to keep and not with its every identifier.
const NAMED_TOKENS: usize = 10;

// The write tools of one turn: the root they write beneath, the turn their
// events name, the files they have written, the files `write_files` may
// not remove or write short of their tokens, and what those files held
// when the turn began.
pub struct Writer {
    root: PathBuf,
    turn: String,
    written: Written,
    guarded: BTreeMap<String, Guard>,
    snapshots: BTreeMap<String, Vec<u8>>,
}

// What one guarded file holds for the tools: the tokens a whole write
// keeps and the markers an edit fills by key.
struct Guard {
    keeps: Vec<String>,
    markers: Vec<Marker>,
}

impl Writer {
    pub fn new(root: &str, turn: &str) -> Self {
        Self {
            root: PathBuf::from(root),
            turn: turn.to_owned(),
            written: Written::default(),
            guarded: BTreeMap::new(),
            snapshots: BTreeMap::new(),
        }
    }

    // The writer guarding `files`, each spelled as the tree does, what the
    // tree holds at each read now for `restore_files`; a path the rule
    // refuses guards nothing, since no call can name it either, and a path
    // the tree lacks is guarded with nothing to restore.
    pub fn guarding(mut self, files: &[Guarded]) -> Result<Self, Error> {
        for guarded in files {
            let Ok(file) = beneath(&guarded.path) else { continue };
            if holds_file(&self.root, &file) {
                let bytes = fs::read(self.root.join(&file)).map_err(|err| {
                    server_error!("reading the scaffolded `{file}` before the turn: {err}")
                })?;
                self.snapshots.insert(file.clone(), bytes);
            }
            self.guarded.insert(
                file,
                Guard {
                    keeps: guarded.keeps.clone(),
                    markers: guarded.markers.clone(),
                },
            );
        }
        Ok(self)
    }

    // The two write tools, and `restore_files` where a file is guarded.
    #[must_use]
    pub fn tools(&self) -> Vec<Tool> {
        let keyed = if self.guarded.values().any(|guard| !guard.markers.is_empty()) {
            " A `find` that is the key of a marker the brief quotes, alone, replaces the block \
             the scaffold laid for that marker, so its lines are never copied."
        } else {
            ""
        };
        let edit = format!(
            "Edit within one or more files beneath `$WORKSPACE`: each edit replaces the one \
             place `find` matches in the file at `path` with `replace`, and the file is \
             otherwise as it was. A `find` matches as written, or with its line breaks and \
             indentation read past outside string literals and comments, so a block quoted \
             in the brief matches where the formatter has re-laid it.{keyed} Every edit is \
             held before any is applied: a `find` matching nowhere or more than once, a path \
             the tree holds no file at, or a path `write_files` would refuse, refuses the \
             call whole and changes nothing. To create or replace a file whole, use \
             `write_files`."
        );
        let mut tools = vec![
            Tool::Function(Function::of::<WriteFiles>(
                WRITE_FILES,
                "Write one or more files beneath `$WORKSPACE`, each created or replaced whole, \
                 the directories above it created, and remove the files `delete` names. Each \
                 path is `/`-separated and relative to `$WORKSPACE`; a call naming a path \
                 outside it, under `.emery/` or `.git/`, naming a projection, or leading \
                 through a symlink, is refused whole and changes nothing, as is one whose \
                 entry would leave a file empty or under half its size without `shrink` set, \
                 one whose entry names a file the brief says was scaffolded before this turn \
                 without every marker and name the skeleton holds, or one deleting such a \
                 file. For a change within a file, use `edit_files`.",
            )),
            Tool::Function(Function::of::<EditFiles>(EDIT_FILES, &edit)),
        ];
        if !self.guarded.is_empty() {
            tools.push(Tool::Function(Function::of::<RestoreFiles>(
                RESTORE_FILES,
                "Put one or more files the brief says were scaffolded before this turn back as \
                 they were scaffolded, each path `/`-separated and relative to `$WORKSPACE`. \
                 This is recovery for a skeleton a refused `write_files` call left wrong: every \
                 edit made to the file this turn is discarded with it and the file is filled \
                 again from its skeleton, so a file whose edits stand is not restored, and it \
                 is never called after `edit_files` or a failed check. A path that was not \
                 scaffolded, or that the tree lacked before this turn, refuses the call whole \
                 and changes nothing.",
            )));
        }
        tools
    }

    #[must_use]
    pub fn written(&self) -> Written {
        self.written.clone()
    }

    // Answers `write_files`, `edit_files`, and `restore_files` here and
    // hands every other call to `rest`.
    pub fn serve(self, mut rest: Tools) -> Tools {
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
                        serde_json::to_string(&WriteResponse {
                            deleted: done.deleted,
                            written: json_of(&done.wrote),
                        })
                        .expect("write_files response serialises")
                    })
                }
                EDIT_FILES => {
                    let outcome = self.edit(&call);
                    tracing::debug!(
                        turn = %self.turn,
                        tool = EDIT_FILES,
                        edited = ?outcome.as_ref().ok().map(|done| done.wrote.as_slice()),
                        placed = ?outcome.as_ref().ok().map(|done| done.placed.as_slice()),
                        error = outcome.as_ref().err().map(String::as_str),
                        "responded"
                    );
                    outcome.map(|done| {
                        serde_json::to_string(&EditResponse {
                            edited: json_of(&done.wrote),
                        })
                        .expect("edit_files response serialises")
                    })
                }
                RESTORE_FILES if !self.guarded.is_empty() => {
                    let outcome = self.restore(&call);
                    tracing::debug!(
                        turn = %self.turn,
                        tool = RESTORE_FILES,
                        restored = ?outcome.as_ref().ok().map(Vec::as_slice),
                        error = outcome.as_ref().err().map(String::as_str),
                        "responded"
                    );
                    outcome.map(|restored| {
                        serde_json::to_string(&RestoreResponse {
                            restored: json_of(&restored),
                        })
                        .expect("restore_files response serialises")
                    })
                }
                _ => return rest(call),
            };
            Box::pin(ready(response))
        })
    }

    // Every path is held to the root and to the tree before the first write,
    // every entry replacing a held file to the shrink rule and to the guard,
    // and every removal to the guard, so a call refused anywhere leaves the
    // tree as it was. What the tree holds at each is read once and moved
    // with every write, so a later entry naming a path meets what the call
    // wrote there rather than what stood before it; a removal naming a path
    // an entry writes is the entry's replacement spelled out, and removes
    // nothing.
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
                    if let Some(why) = self.guards(&file, tree[&file], &content) {
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
                Ok(file) if accepted.iter().any(|(written, _)| *written == file) => {}
                Ok(file) => {
                    if let Some(why) = self.guards_removal(&file) {
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
    // `find` to one place in the file as the call's earlier edits leave it,
    // before any file is written, so a call refused anywhere leaves the tree
    // as it was; a file edited more than once is written once.
    fn edit(&self, call: &ToolCall) -> Result<Edited, String> {
        let EditFiles { edits } = call.arguments().map_err(|err| format!("{EDIT_FILES}: {err}"))?;
        if edits.is_empty() {
            return Err(format!("{EDIT_FILES}: `edits` names no edit"));
        }

        // the edits applied in memory, in order
        let mut tree = BTreeMap::new();
        let mut contents: BTreeMap<String, String> = BTreeMap::new();
        let mut placed = Vec::with_capacity(edits.len());
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
            let markers = self.guarded.get(&file).map_or(&[][..], |guard| guard.markers.as_slice());
            match place(content, &find, markers) {
                Ok((range, step)) => {
                    content.replace_range(range, &replace);
                    placed.push(Placed { file, step });
                }
                Err(Miss::Nothing) => refused.push(format!(
                    "`{path}`: `find` matches nothing in the file; {}",
                    nearest(content, &find)
                )),
                Err(Miss::Several(n)) => {
                    refused.push(format!("`{path}`: `find` matches {n} places in the file"));
                }
                Err(Miss::Filled(key)) => refused.push(format!(
                    "`{path}`: `find` names the marker `{key}`, but the file no longer holds the \
                     block the scaffold laid for it; copy what stands there into `find`"
                )),
            }
        }
        if !refused.is_empty() {
            return Err(format!("{EDIT_FILES}: nothing edited: {}", refused.join("; ")));
        }

        // the edited files written whole
        let mut wrote = Vec::with_capacity(contents.len());
        for (file, content) in contents {
            write_whole(&self.root, &file, Standing::File, content.as_bytes())
                .map_err(|err| format!("{EDIT_FILES}: writing `{file}`: {err}"))?;
            self.written.record(file.clone());
            wrote.push(Wrote {
                file,
                bytes: content.len(),
            });
        }
        Ok(Edited { wrote, placed })
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

    // The guard over a whole write: a file the adapter scaffolded before the
    // turn, and the tree still holds, keeps every token its guard names,
    // since one written short of them loses the markers the adapter laid in
    // it. A path the tree no longer holds is written as any other.
    fn guards(&self, file: &str, held: Standing, content: &str) -> Option<String> {
        let keeps = &self.guarded.get(file)?.keeps;
        if held != Standing::File {
            return None;
        }
        let missing: Vec<&str> = keeps
            .iter()
            .map(String::as_str)
            .filter(|token| !token.is_empty() && !content.contains(token))
            .collect();
        if missing.is_empty() {
            return None;
        }
        let named: Vec<String> =
            missing.iter().take(NAMED_TOKENS).map(|token| format!("`{token}`")).collect();
        let more = missing.len().saturating_sub(NAMED_TOKENS);
        let more = if more == 0 { String::new() } else { format!(", and {more} more") };
        Some(format!(
            "`{file}` was scaffolded before this turn and this content drops what the skeleton \
             holds: {}{more}; change the file through `{EDIT_FILES}`, or send the whole file \
             with those kept",
            named.join(", ")
        ))
    }

    // The guard over a removal: a file the adapter scaffolded before the
    // turn is never deleted, whatever it keeps.
    fn guards_removal(&self, file: &str) -> Option<String> {
        self.guarded.contains_key(file).then(|| {
            format!(
                "`{file}` was scaffolded before this turn and is not deleted; change it through \
                 `{EDIT_FILES}`, which keeps what the skeleton holds"
            )
        })
    }

    // Every path is held to the rule and to a snapshot before any file is
    // written, so a call refused anywhere leaves the tree as it was; a
    // restored file is as the turn found it, so the turn no longer counts
    // as having written it.
    fn restore(&self, call: &ToolCall) -> Result<Vec<Wrote>, String> {
        let RestoreFiles { paths } =
            call.arguments().map_err(|err| format!("{RESTORE_FILES}: {err}"))?;
        if paths.is_empty() {
            return Err(format!("{RESTORE_FILES}: `paths` names no file"));
        }

        // every path to a snapshot
        let mut tree = BTreeMap::new();
        let mut accepted = Vec::with_capacity(paths.len());
        let mut refused = Vec::new();
        for path in paths {
            let file = match self.held(&path, &mut tree) {
                Ok(file) => file,
                Err(why) => {
                    refused.push(why);
                    continue;
                }
            };
            match self.snapshots.get(&file) {
                Some(bytes) => accepted.push((file, bytes)),
                None if self.guarded.contains_key(&file) => refused.push(format!(
                    "`{path}` was scaffolded for this slice but the tree lacked it before this \
                     turn; there is nothing to put back"
                )),
                None => refused.push(format!(
                    "`{path}` was not scaffolded before this turn; `{RESTORE_FILES}` puts back \
                     the files the brief names as scaffolded"
                )),
            }
        }
        if !refused.is_empty() {
            return Err(format!("{RESTORE_FILES}: nothing restored: {}", refused.join("; ")));
        }

        // the files put back
        let mut restored = Vec::with_capacity(accepted.len());
        for (file, bytes) in accepted {
            write_whole(&self.root, &file, tree[&file], bytes)
                .map_err(|err| format!("{RESTORE_FILES}: writing `{file}`: {err}"))?;
            self.written.forget(&file);
            restored.push(Wrote {
                file,
                bytes: bytes.len(),
            });
        }
        Ok(restored)
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

fn holds_file(root: &Path, file: &str) -> bool {
    matches!(standing(root, file), Ok(Standing::File))
}

// What one `write_files` call did: the files written and the files removed.
struct Done {
    wrote: Vec<Wrote>,
    deleted: Vec<String>,
}

// What one `edit_files` call did: the files written, and where each edit
// was placed, in the call's order.
struct Edited {
    wrote: Vec<Wrote>,
    placed: Vec<Placed>,
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
pub struct Written(Arc<Mutex<BTreeSet<String>>>);

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
    pub fn findings(root: &Path, report: &Report) -> Vec<String> {
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
    pub fn fill(&self, root: &Path, report: &mut Report) {
        let mut files: BTreeSet<String> =
            report.written.iter().filter_map(|path| beneath(path).ok()).collect();
        files.extend(self.lock().iter().filter(|file| holds_file(root, file)).cloned());
        report.written = files.into_iter().collect();
    }

    fn lock(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
