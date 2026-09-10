//! Adapter references and loading
//!
//! How an operator names an adapter and how the engine brings it into the
//! run. An [`AdapterRef`] is a package reference, a bare name, or a local
//! component path; the [`Loader`] resolves each one for the duration of a
//! run, verifying an optional digest pin and refusing adapters that require
//! a newer Emery than the one running.
//!
//! Loads are remembered for the run so two sources on the same adapter
//! share one guest, and a conflicting pin on the second source is caught
//! here rather than surfacing as a confusing host error.

use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use emery_source::Source;
use emery_source::claims::is_kebab;
use omnia_guest::plugins::{Digest, Location, PluginCache, PluginRef};
use omnia_guest::{Error, Plugins, bad_request, not_found};
use serde::{Deserialize, Serialize};

use crate::preopen_path;

/// Loads adapters for one run over a provider. Loads are memoized by
/// identity, so a second source on the same adapter reuses the held guest,
/// and a disagreeing pin is refused as `already-active` by the memo rather
/// than by the host.
pub struct Loader<'a, P: Source + Plugins> {
    provider: &'a P,
    // The memo wraps the same provider; `PluginCache` exposes no accessor
    // for it, so the version gate keeps its own reference.
    cache: PluginCache<&'a P>,
}

impl<'a, P: Source + Plugins> Loader<'a, P> {
    /// Creates a loader with an empty memo over `provider`.
    pub const fn new(provider: &'a P) -> Self {
        Self {
            provider,
            cache: PluginCache::new(provider),
        }
    }

    /// Loads the adapter a reference names — a local component or registry
    /// package — enforcing its minimum `emery-version`, and returns the
    /// adapter id the `Source` capability addresses it by.
    ///
    /// # Errors
    ///
    /// Returns reference, load, or version failures.
    pub async fn load(
        &self, adapter: &AdapterRef, pin: Option<&Digest>, registry: Option<&str>,
    ) -> Result<String, Error> {
        let name = adapter.name();
        let id = match adapter.request(pin, registry)? {
            Some(request) => self.cache.ensure(&request).await?.id().to_owned(),
            None => adapter_id(name),
        };
        check_version(self.provider, name, &id)?;

        Ok(id)
    }
}

// Builds the id a bare or local adapter is addressed by: the `source:` role
// prefix over its name. A registry package is addressed by the package
// reference itself.
fn adapter_id(name: &str) -> String {
    format!("source:{name}")
}

// Refuses the adapter when the running emery is older than the minimum its
// `emery-version` metadata declares.
fn check_version<P: Source>(provider: &P, name: &str, id: &str) -> Result<(), Error> {
    let Some(declared) = provider.metadata(id).emery_version else {
        return Ok(());
    };
    let minimum = semver::Version::parse(&declared).map_err(|err| {
        bad_request!("adapter `{name}` ({id}) has an invalid `emery-version` `{declared}`: {err}")
    })?;

    // The running version is this crate's own, so it always parses.
    let running = semver::Version::parse(env!("CARGO_PKG_VERSION"));
    if running.is_ok_and(|running| running < minimum) {
        return Err(Error::BadRequest {
            code: "unsupported-version".into(),
            description: format!("adapter {name} ({id}) requires emery {minimum} or newer"),
        });
    }

    Ok(())
}

/// An operator-supplied adapter reference; on the wire it is the operator
/// string, parsed on the way in.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum AdapterRef {
    /// Package reference (`emery:omnia@1.0.0`, `omnia@1.0.0`, etc.).
    Package {
        /// Kebab-case package namespace (`emery` for the shorthand).
        namespace: String,
        /// Kebab-case adapter name.
        name: String,
        /// Mandatory exact SemVer pin.
        version: semver::Version,
    },
    /// Bare adapter name (`omnia`): the kebab-case adapter name.
    Bare(String),
    /// Local component file (`./intent.wasm`).
    Component {
        /// Kebab-case adapter name derived from the file stem.
        name: String,
        /// Project-relative component path.
        path: PathBuf,
    },
}

impl FromStr for AdapterRef {
    type Err = Error;

