//! Version-control paths, repositories, working copies, and commit trailers.
//!
//! Opens [`omnia_sdk::vcs::Workspace`] handles over deployment-local paths
//! beneath [`ROOT`]: [`SourceClone`] manages cached source clones, [`BuildRepo`]
//! resolves the target repository a specification lives in and a build lands in,
//! [`WorkingCopy`] manages isolated worktrees, and [`Message`] and
//! [`RevisionMessage`] format and parse the trailers of a slice's commit and
//! a revision's. Emery's vocabulary stays "label"; at the omnia boundary that
//! is a bookmark.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use emery_adapter::SPECIFICATION;
use emery_adapter::target::{MergeRule, MergeStrategy};
use emery_adapter::{SKIP_DIRS, is_kebab};
use omnia_sdk::vcs::{self, Change, Rule, Strategy, Unmerged, Workspace};
use omnia_sdk::{Error, ErrorKind, bad_gateway, bad_request, server_error};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::revision::{Amendment, ChangeClass, PartChange, PartClass, PartId, ReqId, SliceId};

/// The directory beneath the project root that holds every clone and working copy.
pub const ROOT: &str = ".emery/vcs";

/// The working copy a build integrates its slices in.
pub const INTEGRATION: &str = "./.emery/vcs/integration";

/// The working copy a revision is read from and sealed in.
pub const REVISION: &str = "./.emery/vcs/revision";

/// The directory beneath the project root that holds the working copy each slice builds in.
pub const WORKTREES: &str = "./.emery/vcs/worktrees";

/// The remote a clone fetches from: the URL it was cloned from.
pub const ORIGIN: &str = "origin";

/// The commit a build into a cloned repository starts from: the origin's default branch, as the last fetch left it.
pub const DEFAULT_BRANCH: &str = "origin/HEAD";

/// The message of the one commit the engine creates a target repository with.
pub const INITIALISE: &str = "Initialise";

/// The project's own checkout, as the capability names it and as a target's `repository` names it to build into.
pub const PROJECT: &str = ".";

/// The working copy a repository source is read from, by source name.
#[must_use]
pub fn source_checkout(name: &str) -> String {
    format!("./{ROOT}/sources/{name}")
}

/// The repository a build lands in when the target is not the project, by target name.
#[must_use]
pub fn target_path(name: &str) -> String {
    format!("./{ROOT}/repos/{name}")
}

/// The working copy a slice builds in, by slice id.
#[must_use]
pub fn slice_worktree(slice: SliceId) -> String {
    format!("{WORKTREES}/{slice}")
}

/// Maps the merge rules a target adapter declares onto the capability's.
#[must_use]
pub fn rules(declared: &[MergeRule]) -> Vec<Rule> {
    declared
        .iter()
        .map(|rule| Rule {
            pattern: rule.paths.to_string(),
            strategy: match rule.strategy {
                MergeStrategy::Union => Strategy::Union,
                MergeStrategy::Ours => Strategy::Ours,
                MergeStrategy::Theirs => Strategy::Theirs,
            },
        })
        .collect()
}

/// The strategy the first of `rules` matching `path` declares, in the capability's reading.
///
/// A pattern with no `/` matches the path's final component at any depth;
/// one with a `/` is anchored to the root, a leading `/` dropped. `?` and
/// `*` stay within a component, `**/` crosses whole components, and any
/// other `**` crosses anything. `None` when no rule matches.
#[must_use]
pub fn strategy_for(rules: &[Rule], path: &str) -> Option<Strategy> {
    rules.iter().find(|rule| matches(&rule.pattern, path)).map(|rule| rule.strategy)
}

/// What a merge sealed: the commit it reached, or the paths it left conflicted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    /// The commit the working copy reached when the merge sealed.
    pub commit: Option<String>,
    /// Paths the merge left in conflict when [`commit`] is `None`.
    pub conflicts: Vec<String>,
}

fn is_not_found(error: &Error) -> bool {
    matches!(error.kind(), ErrorKind::NotFound) || error.code() == "not-found"
}

async fn workspace_at(at: &str, subject: Subject<'_>) -> Result<Workspace, Error> {
    vcs::open(at).await.map_err(|error| subject.refusal(error))
}

fn matches(pattern: &str, path: &str) -> bool {
    if pattern.contains('/') {
        glob(pattern.strip_prefix('/').unwrap_or(pattern).as_bytes(), path.as_bytes())
    } else {
        glob(pattern.as_bytes(), path.rsplit('/').next().unwrap_or(path).as_bytes())
    }
}

