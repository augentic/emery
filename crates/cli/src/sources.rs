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

use emery_engine::specify::{SourceConfig, SourceContent};
use emery_engine::{AdapterRef, preopen_path};
use omnia_guest::{Error, bad_request, server_error};

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
    adapters: &[String], descriptions: &[String], config: Option<&str>,
) -> Result<Vec<SourceConfig>, Error> {
    match config {
        Some(path) => {
            if !adapters.is_empty() || !descriptions.is_empty() {
                return Err(bad_request!(
                    "--config cannot be combined with `<adapter>` or `--description`"
                ));
            }

            let path = preopen_path(Path::new(path)).map_err(|err| {
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
    if !path.try_exists().map_err(|e| server_error!("reading {CONFIG_FILE}: {e}"))? {
        return Ok(Vec::new());
    }
    from_file(path)
}

// Builds the sources named on the command line: each positional adapter
// lends the workspace at `.`, each `--description` entry is an inline value,
// and the key is the adapter name.
fn from_argv(adapters: &[String], descriptions: &[String]) -> Result<Vec<SourceConfig>, Error> {
    let mut sources = Vec::new();
    for value in adapters {
        let adapter: AdapterRef = value.parse()?;
        sources.push(SourceConfig {
            key: adapter.name().to_owned(),
            adapter,
            content: SourceContent::Workspace(".".to_string()),
            digest: None,
            registry: None,
        });
    }

    for entry in descriptions {
        let Some((reference, text)) =
            entry.split_once('=').filter(|(reference, _)| !reference.is_empty())
        else {
            return Err(bad_request!(
                "invalid argument --description: expected `<adapter>=<text>`, got `{entry}`"
            ));
        };
        let adapter: AdapterRef = reference.parse()?;
        sources.push(SourceConfig {
            key: adapter.name().to_owned(),
            adapter,
            content: SourceContent::Value(text.to_string()),
            digest: None,
            registry: None,
        });
    }

    Ok(sources)
}

// Reads and decodes an operator-owned config file; any parse failure is
// refused, and the engine never writes the file.
fn from_file(path: &Path) -> Result<Vec<SourceConfig>, Error> {
    let raw = std::fs::read_to_string(path).map_err(|source| {
        let path = path.display();
        server_error!("reading {path}: {source}")
    })?;
    let file: ConfigFile = toml::from_str(&raw).map_err(|err| {
        let path = path.display();
        bad_request!("{path}: {err}")
    })?;

    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    file.source.iter().map(|entry| entry.decode(base)).collect()
}

// The operator-authored schema: ordered `[[source]]` entries whose
// `name` is the source key, with exactly one optional content key.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
struct ConfigFile {
    source: Vec<SourceEntry>,
}

// `name` and `adapter` are required; every other key is optional.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SourceEntry {
    name: String,
    adapter: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    git: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    registry: Option<String>,
    #[serde(default)]
    digest: Option<String>,
}

impl SourceEntry {
    // Decodes the entry into the engine's source, anchoring its relative
    // paths at `base`, the config file's directory.
    fn decode(&self, base: &Path) -> Result<SourceConfig, Error> {
        let name = &self.name;
        // A local component path resolves relative to the file, like Cargo
        // `path` dependencies; other reference kinds pass through unchanged.
        let adapter = match self.adapter.parse::<AdapterRef>()? {
            AdapterRef::Component { name, path } => AdapterRef::Component {
                name,
                path: resolved(base, &path)?,
            },
            other => other,
        };
        let digest = self
            .digest
            .as_deref()
            .map(|pin| pin.parse().map_err(|err| bad_request!("source `{name}`: {err}")))
            .transpose()?;

        let locations = [
            self.path.is_some(),
            self.git.is_some(),
            self.url.is_some(),
            self.description.is_some(),
        ];
        if locations.iter().filter(|present| **present).count() > 1 {
            return Err(bad_request!(
                "source `{name}` sets more than one of `path`, `git`, `url`, `description`"
            ));
        }
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

        let content = match (&self.path, &self.description) {
            (Some(relative), None) => {
                SourceContent::Workspace(resolved(base, Path::new(relative))?.display().to_string())
            }
            (None, Some(text)) => SourceContent::Value(text.clone()),
            (None, None) => SourceContent::Workspace(".".to_string()),
            (Some(_), Some(_)) => unreachable!("two content keys refused above"),
        };

        Ok(SourceConfig {
            key: name.clone(),
            adapter,
            content,
            digest,
            registry: self.registry.clone(),
        })
    }
}

// Anchors `relative` at the file's directory, refusing any path outside
// the `.` project preopen.
fn resolved(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative))
}
