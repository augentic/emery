//! Resolves `specify` sources and the target from CLI arguments or `emery.toml`.

use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::Context;
use emery_engine::build::BuildInput;
use emery_engine::specify::{Scope, SourceConfig, SourceContent, SourceRepository, SpecifyInput};
use emery_engine::vcs::TargetRef;
use emery_engine::{AdapterRef, Rank, preopen_join, preopen_path};
use omnia_sdk::plugins::Digest;
use omnia_sdk::{Error, bad_request};

/// The config file a run naming no adapters looks for at the project root.
pub const CONFIG_FILE: &str = "emery.toml";

/// Positional adapters, `--description` values, and an optional `--config` path.
///
/// The file's `[[source]]` tables are the base and the argv sources layer
/// over them: an argv source takes the place of the file source of its
/// name, whole, and is appended otherwise, in argv order. A run naming
/// sources and no file reads no file; one naming neither discovers the
/// project-root file. The `[target]` table of the file read is the run's
/// target, the repository the specification lives in, so a run naming its
/// sources on the command line knows it only under `--config`.
pub struct SourceCarriers<'a> {
    /// Workspace-backed adapter references from argv.
    pub adapters: &'a [String],
    /// Inline `--description <adapter>=<text>` entries.
    pub descriptions: &'a [String],
    /// When set, names the config file the argv sources layer over.
    pub config: Option<&'a Path>,
}

impl TryFrom<SourceCarriers<'_>> for SpecifyInput {
    type Error = Error;

    fn try_from(carriers: SourceCarriers<'_>) -> Result<Self, Error> {
        let SourceCarriers {
            adapters,
            descriptions,
            config,
        } = carriers;
        let argv = !adapters.is_empty() || !descriptions.is_empty();

        // the file, named or discovered, and the sources and target it states
        let file = if config.is_none() && argv { None } else { Carrier::resolve(config)? };
        let (mut sources, target) = match &file {
            Some(carrier) => ConfigFile::read(&carrier.path)?.into_parts(&carrier.path)?,
            None => (Vec::new(), None),
        };
        let held = sources.len();

        // the argv sources, each in the place of the file source of its name
        let mut over = Vec::with_capacity(adapters.len() + descriptions.len());
        for reference in adapters {
            over.push(argv_source(reference, SourceContent::Workspace(".".into()))?);
        }
        for entry in descriptions {
            let (reference, text) = description(entry)?;
            over.push(argv_source(reference, SourceContent::Value(text.into()))?);
        }
        let mut replaced = vec![false; held];
        for source in over {
            let place = (0..held).find(|&at| !replaced[at] && sources[at].name == source.name);
            match place {
                Some(at) => {
                    sources[at] = source;
                    replaced[at] = true;
                }
                None => sources.push(source),
            }
        }

        let label = file.as_ref().map_or("argv", |carrier| carrier.label.as_str());
        tracing::debug!(carrier = %label, sources = sources.len(), "sources decoded");

        Ok(Self {
            sources,
            apply: false,
            target,
        })
    }
}

/// The target whose repository the specification lives in, as `apply`, `show`, and `specify` read it.
///
/// The `[target]` table of `config`, or of the discovered project-root file
/// when `config` names none; `repository` takes the place of its
/// `repository`. `None` when no file states a target, so the verb decides
/// whether it needs one.
///
/// # Errors
///
/// Returns [`Error::BadRequest`] when the file cannot be read or decoded.
pub fn target(config: Option<&Path>, repository: Option<&str>) -> Result<Option<TargetRef>, Error> {
    let Some(carrier) = Carrier::resolve(config)? else {
        return Ok(None);
    };
    let Some(mut entry) = ConfigFile::read(&carrier.path)?.target else {
        return Ok(None);
    };
    if let Some(repository) = repository {
        entry.repository = Some(repository.to_owned());
    }
    Ok(Some(entry.into_ref()))
}

/// The `build-target-required` refusal of `verb`, which reads or seals the current revision and was handed no `[target]`.
pub fn missing_target(verb: &str) -> Error {
    target_required(&format!(
        "the specification has no repository to live in: {verb} needs a `[target]` table \
         naming one"
    ))
}