// Whether `pattern` matches `text` whole; the patterns are an adapter's
// few short globs, so backtracking costs nothing worth a table.
fn glob(pattern: &[u8], text: &[u8]) -> bool {
    match pattern {
        [] => text.is_empty(),
        [b'*', b'*', b'/', rest @ ..] => {
            glob(rest, text)
                || text
                    .iter()
                    .enumerate()
                    .filter(|&(_, &byte)| byte == b'/')
                    .any(|(at, _)| glob(rest, &text[at + 1..]))
        }
        [b'*', b'*', rest @ ..] => (0..=text.len()).any(|at| glob(rest, &text[at..])),
        [b'*', rest @ ..] => {
            let component = text.iter().position(|&byte| byte == b'/').unwrap_or(text.len());
            (0..=component).any(|at| glob(rest, &text[at..]))
        }
        [b'?', rest @ ..] => match text {
            [byte, after @ ..] if *byte != b'/' => glob(rest, after),
            _ => false,
        },
        [want, rest @ ..] => match text {
            [byte, after @ ..] if byte == want => glob(rest, after),
            _ => false,
        },
    }
}

/// The message a build seals one slice under, in its working copy and as its merge.
///
/// The first line is `<id> <name>`; the trailers tie the commit to its
/// revision, so a later run reads back from the labelled history which
/// slices and requirements it holds. [`Display`](fmt::Display) writes it and
/// [`parse`](Message::parse) reads it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The slice sealed.
    pub slice: SliceId,
    /// The part the slice was cut from.
    pub part: PartId,
    /// The slice's name.
    pub name: String,
    /// The revision whose plan the slice is of.
    pub revision: String,
    /// The requirement budget the slice was cut under.
    pub budget: usize,
    /// The slice's requirements.
    pub requirements: Vec<ReqId>,
    /// The requirements the build reported covered.
    pub covered: Vec<ReqId>,
    /// The adapter that built it, as the run named it.
    pub adapter: String,
    /// The integrated head the slice was built over.
    pub base: String,
    /// The wave the slice was built in, from one.
    pub wave: usize,
}

impl Message {
    /// Reads a message back from a commit's text; `None` when a build did not write it.
    ///
    /// Every trailer must be present and well formed, so what a run's own
    /// commits say is read and anything else in the history is passed over.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let (id, name) = lines.next()?.split_once(' ')?;
        let slice: SliceId = id.parse().ok()?;
        let trailers: BTreeMap<&str, &str> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| (key, value.trim()))
            .collect();
        let ids = |key: &str| -> Option<Vec<ReqId>> {
            let listed = trailers.get(key)?;
            if listed.is_empty() {
                return Some(Vec::new());
            }
            listed.split(", ").map(|id| id.parse().ok()).collect()
        };
        if trailers.get("Slice")?.parse::<SliceId>().ok()? != slice {
            return None;
        }
        Some(Self {
            slice,
            part: trailers.get("Part")?.parse().ok()?,
            name: name.trim().to_owned(),
            revision: (*trailers.get("Revision")?).to_owned(),
            budget: trailers.get("Budget")?.parse().ok()?,
            requirements: ids("Requirements")?,
            covered: ids("Covered")?,
            adapter: (*trailers.get("Adapter")?).to_owned(),
            base: (*trailers.get("Base")?).to_owned(),
            wave: trailers.get("Wave")?.parse().ok()?,
        })
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let list =
            |ids: &[ReqId]| ids.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ");
        write!(
            f,
            "{slice} {name}\n\nSlice: {slice}\nPart: {part}\nRevision: {revision}\nBudget: \
             {budget}\nRequirements: {requirements}\nCovered: {covered}\nAdapter: {adapter}\nBase: \
             {base}\nWave: {wave}",
            slice = self.slice,
            part = self.part,
            name = self.name,
            revision = self.revision,
            budget = self.budget,
            requirements = list(&self.requirements),
            covered = list(&self.covered),
            adapter = self.adapter,
            base = self.base,
            wave = self.wave,
        )
    }
}

/// The message a revision is sealed under in the repository its specification lives in.
///
/// The first line is `Revision <id>`; the trailers tie the commit to the
/// revision, the revision it amends, the commit it was sealed over, and,
/// for a sealed amendment, what it changed against its parent, so a later
/// run reads back from the history which revision a tree holds.
/// [`Display`](fmt::Display) writes it and [`parse`](RevisionMessage::parse)
/// reads it back. [`Message::parse`] passes it over, since its first word
/// is no slice id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionMessage {
    /// The revision sealed.
    pub revision: String,
    /// The revision it amends, by id; `None` for one drafted from the sources alone.
    pub parent: Option<String>,
    /// The commit the revision was sealed over; `None` for a repository's root commit.
    pub base: Option<String>,
    /// What the revision changed against its parent; `None` unless it was
    /// sealed from an amendment.
    pub summary: Option<Summary>,
}

impl RevisionMessage {
    /// Reads a message back from a commit's text; `None` when a seal did not write it.
    ///
    /// The first line and the `Revision:` trailer must agree; `Parent:` and
    /// `Base:` are read where present, and the summary trailers where any of
    /// them is, each then well formed, so a commit of any other shape in the
    /// history is passed over. A trailer of any other name is.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let revision = lines.next()?.strip_prefix("Revision ")?.trim();
        if revision.is_empty() {
            return None;
        }
        let trailers: BTreeMap<&str, &str> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| (key, value.trim()))
            .collect();
        if *trailers.get("Revision")? != revision {
            return None;
        }
        let summary = if Summary::KEYS.iter().any(|key| trailers.contains_key(key)) {
            Some(Summary::parse(&trailers)?)
        } else {
            None
        };
        Some(Self {
            revision: revision.to_owned(),
            parent: trailers
                .get("Parent")
                .copied()
                .filter(|parent| !parent.is_empty())
                .map(str::to_owned),
            base: trailers.get("Base").copied().filter(|base| !base.is_empty()).map(str::to_owned),
            summary,
        })
    }
}

