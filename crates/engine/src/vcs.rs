//! Names the repositories a run reads from and builds into.
//!
//! A repository a run names by URL — a source's or the target's — is one
//! [`Repository`]: a clone beneath the project's [`ROOT`], keyed by the URL,
//! kept across runs and refreshed by a fetch. The project's own checkout is
//! the `.` mount, never written: a source is read, and the plan built, in a
//! [`WorkingCopy`] cut from a [`Repo`] and removed when the run is done with
//! it.
//!
//! Every path here is deployment-local, as the [`Vcs`] capability takes it:
//! `.` is the project mount and `./.emery/vcs/...` lies beneath it.

use std::fmt;

use emery_adapter::SKIP_DIRS;
use omnia_sdk::vcs::{self, Change, CloneOptions};
use omnia_sdk::{Error, Vcs, bad_gateway, server_error};
use sha2::{Digest as _, Sha256};

/// The directory beneath the project root that holds every clone and working copy.
pub const ROOT: &str = ".emery/vcs";

/// The working copy a build integrates its slices in.
pub const INTEGRATION: &str = "./.emery/vcs/integration";

/// The remote a clone fetches from: the URL it was cloned from.
pub const ORIGIN: &str = "origin";

// The project's own checkout, as the capability names it.
const PROJECT: &str = ".";

/// The working copy a repository source is read from, by source name.
#[must_use]
pub fn source_checkout(name: &str) -> String {
    format!("./{ROOT}/sources/{name}")
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

    /// Brings the clone up to date with its origin, cloning it first when the run has none.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::NotFound`] with code `revision-not-found` when the
    ///   URL names no repository.
    /// - Returns [`Error::BadGateway`] when the remote refuses, cannot be
    ///   reached, or wants credentials.
    /// - Returns [`Error::ServerError`] when the capability fails otherwise.
    pub async fn ensure<V: Vcs>(&self, vcs: &V) -> Result<(), Error> {
        let at = self.path();
        match vcs.fetch(&at, ORIGIN).await {
            Ok(()) => {
                tracing::info!(repository = %self.url, clone = %at, "fetched");
                Ok(())
            }
            Err(vcs::Error::NotARepository) => {
                tracing::info!(repository = %self.url, clone = %at, "cloning");
                vcs.clone_repo(&self.url, &at, CloneOptions { depth: None })
                    .await
                    .map_err(|error| self.subject().refusal(error))
            }
            Err(error) => Err(self.subject().refusal(error)),
        }
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
    /// A clone of the repository the target names.
    Clone(Repository),
}

impl Repo {
    /// The deployment-local path a working copy is cut from.
    #[must_use]
    pub fn path(&self) -> String {
        match self {
            Self::Project => PROJECT.to_owned(),
            Self::Clone(repository) => repository.path(),
        }
    }

    /// Points `label` at `commit` and, with `remote`, sends it there.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotFound`] with code `revision-not-found` when the
    /// repository has no such remote, [`Error::BadGateway`] when it cannot
    /// be reached, and [`Error::ServerError`] otherwise.
    pub async fn publish<V: Vcs>(
        &self, vcs: &V, label: &str, commit: &str, remote: Option<&str>,
    ) -> Result<(), Error> {
        let subject = match self {
            Self::Project => Subject::Project,
            Self::Clone(repository) => repository.subject(),
        };
        let repo = self.path();
        vcs.label(&repo, label, commit).await.map_err(|error| subject.refusal(error))?;
        tracing::info!(label, commit, "labelled");
        if let Some(remote) = remote {
            vcs.push(&repo, remote, label).await.map_err(|error| subject.refusal(error))?;
            tracing::info!(label, remote, "pushed");
        }
        Ok(())
    }
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
        vcs.pending(&self.at).await.map_err(|error| self.subject().refusal(error))
    }

    /// Seals every pending change as one commit under `message`; `None` when there is nothing to seal.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the capability fails.
    pub async fn commit<V: Vcs>(&self, vcs: &V, message: &str) -> Result<Option<String>, Error> {
        vcs.commit(&self.at, message).await.map_err(|error| self.subject().refusal(error))
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

/// The sealed commit the project's own checkout sits on.
///
/// Pending changes beneath the engine's own [`SKIP_DIRS`] are never counted.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] with code `repository-required` when the
///   project is no repository, and with code `base-not-sealed` when its
///   checkout holds pending changes, listed, or no commit yet.
/// - Returns [`Error::ServerError`] when the capability fails otherwise.
pub async fn project_base<V: Vcs>(vcs: &V) -> Result<String, Error> {
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
                Self::Repository(_) | Self::WorkingCopy(_) => {
                    server_error!("{self} is not a repository")
                }
            },
            vcs::Error::Pending(paths) => Error::BadRequest {
                code: "base-not-sealed".into(),
                description: format!("{self} holds pending changes: {}", paths.join(", ")),
            },
            vcs::Error::Access(detail) => bad_gateway!("{self}: {detail}"),
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
