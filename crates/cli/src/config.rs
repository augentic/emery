//! Resolves sources and the target from CLI arguments and `emery.toml`.

use std::fmt;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::Context;
use emery_engine::build::{BudgetSource, BuildInput};
use emery_engine::show::{Artifact, ShowInput};
use emery_engine::specify::{Scope, SourceConfig, SourceContent, SourceRepository};
use emery_engine::vcs::TargetRef;
use emery_engine::{AdapterRef, Rank, preopen_join, preopen_path};
use omnia_sdk::plugins::Digest;
use omnia_sdk::{Error, bad_request};

// The config file a run naming no adapters looks for at the project root.
pub const CONFIG_FILE: &str = "emery.toml";

// Positional adapters, `--description` values, and an optional `--config` path.
//
// The file's `[[source]]` tables are the base and the argv sources layer
// over them: an argv source takes the place of the file source of its
// name, whole, and is appended otherwise, in argv order. A run naming
// sources and no file reads no file; one naming neither discovers the
// project-root file. The `[target]` table of the file read is the run's
// target, the repository the specification lives in, so a run naming its
// sources on the command line knows it only under `--config`.
pub struct SourceCarriers<'a> {
    pub adapters: &'a [String],
    pub descriptions: &'a [String],
    pub config: Option<&'a Path>,
}

impl SourceCarriers<'_> {
    pub fn resolve(self) -> Result<(Vec<SourceConfig>, Option<TargetRef>), Error> {
        let Self {
            adapters,
            descriptions,
            config,
        } = self;
        let argv = !adapters.is_empty() || !descriptions.is_empty();

        // the file, named or discovered, and the sources and target it states
        let file = if config.is_none() && argv { None } else { ConfigFile::load(config)? };
        let path = file.as_ref().map(|file| file.path.clone());
        let (mut sources, target) = match file {
            Some(file) => file.into_parts()?,
            None => (Vec::new(), None),
        };
        let held = sources.len();

        // the argv sources, each in the place of the file source of its name
        let mut over = Vec::with_capacity(adapters.len() + descriptions.len());
        for reference in adapters {
            over.push(argv_source(reference, SourceContent::Workspace(".".to_owned()))?);
        }
        for entry in descriptions {
            let (reference, text) = description(entry)?;
            over.push(argv_source(reference, SourceContent::Value(text.to_owned()))?);
        }
        let mut replaced = vec![false; held];
        for source in over {
            if let Some(at) = (0..held).find(|&at| !replaced[at] && sources[at].name == source.name)
            {
                sources[at] = source;
                replaced[at] = true;
            } else {
                sources.push(source);
            }
        }

        tracing::debug!(
            carrier = %carrier_label(path.as_deref()),
            sources = sources.len(),
            "sources decoded"
        );
        Ok((sources, target))
    }

    // Resolves `amend` sources and its target: the change file's `[target]`
    // table when present, falling back to the project-root `emery.toml`.
    pub fn resolve_amend(self) -> Result<(Vec<SourceConfig>, Option<TargetRef>), Error> {
        let has_config = self.config.is_some();
        let (sources, configured) = self.resolve()?;
        let target = match configured {
            Some(target) => Some(target),
            None if has_config => target(None, None)?,
            None => None,
        };
        Ok((sources, target))
    }
}

// The target whose repository the specification lives in, as `apply`, `show`,
// and `amend` read it.
//
// Reads the `[target]` table of `config`, or of the discovered project-root
// file when `config` names none; `repository` takes the place of its
// `repository`. Answers `None` when no file states a target, so the verb
// decides whether it needs one.
pub fn target(config: Option<&Path>, repository: Option<&str>) -> Result<Option<TargetRef>, Error> {
    let Some(file) = ConfigFile::load(config)? else {
        return Ok(None);
    };
    let Some(mut entry) = file.target else {
        return Ok(None);
    };
    if let Some(repository) = repository {
        entry.repository = Some(repository.to_owned());
    }
    Ok(Some(entry.into_ref()))
}

// Resolves `ShowInput` from the `[target]` table of `config` (or the
// discovered project-root file) and `--budget`: the staged candidate and
// the amendment read no file, while a current-revision artifact carries the
// target's repository, adapter, digest, and budget.
pub fn show(
    artifact: Artifact, staged: bool, config: Option<&Path>, flag_budget: Option<NonZeroUsize>,
) -> Result<ShowInput, Error> {
    if !staged
        && !matches!(artifact, Artifact::Amendment)
        && let Some(entry) = ConfigFile::load(config)?.and_then(|file| file.target)
    {
        return Ok(entry.into_show(artifact, flag_budget));
    }
    let (budget, budget_source) = resolve_budget(flag_budget, None);
    Ok(ShowInput {
        staged,
        budget,
        budget_source,
        ..ShowInput::new(artifact, None)
    })
}