impl fmt::Display for RevisionMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Revision {revision}\n\nRevision: {revision}", revision = self.revision)?;
        if let Some(parent) = &self.parent {
            write!(f, "\nParent: {parent}")?;
        }
        if let Some(base) = &self.base {
            write!(f, "\nBase: {base}")?;
        }
        if let Some(summary) = &self.summary {
            write!(f, "\n{summary}")?;
        }
        Ok(())
    }
}

/// What a sealed amendment changed against the revision it amends, as its commit's trailers carry it.
///
/// `Added:`, `Changed:`, and `Retired:` count the requirements under each
/// class; `Parts:` lists the parts the plan adds, changes, or removes as
/// `PART-002 orders changed`, and is left out when there are none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Requirements the parent does not hold, stated by the change.
    pub added: usize,
    /// Requirements of the parent restated by the change, their ids kept.
    pub changed: usize,
    /// Requirements of the parent the change withdraws.
    pub retired: usize,
    /// The parts the plan adds, changes, or removes against the parent's, in id order.
    pub parts: Vec<PartSummary>,
}

impl Summary {
    const KEYS: [&str; 4] = ["Added", "Changed", "Retired", "Parts"];

    /// Summarises what `amendment` changes against its base.
    #[must_use]
    pub fn of(amendment: &Amendment) -> Self {
        Self {
            added: amendment.count(ChangeClass::Added),
            changed: amendment.count(ChangeClass::Changed),
            retired: amendment.count(ChangeClass::Retired),
            parts: amendment.parts.iter().map(PartSummary::from).collect(),
        }
    }

    // The summary the trailers carry; `None` when a count is missing or
    // malformed, or a part entry is not `<id> <name> <class>`.
    fn parse(trailers: &BTreeMap<&str, &str>) -> Option<Self> {
        let count = |key: &str| trailers.get(key)?.parse::<usize>().ok();
        let parts = match trailers.get("Parts") {
            Some(listed) => listed
                .split(", ")
                .map(|entry| {
                    let mut words = entry.split_whitespace();
                    let (id, name, class) = (words.next()?, words.next()?, words.next()?);
                    if words.next().is_some() || !is_kebab(name) {
                        return None;
                    }
                    Some(PartSummary {
                        id: id.parse().ok()?,
                        name: name.to_owned(),
                        class: class.parse().ok()?,
                    })
                })
                .collect::<Option<Vec<_>>>()?,
            None => Vec::new(),
        };
        Some(Self {
            added: count("Added")?,
            changed: count("Changed")?,
            retired: count("Retired")?,
            parts,
        })
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Added: {}\nChanged: {}\nRetired: {}", self.added, self.changed, self.retired)?;
        if !self.parts.is_empty() {
            let listed: Vec<String> = self.parts.iter().map(ToString::to_string).collect();
            write!(f, "\nParts: {}", listed.join(", "))?;
        }
        Ok(())
    }
}

/// One part of a [`Summary`], as the `Parts:` trailer names it.
///
/// A [`PartChange`] less whose brief the part carries, which the
/// amendment's review reads and the history keeps nothing of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartSummary {
    /// The part's identifier.
    pub id: PartId,
    /// The part's name.
    pub name: String,
    /// What the amendment did to the part.
    pub class: PartClass,
}

impl From<&PartChange> for PartSummary {
    fn from(change: &PartChange) -> Self {
        Self {
            id: change.id,
            name: change.name.clone(),
            class: change.class,
        }
    }
}

impl fmt::Display for PartSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.id, self.name, self.class)
    }
}

/// A source's repository named by URL, cloned once beneath [`ROOT`].
///
/// Two URLs that spell one repository — differing in case of scheme or
/// host, a trailing `/` or `.git`, or the `git@host:path` form of an SSH
/// URL — share one clone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceClone {
    url: String,
    key: String,
}

impl SourceClone {
    /// Names the repository at `url`.
    #[must_use]
    pub fn new(url: &str) -> Self {
        let normalised = normalise(url);
        let digest = Sha256::digest(normalised.as_bytes());
        Self {
            url: url.trim().to_owned(),
            key: hex::encode(&digest[..8]),
        }
    }

    /// The URL as the run named it.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The deployment-local path of the clone, `./.emery/vcs/repos/<key>`.
    #[must_use]
    pub fn path(&self) -> String {
        format!("./{ROOT}/repos/{}", self.key)
    }

