//! Decodes a run's sources, target, and registries from its carriers.
//!
//! The carriers are argv — positional adapters and `--description` values
//! for `specify`, one positional adapter for `build` — and an operator-owned
//! `emery.toml`, never both. A run naming its adapters on the command line
//! still reads the project-root file's `[registries]` table, and that table
//! alone.

use std::convert::{TryFrom, TryInto};
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::Context;
use emery_engine::build::BuildInput;
use emery_engine::specify::{SourceConfig, SourceContent};
use emery_engine::{AdapterRef, Registries, preopen_join, preopen_path};
use omnia_sdk::plugins::Digest;
use omnia_sdk::{Error, bad_request};
use serde::de::{DeserializeOwned, IgnoredAny};

/// The config file a run naming no adapters looks for at the project root.
pub const CONFIG_FILE: &str = "emery.toml";

/// What a `specify` run's carriers decode.
#[derive(Debug, Default)]
pub struct Decoded {
    /// The sources, in declaration order.
    pub sources: Vec<SourceConfig>,
    /// The registries the run's package adapters fetch from.
    pub registries: Registries,
}

enum Carrier<'a> {
    Config(&'a Path),
    DiscoverOrEmpty,
    Argv { adapters: &'a [String], descriptions: &'a [String] },
}

/// Positional adapters, `--description` values, and an optional `--config` path.
pub struct SourceCarriers<'a> {
    /// Workspace-backed adapter references from argv.
    pub adapters: &'a [String],
    /// Inline `--description <adapter>=<text>` entries.
    pub descriptions: &'a [String],
    /// When set, names the config file and forbids argv sources.
    pub config: Option<&'a Path>,
}

impl<'a> TryFrom<SourceCarriers<'a>> for Carrier<'a> {
    type Error = Error;

    fn try_from(carriers: SourceCarriers<'a>) -> Result<Self, Error> {
        let SourceCarriers {
            adapters,
            descriptions,
            config,
        } = carriers;
        match config {
            Some(path) => {
                if !adapters.is_empty() || !descriptions.is_empty() {
                    return Err(bad_request!(
                        "--config cannot be combined with `<adapter>` or `--description`"
                    ));
                }
                Ok(Self::Config(path))
            }
            None if adapters.is_empty() && descriptions.is_empty() => Ok(Self::DiscoverOrEmpty),
            None => Ok(Self::Argv {
                adapters,
                descriptions,
            }),
        }
    }
}

impl<'a> TryFrom<SourceCarriers<'a>> for Decoded {
    type Error = Error;

    fn try_from(carriers: SourceCarriers<'a>) -> Result<Self, Error> {
        Carrier::try_from(carriers)?.try_into()
    }
}

impl TryFrom<Carrier<'_>> for Decoded {
    type Error = Error;

    fn try_from(carrier: Carrier<'_>) -> Result<Self, Error> {
        let (label, decoded) = match carrier {
            Carrier::Config(path) => {
                let path = config_path(path)?;
                (path.display().to_string(), Self::try_from(path.as_path())?)
            }
            Carrier::DiscoverOrEmpty => {
                let decoded = match discover()? {
                    Some(path) => Self::try_from(path)?,
                    None => Self::default(),
                };
                (CONFIG_FILE.to_string(), decoded)
            }
            Carrier::Argv {
                adapters,
                descriptions,
            } => {
                let mut sources = Vec::with_capacity(adapters.len() + descriptions.len());

                for reference in adapters {
                    sources.push(argv_source(reference, SourceContent::Workspace(".".into()))?);
                }
                for entry in descriptions {
                    let (reference, text) = description(entry)?;
                    sources.push(argv_source(reference, SourceContent::Value(text.into()))?);
                }

                let registries = project_registries()?;
                ("argv".to_string(), Self { sources, registries })
            }
        };
        tracing::debug!(carrier = %label, sources = decoded.sources.len(), "sources decoded");

        Ok(decoded)
    }
}

impl TryFrom<&Path> for Decoded {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Error> {
        let file: ConfigFile<Vec<SourceEntry>, IgnoredAny> = ConfigFile::read(path)?;

        let base = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let base = ConfigBase(base);
        let sources = file
            .source
            .into_iter()
            .map(|entry| entry.try_into_config(base))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            sources,
            registries: file.registries,
        })
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
                    registries: project_registries()?,
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

fn target_from(path: &Path) -> Result<BuildInput, Error> {
    let file: ConfigFile<IgnoredAny, Option<TargetEntry>> = ConfigFile::read(path)?;
    let Some(target) = file.target else {
        return Err(target_required(&format!("{} has no `[target]` table", path.display())));
    };
    Ok(BuildInput {
        adapter: target.adapter,
        digest: target.digest,
        registries: file.registries,
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
        name: adapter.name()?,
        adapter,
        content,
        digest: None,
    })
}

// The project-root file's `[registries]` alone: its `[[source]]` entries and
// `[target]` table are skipped undecoded, so what they hold never refuses a
// run that named its own adapters.
fn project_registries() -> Result<Registries, Error> {
    Ok(match discover()? {
        Some(path) => ConfigFile::<IgnoredAny, IgnoredAny>::read(path)?.registries,
        None => Registries::default(),
    })
}

fn discover() -> Result<Option<&'static Path>, Error> {
    let path = Path::new(CONFIG_FILE);
    let found = path.try_exists().with_context(|| format!("reading {CONFIG_FILE}"))?;
    Ok(found.then_some(path))
}

// `Sources` and `Target` are the shapes the `[[source]]` entries and the
// `[target]` table are read as: decoded, or skipped by a run reading the
// file for another part alone.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
struct ConfigFile<Sources, Target> {
    source: Sources,
    target: Target,
    registries: Registries,
}

impl<Sources: DeserializeOwned + Default, Target: DeserializeOwned + Default>
    ConfigFile<Sources, Target>
{
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
    // Omitted, the adapter's name, as on the command line.
    name: Option<String>,
    // A local component is beneath the adapters root, never beside the file.
    adapter: AdapterRef,
    path: Option<PathBuf>,
    description: Option<String>,
    digest: Option<Digest>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct TargetEntry {
    adapter: AdapterRef,
    digest: Option<Digest>,
}

#[derive(Clone, Copy)]
struct ConfigBase<'a>(&'a Path);

impl SourceEntry {
    fn try_into_config(self, base: ConfigBase<'_>) -> Result<SourceConfig, Error> {
        let ConfigBase(base) = base;
        let adapter = self.adapter;
        let name = match self.name {
            Some(name) => name,
            None => adapter.name()?,
        };

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
        })
    }
}
