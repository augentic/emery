//! Adapter references and loading
//!
//! How an operator names an adapter and how the engine brings it into the
//! run. An [`AdapterRef`] is a registry package, a statically declared
//! guest, or a local file; [`load`] loads the adapters a run's sources name,
//! each once and all together, and refuses any that requires a newer Emery
//! than the one running.
//!
//! The reference is the identity: the plugin loader registers an adapter,
//! and the `Source` capability dispatches to it, by the [`AdapterRef`]'s
//! `Display`.

use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use emery_adapter::is_kebab;
use emery_adapter::source::Source;
use futures::future;
use omnia_guest::plugins::{Location, PluginRef};
use omnia_guest::{Error, Plugins, bad_request, not_found};
use serde::{Deserialize, Serialize};

use crate::preopen_path;

/// Loads each distinct adapter among `adapters` — a reference and, for a
/// package, the registry endpoint override it loads from (`None` selects
/// the acquirer's default) — and registers it with the `Source` capability
/// by its `AdapterRef` identity. An adapter several sources share is loaded
/// once, under the first source's registry.
///
/// # Errors
///
/// Returns reference, load, or version failures.
pub async fn load<'a, P: Source + Plugins>(
    provider: &P, adapters: impl IntoIterator<Item = (&'a AdapterRef, Option<&'a str>)>,
) -> Result<(), Error> {
    let mut ids = Vec::new();
    let mut plugins = Vec::new();
    for (adapter, registry) in adapters {
        let id = adapter.to_string();
        if ids.contains(&id) {
            continue;
        }

        let location = match adapter {
            AdapterRef::Static(_) => None,
            AdapterRef::Package(_) => Some(Location::Registry(registry.map(String::from))),
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

    // Every load at once, so a run waits only for its slowest acquisition;
    // the host load is idempotent, so the order they land in is immaterial.
    // The first failure in declaration order is the one reported.
    let loads = plugins.iter().map(|plugin| Plugins::load(provider, plugin));
    for loaded in future::join_all(loads).await {
        loaded?;
    }

    for id in &ids {
        check_version(provider, id)?;
    }

    Ok(())
}

fn check_version<P: Source>(provider: &P, id: &str) -> Result<(), Error> {
    let Some(declared) = provider.metadata(id).emery_version else {
        return Ok(());
    };

    let minimum = semver::Version::parse(&declared).map_err(|err| {
        bad_request!("adapter `{id}` has an invalid `emery-version` `{declared}`: {err}")
    })?;
    let running = semver::Version::parse(env!("CARGO_PKG_VERSION")).with_context(|| {
        format!("the running emery version `{}` is not SemVer", env!("CARGO_PKG_VERSION"))
    })?;
    if running < minimum {
        return Err(Error::BadRequest {
            code: "unsupported-version".into(),
            description: format!("adapter `{id}` requires emery {minimum} or newer"),
        });
    }

    Ok(())
}

/// An operator-supplied adapter reference, and the adapter's identity.
///
/// On the wire it is the operator's string; in memory, that string
/// normalised (`intent@1.0.0` becomes `emery:intent@1.0.0`, a `file://`
/// prefix is dropped). `Display` and the string conversion give that string
/// back, and the plugin loader and the `Source` capability address the
/// adapter by it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum AdapterRef {
    /// Project-relative path to a `.wasm` component.
    File(PathBuf),
    /// Registry package, always `<namespace>:<name>@<version>`.
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

    /// Parses `./intent.wasm`, `intent`, `emery:intent@1.0.0`, or its
    /// shorthand `intent@1.0.0`.
    ///
    /// # Errors
    ///
    /// Returns `BadRequest` for an empty value, a GitHub URL, or a malformed
    /// package reference.
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
