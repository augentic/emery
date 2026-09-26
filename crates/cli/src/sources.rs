//! Decodes a `specify` run's sources and registries from its carriers.
//!
//! The carriers are argv — positional adapters and `--description` values —
//! and an operator-owned `emery.toml`, never both. A run naming its sources
//! on the command line still reads the project-root file's `[registries]`
//! table, and that table alone.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;
use emery_engine::specify::{SourceConfig, SourceContent};
use emery_engine::{AdapterRef, Registries, preopen_path};
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

/// Decodes the run's sources and registries from the `specify` arguments.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when `--config` is combined with positional
///   adapters or `--description` sources, or when any source is malformed.
/// - Returns [`Error::ServerError`] when a config file cannot be read.
pub fn decode(
    adapters: &[String], descriptions: &[String], config: Option<&Path>,
) -> Result<Decoded, Error> {
    let (carrier, decoded) = match config {
        Some(path) => {
            if !adapters.is_empty() || !descriptions.is_empty() {
                return Err(bad_request!(
                    "--config cannot be combined with `<adapter>` or `--description`"
                ));
            }

            let path = preopen_path(path).map_err(|err| {
                let description = err.description();
                bad_request!("invalid argument --config: {description}")
            })?;

            (path.display().to_string(), from_file(&path)?)
        }
        None if adapters.is_empty() && descriptions.is_empty() => {
            let decoded = match discover()? {
                Some(path) => from_file(path)?,
                None => Decoded::default(),
            };
            (CONFIG_FILE.to_string(), decoded)
        }
        // argv names the sources; the project-root file routes their packages
        None => {
            let sources = from_argv(adapters, descriptions)?;
            let registries = match discover()? {
                Some(path) => registries(path)?,
                None => Registries::default(),
            };
            ("argv".to_string(), Decoded { sources, registries })
        }
    };
    tracing::debug!(%carrier, sources = decoded.sources.len(), "sources decoded");

    Ok(decoded)
}

fn discover() -> Result<Option<&'static Path>, Error> {
    let path = Path::new(CONFIG_FILE);
    let found = path.try_exists().with_context(|| format!("reading {CONFIG_FILE}"))?;
    Ok(found.then_some(path))
}

fn from_argv(adapters: &[String], descriptions: &[String]) -> Result<Vec<SourceConfig>, Error> {
    let workspaces = adapters
        .iter()
        .map(|reference| source(reference, SourceContent::Workspace(".".to_string())));
    let values = descriptions.iter().map(|entry| {
        let (reference, text) = entry
            .split_once('=')
            .filter(|(reference, _)| !reference.is_empty())
            .ok_or_else(|| {
                bad_request!(
                    "invalid argument --description: expected `<adapter>=<text>`, got `{entry}`"
                )
            })?;
        source(reference, SourceContent::Value(text.to_string()))
    });

    workspaces.chain(values).collect()
}

fn source(reference: &str, content: SourceContent) -> Result<SourceConfig, Error> {
    let adapter = anchored(reference.parse()?, Path::new("."))?;
    Ok(SourceConfig {
        key: key(&adapter),
        adapter,
        content,
        digest: None,
    })
}

fn key(adapter: &AdapterRef) -> String {
    match adapter {
        AdapterRef::Static(name) | AdapterRef::Package { name, .. } => name.clone(),
        AdapterRef::File(path) => {
            let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
            stem.replace('_', "-")
        }
    }
}

fn anchored(adapter: AdapterRef, base: &Path) -> Result<AdapterRef, Error> {
    Ok(match adapter {
        AdapterRef::File(path) => AdapterRef::File(resolved(base, &path)?),
        other => other,
    })
}

fn from_file(path: &Path) -> Result<Decoded, Error> {
    let file: ConfigFile = parse(path)?;

    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let sources =
        file.source.into_iter().map(|entry| entry.decode(base)).collect::<Result<Vec<_>, _>>()?;

    Ok(Decoded {
        sources,
        registries: file.registries,
    })
}

// The `[[source]]` entries are skipped undecoded, so what they hold never
// refuses a run that named its own sources.
fn registries(path: &Path) -> Result<Registries, Error> {
    let file: ConfigFile<IgnoredAny> = parse(path)?;
    Ok(file.registries)
}

fn parse<File: DeserializeOwned>(path: &Path) -> Result<File, Error> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&raw).map_err(|err| {
        let path = path.display();
        bad_request!("{path}: {err}")
    })
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

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SourceEntry {
    // Omitted, the adapter's key, as on the command line.
    name: Option<String>,
    adapter: AdapterRef,
    path: Option<PathBuf>,
    description: Option<String>,
    digest: Option<Digest>,
}

impl SourceEntry {
    fn decode(self, base: &Path) -> Result<SourceConfig, Error> {
        let adapter = anchored(self.adapter, base)?;
        let name = self.name.unwrap_or_else(|| key(&adapter));
        let content = match (self.path, self.description) {
            (Some(_), Some(_)) => {
                return Err(bad_request!(
                    "source `{name}` sets both `path` and `description`; a source has one \
                     content key"
                ));
            }
            (Some(relative), None) => {
                SourceContent::Workspace(resolved(base, &relative)?.display().to_string())
            }
            (None, Some(text)) => SourceContent::Value(text),
            (None, None) => SourceContent::Workspace(".".to_string()),
        };

        Ok(SourceConfig {
            key: name,
            adapter,
            content,
            digest: self.digest,
        })
    }
}

fn resolved(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative))
}