    /// Clones the repository, or brings the clone a run already has up to date with its origin.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::NotFound`] with code `revision-not-found` when the
    ///   URL names no repository.
    /// - Returns [`Error::BadGateway`] when the remote refuses, cannot be
    ///   reached, or wants credentials.
    /// - Returns [`Error::ServerError`] when the capability fails otherwise.
    pub async fn ensure(&self) -> Result<(), Error> {
        ensure(&self.url, &self.path()).await
    }

    /// The commit `revision` names in the clone.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no such revision, and the classes
    /// [`ensure`](Self::ensure) names otherwise.
    pub async fn resolve(&self, revision: &str) -> Result<String, Error> {
        let subject = self.subject();
        let workspace = workspace_at(&self.path(), subject).await?;
        match workspace
            .resolve(revision.to_owned())
            .await
            .map_err(|error| subject.refusal(error))?
        {
            Some(commit) => Ok(commit),
            None => Err(Error::not_found(
                "revision-not-found",
                format!("{subject} has no `{revision}`"),
            )),
        }
    }

    fn subject(&self) -> Subject<'_> {
        Subject::Repository(&self.url)
    }
}

// One spelling for every URL that names one repository: trimmed, the
// `git@host:path` form as `ssh://git@host/path`, scheme and host lowercased,
// a trailing `/` and `.git` dropped.
fn normalise(url: &str) -> String {
    let url = url.trim();
    let url = match scp_like(url) {
        Some((authority, path)) => format!("ssh://{authority}/{path}"),
        None => url.to_owned(),
    };
    let (scheme, rest) = url.split_once("://").unwrap_or(("", url.as_str()));
    let (authority, path) =
        if scheme.is_empty() { ("", rest) } else { rest.split_once('/').unwrap_or((rest, "")) };
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path).trim_end_matches('/');
    if scheme.is_empty() {
        path.to_owned()
    } else {
        format!("{}://{}/{path}", scheme.to_ascii_lowercase(), authority.to_ascii_lowercase())
    }
}

// `user@host:path` and `host:path`, which git reads as SSH: a colon before
// any slash, and no scheme.
fn scp_like(url: &str) -> Option<(&str, &str)> {
    if url.contains("://") {
        return None;
    }
    let (authority, path) = url.split_once(':')?;
    (!authority.is_empty() && !authority.contains('/') && !path.starts_with("//"))
        .then_some((authority, path))
}

// Clones `url` at `at`, or fetches its origin when a run already cloned it:
// the clone is the one operation that creates its location, so a read or a
// fetch at a path the run does not hold yet is refused there.
async fn ensure(url: &str, at: &str) -> Result<(), Error> {
    let subject = Subject::Repository(url);
    match vcs::clone(url, at, None).await.map_err(|error| subject.refusal(error))? {
        Ok(_workspace) => {
            tracing::info!(repository = url, clone = at, "cloned");
            Ok(())
        }
        Err(_occupied) => {
            tracing::info!(repository = url, clone = at, "fetching");
            let workspace = workspace_at(at, subject).await?;
            workspace.fetch(ORIGIN).await.map_err(|error| subject.refusal(error))
        }
    }
}

// What a run addressed when the capability refused, which decides the
// class and code the refusal reaches the operator as.
#[derive(Clone, Copy)]
enum Subject<'a> {
    Project,
    Repository(&'a str),
    Target(&'a str),
    WorkingCopy(&'a str),
}

impl Subject<'_> {
    fn refusal(self, error: Error) -> Error {
        match error.kind() {
            ErrorKind::NotFound => Error::not_found(
                "revision-not-found",
                if error.description().is_empty() {
                    format!("{self} has no such revision")
                } else {
                    format!("{self}: {}", error.description())
                },
            ),
            ErrorKind::BadRequest if error.code() == "repository-required" => error,
            ErrorKind::BadRequest if error.code() == "base-not-sealed" => error,
            ErrorKind::Conflict if error.code() == "pending" => Error::bad_request(
                "base-not-sealed",
                format!("{self} holds pending changes: {}", error.description()),
            ),
            ErrorKind::Conflict if error.code() == "diverged" => {
                server_error!("{self}: {}", error.description())
            }
            ErrorKind::Conflict if error.code() == "occupied" => {
                server_error!("{self}: {}", error.description())
            }
            ErrorKind::BadGateway => bad_gateway!("{self}: {}", error.description()),
            ErrorKind::ServerError if error.code() == "denied" => match self {
                Self::Project => Error::bad_request(
                    "repository-required",
                    "the project directory is not a repository",
                ),
                Self::Repository(_) | Self::Target(_) | Self::WorkingCopy(_) => {
                    server_error!("{self} is not a repository")
                }
            },
            ErrorKind::ServerError if error.code() == "not-found" => match self {
                Self::Project => Error::bad_request(
                    "repository-required",
                    "the project directory is not a repository",
                ),
                _ => Error::not_found(
                    "revision-not-found",
                    format!("{self}: {}", error.description()),
                ),
            },
            _ => server_error!("{self}: {}", error.description()),
        }
    }
}

impl fmt::Display for Subject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project => f.write_str("the project repository"),
            Self::Repository(url) => write!(f, "repository `{url}`"),
            Self::Target(name) => write!(f, "the target repository `{}`", target_path(name)),
            Self::WorkingCopy(at) => write!(f, "working copy `{at}`"),
        }
    }
}

