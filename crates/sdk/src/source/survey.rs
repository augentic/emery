//! Lists a tree adapter's files and cuts them into materials.
//!
//! A tree adapter surveys before its first material is mined: [`files`] lists
//! the files beneath the root, and one of two cuts groups them.
//! [`by_directory`] is mechanical — one group per top-level directory.
//! [`by_model`] asks the model once, under the adapter's `prompts/survey.md`,
//! to group the files by what they serve — a route, a command, an exported
//! API — which no directory layout states.
//!
//! Both cuts fold under a grain floor: a group too small to be worth its own
//! model call joins one remainder, with every file no group claims, so the
//! materials cover the tree whole however it was cut. The walk never offers
//! the engine's own files — `spec.md`, `design.md`, `.omnia/` — so no adapter
//! can mine a projection of the last revision back into evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::Context as _;
use emery_adapter::source::SourceContent;
use emery_prose::registry::{self, Doc};
use omnia_guest::model::Question;
use omnia_guest::{Error, Model, bad_request, server_error};
use schemars::JsonSchema;
use serde::Deserialize;

use super::Context;
use crate::references;

/// A directory entry the walk offers to an adapter's `keep`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Entry {
    /// A directory; refusing it prunes everything beneath.
    Dir,
    /// A regular file; refusing it leaves it out of the survey.
    File,
}

/// Returns the files beneath `root`, sorted, as `/`-separated paths relative to it.
///
/// `keep` is asked about every entry with its root-relative path and kind; a
/// refused directory is not entered. The engine's own `.omnia/` directories
/// and `spec.md` / `design.md` files are never offered, wherever they appear.
/// Symlinks are not followed.
///
/// # Errors
///
/// Returns [`Error::ServerError`] when a directory cannot be read, and
/// [`Error::BadRequest`] for an entry whose name is not UTF-8, which no `path`
/// anchor could cite.
pub fn files(
    root: &Path, mut keep: impl FnMut(&Path, Entry) -> bool,
) -> Result<Vec<String>, Error> {
    let mut found = walk(root, "", &mut keep)?;
    found.sort();
    Ok(found)
}

/// Groups `files` by top-level directory, folding directories under `floor`.
///
/// Each directory holding at least `floor` files is one group, in directory
/// order. The root's own files and every smaller directory's fold into one
/// sorted remainder, last; an empty remainder is dropped. Files keep their
/// order within a group, so sorted input yields sorted groups.
///
/// # Examples
///
/// ```
/// use emery_sdk::survey::by_directory;
///
/// let files =
///     ["README.md", "api/orders.md", "api/users.md", "notes/todo.md"].map(String::from).to_vec();
///
/// let groups = by_directory(files, 2);
/// assert_eq!(groups, [vec!["api/orders.md", "api/users.md"], vec!["README.md", "notes/todo.md"]]);
/// ```
#[must_use]
pub fn by_directory(files: Vec<String>, floor: usize) -> Vec<Vec<String>> {
    let mut directories: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut remainder = Vec::new();
    for file in files {
        match file.split_once('/') {
            Some((directory, _)) => directories.entry(directory.to_owned()).or_default().push(file),
            None => remainder.push(file),
        }
    }

    let mut groups = Vec::with_capacity(directories.len() + 1);
    for group in directories.into_values() {
        if group.len() >= floor {
            groups.push(group);
        } else {
            remainder.extend(group);
        }
    }
    if !remainder.is_empty() {
        remainder.sort();
        groups.push(remainder);
    }
    groups
}

/// Groups `files` by asking the model once how they serve the source.
///
/// The adapter's `prompts/survey.md` among `docs` is the system prompt. The
/// turn names the adapter and source, lists the candidate files, and lends the
/// input's root so the model can read them; the `list_docs` and `read_doc`
/// tools answer from `docs`. The model answers one [`Partition`], checked
/// whole: a group naming no file, a file not among `files`, or a file in two
/// groups goes back as findings for another round.
///
/// The accepted groups come back in answer order, each sorted. A group of
/// fewer than `floor` files folds, with every file the model left out, into
/// one sorted remainder, last; an empty remainder is dropped. The model
/// chooses the grouping, never omission, so coverage is total. A tree with no
/// files spends no turn and cuts into nothing.
///
/// # Errors
///
/// - [`Error::ServerError`] when `prompts/survey.md` is not embedded or the
///   input is an inline value; both are found before any turn is spent.
/// - [`Error::BadRequest`] when the host refuses the request or the rounds
///   are spent with findings outstanding.
/// - [`Error::BadGateway`] for a tool or transport failure.
pub async fn by_model<P: Model>(
    model: &P, ctx: &Context<'_>, docs: &'static [Doc], files: &[String], floor: usize,
) -> Result<Vec<Vec<String>>, Error> {
    let key = &ctx.input.key;
    let system = registry::body(docs, "prompts/survey.md")
        .ok_or_else(|| server_error!("`prompts/survey.md` is not embedded"))?;
    let SourceContent::Workspace(root) = &ctx.input.content else {
        return Err(server_error!(
            "`{key}`: a model survey needs a workspace input, not an inline value"
        ));
    };
    if files.is_empty() {
        return Ok(Vec::new());
    }

    let partition = Question::<Partition>::new("survey")
        .system(system)
        .tools(references::tools())
        .workspace(root)
        .ask(model, turn(ctx, root, files, floor), Some(references::answering(docs)), |answer| {
            let findings = answer.findings(files);
            if findings.is_empty() { Ok(()) } else { Err(findings) }
        })
        .await
        .map_err(Error::from)?;

    Ok(partition.fold(files, floor))
}

