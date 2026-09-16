//! Lists a tree adapter's files and asks the model for the surfaces a source exposes.
//!
//! A tree adapter surveys before its first seam is mined, one of two ways.
//! [`list`] walks the files beneath the source root under the adapter's
//! `keep`, for an adapter that cuts its tree itself. [`surfaces`] asks the
//! model once, under the adapter's `prompts/survey.md`, for the surfaces the
//! source exposes — a route, a command, a job, an exported API — each with
//! the module a caller enters it at, which no directory layout states; the
//! entry is held to the tree under the same `keep`, the adapter mines each
//! surface from it, and the model groups nothing.
//!
//! Neither offers nor accepts the engine's own files — `spec.md`,
//! `design.md`, `.omnia/` — so no adapter can mine a projection of the last
//! revision back into evidence.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context as _;
use emery_adapter::source::SourceContent;
use emery_prose::Doc;
use omnia_sdk::model::Question;
use omnia_sdk::{Error, Model, bad_request, server_error};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{Context, path, references};

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

/// Lists the files beneath `root`, sorted, as `/`-separated paths relative to it.
///
/// `keep` is asked about every entry; a refused directory is not entered.
/// The engine's own `.omnia/` directories and `spec.md` / `design.md` files
/// are never offered, wherever they appear. Symlinks are not followed.
///
/// # Examples
///
/// ```
/// use emery_sdk::survey;
///
/// # let scratch = tempfile::tempdir()?;
/// # for file in ["README.md", "api/orders.md", "api/users.md", "notes/todo.md", ".git/HEAD"] {
/// #     let path = scratch.path().join(file);
/// #     std::fs::create_dir_all(path.parent().unwrap())?;
/// #     std::fs::write(path, "")?;
/// # }
/// # let root = scratch.path().to_str().unwrap();
/// let files = survey::list(root, |entry| !entry.hidden())?;
///
/// assert_eq!(files, ["README.md", "api/orders.md", "api/users.md", "notes/todo.md"]);
/// # anyhow::Ok(())
/// ```
///
/// # Errors
///
/// Returns [`Error::ServerError`] when a directory cannot be read, and
/// [`Error::BadRequest`] for an entry whose name is not UTF-8, which no
/// `path` anchor could cite.
pub fn list(root: &str, mut keep: impl FnMut(Entry<'_>) -> bool) -> Result<Vec<String>, Error> {
    let mut files = walk(Path::new(root), "", &mut keep)?;
    files.sort();
    Ok(files)
}

/// Asks the model once for the surfaces the source exposes, each with its entry module.
///
/// The adapter's `prompts/survey.md` among `docs` is the system prompt. The
/// turn names the adapter and source from `ctx` and lends the root so the
/// model can read the tree; the `list_docs` and `read_doc` tools answer from
/// `docs`. The model answers one [`Inventory`], checked whole: a surface
/// without a name, two surfaces of one name, or an entry that is not a
/// regular file beneath the root that `keep` accepts — asked about each
/// directory on the way and the file itself, as [`list`] would ask — goes
/// back as findings for another round. The engine's own files are never an
/// entry.
///
/// The surfaces come back in answer order, each entry as a `/`-separated
/// path relative to the root. A module may be the entry of several
/// surfaces, and a module no surface enters is not one: the model finds the
/// boundary, and the adapter mines what each surface reaches from it. A tree
/// the model finds no surface in exposes nothing, and what that means is the
/// adapter's to decide.
///
/// # Errors
///
/// - [`Error::ServerError`] when `prompts/survey.md` is not embedded, or the
///   input is an inline value, which has no tree to survey — both found
///   before any turn is spent.
/// - [`Error::BadRequest`] when the host refuses the request or the rounds
///   are spent with findings outstanding.
/// - [`Error::BadGateway`] for a tool or transport failure.
pub async fn surfaces<P: Model>(
    model: &P, ctx: &Context<'_>, docs: &'static [Doc],
    mut keep: impl FnMut(Entry<'_>) -> bool + Send,
) -> Result<Vec<Surface>, Error> {
    let key = &ctx.input.key;
    let system = emery_prose::body(docs, "prompts/survey.md")
        .ok_or_else(|| server_error!("`prompts/survey.md` is not embedded"))?;
    let SourceContent::Workspace(root) = &ctx.input.content else {
        return Err(server_error!(
            "`{key}`: a survey by model needs a workspace input, not an inline value"
        ));
    };

    let inventory = Question::<Inventory>::new("survey")
        .system(system)
        .tools(references::tools())
        .workspace(root)
        .ask(model, turn(ctx, root), Some(references::answering(docs)), |answer| {
            let findings = answer.findings(root, &mut keep);
            if findings.is_empty() { Ok(()) } else { Err(findings) }
        })
        .await
        .map_err(Error::from)?;

    Ok(inventory
        .surfaces
        .into_iter()
        .map(|surface| Surface {
            // The check accepted the entry, so it is a path beneath the root.
            entry: path::beneath(&surface.entry).unwrap_or(surface.entry),
            name: surface.name,
        })
        .collect())
}

/// The model's survey answer: the surfaces the source exposes.
///
/// A survey prompt's worked example must parse as this shape. An empty
/// inventory is a valid answer — the model found no surface — and what it
/// means is the adapter's to decide. A surface without a name, two surfaces
/// of one name, or an entry that is not a module of the tree is a finding.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery survey answer")]
pub struct Inventory {
    /// The surfaces, in the order the seams will be mined.
    pub surfaces: Vec<Surface>,
}

/// One surface a source exposes: what a caller outside it reaches, and where.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// What the surface is, as the prompt asked it to be named.
    pub name: String,
    /// The module a caller enters the surface at, as a path relative to the root.
    pub entry: String,
}

impl Inventory {
    // What the check holds against a candidate: every surface named once, and
    // entered at a module of the tree that `keep` accepts.
    fn findings(&self, root: &str, keep: &mut impl FnMut(Entry<'_>) -> bool) -> Vec<String> {
        let mut findings = Vec::new();
        let mut names = BTreeSet::new();
        for surface in &self.surfaces {
            if surface.name.trim().is_empty() {
                findings.push(format!("the surface entered at `{}` has no name", surface.entry));
            } else if !names.insert(surface.name.as_str()) {
                findings.push(format!("surface `{}` is listed twice", surface.name));
            }
            if let Err(finding) = module(root, &surface.entry, keep) {
                findings.push(finding);
            }
        }
        findings
    }
}

// `named` as a path beneath `root` when it is a regular file there that
// `keep` accepts — asked about each directory on the way and the file itself,
// as the walk would offer them — and none of the engine's own; otherwise the
// finding.
fn module(
    root: &str, named: &str, keep: &mut impl FnMut(Entry<'_>) -> bool,
) -> Result<String, String> {
    let entry = path::beneath(named).map_err(|reason| format!("`{named}` {reason}"))?;

    let regular = std::fs::symlink_metadata(Path::new(root).join(&entry))
        .is_ok_and(|metadata| metadata.is_file());
    if !regular {
        return Err(format!("no file at `{named}`"));
    }

    let refused = || format!("`{named}` is not a module this adapter mines");
    for (index, _) in entry.match_indices('/') {
        let dir = Entry::Dir(&entry[..index]);
        if SKIP_DIRS.contains(&dir.name()) || !keep(dir) {
            return Err(refused());
        }
    }
    let file = Entry::File(&entry);
    if SKIP_FILES.contains(&file.name()) || !keep(file) {
        return Err(refused());
    }

    Ok(entry)
}

// The survey turn: which source is being surveyed, the root lent, how an
// entry is named, and where the model's work stops.
fn turn(ctx: &Context<'_>, root: &str) -> String {
    format!(
        "Survey the source bound to adapter `{id}` (source key `{key}`) before it is mined.\n\n\
         `$SOURCE_DIR` is the read-only view at `{root}` — the source tree. List the surfaces it \
         exposes as the prompt describes them, each named for what a caller outside the source \
         reaches, with the module the caller enters it at. Name an entry as a `/`-separated path \
         relative to `$SOURCE_DIR`, to a module of the kind the prompt says this adapter mines; \
         a module may be the entry of several surfaces, and a module no surface enters is not \
         named.\n\n\
         Read under `$SOURCE_DIR` to decide; nothing outside it is reachable. The caller mines \
         each surface from its entry, following what it reaches through the whole tree — you \
         follow nothing and group nothing. When the tree declares no surface, answer none rather \
         than inventing one.\n\n\
         The prompt's references are available through this call's `read_doc` tool (`list_docs` \
         enumerates them); load referenced bodies on demand.\n\n\
         Answer with one JSON object matching the survey schema. The caller mines the surfaces; \
         extract nothing yourself.",
        id = ctx.adapter_id,
        key = ctx.input.key,
    )
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
