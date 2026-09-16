//! Lists a tree adapter's files and cuts them into seams.
//!
//! A tree adapter surveys before its first seam is mined: [`Tree::list`]
//! walks the files beneath the source root, and one of two cuts groups them.
//! [`Tree::by_directory`] is mechanical — one group per top-level directory.
//! [`Tree::by_model`] asks the model once, under the adapter's
//! `prompts/survey.md`, to group the files by what they serve — a route, a
//! command, an exported API — which no directory layout states.
//!
//! Both cuts fold under a grain floor: a group too small to be worth its own
//! model call joins one remainder, with every file no group claims, so the
//! seams cover the tree whole however it was cut. The walk never offers
//! the engine's own files — `spec.md`, `design.md`, `.omnia/` — so no adapter
//! can mine a projection of the last revision back into evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::Context as _;
use emery_prose::Doc;
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, bad_request, server_error};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{Context, references};

/// A directory entry the walk offers to an adapter's `keep`, by its root-relative path.
///
/// The path is `/`-separated, as the listing carries it, and UTF-8: an
/// entry whose name is not is refused before any is offered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Entry<'a> {
    /// A directory; refusing it prunes everything beneath.
    Dir(&'a str),
    /// A regular file; refusing it leaves it out of the survey.
    File(&'a str),
}

impl<'a> Entry<'a> {
    /// Returns the `/`-separated path relative to the root, as the listing carries it.
    #[must_use]
    pub const fn path(self) -> &'a str {
        match self {
            Self::Dir(path) | Self::File(path) => path,
        }
    }

    /// Returns the entry's own name: the last segment of its path.
    #[must_use]
    pub fn name(self) -> &'a str {
        let path = self.path();
        path.rsplit_once('/').map_or(path, |(_, name)| name)
    }

    /// Returns the part of the name after its last dot; a leading dot is not one.
    #[must_use]
    pub fn extension(self) -> Option<&'a str> {
        let (stem, extension) = self.name().rsplit_once('.')?;
        (!stem.is_empty()).then_some(extension)
    }

    /// Returns `true` when the name begins with a dot: tooling by convention, never source.
    #[must_use]
    pub fn hidden(self) -> bool {
        self.name().starts_with('.')
    }
}

/// The files beneath one source root, sorted, named relative to it.
///
/// A tree is listed once, by [`Tree::list`], and cut by [`Tree::by_directory`]
/// or [`Tree::by_model`]. The root it was listed under travels with the files,
/// so a cut by model lends the directory the files are relative to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tree<'a> {
    root: &'a str,
    files: Vec<String>,
}

