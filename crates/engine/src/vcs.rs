//! Names the repositories a run reads from and builds into.
//!
//! A repository a source names by URL is one [`Repository`]: a clone
//! beneath the project's [`ROOT`], keyed by the URL, kept across runs and
//! refreshed by a fetch. The repository a build lands in is a [`Repo`]: the
//! project's own checkout, the `.` mount, never written; or the target's
//! own, kept under the target's name, a clone of the URL the target names
//! or one the engine lays when the target names none. A
//! source is read, and the plan built, in a [`WorkingCopy`] cut from one of
//! them and removed when the run is done with it. A build seals each slice
//! under a [`Message`], which it reads back from the labelled history, and
//! from the [`Integration`] copy a run left before its wave was verified,
//! to resume.
//!
//! Every path here is deployment-local, as the [`Vcs`] capability takes it:
//! `.` is the project mount and `./.emery/vcs/...` lies beneath it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use emery_adapter::SKIP_DIRS;
use emery_adapter::target::{MergeRule, MergeStrategy};
use omnia_sdk::vcs::{self, Change, CloneOptions, Merged, Rule, Strategy};
use omnia_sdk::{Error, Vcs, bad_gateway, server_error};
use sha2::{Digest as _, Sha256};

use crate::Files;
use crate::revision::{ReqId, SliceId};

/// The directory beneath the project root that holds every clone and working copy.
pub const ROOT: &str = ".emery/vcs";

/// The working copy a build integrates its slices in.
pub const INTEGRATION: &str = "./.emery/vcs/integration";

/// The directory beneath the project root that holds the working copy each slice builds in.
pub const WORKTREES: &str = "./.emery/vcs/worktrees";

/// The remote a clone fetches from: the URL it was cloned from.
pub const ORIGIN: &str = "origin";

/// The commit a build into a cloned repository starts from: the origin's default branch, as the last fetch left it.
pub const DEFAULT_BRANCH: &str = "origin/HEAD";

/// The message of the one commit the engine lays a target repository with.
pub const INITIALISE: &str = "Initialise";

/// The project's own checkout, as the capability names it and as a target's `repository` names it to build into.
pub const PROJECT: &str = ".";

// The file a laid repository's root commit holds beside what the adapter's
// init laid, when the init laid no ignore file of its own, so that the
// commit exists and the engine's own directory is ignored wherever the
// result is checked out.
const IGNORE: (&str, &[u8]) = (".gitignore", b".emery/\n");

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
            paths: rule.paths.to_string(),
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
    rules.iter().find(|rule| matches(&rule.paths, path)).map(|rule| rule.strategy)
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
/// slices it holds. [`Display`](fmt::Display) writes it and
/// [`parse`](Message::parse) reads it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The slice sealed.
    pub slice: SliceId,
    /// The slice's name.
    pub name: String,
    /// The revision whose plan the slice is of.
    pub revision: String,
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
            name: name.trim().to_owned(),
            revision: (*trailers.get("Revision")?).to_owned(),
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
            "{slice} {name}\n\nSlice: {slice}\nRevision: {revision}\nRequirements: {requirements}\n\
             Covered: {covered}\nAdapter: {adapter}\nBase: {base}\nWave: {wave}",
            slice = self.slice,
            name = self.name,
            revision = self.revision,
            requirements = list(&self.requirements),
            covered = list(&self.covered),
            adapter = self.adapter,
            base = self.base,
            wave = self.wave,
        )
    }
}

/// A repository named by URL, cloned once beneath [`ROOT`].
///
/// Two URLs that spell one repository — differing in case of scheme or
/// host, a trailing `/` or `.git`, or the `git@host:path` form of an SSH
/// URL — share one clone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    url: String,
    key: String,
}

impl Repository {
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
    pub async fn ensure<V: Vcs>(&self, vcs: &V) -> Result<(), Error> {
        ensure(vcs, &self.url, &self.path()).await.map_err(|error| self.subject().refusal(error))
    }

    /// The commit `revision` names in the clone.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no such revision, and the classes
    /// [`ensure`](Self::ensure) names otherwise.
    pub async fn resolve<V: Vcs>(&self, vcs: &V, revision: &str) -> Result<String, Error> {
        vcs.resolve(&self.path(), revision).await.map_err(|error| self.subject().refusal(error))
    }

    fn subject(&self) -> Subject<'_> {
        Subject::Repository(&self.url)
    }
}