/// The model's survey answer: the candidate files partitioned into groups.
///
/// A survey prompt's worked example must parse as this shape. Leaving a file
/// out of every group is allowed — it joins the remainder. Naming a file that
/// was never offered, naming one twice, or a group naming no file is a
/// finding.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery survey answer")]
pub struct Partition {
    /// The groups, in the order the materials will be mined.
    pub groups: Vec<Group>,
}

/// One group of a [`Partition`]: the files that serve one thing together.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// What the group serves, as the prompt asked it to be named.
    pub name: String,
    /// The files, each exactly as it was offered.
    pub files: Vec<String>,
}

impl Partition {
    // What the check holds against a candidate: every file named must be one
    // offered, once; every group must name at least one.
    fn findings(&self, offered: &[String]) -> Vec<String> {
        let mut findings = Vec::new();
        let mut seen = BTreeSet::new();
        for group in &self.groups {
            if group.files.is_empty() {
                findings.push(format!("group `{}` names no file", group.name));
            }
            for file in &group.files {
                if !offered.contains(file) {
                    findings.push(format!("`{file}` is not among the files offered"));
                } else if !seen.insert(file.as_str()) {
                    findings.push(format!("`{file}` appears in more than one group"));
                }
            }
        }
        findings
    }

    // The accepted groups under `floor`: each large enough stands, sorted;
    // the rest and every file no group claims are one sorted remainder.
    fn fold(self, offered: &[String], floor: usize) -> Vec<Vec<String>> {
        let mut assigned = BTreeSet::new();
        let mut groups = Vec::with_capacity(self.groups.len() + 1);
        let mut remainder = Vec::new();
        for group in self.groups {
            assigned.extend(group.files.iter().cloned());
            if group.files.len() >= floor {
                let mut files = group.files;
                files.sort();
                groups.push(files);
            } else {
                remainder.extend(group.files);
            }
        }
        remainder.extend(offered.iter().filter(|file| !assigned.contains(*file)).cloned());
        if !remainder.is_empty() {
            remainder.sort();
            groups.push(remainder);
        }
        groups
    }
}

// The survey turn: which source is being surveyed, the root lent, the
// candidate files as the model must name them, and what the floor does with
// what it leaves out.
fn turn(ctx: &Context<'_>, root: &str, files: &[String], floor: usize) -> String {
    let mut turn = format!(
        "Survey the source bound to adapter `{id}` (source key `{key}`) before it is mined.\n\n\
         `$SOURCE_DIR` is the read-only view at `{root}` — the source tree. Partition these files \
         beneath it into the groups the prompt describes, each named for what it serves; name \
         every file exactly as listed, in one group at most:",
        id = ctx.adapter_id,
        key = ctx.input.key,
    );
    for file in files {
        // Writing to a `String` cannot fail.
        let _ = write!(turn, "\n- `{file}`");
    }
    // Writing to a `String` cannot fail.
    let _ = write!(
        turn,
        "\n\nA group of fewer than {floor} files, and every file you leave out, join one remainder \
         the caller mines together — so leave out what serves no group rather than forcing it \
         into one. Read under `$SOURCE_DIR` to decide; nothing outside it is reachable.\n\n\
         The prompt's references are available through this call's `read_doc` tool (`list_docs` \
         enumerates them); load referenced bodies on demand.\n\n\
         Answer with one JSON object matching the survey schema. The caller mines the groups; \
         extract nothing yourself."
    );
    turn
}

// The engine's own files: output, never input, wherever they sit in a tree.
const SKIP_DIRS: &[&str] = &[".omnia"];
const SKIP_FILES: &[&str] = &["spec.md", "design.md"];

// `dir`'s kept files as `prefix`-relative paths, descending into each kept
// directory.
fn walk(
    dir: &Path, prefix: &str, keep: &mut impl FnMut(&Path, Entry) -> bool,
) -> Result<Vec<String>, Error> {
    let reading = || format!("reading `{}`", dir.display());
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(reading)? {
        let entry = entry.with_context(reading)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(bad_request!(
                "`{}` is not UTF-8; no `path` anchor could cite it",
                entry.path().display()
            ));
        };
        let file_type = entry.file_type().with_context(reading)?;
        let relative = if prefix.is_empty() { name.to_owned() } else { format!("{prefix}/{name}") };

        if file_type.is_dir() {
            if SKIP_DIRS.contains(&name) || !keep(Path::new(&relative), Entry::Dir) {
                continue;
            }
            found.extend(walk(&entry.path(), &relative, keep)?);
        } else if file_type.is_file()
            && !SKIP_FILES.contains(&name)
            && keep(Path::new(&relative), Entry::File)
        {
            found.push(relative);
        }
    }
    Ok(found)
}
