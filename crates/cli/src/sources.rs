//! Builds the source list of a `specify` run from the command line.
//!
//! An operator names adapters and inline descriptions directly, points at an
//! `emery.toml` with `--config`, or names nothing and lets the project-root
//! `emery.toml` be picked up. The list is an input to each run, never
//! something Emery stores, so this module is the only place that knows where
//! sources come from. A config file and command-line sources are refused
//! together rather than merged, so a run has exactly one source of truth.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;
use emery_engine::specify::{SourceConfig, SourceContent};
use emery_engine::{AdapterRef, preopen_path};
use omnia_sdk::{Error, bad_request};

/// The config file a run naming no sources looks for at the project root.
pub const CONFIG_FILE: &str = "emery.toml";

/// Decodes the run's source list from the `specify` arguments.
///
/// # Errors
///
/// Returns [`Error::BadRequest`] when `--config` is combined with positional
/// adapters or `--description` sources, or when any source is malformed, and
/// [`Error::ServerError`] when a config file cannot be read.
pub fn decode(
    adapters: &[String], descriptions: &[String], config: Option<&Path>,
) -> Result<Vec<SourceConfig>, Error> {
    match config {
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

            from_file(&path)
        }
        None if adapters.is_empty() && descriptions.is_empty() => discover(),
        None => from_argv(adapters, descriptions),
    }
}

// Reads the project-root `emery.toml` for a run that names no sources. A
// missing file yields the empty list, which the engine refuses as
// `specify-source-required`; a file that fails to parse is refused here.
fn discover() -> Result<Vec<SourceConfig>, Error> {
    let path = Path::new(CONFIG_FILE);
    if path.try_exists().with_context(|| format!("reading {CONFIG_FILE}"))? {
        from_file(path)
    } else {
        Ok(Vec::new())
    }
}

// Builds the sources named on the command line: each positional adapter
// lends the workspace at `.`, each `--description` entry is an inline value,
// and the key is the adapter's kebab stem.
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

// Builds the source a command-line reference names over `content`, keyed
// by the adapter's kebab stem.
fn source(reference: &str, content: SourceContent) -> Result<SourceConfig, Error> {
    let adapter = anchored(reference.parse()?, Path::new("."))?;
    Ok(SourceConfig {
        key: key(&adapter),
        adapter,
        content,
    })
}

// Derives the source key of a command-line adapter: its kebab stem,
// `intent` for `emery_intent.wasm` and `emery:intent@1.0.0` alike.
fn key(adapter: &AdapterRef) -> String {
    match adapter {
        AdapterRef::Static(name) => name.clone(),
        AdapterRef::Package(package) => {
            let rest = package.split_once(':').map_or(package.as_str(), |(_, rest)| rest);
            rest.split_once('@').map_or(rest, |(name, _)| name).to_string()
        }
        AdapterRef::File(path) => {
            let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
            let stem =
                stem.strip_prefix("emery_").or_else(|| stem.strip_prefix("emery-")).unwrap_or(stem);
            stem.replace('_', "-")
        }
    }
}

// Resolves a local component reference against `base`, the directory its
// path is written relative to, to the one project-relative path that is the
// adapter's identity; other reference kinds pass through.
fn anchored(adapter: AdapterRef, base: &Path) -> Result<AdapterRef, Error> {
    Ok(match adapter {
        AdapterRef::File(path) => AdapterRef::File(resolved(base, &path)?),
        other => other,
    })
}

// Reads and decodes an operator-owned config file; any parse failure is
// refused, and the engine never writes the file.
fn from_file(path: &Path) -> Result<Vec<SourceConfig>, Error> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let file: ConfigFile = toml::from_str(&raw).map_err(|err| {
        let path = path.display();
        bad_request!("{path}: {err}")
    })?;

    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    file.source.into_iter().map(|entry| entry.decode(base)).collect()
}

// The operator-authored schema: ordered `[[source]]` entries whose
// `name` is the source key, with exactly one optional content key.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
struct ConfigFile {
    source: Vec<SourceEntry>,
}

// `name` and `adapter` are required; every other key is optional. The
// adapter reference is parsed by the decoder, so a malformed one is
// refused with its line.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SourceEntry {
    name: String,
    adapter: AdapterRef,
    path: Option<PathBuf>,
    git: Option<String>,
    url: Option<String>,
    description: Option<String>,
}

impl SourceEntry {
    // Decodes the entry into the engine's source, anchoring its relative
    // paths at `base`, the config file's directory.
    fn decode(self, base: &Path) -> Result<SourceConfig, Error> {
        let name = self.name;
        if let Some(remote) = self.git.as_deref().or(self.url.as_deref()) {
            if remote.starts_with("git+") {
                return Err(bad_request!(
                    "source `{name}`: drop the `git+` prefix and write the plain URL"
                ));
            }
            return Err(bad_request!(
                "source `{name}`: `git` and `url` are not supported; use `path` or `description`"
            ));
        }

        // A local component path resolves relative to the file, like Cargo
        // `path` dependencies.
        let adapter = anchored(self.adapter, base)?;
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
        })
    }
}

// Anchors `relative` at the file's directory, refusing any path outside
// the `.` project preopen.
fn resolved(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative))
}