/// The target as a run names it: the name its repository is kept under and where that repository is.
///
/// A run's `[target]` table, or its command line, carries one; a verb
/// settles it into a [`BuildRepo`] through [`repo`](TargetRef::repo).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRef {
    /// The target's name, kebab-case, which its repository is kept under.
    pub name: String,
    /// Where the repository is: [`PROJECT`] for the project checkout, a URL
    /// for a clone, `None` for a repository the engine creates and keeps.
    pub repository: Option<String>,
}

impl TargetRef {
    /// The repository the target names.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] when the name is not kebab-case.
    pub fn repo(&self) -> Result<BuildRepo, Error> {
        BuildRepo::settle(&self.name, self.repository.as_deref())
    }
}

/// The repository a specification lives in, a build's base comes from, and its label goes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildRepo {
    /// The project's own repository, read through the `.` mount.
    Project,
    /// The target's own repository, at [`target_path`] of its name: a clone
    /// of `url`, or, with none, the one the engine creates and keeps.
    Target {
        /// The target's name, which the repository is kept under.
        name: String,
        /// The URL the repository is cloned from, when the target names one.
        url: Option<String>,
    },
}

impl BuildRepo {
    /// The repository a target named `name` keeps at `repository`: the project for [`PROJECT`], a clone for a URL, a created one for none.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] when `name` is not kebab-case.
    pub fn settle(name: &str, repository: Option<&str>) -> Result<Self, Error> {
        if !is_kebab(name) {
            return Err(bad_request!("target `{name}` is not a kebab-case name"));
        }
        Ok(match repository {
            Some(PROJECT) => Self::Project,
            Some(url) => Self::Target {
                name: name.to_owned(),
                url: Some(url.to_owned()),
            },
            None => Self::Target {
                name: name.to_owned(),
                url: None,
            },
        })
    }

    /// The deployment-local path a working copy is cut from.
    #[must_use]
    pub fn path(&self) -> String {
        match self {
            Self::Project => PROJECT.to_owned(),
            Self::Target { name, .. } => target_path(name),
        }
    }

    /// Whether the repository is one the engine creates and keeps.
    #[must_use]
    pub const fn is_created(&self) -> bool {
        matches!(self, Self::Target { url: None, .. })
    }

    /// The commit the repository stands at, with the repository brought to where it can be read.
    ///
    /// - The project's is the commit its checkout sits on, whatever it
    ///   holds pending.
    /// - A clone's is [`DEFAULT_BRANCH`], the origin's default branch:
    ///   cloned when the run has no clone, fetched when it has one.
    /// - A created repository's is its head.
    ///
    /// `None` when the project checkout has no commit yet, or no run has
    /// created the created repository, or created it and sealed nothing.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] with code `repository-required` when
    ///   the project is no repository.
    /// - Returns [`Error::NotFound`] with code `revision-not-found` when the
    ///   URL names no repository or the clone has no default branch.
    /// - Returns [`Error::BadGateway`] when the remote refuses, cannot be
    ///   reached, or wants credentials.
    /// - Returns [`Error::ServerError`] when the capability fails otherwise.
    pub async fn head(&self) -> Result<Option<String>, Error> {
        match self {
            Self::Project => {
                let workspace = match vcs::open(PROJECT).await {
                    Ok(workspace) => workspace,
                    Err(error) if is_not_found(&error) => {
                        return Err(Subject::Project.refusal(error));
                    }
                    Err(error) => return Err(Subject::Project.refusal(error)),
                };
                match workspace.head().await.map_err(|error| Subject::Project.refusal(error))? {
                    Some(head) => Ok(Some(head)),
                    None => Ok(None),
                }
            }
            Self::Target { name, url: Some(url) } => {
                let at = target_path(name);
                let subject = Subject::Repository(url);
                ensure(url, &at).await?;
                let workspace = workspace_at(&at, subject).await?;
                match workspace
                    .resolve(DEFAULT_BRANCH.to_owned())
                    .await
                    .map_err(|error| subject.refusal(error))?
                {
                    Some(head) => Ok(Some(head)),
                    None => Err(Error::not_found(
                        "revision-not-found",
                        format!("{subject} has no `{DEFAULT_BRANCH}`"),
                    )),
                }
            }
            Self::Target { name, url: None } => {
                let at = target_path(name);
                match vcs::open(&at).await {
                    Err(error) if is_not_found(&error) => Ok(None),
                    Err(error) => Err(self.subject().refusal(error)),
                    Ok(workspace) => match workspace
                        .head()
                        .await
                        .map_err(|error| self.subject().refusal(error))?
                    {
                        Some(head) => Ok(Some(head)),
                        None => Ok(None),
                    },
                }
            }
        }
    }