impl<'a> Tree<'a> {
    /// Lists the files beneath `root`, sorted, as `/`-separated paths relative to it.
    ///
    /// `keep` is asked about every entry; a refused directory is not
    /// entered. The engine's own `.omnia/` directories and `spec.md` /
    /// `design.md` files are never offered, wherever they appear. Symlinks are
    /// not followed.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_sdk::survey::Tree;
    ///
    /// # let scratch = tempfile::tempdir()?;
    /// # for file in ["README.md", "api/orders.md", "api/users.md", "notes/todo.md", ".git/HEAD"] {
    /// #     let path = scratch.path().join(file);
    /// #     std::fs::create_dir_all(path.parent().unwrap())?;
    /// #     std::fs::write(path, "")?;
    /// # }
    /// # let root = scratch.path().to_str().unwrap();
    /// let tree = Tree::list(root, |entry| !entry.hidden())?;
    ///
    /// assert_eq!(tree.files(), ["README.md", "api/orders.md", "api/users.md", "notes/todo.md"]);
    /// assert_eq!(
    ///     tree.by_directory(2),
    ///     [vec!["api/orders.md", "api/users.md"], vec!["README.md", "notes/todo.md"]]
    /// );
    /// # anyhow::Ok(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when a directory cannot be read, and
    /// [`Error::BadRequest`] for an entry whose name is not UTF-8, which no
    /// `path` anchor could cite.
    pub fn list(root: &'a str, mut keep: impl FnMut(Entry<'_>) -> bool) -> Result<Self, Error> {
        let mut files = walk(Path::new(root), "", &mut keep)?;
        files.sort();
        Ok(Self { root, files })
    }

    /// Returns the root the files are relative to, as the engine lent it.
    #[must_use]
    pub const fn root(&self) -> &'a str {
        self.root
    }

    /// Returns the files, sorted, as `/`-separated paths relative to the root.
    #[must_use]
    pub fn files(&self) -> &[String] {
        &self.files
    }

    /// Groups the files by top-level directory, folding directories under `floor`.
    ///
    /// Each directory holding at least `floor` files is one group, in
    /// directory order. The root's own files and every smaller directory's
    /// fold into one sorted remainder, last; an empty remainder is dropped.
    /// Files keep their order within a group, so the groups are sorted.
    #[must_use]
    pub fn by_directory(&self, floor: usize) -> Vec<Vec<String>> {
        let mut directories: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        let mut remainder = Vec::new();
        for file in &self.files {
            match file.split_once('/') {
                Some((directory, _)) => {
                    directories.entry(directory).or_default().push(file.clone());
                }
                None => remainder.push(file.clone()),
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

    /// Groups the files by asking the model once how they serve the source.
    ///
    /// The adapter's `prompts/survey.md` among `docs` is the system prompt.
    /// The turn names the adapter and source from `ctx`, lists the files, and
    /// lends the root so the model can read them; the `list_docs` and
    /// `read_doc` tools answer from `docs`. The model answers one
    /// [`Partition`], checked whole: a group naming no file, a file not among
    /// the tree's, or a file in two groups goes back as findings for another
    /// round.
    ///
    /// The accepted groups come back in answer order, each sorted. A group of
    /// fewer than `floor` files folds, with every file the model left out,
    /// into one sorted remainder, last; an empty remainder is dropped. The
    /// model chooses the grouping, never omission, so coverage is total. A
    /// tree with no files spends no turn and cuts into nothing.
    ///
    /// # Errors
    ///
    /// - [`Error::ServerError`] when `prompts/survey.md` is not embedded,
    ///   found before any turn is spent.
    /// - [`Error::BadRequest`] when the host refuses the request or the rounds
    ///   are spent with findings outstanding.
    /// - [`Error::BadGateway`] for a tool or transport failure.
    pub async fn by_model<P: Model>(
        &self, model: &P, ctx: &Context<'_>, docs: &'static [Doc], floor: usize,
    ) -> Result<Vec<Vec<String>>, Error> {
        let system = emery_prose::body(docs, "prompts/survey.md")
            .ok_or_else(|| server_error!("`prompts/survey.md` is not embedded"))?;
        if self.files.is_empty() {
            return Ok(Vec::new());
        }

        let files = &self.files;
        let partition = Question::<Partition>::new("survey")
            .system(system)
            .tools(references::tools())
            .workspace(self.root)
            .ask(model, turn(ctx, self, floor), Some(references::answering(docs)), |answer| {
                let findings = answer.findings(files);
                if findings.is_empty() { Ok(()) } else { Err(findings) }
            })
            .await
            .map_err(Error::from)?;

        Ok(partition.fold(files, floor))
    }
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
    /// The groups, in the order the seams will be mined.
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
fn turn(ctx: &Context<'_>, tree: &Tree<'_>, floor: usize) -> String {
    let mut turn = format!(
        "Survey the source bound to adapter `{id}` (source key `{key}`) before it is mined.\n\n\
         `$SOURCE_DIR` is the read-only view at `{root}` — the source tree. Partition these files \
         beneath it into the groups the prompt describes, each named for what it serves; name \
         every file exactly as listed, in one group at most:",
        id = ctx.adapter_id,
        key = ctx.input.key,
        root = tree.root,
    );
    for file in &tree.files {
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
    dir: &Path, prefix: &str, keep: &mut impl FnMut(Entry<'_>) -> bool,
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
            if SKIP_DIRS.contains(&name) || !keep(Entry::Dir(&relative)) {
                continue;
            }
            found.extend(walk(&entry.path(), &relative, keep)?);
        } else if file_type.is_file() && !SKIP_FILES.contains(&name) && keep(Entry::File(&relative))
        {
            found.push(relative);
        }
    }
    Ok(found)
}