// The `build-target-required` refusal of `verb`, which reads or seals the current revision and was handed no `[target]`.
pub fn missing_target(verb: &str) -> Error {
    target_required(format!(
        "the specification has no repository to live in: {verb} needs a `[target]` table \
         naming one"
    ))
}

// A positional target adapter, a `--repository`, an optional `--config` path,
// an optional `--jobs` cap, and an optional `--budget` cap.
//
// The file's `[target]` table is the base and the flags layer over it: an
// argv adapter takes the place of its `adapter`, dropping the `digest` that
// pinned the file's, under the file's `name` where it states one; a
// `--repository` takes the place of its `repository`, and `--budget` takes
// the place of its `budget`. A run naming an adapter and no file reads none;
// one naming no adapter discovers the project-root file.
pub struct TargetCarriers<'a> {
    pub adapter: Option<&'a str>,
    pub repository: Option<&'a str>,
    pub config: Option<&'a Path>,
    pub jobs: Option<NonZeroUsize>,
    pub budget: Option<NonZeroUsize>,
}

impl TargetCarriers<'_> {
    pub fn resolve(self) -> Result<BuildInput, Error> {
        let Self {
            adapter,
            repository,
            config,
            jobs,
            budget,
        } = self;

        // the file, named or discovered, and the target it states
        let (path, mut target) = if let (None, Some(reference)) = (config, adapter) {
            (None, TargetEntry::argv(reference)?)
        } else {
            let file =
                ConfigFile::load(config)?.ok_or_else(|| target_required("no target adapter"))?;
            let path = file.path.clone();
            (Some(path), file.require_target()?)
        };

        // the flags, each in the place of the file's key
        if let Some(reference) = adapter {
            target.adapter = AdapterRef::from_str(reference)?;
            target.digest = None;
        }
        if let Some(repository) = repository {
            target.repository = Some(repository.to_owned());
        }

        let input = target.into_input(jobs, budget);
        tracing::debug!(
            carrier = %carrier_label(path.as_deref()),
            adapter = %input.adapter,
            "target decoded"
        );
        Ok(input)
    }
}

fn carrier_label(path: Option<&Path>) -> impl fmt::Display {
    fmt::from_fn(move |f| match path {
        Some(path) => write!(f, "{}", path.display()),
        None => f.write_str("argv"),
    })
}

fn target_required(description: impl Into<String>) -> Error {
    Error::BadRequest {
        code: "build-target-required".into(),
        description: description.into(),
    }
}

fn config_path(path: &Path) -> Result<PathBuf, Error> {
    preopen_path(path).map_err(|err| {
        let description = err.description();
        bad_request!("invalid argument --config: {description}")
    })
}

fn description(entry: &str) -> Result<(&str, &str), Error> {
    entry.split_once('=').filter(|(reference, _)| !reference.is_empty()).ok_or_else(|| {
        bad_request!("invalid argument --description: expected `<adapter>=<text>`, got `{entry}`")
    })
}

fn argv_source(reference: &str, content: SourceContent) -> Result<SourceConfig, Error> {
    let adapter = AdapterRef::from_str(reference)?;
    Ok(SourceConfig {
        name: adapter.name().to_owned(),
        adapter,
        content,
        repository: None,
        digest: None,
        rank: None,
        scope: Scope::default(),
    })
}

