//! Decodes a `specify` run's sources and registries from its carriers.
//!
//! The carriers are argv — positional adapters and `--description` values —
//! and an operator-owned `emery.toml`, never both. A run naming its sources
//! on the command line still reads the project-root file's `[registries]`
//! table, and that table alone.

use std::convert::{TryFrom, TryInto};
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use anyhow::Context;
use emery_engine::specify::{SourceConfig, SourceContent};
use emery_engine::{AdapterRef, Registries, preopen_join, preopen_path};
use omnia_sdk::plugins::Digest;
use omnia_sdk::{Error, bad_request};
use serde::de::{DeserializeOwned, IgnoredAny};

/// The config file a run naming no sources looks for at the project root.
pub const CONFIG_FILE: &str = "emery.toml";

/// What a run's carriers decode.
#[derive(Debug, Default)]
pub struct Decoded {
    /// The sources, in declaration order.
    pub sources: Vec<SourceConfig>,
    /// The registries the run's package adapters fetch from.
    pub registries: Registries,
}

/// The mutually exclusive ways a run names its sources.
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
                let path = preopen_path(path).map_err(|err| {
                    let description = err.description();
                    bad_request!("invalid argument --config: {description}")
                })?;
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

                let registries = match discover()? {
                    Some(path) => registries_from(path)?,
                    None => Registries::default(),
                };
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
        let file: ConfigFile = ConfigFile::read(path)?;

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

fn description(entry: &str) -> Result<(&str, &str), Error> {
    entry.split_once('=').filter(|(reference, _)| !reference.is_empty()).ok_or_else(|| {
        bad_request!("invalid argument --description: expected `<adapter>=<text>`, got `{entry}`")
    })
}

fn argv_source(reference: &str, content: SourceContent) -> Result<SourceConfig, Error> {
    let adapter = AdapterRef::from_str(reference)?.anchored_to(Path::new("."))?;
    Ok(SourceConfig {
        name: adapter.name()?,
        adapter,
        content,
        digest: None,
    })
}

// The `[[source]]` entries are skipped undecoded, so what they hold never
// refuses a run that named its own sources.
fn registries_from(path: &Path) -> Result<Registries, Error> {
    Ok(ConfigFile::<IgnoredAny>::read(path)?.registries)
}

fn discover() -> Result<Option<&'static Path>, Error> {
    let path = Path::new(CONFIG_FILE);
    let found = path.try_exists().with_context(|| format!("reading {CONFIG_FILE}"))?;
    Ok(found.then_some(path))
}

// `Sources` is the shape the `[[source]]` entries are read as: decoded, or
// skipped by a run reading the file for its table alone.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
struct ConfigFile<Sources = Vec<SourceEntry>> {
    source: Sources,
    registries: Registries,
}

impl<File: DeserializeOwned + Default> ConfigFile<File> {
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
    adapter: AdapterRef,
    path: Option<PathBuf>,
    description: Option<String>,
    digest: Option<Digest>,
}

#[derive(Clone, Copy)]
struct ConfigBase<'a>(&'a Path);

impl SourceEntry {
    fn try_into_config(self, base: ConfigBase<'_>) -> Result<SourceConfig, Error> {
        let ConfigBase(base) = base;
        let adapter = self.adapter.anchored_to(base)?;
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