/// A positional target adapter, a `--repository`, and an optional `--config` path.
///
/// The file's `[target]` table is the base and the flags layer over it: an
/// argv adapter takes the place of its `adapter`, dropping the `digest` that
/// pinned the file's, under the file's `name` where it states one; a
/// `--repository` takes the place of its `repository`. A run naming an
/// adapter and no file reads none; one naming no adapter discovers the
/// project-root file.
pub struct TargetCarriers<'a> {
    /// The target adapter reference from argv.
    pub adapter: Option<&'a str>,
    /// The `--repository` the build lands in: a URL, or `.` for the project checkout.
    pub repository: Option<&'a str>,
    /// When set, names the config file the flags layer over.
    pub config: Option<&'a Path>,
}

impl TryFrom<TargetCarriers<'_>> for BuildInput {
    type Error = Error;

    fn try_from(carriers: TargetCarriers<'_>) -> Result<Self, Error> {
        let TargetCarriers {
            adapter,
            repository,
            config,
        } = carriers;

        // the file, named or discovered, and the target it states
        let (file, mut target) = if let (None, Some(reference)) = (config, adapter) {
            (None, TargetEntry::argv(reference)?)
        } else {
            let carrier =
                Carrier::resolve(config)?.ok_or_else(|| target_required("no target adapter"))?;
            let target = target_from(&carrier.path)?;
            (Some(carrier), target)
        };

        // the flags, each in the place of the file's key
        if let Some(reference) = adapter {
            target.adapter = AdapterRef::from_str(reference)?;
            target.digest = None;
        }
        if let Some(repository) = repository {
            target.repository = Some(repository.to_owned());
        }

        let input = target.into_input();
        let label = file.as_ref().map_or("argv", |carrier| carrier.label.as_str());
        tracing::debug!(carrier = %label, adapter = %input.adapter, "target decoded");

        Ok(input)
    }
}

// The config file a run reads, and the name the log knows it by.
struct Carrier {
    path: PathBuf,
    label: String,
}

impl Carrier {
    fn resolve(config: Option<&Path>) -> Result<Option<Self>, Error> {
        match config {
            Some(path) => Self::named(path).map(Some),
            None => Ok(discover()?.map(Self::discovered)),
        }
    }

    fn named(path: &Path) -> Result<Self, Error> {
        let path = config_path(path)?;
        Ok(Self {
            label: path.display().to_string(),
            path,
        })
    }

    fn discovered(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            label: CONFIG_FILE.to_owned(),
        }
    }
}

fn target_from(path: &Path) -> Result<TargetEntry, Error> {
    let file = ConfigFile::read(path)?;
    file.target
        .ok_or_else(|| target_required(&format!("{} has no `[target]` table", path.display())))
}

fn target_required(description: &str) -> Error {
    Error::BadRequest {
        code: "build-target-required".into(),
        description: description.to_owned(),
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

fn discover() -> Result<Option<&'static Path>, Error> {
    let path = Path::new(CONFIG_FILE);
    let found = path.try_exists().with_context(|| format!("reading {CONFIG_FILE}"))?;
    Ok(found.then_some(path))
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields, default)]
struct ConfigFile {
    source: Vec<SourceEntry>,
    target: Option<TargetEntry>,
}

impl ConfigFile {
    fn read(path: &Path) -> Result<Self, Error> {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw).map_err(|err| {
            let path = path.display();
            bad_request!("{path}: {err}")
        })
    }

    fn into_parts(self, path: &Path) -> Result<(Vec<SourceConfig>, Option<TargetRef>), Error> {
        let base = path
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
            (None, None) => SourceContent::Workspace(".".to_string()),
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
}

impl TargetEntry {
    fn argv(reference: &str) -> Result<Self, Error> {
        Ok(Self {
            name: None,
            adapter: AdapterRef::from_str(reference)?,
            digest: None,
            repository: None,
        })
    }

    fn into_input(self) -> BuildInput {
        let adapter = self.adapter;
        BuildInput {
            name: self.name.unwrap_or_else(|| adapter.name().to_owned()),
            adapter,
            digest: self.digest,
            repository: self.repository,
            jobs: None,
        }
    }

    fn into_ref(self) -> TargetRef {
        TargetRef {
            name: self.name.unwrap_or_else(|| self.adapter.name().to_owned()),
            repository: self.repository,
        }
    }
}