    /// Holds `head`, the commit the repository stands at, as the one a build starts from.
    ///
    /// The project's checkout must hold nothing pending outside the
    /// engine's own [`SKIP_DIRS`], so a build starts from a sealed tree; a
    /// clone's and a created repository's head is its base as it stands.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] with code `repository-required` when
    ///   the project is no repository, and with code `base-not-sealed` when
    ///   its checkout holds pending changes, listed.
    /// - Returns [`Error::ServerError`] when the capability fails otherwise.
    pub async fn base(&self, head: &str) -> Result<String, Error> {
        if matches!(self, Self::Project) {
            project_clean().await?;
        }
        Ok(head.to_owned())
    }

    /// Creates the repository the engine keeps, or finds the one a run already created.
    ///
    /// The other two kinds are never created: the project is the
    /// operator's and a clone is its origin's.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn init(&self) -> Result<(), Error> {
        let Self::Target { name, url: None } = self else {
            return Ok(());
        };
        let at = target_path(name);
        match vcs::init(&at).await.map_err(|error| self.subject().refusal(error))? {
            Ok(_workspace) => {
                tracing::info!(repository = %at, "created the target repository");
                Ok(())
            }
            Err(_occupied) => Ok(()),
        }
    }

    /// What `label` already holds over `base` for `revision`: the labelled head and the requirements merged beneath it.
    ///
    /// The label is read back as a label alone, never a tag or a remote's
    /// branch of that spelling, and only where it descends from `base`. The
    /// requirements are read from the [`Message`]s a build sealed along the
    /// label's first-parent chain; a commit a build did not write, or one
    /// of another revision, is passed over, so a label that descends from
    /// `base` is resumed from wherever it stands, with no requirement beneath
    /// it when the chain names none. `None` means the repository has no such
    /// label, or the label does not descend from `base`, so a build starts
    /// from `base`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no `base`, and [`Error::ServerError`] when the
    /// capability fails otherwise.
    pub async fn merged_under(
        &self, label: &str, base: &str, revision: &str,
    ) -> Result<Option<(String, BTreeSet<ReqId>)>, Error> {
        let Some(head) = self.labelled(label).await? else {
            return Ok(None);
        };
        if !self.descends(base, &head).await? {
            tracing::warn!(
                label,
                base,
                head,
                "the label does not descend from the base; building from the base"
            );
            return Ok(None);
        }
        let workspace = workspace_at(&self.path(), self.subject()).await?;
        let entries = workspace
            .log_all(&head, Some(base))
            .await
            .map_err(|error| self.subject().refusal(error))?;
        let merged: BTreeSet<ReqId> = entries
            .iter()
            .filter_map(|entry| Message::parse(&entry.message))
            .filter(|message| message.revision == revision)
            .flat_map(|message| message.requirements)
            .collect();
        if merged.is_empty() {
            tracing::info!(
                label,
                head,
                "the label holds no slice of this revision; every slice builds from it"
            );
        }
        Ok(Some((head, merged)))
    }

    /// Cuts the integration working copy for a build of `revision` over `head`, resuming what an earlier run staged in it.
    ///
    /// A copy no run left is cut at `head`. One an earlier run left is read
    /// before it is removed: where its head descends from `head` and every
    /// slice sealed along the first-parent chain between them is this
    /// revision's, those slices are staged — merged by a run that ended
    /// before their wave was verified — and the copy is cut again at that
    /// head, so what it held uncommitted goes and the merges stay. A copy
    /// sitting on `head`, off it, or holding a slice of another revision is
    /// cut again at `head`, nothing staged.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no `head`, and [`Error::ServerError`] when the
    /// capability fails otherwise.
    pub async fn cut_integration(&self, head: &str, revision: &str) -> Result<Integration, Error> {
        let copy = WorkingCopy {
            at: INTEGRATION.to_owned(),
        };
        let repo = workspace_at(&self.path(), self.subject()).await?;
        match repo.add(INTEGRATION, head).await.map_err(|error| copy.subject().refusal(error))? {
            Ok(_workspace) => {
                return Ok(Integration {
                    copy,
                    head: head.to_owned(),
                    staged: Vec::new(),
                });
            }
            Err(_occupied) => {}
        }

        // what the copy an earlier run left holds over the head, read
        // before the copy goes
        let (at, staged) = if let Some((at, staged)) = self.staged(head, revision).await? {
            tracing::info!(
                head = %at,
                staged = staged.len(),
                "resuming the integration working copy an earlier run left unverified"
            );
            (at, staged)
        } else {
            tracing::info!(at = INTEGRATION, "removing the working copy an earlier run left");
            (head.to_owned(), Vec::new())
        };
        let integration = workspace_at(INTEGRATION, copy.subject()).await?;
        integration.remove().await.map_err(|error| copy.subject().refusal(error))?;
        match repo.add(INTEGRATION, &at).await.map_err(|error| copy.subject().refusal(error))? {
            Ok(_workspace) => {}
            Err(_occupied) => {
                return Err(server_error!(
                    "the integration working copy could not be cut at `{at}`"
                ));
            }
        }
        Ok(Integration {
            copy,
            head: at,
            staged,
        })
    }

    // The head of the integration copy an earlier run left and the slices
    // it merged over `head` for `revision`; `None` when there is nothing
    // to resume from. A head the repository does not know is a copy of
    // another repository, which is nothing to resume from either.
    async fn staged(
        &self, head: &str, revision: &str,
    ) -> Result<Option<(String, Vec<StagedSlice>)>, Error> {
        let subject = Subject::WorkingCopy(INTEGRATION);
        let integration = match vcs::open(INTEGRATION).await {
            Err(error) if is_not_found(&error) => return Ok(None),
            Err(error) => return Err(subject.refusal(error)),
            Ok(workspace) => workspace,
        };
        let staged = match integration.head().await.map_err(|error| subject.refusal(error))? {
            Some(staged) => staged,
            None => return Ok(None),
        };
        if staged == head {
            return Ok(None);
        }
        let repo = workspace_at(&self.path(), self.subject()).await?;
        let descends = match repo.is_ancestor(head.to_owned(), staged.clone()).await {
            Ok(descends) => descends,
            Err(error) if is_not_found(&error) => false,
            Err(error) => return Err(self.subject().refusal(error)),
        };
        if !descends {
            tracing::warn!(
                head,
                staged,
                "the integration working copy does not descend from the head; building from the \
                 head"
            );
            return Ok(None);
        }

        // every slice sealed over the head is this revision's, or none is
        let entries = repo
            .log_all(&staged, Some(head))
            .await
            .map_err(|error| self.subject().refusal(error))?;
        let mut slices = BTreeMap::new();
        for message in entries.iter().filter_map(|entry| Message::parse(&entry.message)) {
            if message.revision != revision {
                tracing::warn!(
                    staged,
                    slice = %message.slice,
                    revision = message.revision,
                    "the integration working copy holds a slice of another revision; building \
                     from the head"
                );
                return Ok(None);
            }
            slices.insert(message.slice, message.requirements);
        }
        if slices.is_empty() {
            return Ok(None);
        }
        let staged_slices =
            slices.into_iter().map(|(id, requirements)| StagedSlice { id, requirements }).collect();
        Ok(Some((staged, staged_slices)))
    }

    /// The commit `label` stands at, read back as a label alone; `None` when the repository has no such label.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn labelled(&self, label: &str) -> Result<Option<String>, Error> {
        let workspace = workspace_at(&self.path(), self.subject()).await?;
        match workspace
            .bookmark(label.to_owned())
            .await
            .map_err(|error| self.subject().refusal(error))?
        {
            Some(commit) => Ok(Some(commit)),
            None => Ok(None),
        }
    }

    /// Whether `descendant` descends from `ancestor` in the repository.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds neither commit, and [`Error::ServerError`] when the
    /// capability fails otherwise.
    pub async fn descends(&self, ancestor: &str, descendant: &str) -> Result<bool, Error> {
        let workspace = workspace_at(&self.path(), self.subject()).await?;
        workspace
            .is_ancestor(ancestor.to_owned(), descendant.to_owned())
            .await
            .map_err(|error| self.subject().refusal(error))
    }

    /// Points `label` at `commit`, creating or moving it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no such commit, and [`Error::ServerError`] when the
    /// capability fails otherwise.
    pub async fn label(&self, label: &str, commit: &str) -> Result<(), Error> {
        let workspace = workspace_at(&self.path(), self.subject()).await?;
        match workspace
            .set_bookmark(label.to_owned(), commit.to_owned(), None)
            .await
            .map_err(|error| self.subject().refusal(error))?
        {
            Ok(()) => {}
            Err(_conflict) => {
                return Err(server_error!(
                    "the label `{label}` could not be moved to commit `{commit}`"
                ));
            }
        }
        tracing::info!(label, commit, "labelled");
        Ok(())
    }

    fn subject(&self) -> Subject<'_> {
        match self {
            Self::Project => Subject::Project,
            Self::Target { url: Some(url), .. } => Subject::Repository(url),
            Self::Target { name, url: None } => Subject::Target(name),
        }
    }
}