    /// Parses an adapter from a string. Valid strings are:
    ///   - `emery:intent@1.0.0`
    ///   - `intent@1.0.0`
    ///   - `intent`
    ///   - `./intent.wasm`
    ///
    /// # Errors
    ///
    /// Returns typed errors for malformed values, GitHub URLs, invalid
    /// pins, or a component path with no usable stem.
    fn from_str(value: &str) -> Result<Self, Error> {
        let value = value.trim();
        if value.is_empty() {
            return Err(bad_request!("adapter reference is empty"));
        }
        if value.starts_with("https://github.com/") {
            return Err(bad_request!("adapter `{value}`: GitHub URLs are not supported"));
        }

        // URL authorities and Windows drive paths are not package references.
        if let Some((namespace, rest)) = value.split_once(':')
            && !rest.starts_with('/')
            && is_kebab(namespace)
        {
            return Self::package(namespace, rest, value);
        }

        match value.split_once('@') {
            // `<name>@<version>` is sugar for the `emery` namespace; a
            // non-SemVer suffix falls through to the path grammar.
            Some((name, _)) if is_kebab(name) => {
                if let Ok(package) = Self::package("emery", value, value) {
                    return Ok(package);
                }
            }
            None if is_kebab(value) => return Ok(Self::Bare(value.to_owned())),
            _ => {}
        }

        Self::component(value.strip_prefix("file://").unwrap_or(value))
    }
}

impl Display for AdapterRef {
    // Writes the reference as an operator would type it; a component renders
    // as its path.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package {
                namespace,
                name,
                version,
            } => write!(f, "{namespace}:{name}@{version}"),
            Self::Bare(name) => f.write_str(name),
            Self::Component { path, .. } => path.display().fmt(f),
        }
    }
}

impl AdapterRef {
    fn package(namespace: &str, rest: &str, original: &str) -> Result<Self, Error> {
        let (name, version) = rest
            .split_once('@')
            .ok_or_else(|| bad_request!("adapter `{original}` is missing `@<version>`"))?;
        if name.is_empty() {
            return Err(bad_request!("adapter `{original}` is missing a name before `@`"));
        }

        let version = semver::Version::parse(version).map_err(|err| {
            bad_request!("adapter `{original}` has an invalid version `{version}`: {err}")
        })?;

        Ok(Self::Package {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            version,
        })
    }

    /// Returns the kebab-case adapter name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Bare(name) | Self::Package { name, .. } | Self::Component { name, .. } => name,
        }
    }

    // Builds a component reference from `path`, naming the adapter after the
    // file stem minus any `emery_` / `emery-` crate prefix, in kebab case.
    fn component(path: &str) -> Result<Self, Error> {
        let stem = Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| bad_request!("cannot derive adapter name from {path}"))?;
        let stem =
            stem.strip_prefix("emery_").or_else(|| stem.strip_prefix("emery-")).unwrap_or(stem);

        Ok(Self::Component {
            name: stem.replace('_', "-"),
            path: PathBuf::from(path),
        })
    }

    // Builds the `omnia:plugins/loader` request this reference names — `None`
    // for a bare name, which addresses a statically declared guest.
    fn request(
        &self, pin: Option<&Digest>, registry: Option<&str>,
    ) -> Result<Option<PluginRef>, Error> {
        let (package, location) = match self {
            Self::Bare(_) => return Ok(None),
            Self::Package { .. } => {
                (self.to_string(), Location::Registry(registry.map(ToOwned::to_owned)))
            }
            // The loader reads the file fresh and refuses a missing path
            // itself; this typo gate only lands it on `not_found` instead.
            Self::Component { name, path } => {
                let relative = preopen_path(path)?;
                if !relative.is_file() || relative.extension().is_none_or(|ext| ext != "wasm") {
                    let path = path.display();
                    let relative = relative.display();

                    return Err(not_found!(
                        "adapter `{path}` did not resolve to a `.wasm` component at {relative}"
                    ));
                }
                (adapter_id(name), Location::Path(relative.display().to_string()))
            }
        };

        Ok(Some(
            PluginRef::builder()
                .package(package)
                .location(location)
                .maybe_digest(pin.cloned())
                .build(),
        ))
    }
}

impl TryFrom<String> for AdapterRef {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Error> {
        value.parse()
    }
}

impl From<AdapterRef> for String {
    fn from(adapter: AdapterRef) -> Self {
        adapter.to_string()
    }
}
