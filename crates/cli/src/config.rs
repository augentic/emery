//! Resolves `specify` sources and `build` targets from CLI arguments or `emery.toml`.

use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::Context;
use emery_engine::build::BuildInput;
use emery_engine::specify::{SourceConfig, SourceContent, SourceRepository, SpecifyInput};
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
/// sources and no file reads none; one naming neither discovers the
/// project-root file.
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

        // the file, named or discovered, and the sources it states
        let file = match (config, argv) {
            (Some(path), _) => Some(Carrier::named(path)?),
            (None, false) => discover()?.map(Carrier::discovered),
            (None, true) => None,
        };
        let mut sources = match &file {
            Some(carrier) => sources_from(&carrier.path)?,
            None => Vec::new(),
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
        })
    }
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
        let (file, mut target) = match (config, adapter) {
            (Some(path), _) => {
                let carrier = Carrier::named(path)?;
                let target = target_from(&carrier.path)?;
                (Some(carrier), target)
            }
            (None, Some(reference)) => (None, TargetEntry::argv(reference)?),
            (None, None) => {
                let Some(path) = discover()? else {
                    return Err(target_required("no target adapter"));
                };
                let carrier = Carrier::discovered(path);
                let target = target_from(&carrier.path)?;
                (Some(carrier), target)
            }
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

fn sources_from(path: &Path) -> Result<Vec<SourceConfig>, Error> {
    let file = ConfigFile::read(path)?;
    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    file.source.into_iter().map(|entry| entry.into_config(base)).collect()
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
}
