//! Sources from the command line
//!
//! Builds the list of sources a `specify` run works from. An
//! operator can name adapters and inline descriptions directly on the
//! command line, point at an `emery.toml` with `--config`, or name nothing
//! and let the project-root `emery.toml` be picked up.
//!
//! The source list is an input to each run, never something Emery stores,
//! so this module is the only place that knows where sources come from.
//! Mixing a config file with command-line sources is refused rather than
//! merged, so a run has exactly one source of truth.

use std::path::{Path, PathBuf};

use anyhow::Context;
use emery_engine::specify::{SourceConfig, SourceContent};
use emery_engine::{AdapterRef, preopen_path};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, bad_request};

/// The project-root config discovered by a run naming no sources.
pub const CONFIG_FILE: &str = "emery.toml";

/// Decodes the run's source list from the `specify` arguments.
///
/// # Errors
///
/// Returns a `BadRequest` when `--config` is mixed with positional
/// adapters or `--description` sources, and propagates the argv,
/// file, and discovery decoder failures.
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
// and the key is the adapter name.
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

// Builds the source a command-line reference names: the adapter, unpinned
// and keyed by its name, over `content`.
fn source(reference: &str, content: SourceContent) -> Result<SourceConfig, Error> {
    let adapter: AdapterRef = reference.parse()?;
    Ok(SourceConfig {
        key: adapter.name().to_string(),
        adapter,
        content,
        digest: None,
        registry: None,
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
// adapter reference and the digest pin are parsed by the decoder, so a
// malformed one is refused with its line.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SourceEntry {
    name: String,
    adapter: AdapterRef,
    path: Option<PathBuf>,
    git: Option<String>,
    url: Option<String>,
    description: Option<String>,
    registry: Option<String>,
    digest: Option<Digest>,
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
        // `path` dependencies; other reference kinds pass through unchanged.
        let adapter = match self.adapter {
            AdapterRef::Component { name, path } => AdapterRef::Component {
                name,
                path: resolved(base, &path)?,
            },
            other => other,
        };
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
            registry: self.registry,
        })
    }
}

// Anchors `relative` at the file's directory, refusing any path outside
// the `.` project preopen.
fn resolved(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative))
}
