//! Decodes a run's sources and target from its carriers.
//!
//! The carriers are argv — positional adapters and `--description` values
//! for `specify`, one positional adapter for `build` — and an operator-owned
//! `emery.toml`, never both. The file is read whole, so every table it holds
//! must parse; a run naming its adapters on the command line reads no file.

use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::Context;
use emery_engine::build::BuildInput;
use emery_engine::specify::{SourceConfig, SourceContent, SpecifyInput};
use emery_engine::{AdapterRef, Rank, preopen_join, preopen_path};
use omnia_sdk::plugins::Digest;
use omnia_sdk::{Error, bad_request};

/// The config file a run naming no adapters looks for at the project root.
pub const CONFIG_FILE: &str = "emery.toml";

/// Positional adapters, `--description` values, and an optional `--config` path.
pub struct SourceCarriers<'a> {
    /// Workspace-backed adapter references from argv.
    pub adapters: &'a [String],
    /// Inline `--description <adapter>=<text>` entries.
    pub descriptions: &'a [String],
    /// When set, names the config file and forbids argv sources.
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
        let (label, input) = match (argv, config) {
            (true, Some(_)) => {
                return Err(bad_request!(
                    "--config cannot be combined with `<adapter>` or `--description`"
                ));
            }
            (true, None) => {
                let mut sources = Vec::with_capacity(adapters.len() + descriptions.len());
                for reference in adapters {
                    sources.push(argv_source(reference, SourceContent::Workspace(".".into()))?);
                }
                for entry in descriptions {
                    let (reference, text) = description(entry)?;
                    sources.push(argv_source(reference, SourceContent::Value(text.into()))?);
                }
                ("argv".to_string(), Self { sources })
            }
            (false, Some(path)) => {
                let path = config_path(path)?;
                (path.display().to_string(), sources_from(&path)?)
            }
            (false, None) => {
                let input = match discover()? {
                    Some(path) => sources_from(path)?,
                    None => Self { sources: Vec::new() },
                };
                (CONFIG_FILE.to_string(), input)
            }
        };
        tracing::debug!(carrier = %label, sources = input.sources.len(), "sources decoded");

        Ok(input)
    }
}

/// A positional target adapter and an optional `--config` path.
pub struct TargetCarriers<'a> {
    /// The target adapter reference from argv.
    pub adapter: Option<&'a str>,
    /// When set, names the config file and forbids an argv adapter.
    pub config: Option<&'a Path>,
}

impl TryFrom<TargetCarriers<'_>> for BuildInput {
    type Error = Error;

    fn try_from(carriers: TargetCarriers<'_>) -> Result<Self, Error> {
        let (label, input) = match (carriers.adapter, carriers.config) {
            (Some(_), Some(_)) => {
                return Err(bad_request!("--config cannot be combined with `<adapter>`"));
            }
            (Some(reference), None) => {
                let input = Self {
                    adapter: AdapterRef::from_str(reference)?,
                    digest: None,
                };
                ("argv".to_string(), input)
            }
            (None, Some(path)) => {
                let path = config_path(path)?;
                (path.display().to_string(), target_from(&path)?)
            }
            (None, None) => {
                let Some(path) = discover()? else {
                    return Err(target_required("no target adapter"));
                };
                (CONFIG_FILE.to_string(), target_from(path)?)
            }
        };
        tracing::debug!(carrier = %label, adapter = %input.adapter, "target decoded");

        Ok(input)
    }
}

fn sources_from(path: &Path) -> Result<SpecifyInput, Error> {
    let file = ConfigFile::read(path)?;
    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let sources = file
        .source
        .into_iter()
        .map(|entry| entry.into_config(base))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpecifyInput { sources })
}

fn target_from(path: &Path) -> Result<BuildInput, Error> {
    let file = ConfigFile::read(path)?;
    let Some(target) = file.target else {
        return Err(target_required(&format!("{} has no `[target]` table", path.display())));
    };
    Ok(BuildInput {
        adapter: target.adapter,
        digest: target.digest,
    })
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
    digest: Option<Digest>,
    rank: Option<Rank>,
}

impl SourceEntry {
    // `base` is the directory the file's `path` keys are relative to.
    fn into_config(self, base: &Path) -> Result<SourceConfig, Error> {
        let adapter = self.adapter;
        let name = self.name.unwrap_or_else(|| adapter.name().to_owned());

        let content = match (self.path, self.description) {
            (Some(_), Some(_)) => {
                return Err(bad_request!(
                    "source `{name}` sets both `path` and `description`; a source has one \
                     content key"
                ));
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
            digest: self.digest,
            rank: self.rank,
        })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct TargetEntry {
    adapter: AdapterRef,
    digest: Option<Digest>,
}
