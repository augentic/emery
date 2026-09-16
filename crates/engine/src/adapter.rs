//! How an operator names an adapter, and how the engine loads it.
//!
//! An [`AdapterRef`] is a registry package, a guest the deployment declares,
//! or a local `.wasm` file. [`load`] brings every adapter a run's sources name
//! into the run — each once, all together — refuses any that requires a newer
//! Emery than the one running, and reads the kind of source each declares,
//! which ranks its evidence before any extract.
//!
//! The reference is the identity: the plugin loader registers an adapter, and
//! the `Source` capability dispatches to it, by the [`AdapterRef`]'s
//! `Display`.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use emery_adapter::is_kebab;
use emery_adapter::source::{Source, SourceKind};
use futures::future;
use omnia_sdk::plugins::{Location, PluginRef};
use omnia_sdk::{Error, Plugins, bad_request, not_found};
use serde::{Deserialize, Serialize};

use crate::preopen_path;

/// Loads the `adapters` and returns the kind of source each declares, by id.
///
/// An adapter several sources share is loaded once, and every adapter is
/// loaded before any is read. Each adapter's metadata is read once: it gates
/// the Emery version the adapter requires and supplies its kind.
///
/// # Errors
///
/// - [`Error::BadRequest`] for a file reference that escapes the project, an
///   adapter with a malformed `emery-version`, or one that requires a newer
///   Emery than this (code `unsupported-version`).
/// - [`Error::NotFound`] for a file reference that names no file.
///
/// Plugin load failures pass through with their own class.
pub async fn load<'a, P: Source + Plugins>(
    provider: &P, adapters: impl IntoIterator<Item = &'a AdapterRef>,
) -> Result<BTreeMap<String, SourceKind>, Error> {
    let mut ids = Vec::new();
    let mut plugins = Vec::new();

    for adapter in adapters {
        let id = adapter.to_string();
        if ids.contains(&id) {
            continue;
        }

        let location = match adapter {
            AdapterRef::Static(_) => None,
            // A package names no endpoint: the deployment's registry policy
            // resolves its namespace, so no project file can redirect a load.
            AdapterRef::Package(_) => Some(Location::Registry(None)),
            AdapterRef::File(path) => {
                let local = preopen_path(path)?;
                if !local.is_file() {
                    return Err(not_found!("adapter `{id}` not found"));
                }
                Some(Location::Path(local.display().to_string()))
            }
        };

        if let Some(location) = location {
            plugins.push(PluginRef::builder().package(id.as_str()).location(location).build());
        }
        ids.push(id);
    }

    // load all adapters in parallel
    let loaders = plugins.iter().map(|plugin| Plugins::load(provider, plugin));
    for loaded in future::join_all(loaders).await {
        loaded?;
    }

    let version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .with_context(|| format!("issue with emery version `{}`", env!("CARGO_PKG_VERSION")))?;

    // gate each adapter's version and record its kind
    let mut kinds = BTreeMap::new();
    for id in ids {
        let metadata = Source::metadata(provider, &id);
        if let Some(declared) = &metadata.emery_version {
            is_supported(&id, declared, &version)?;
        }
        kinds.insert(id, metadata.kind);
    }

    Ok(kinds)
}

// Refuses an adapter whose declared minimum `emery-version` the running
// binary does not meet.
fn is_supported(id: &str, declared: &str, running: &semver::Version) -> Result<(), Error> {
    let minimum = semver::Version::parse(declared).map_err(|err| {
        bad_request!("adapter `{id}` has an invalid `emery-version` `{declared}`: {err}")
    })?;

    if *running < minimum {
        return Err(Error::BadRequest {
            code: "unsupported-version".into(),
            description: format!("adapter `{id}` requires emery {minimum} or newer"),
        });
    }

    Ok(())
}

/// How an operator names an adapter, and the adapter's identity.
///
/// On the wire it is the operator's string. In memory it is that string
/// normalised — `intent@1.0.0` becomes `emery:intent@1.0.0`, and a `file://`
/// prefix is dropped — and `Display` gives the normalised string back. The
/// plugin loader and the `Source` capability address the adapter by it.
///
/// # Examples
///
/// ```
/// use emery_engine::AdapterRef;
///
/// let package: AdapterRef = "intent@1.0.0".parse()?;
/// assert_eq!(package.to_string(), "emery:intent@1.0.0");
///
/// let file: AdapterRef = "file://./intent.wasm".parse()?;
/// assert_eq!(file.to_string(), "./intent.wasm");
///
/// let declared: AdapterRef = "intent".parse()?;
/// assert_eq!(declared.to_string(), "intent");
/// # Ok::<(), omnia_sdk::Error>(())
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum AdapterRef {
    /// A project-relative path to a `.wasm` component.
    File(PathBuf),
    /// A registry package, always `<namespace>:<name>@<version>`.
    Package(String),
    /// A guest the deployment declares, named bare (`intent`).
    Static(String),
}

impl Display for AdapterRef {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Package(text) | Self::Static(text) => f.write_str(text),
            Self::File(path) => write!(f, "{}", path.display()),
        }
    }
}

impl FromStr for AdapterRef {
    type Err = Error;

    /// Parses `./intent.wasm`, `intent`, `emery:intent@1.0.0`, or the
    /// shorthand `intent@1.0.0`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] for an empty value, a GitHub URL, or a
    /// malformed package reference.
    fn from_str(value: &str) -> Result<Self, Error> {
        let value = value.trim();
        if value.is_empty() {
            return Err(bad_request!("adapter reference is empty"));
        }
        if value.starts_with("https://github.com/") {
            return Err(bad_request!("adapter `{value}`: GitHub URLs are not supported"));
        }
        let path = value.strip_prefix("file://").unwrap_or(value);
        if Path::new(path).extension().is_some_and(|ext| ext == "wasm") {
            return Ok(Self::File(PathBuf::from(path)));
        }
        if is_kebab(value) {
            return Ok(Self::Static(value.to_string()));
        }

        // `<namespace>:<name>@<version>`; the namespace defaults to `emery`.
        let (namespace, rest) = value.split_once(':').unwrap_or(("emery", value));
        let (name, version) = rest
            .split_once('@')
            .ok_or_else(|| bad_request!("adapter `{value}` is missing `@<version>`"))?;
        if name.is_empty() {
            return Err(bad_request!("adapter `{value}` is missing a name before `@`"));
        }
        if !is_kebab(namespace) || !is_kebab(name) {
            return Err(bad_request!("adapter `{value}` is not `<namespace>:<name>@<version>`"));
        }
        let version = semver::Version::parse(version).map_err(|err| {
            bad_request!("adapter `{value}` has an invalid version `{version}`: {err}")
        })?;

        Ok(Self::Package(format!("{namespace}:{name}@{version}")))
    }
}

// The serde side of the parse: a decoder reports the refusal's description
// at the offending field, so the message carries no error code of its own.
impl TryFrom<String> for AdapterRef {
    type Error = String;

    fn try_from(value: String) -> Result<Self, String> {
        value.parse().map_err(|err: Error| err.description())
    }
}

impl From<AdapterRef> for String {
    fn from(adapter: AdapterRef) -> Self {
        match adapter {
            AdapterRef::Package(text) | AdapterRef::Static(text) => text,
            AdapterRef::File(path) => path.display().to_string(),
        }
    }
}