fn discover() -> Result<Option<PathBuf>, Error> {
    let path = Path::new(CONFIG_FILE);
    let found = path.try_exists().with_context(|| format!("reading {CONFIG_FILE}"))?;
    Ok(found.then(|| path.to_owned()))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ConfigFile {
    #[serde(skip)]
    path: PathBuf,
    source: Vec<SourceEntry>,
    target: Option<TargetEntry>,
}

impl ConfigFile {
    fn load(config: Option<&Path>) -> Result<Option<Self>, Error> {
        let path = match config {
            Some(path) => Some(config_path(path)?),
            None => discover()?,
        };
        path.map(Self::read).transpose()
    }

    fn read(path: PathBuf) -> Result<Self, Error> {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let mut file: Self = toml::from_str(&raw).map_err(|err| {
            let path = path.display();
            bad_request!("{path}: {err}")
        })?;
        file.path = path;
        Ok(file)
    }

    fn into_parts(self) -> Result<(Vec<SourceConfig>, Option<TargetRef>), Error> {
        let base = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let sources = self
            .source
            .into_iter()
            .map(|entry| entry.into_config(base))
            .collect::<Result<_, _>>()?;
        Ok((sources, self.target.map(TargetEntry::into_ref)))
    }

    fn require_target(self) -> Result<TargetEntry, Error> {
        let path = self.path;
        self.target
            .ok_or_else(|| target_required(format!("{} has no `[target]` table", path.display())))
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SourceEntry {
    // Omitted, the adapter's package name, as on the command line.
    name: Option<String>,
    adapter: AdapterRef,
    path: Option<PathBuf>,
    description: Option<String>,
    repository: Option<String>,
    revision: Option<String>,
    digest: Option<Digest>,
    rank: Option<Rank>,
    #[serde(default)]
    scope: Scope,
}

impl SourceEntry {
    // `base` is the directory the file's `path` keys are relative to; with
    // a `repository`, `path` is relative to the clone instead.
    fn into_config(self, base: &Path) -> Result<SourceConfig, Error> {
        let adapter = self.adapter;
        let name = self.name.unwrap_or_else(|| adapter.name().to_owned());

        let repository = match (self.repository, self.revision) {
            (Some(url), Some(revision)) => Some(SourceRepository { url, revision }),
            (Some(_), None) => {
                return Err(bad_request!(
                    "source `{name}` sets `repository` without `revision`; a repository is read \
                     at a label, tag, or commit"
                ));
            }
            (None, Some(_)) => {
                return Err(bad_request!("source `{name}` sets `revision` without `repository`"));
            }
            (None, None) => None,
        };

        let content = match (self.path, self.description) {
            (Some(_), Some(_)) => {
                return Err(bad_request!(
                    "source `{name}` sets both `path` and `description`; a source has one \
                     content key"
                ));
            }
            (_, Some(_)) if repository.is_some() => {
                return Err(bad_request!(
                    "source `{name}` sets both `repository` and `description`; a repository is \
                     read at a `path`"
                ));
            }
            (Some(relative), None) if repository.is_some() => {
                SourceContent::Workspace(relative.display().to_string())
            }
            (Some(relative), None) => {
                SourceContent::Workspace(preopen_join(base, &relative)?.display().to_string())
            }
            (None, Some(text)) => SourceContent::Value(text),
            (None, None) => SourceContent::Workspace(".".to_owned()),
        };

        Ok(SourceConfig {
            name,
            adapter,
            content,
            repository,
            digest: self.digest,
            rank: self.rank,
            scope: self.scope,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct TargetEntry {
    // Omitted, the adapter's package name, as on the command line.
    name: Option<String>,
    adapter: AdapterRef,
    digest: Option<Digest>,
    repository: Option<String>,
    budget: Option<NonZeroUsize>,
}

impl TargetEntry {
    fn argv(reference: &str) -> Result<Self, Error> {
        Ok(Self {
            name: None,
            adapter: AdapterRef::from_str(reference)?,
            digest: None,
            repository: None,
            budget: None,
        })
    }

    fn into_input(
        self, jobs: Option<NonZeroUsize>, flag_budget: Option<NonZeroUsize>,
    ) -> BuildInput {
        let name = self.name.unwrap_or_else(|| self.adapter.name().to_owned());
        let (budget, budget_source) = resolve_budget(flag_budget, self.budget);
        BuildInput {
            name,
            adapter: self.adapter,
            digest: self.digest,
            repository: self.repository,
            jobs,
            budget,
            budget_source,
        }
    }

    fn into_ref(self) -> TargetRef {
        TargetRef {
            name: self.name.unwrap_or_else(|| self.adapter.name().to_owned()),
            repository: self.repository,
        }
    }

    fn into_show(self, artifact: Artifact, flag_budget: Option<NonZeroUsize>) -> ShowInput {
        let Self {
            name,
            adapter,
            digest,
            repository,
            budget,
        } = self;
        let (budget, budget_source) = resolve_budget(flag_budget, budget);
        let target = TargetRef {
            name: name.unwrap_or_else(|| adapter.name().to_owned()),
            repository,
        };
        ShowInput {
            budget,
            budget_source,
            adapter: Some(adapter),
            digest,
            ..ShowInput::new(artifact, Some(target))
        }
    }
}

const fn resolve_budget(
    flag: Option<NonZeroUsize>, configured: Option<NonZeroUsize>,
) -> (Option<NonZeroUsize>, Option<BudgetSource>) {
    match (flag, configured) {
        (Some(budget), _) => (Some(budget), Some(BudgetSource::Flag)),
        (None, Some(budget)) => (Some(budget), Some(BudgetSource::Config)),
        (None, None) => (None, None),
    }
}
