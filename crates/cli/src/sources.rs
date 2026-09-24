//! Builds a `specify` source list from command-line arguments.
//!
//! Sources may come from positional adapters, inline descriptions, or an
//! `emery.toml` file, which also carries the `[registries]` table package
//! adapters route through. Configuration files cannot be combined with direct
//! command-line sources. When no source is specified, the project-root
//! `emery.toml` is used if present; a run naming its sources on the command
//! line still reads that file's `[registries]` table.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::Context;
use emery_engine::specify::{SourceConfig, SourceContent};
use emery_engine::{AdapterRef, Registries, preopen_path};
use omnia_sdk::plugins::Digest;
use omnia_sdk::{Error, bad_request};

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
            (CONFIG_FILE.to_string(), discover()?)
        }
        // argv names the sources; the project-root file routes their packages
        None => {
            let sources = from_argv(adapters, descriptions)?;
            let registries = discover()?.registries;
            ("argv".to_string(), Decoded { sources, registries })
        }
    };
    tracing::debug!(%carrier, sources = decoded.sources.len(), "sources decoded");

    Ok(decoded)
}

// Reads the project-root `emery.toml`. A missing file yields nothing — an
// empty source list, which the engine refuses as `specify-source-required`
// when it is the run's only carrier; a file that fails to parse is refused
// here.
fn discover() -> Result<Decoded, Error> {
    let path = Path::new(CONFIG_FILE);
    if path.try_exists().with_context(|| format!("reading {CONFIG_FILE}"))? {
        from_file(path)
    } else {
        Ok(Decoded::default())
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
        digest: None,
    })
}

// Derives the source key of an adapter: the bare name, a package's name
// (`intent` for `emery:intent@1.0.0`), or a component file's stem kebab-cased
// (`intent` for `intent.wasm`, `my-adapter` for `my_adapter.wasm`).
fn key(adapter: &AdapterRef) -> String {
    match adapter {
        AdapterRef::Static(name) | AdapterRef::Package { name, .. } => name.clone(),
        AdapterRef::File(path) => {
            let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
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
fn from_file(path: &Path) -> Result<Decoded, Error> {
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
    let sources =
        file.source.into_iter().map(|entry| entry.decode(base)).collect::<Result<Vec<_>, _>>()?;

    Ok(Decoded {
        sources,
        registries: file.registries,
    })
}

// The operator-authored schema: ordered `[[source]]` entries, each with
// exactly one optional content key, and the `[registries]` table routing
// package namespaces. An unknown key is refused with its name and line.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(default)]
struct ConfigFile {
    source: Vec<SourceEntry>,
    registries: Registries,
}

// `adapter` is required; every other key is optional. The adapter reference
// is parsed by the decoder, so a malformed one is refused with its line.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SourceEntry {
    // The source key; omitted, the adapter's key, as on the command line.
    name: Option<String>,
    adapter: AdapterRef,
    path: Option<PathBuf>,
    description: Option<String>,
    // The `sha256:` pin the adapter's component must resolve to.
    digest: Option<Digest>,
}

impl SourceEntry {
    // Decodes the entry into the engine's source, anchoring its relative
    // paths at `base`, the config file's directory.
    fn decode(self, base: &Path) -> Result<SourceConfig, Error> {
        // A local component path resolves relative to the file, like Cargo
        // `path` dependencies.
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

// Anchors `relative` at the file's directory, refusing any path outside
// the `.` project preopen.
fn resolved(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative))
}