/// The repository a build's base comes from and its label goes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repo {
    /// The project's own repository, read through the `.` mount.
    Project,
    /// The target's own repository, at [`target_path`] of its name: a clone
    /// of `url`, or, with none, the one the engine lays and keeps.
    Target {
        /// The target's name, which the repository is kept under.
        name: String,
        /// The URL the repository is cloned from, when the target names one.
        url: Option<String>,
    },
}

impl Repo {
    /// The deployment-local path a working copy is cut from.
    #[must_use]
    pub fn path(&self) -> String {
        match self {
            Self::Project => PROJECT.to_owned(),
            Self::Target { name, .. } => target_path(name),
        }
    }

    /// The commit a build starts from, with the repository brought to where it can be read.
    ///
    /// - The project's is the sealed commit its checkout sits on. Pending
    ///   changes beneath the engine's own [`SKIP_DIRS`] are never counted.
    /// - A clone's is [`DEFAULT_BRANCH`], the origin's default branch:
    ///   cloned when the run has no clone, fetched when it has one.
    /// - A laid repository's is its root commit, [`INITIALISE`], holding
    ///   what `init` laid in the tree — the adapter's layout, given the
    ///   repository's path and answering what it left pending — and a
    ///   `.gitignore` of the engine's own directory beside it when `init`
    ///   laid none. It is laid when no run has laid it yet, through `init`
    ///   and [`Files::write`], and kept, so a later run starts from the
    ///   same commit and resumes the label set over it. One laid but never
    ///   sealed is sealed now, `init` run over it again. The other two
    ///   bases never run `init`.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] with code `repository-required` when
    ///   the project is no repository, and with code `base-not-sealed` when
    ///   its checkout holds pending changes, listed, or no commit yet.
    /// - Returns [`Error::NotFound`] with code `revision-not-found` when the
    ///   URL names no repository, the clone has no default branch, or what
    ///   sits at the path was laid or cloned under no such origin.
    /// - Returns [`Error::BadGateway`] when the remote refuses, cannot be
    ///   reached, or wants credentials.
    /// - Returns [`Error::ServerError`] when the capability fails otherwise,
    ///   or a laid repository seals nothing.
    /// - Returns what `init` fails with.
    pub async fn base<P: Vcs + Files>(
        &self, provider: &P, init: impl AsyncFnOnce(&str) -> Result<Vec<Change>, Error>,
    ) -> Result<String, Error> {
        match self {
            Self::Project => project_base(provider).await,
            Self::Target { name, url: Some(url) } => {
                let at = target_path(name);
                let subject = Subject::Repository(url);
                ensure(provider, url, &at).await.map_err(|error| subject.refusal(error))?;
                provider.resolve(&at, DEFAULT_BRANCH).await.map_err(|error| subject.refusal(error))
            }
            Self::Target { name, url: None } => lay(provider, name, init).await,
        }
    }