// Holds the project checkout to nothing pending outside the engine's own
// directories, so a build starts from a sealed tree.
async fn project_clean() -> Result<(), Error> {
    let pending = pending(PROJECT).await?;
    let unsealed: Vec<String> = pending
        .into_iter()
        .map(|change| change.path)
        .filter(|path| !path.split('/').next().is_some_and(|first| SKIP_DIRS.contains(&first)))
        .collect();
    if !unsealed.is_empty() {
        return Err(Error::bad_request(
            "base-not-sealed",
            format!("the project checkout holds pending changes: {}", unsealed.join(", ")),
        ));
    }
    Ok(())
}

/// The integration working copy a build runs over, and what it was found holding.
///
/// Cut by [`BuildRepo::cut_integration`]: at the head the build starts from,
/// or, where an earlier run of the revision left slices merged in it
/// before their wave was verified, at that run's head with those slices
/// staged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Integration {
    /// The working copy, at [`INTEGRATION`].
    pub copy: WorkingCopy,
    /// The commit the copy sits on: the staged head, or the head the build
    /// starts from.
    pub head: String,
    /// The slices an earlier run merged over the build's head and never
    /// verified, in id order; empty when the copy was cut at that head.
    pub staged: Vec<StagedSlice>,
}

/// One slice an earlier run merged into the integration working copy before its wave was verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedSlice {
    /// The slice identifier from the commit's `Slice:` trailer.
    pub id: SliceId,
    /// The requirement identifiers from the commit's `Requirements:` trailer.
    pub requirements: Vec<ReqId>,
}

/// A working copy cut from a repository for one run, on no label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingCopy {
    at: String,
}

impl WorkingCopy {
    /// Cuts a working copy of `repo` at `at`, on `revision`.
    ///
    /// A working copy a failed run left at `at` is removed first.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no such revision, and [`Error::ServerError`] when the
    /// capability fails otherwise.
    pub async fn cut(repo: &str, at: &str, revision: &str) -> Result<Self, Error> {
        let copy = Self { at: at.to_owned() };
        let repo_workspace = workspace_at(repo, Subject::WorkingCopy(at)).await?;
        match repo_workspace
            .add(at, revision)
            .await
            .map_err(|error| copy.subject().refusal(error))?
        {
            Ok(_workspace) => Ok(copy),
            Err(_occupied) => {
                tracing::info!(at, "removing the working copy an earlier run left");
                let existing = workspace_at(at, copy.subject()).await?;
                existing.remove().await.map_err(|error| copy.subject().refusal(error))?;
                match repo_workspace
                    .add(at, revision)
                    .await
                    .map_err(|error| copy.subject().refusal(error))?
                {
                    Ok(_workspace) => Ok(copy),
                    Err(_occupied) => {
                        Err(server_error!("the working copy could not be cut at `{at}`"))
                    }
                }
            }
        }
    }

    /// A repository's own checkout at `at`, addressed as a working copy that is never removed.
    #[must_use]
    pub fn checkout(at: &str) -> Self {
        Self { at: at.to_owned() }
    }

    /// The deployment-local path of the working copy.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.at
    }

    /// What the working copy holds that its head does not.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn pending(&self) -> Result<Vec<Change>, Error> {
        pending(&self.at).await
    }

    /// Seals every pending change as one commit under `message`; `None` when there is nothing to seal.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn commit(&self, message: &str) -> Result<Option<String>, Error> {
        let workspace = workspace_at(&self.at, self.subject()).await?;
        workspace.commit(message.to_owned()).await.map_err(|error| self.subject().refusal(error))
    }

    /// Merges `revision` into the working copy under `policy` and advances its head.
    ///
    /// A conflict no rule resolves is data: the paths come back and the
    /// working copy is back on its head.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no such commit, and [`Error::ServerError`] when the
    /// capability fails otherwise.
    pub async fn merge(
        &self, revision: &str, message: &str, policy: &[Rule],
    ) -> Result<Merged, Error> {
        let workspace = workspace_at(&self.at, self.subject()).await?;
        match workspace
            .merge(revision.to_owned(), message.to_owned(), policy.to_vec())
            .await
            .map_err(|error| self.subject().refusal(error))?
        {
            Ok(commit) => Ok(Merged {
                commit: Some(commit),
                conflicts: Vec::new(),
            }),
            Err(Unmerged::Conflicts(paths)) => Ok(Merged {
                commit: None,
                conflicts: paths,
            }),
            Err(Unmerged::Pending(paths)) => Err(Error::bad_request(
                "base-not-sealed",
                format!("{} holds pending changes: {}", self.subject(), paths.join(", ")),
            )),
        }
    }

    /// The commit the working copy sits on.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn head(&self) -> Result<String, Error> {
        let workspace = workspace_at(&self.at, self.subject()).await?;
        workspace
            .head()
            .await
            .map_err(|error| self.subject().refusal(error))?
            .ok_or_else(|| server_error!("{} has no commit yet", self.subject()))
    }

    /// Removes the working copy and everything it holds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn remove(self) -> Result<(), Error> {
        let workspace = workspace_at(&self.at, self.subject()).await?;
        workspace.remove().await.map_err(|error| self.subject().refusal(error))
    }

    fn subject(&self) -> Subject<'_> {
        Subject::WorkingCopy(&self.at)
    }
}

/// What the tree at `at`, a working copy or a repository's own checkout, holds that its head does not.
///
/// # Errors
///
/// Returns [`Error::ServerError`] when the capability fails.
pub async fn pending(at: &str) -> Result<Vec<Change>, Error> {
    let subject = Subject::WorkingCopy(at);
    let workspace = workspace_at(at, subject).await?;
    workspace.status_all().await.map_err(|error| subject.refusal(error))
}