    /// What `label` already holds over `base` for `revision`: the labelled head and the slices merged beneath it.
    ///
    /// The label is read back as a label alone, never a tag or a remote's
    /// branch of that spelling, and only where it descends from `base`. The
    /// slices are read from the [`Message`]s a build sealed along the
    /// label's first-parent chain; a commit a build did not write, or one
    /// of another revision, is passed over, so a label that descends from
    /// `base` is resumed from wherever it stands, with no slice beneath it
    /// when the chain names none. `None` means the repository has no such
    /// label, or the label does not descend from `base`, so a build starts
    /// from `base`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no `base`, and [`Error::ServerError`] when the
    /// capability fails otherwise.
    pub async fn merged_under<V: Vcs>(
        &self, vcs: &V, label: &str, base: &str, revision: &str,
    ) -> Result<Option<(String, BTreeSet<SliceId>)>, Error> {
        let repo = self.path();
        let head = match vcs.labelled(&repo, label).await {
            Ok(head) => head,
            Err(vcs::Error::NotFound(_)) => return Ok(None),
            Err(error) => return Err(self.subject().refusal(error)),
        };
        let descends = vcs
            .descends(&repo, base, &head)
            .await
            .map_err(|error| self.subject().refusal(error))?;
        if !descends {
            tracing::warn!(
                label,
                base,
                head,
                "the label does not descend from the base; building from the base"
            );
            return Ok(None);
        }
        let entries =
            vcs.log(&repo, &head, base).await.map_err(|error| self.subject().refusal(error))?;
        let merged: BTreeSet<SliceId> = entries
            .iter()
            .filter_map(|entry| Message::parse(&entry.message))
            .filter(|message| message.revision == revision)
            .map(|message| message.slice)
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
    pub async fn cut_integration<V: Vcs>(
        &self, vcs: &V, head: &str, revision: &str,
    ) -> Result<Integration, Error> {
        let repo = self.path();
        let copy = WorkingCopy {
            at: INTEGRATION.to_owned(),
        };
        match vcs.add(&repo, INTEGRATION, head).await {
            Ok(()) => {
                return Ok(Integration {
                    copy,
                    head: head.to_owned(),
                    staged: BTreeSet::new(),
                });
            }
            Err(vcs::Error::Exists(_)) => {}
            Err(error) => return Err(copy.subject().refusal(error)),
        }

        // what the copy an earlier run left holds over the head, read
        // before the copy goes
        let (at, staged) = if let Some((at, staged)) = self.staged(vcs, head, revision).await? {
            tracing::info!(
                head = %at,
                staged = staged.len(),
                "resuming the integration working copy an earlier run left unverified"
            );
            (at, staged)
        } else {
            tracing::info!(at = INTEGRATION, "removing the working copy an earlier run left");
            (head.to_owned(), BTreeSet::new())
        };
        vcs.remove(INTEGRATION).await.map_err(|error| copy.subject().refusal(error))?;
        vcs.add(&repo, INTEGRATION, &at).await.map_err(|error| copy.subject().refusal(error))?;
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
    async fn staged<V: Vcs>(
        &self, vcs: &V, head: &str, revision: &str,
    ) -> Result<Option<(String, BTreeSet<SliceId>)>, Error> {
        let repo = self.path();
        let subject = Subject::WorkingCopy(INTEGRATION);
        let staged = match vcs.head(INTEGRATION).await {
            Ok(staged) => staged,
            Err(vcs::Error::NotFound(_)) => return Ok(None),
            Err(error) => return Err(subject.refusal(error)),
        };
        if staged == head {
            return Ok(None);
        }
        let descends = match vcs.descends(&repo, head, &staged).await {
            Ok(descends) => descends,
            Err(vcs::Error::NotFound(_)) => false,
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
        let entries =
            vcs.log(&repo, &staged, head).await.map_err(|error| self.subject().refusal(error))?;
        let mut slices = BTreeSet::new();
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
            slices.insert(message.slice);
        }
        if slices.is_empty() {
            return Ok(None);
        }
        Ok(Some((staged, slices)))
    }

    /// Points `label` at `commit`, creating or moving it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository holds no such commit, and [`Error::ServerError`] when the
    /// capability fails otherwise.
    pub async fn label<V: Vcs>(&self, vcs: &V, label: &str, commit: &str) -> Result<(), Error> {
        vcs.label(&self.path(), label, commit)
            .await
            .map_err(|error| self.subject().refusal(error))?;
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

/// The integration working copy a build runs over, and what it was found holding.
///
/// Cut by [`Repo::cut_integration`]: at the head the build starts from,
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
    pub staged: BTreeSet<SliceId>,
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
    pub async fn cut<V: Vcs>(vcs: &V, repo: &str, at: &str, revision: &str) -> Result<Self, Error> {
        let copy = Self { at: at.to_owned() };
        match vcs.add(repo, at, revision).await {
            Ok(()) => Ok(copy),
            Err(vcs::Error::Exists(_)) => {
                tracing::info!(at, "removing the working copy an earlier run left");
                vcs.remove(at).await.map_err(|error| copy.subject().refusal(error))?;
                vcs.add(repo, at, revision).await.map_err(|error| copy.subject().refusal(error))?;
                Ok(copy)
            }
            Err(error) => Err(copy.subject().refusal(error)),
        }
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
    pub async fn pending<V: Vcs>(&self, vcs: &V) -> Result<Vec<Change>, Error> {
        pending(vcs, &self.at).await
    }

    /// Seals every pending change as one commit under `message`; `None` when there is nothing to seal.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn commit<V: Vcs>(&self, vcs: &V, message: &str) -> Result<Option<String>, Error> {
        vcs.commit(&self.at, message).await.map_err(|error| self.subject().refusal(error))
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
    pub async fn merge<V: Vcs>(
        &self, vcs: &V, revision: &str, message: &str, policy: &[Rule],
    ) -> Result<Merged, Error> {
        vcs.merge(&self.at, revision, message, policy)
            .await
            .map_err(|error| self.subject().refusal(error))
    }

    /// The commit the working copy sits on.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn head<V: Vcs>(&self, vcs: &V) -> Result<String, Error> {
        vcs.head(&self.at).await.map_err(|error| self.subject().refusal(error))
    }

    /// Removes the working copy and everything it holds.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn remove<V: Vcs>(self, vcs: &V) -> Result<(), Error> {
        vcs.remove(&self.at).await.map_err(|error| self.subject().refusal(error))
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
pub async fn pending<V: Vcs>(vcs: &V, at: &str) -> Result<Vec<Change>, Error> {
    vcs.pending(at).await.map_err(|error| Subject::WorkingCopy(at).refusal(error))
}

// Clones `url` at `at`, or fetches its origin when a run already cloned it:
// the clone is the one operation that creates its location, so a read or a
// fetch at a path the run does not hold yet is refused there.
async fn ensure<V: Vcs>(vcs: &V, url: &str, at: &str) -> Result<(), vcs::Error> {
    match vcs.clone_repo(url, at, CloneOptions { depth: None }).await {
        Ok(()) => {
            tracing::info!(repository = url, clone = at, "cloned");
            Ok(())
        }
        Err(vcs::Error::Exists(_)) => {
            tracing::info!(repository = url, clone = at, "fetching");
            vcs.fetch(at, ORIGIN).await
        }
        Err(error) => Err(error),
    }
}

// The root commit of the repository laid under `name`, laid when no run has
// laid it yet: `init` answers `Exists` for one a run already holds, whose
// head is the base, or `NotFound` under it for one laid and never sealed.
async fn lay<P: Vcs + Files>(
    provider: &P, name: &str, init: impl AsyncFnOnce(&str) -> Result<Vec<Change>, Error>,
) -> Result<String, Error> {
    let at = target_path(name);
    let subject = Subject::Target(name);
    match provider.init(&at).await {
        Ok(()) => {}
        Err(vcs::Error::Exists(_)) => match provider.head(&at).await {
            Ok(head) => return Ok(head),
            Err(vcs::Error::NotFound(_)) => {}
            Err(error) => return Err(subject.refusal(error)),
        },
        Err(error) => return Err(subject.refusal(error)),
    }

    // the adapter's layout, and the engine's ignore file where it laid none
    let laid = init(&at).await?;
    let (path, content) = IGNORE;
    if !laid.iter().any(|change| change.path == path) {
        provider.write(&at, path, content).await?;
    }
    let commit = provider
        .commit(&at, INITIALISE)
        .await
        .map_err(|error| subject.refusal(error))?
        .ok_or_else(|| server_error!("{subject} sealed nothing as its root commit"))?;
    tracing::info!(repository = %at, %commit, "laid the target repository");
    Ok(commit)
}

async fn project_base<V: Vcs>(vcs: &V) -> Result<String, Error> {
    let pending = vcs.pending(PROJECT).await.map_err(|error| Subject::Project.refusal(error))?;
    let unsealed: Vec<String> = pending
        .into_iter()
        .map(|change| change.path)
        .filter(|path| !path.split('/').next().is_some_and(|first| SKIP_DIRS.contains(&first)))
        .collect();
    if !unsealed.is_empty() {
        return Err(Error::BadRequest {
            code: "base-not-sealed".into(),
            description: format!(
                "the project checkout holds pending changes: {}",
                unsealed.join(", ")
            ),
        });
    }

    match vcs.head(PROJECT).await {
        Ok(head) => Ok(head),
        Err(vcs::Error::NotFound(_)) => Err(Error::BadRequest {
            code: "base-not-sealed".into(),
            description: "the project repository has no commit to build from".into(),
        }),
        Err(error) => Err(Subject::Project.refusal(error)),
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
    fn refusal(self, error: vcs::Error) -> Error {
        match error {
            vcs::Error::NotFound(what) => Error::NotFound {
                code: "revision-not-found".into(),
                description: format!("{self} has no `{what}`"),
            },
            vcs::Error::NotARepository => match self {
                Self::Project => Error::BadRequest {
                    code: "repository-required".into(),
                    description: "the project directory is not a repository".into(),
                },
                Self::Repository(_) | Self::Target(_) | Self::WorkingCopy(_) => {
                    server_error!("{self} is not a repository")
                }
            },
            vcs::Error::Pending(paths) => Error::BadRequest {
                code: "base-not-sealed".into(),
                description: format!("{self} holds pending changes: {}", paths.join(", ")),
            },
            vcs::Error::Access(detail) => bad_gateway!("{self}: {detail}"),
            vcs::Error::Diverged(what) => server_error!("{self}: `{what}` diverged"),
            vcs::Error::Exists(what) => server_error!("{self}: `{what}` already exists"),
            vcs::Error::Other(detail) => server_error!("{self}: {detail}"),
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
