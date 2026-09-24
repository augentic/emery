//! Parses adapter references and loads the adapters required by a run.
//!
//! An [`AdapterRef`] identifies a declared guest, registry package, or local
//! WebAssembly component. [`load`] loads each unique reference through the
//! deployment loader — refusing two that would register under one name
//! before either loads — checks version compatibility, and returns what each
//! adapter [`Loaded`] as: the identity it dispatches by and the source
//! authority it declares. [`Registries`] routes a package reference to the
//! registry that serves its namespace.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use emery_adapter::is_kebab;
use emery_adapter::source::{Source, SourceKind};
use futures::future;
use omnia_sdk::plugins::{Digest, Location, PluginRef};
use omnia_sdk::{Error, Plugins, bad_request, not_found};
use serde::{Deserialize, Serialize};

use crate::preopen_path;

/// The registry serving each package namespace.
///
/// A project's `[registries]` table, `<namespace> = "<registry>"`, over the
/// one route the engine knows: `emery` resolves to `augentic.io` unless a
/// line re-routes it. A namespace nothing routes refuses the package before
/// any load.
///
/// # Examples
///
/// ```
/// use emery_engine::Registries;
///
/// let registries: Registries = serde_json::from_str(r#"{ "acme": "registry.acme.io" }"#)?;
/// assert_eq!(registries.get("acme"), Some("registry.acme.io"));
/// assert_eq!(registries.get("emery"), Some("augentic.io"));
/// assert_eq!(registries.get("other"), None);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Registries(BTreeMap<String, String>);

// The route the engine knows without any project line.
const FIRST_PARTY: (&str, &str) = ("emery", "augentic.io");

impl Registries {
    /// Returns the registry serving `namespace`, if a project line or the
    /// first-party route names one.
    #[must_use]
    pub fn get(&self, namespace: &str) -> Option<&str> {
        self.0
            .get(namespace)
            .map(String::as_str)
            .or_else(|| (namespace == FIRST_PARTY.0).then_some(FIRST_PARTY.1))
    }
}

/// What one adapter loaded as.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The identity the loader registered the adapter under, which every
    /// source dispatch names.
    pub id: String,
    /// The kind of source the adapter declares, which ranks its claims.
    pub kind: SourceKind,
}

/// Loads each referenced adapter and returns what it loaded as.
///
/// Duplicate references are loaded once, under one pin. All components are
/// loaded before metadata is queried. A declared minimum Emery version must
/// not exceed the running version. The result is keyed by the reference's
/// [`Display`] form.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] for a path outside the project, a digest
///   on a declared guest, two digests on one adapter, two references that
///   would register under one name, a package whose namespace `registries`
///   does not route, malformed version metadata, or an incompatible adapter.
///   Incompatible versions use code `unsupported-version`.
/// - Returns [`Error::NotFound`] when a local component does not exist.
///
/// Errors from [`Plugins::load`] are returned unchanged.
pub async fn load<'a, P: Source + Plugins>(
    provider: &P, adapters: impl IntoIterator<Item = (&'a AdapterRef, Option<&'a Digest>)>,
    registries: &Registries,
) -> Result<BTreeMap<String, Loaded>, Error> {
    // one load per distinct reference, under one pin
    let mut plugins: BTreeMap<String, PluginRef> = BTreeMap::new();
    for (adapter, digest) in adapters {
        let id = adapter.to_string();
        if let Some(plugin) = plugins.get_mut(&id) {
            match (plugin.digest.as_ref(), digest) {
                (Some(first), Some(again)) if first != again => {
                    return Err(bad_request!("adapter `{id}` is pinned to two digests"));
                }
                (None, Some(again)) => plugin.digest = Some(again.clone()),
                _ => {}
            }
            continue;
        }
        let location = location(adapter, digest, registries)?;
        plugins.insert(
            id,
            PluginRef {
                location,
                digest: digest.cloned(),
            },
        );
    }

    // refuse two references the loader would register as one guest
    let mut names: BTreeMap<&str, &str> = BTreeMap::new();
    for (id, plugin) in &plugins {
        let name = plugin.location.name();
        if let Some(first) = names.insert(name, id) {
            return Err(bad_request!(
                "adapters `{first}` and `{id}` would both register as `{name}`; rename one \
                 component"
            ));
        }
    }

    // load all adapters in parallel
    let (ids, plugins): (Vec<String>, Vec<PluginRef>) = plugins.into_iter().unzip();
    let loaders = plugins.iter().map(|plugin| Plugins::load(provider, plugin));
    let handles = future::try_join_all(loaders).await?;

    let version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .with_context(|| format!("issue with emery version `{}`", env!("CARGO_PKG_VERSION")))?;

    // gate each adapter's version and record what it loaded as
    let mut loaded = BTreeMap::new();
    for (adapter, plugin) in ids.into_iter().zip(handles) {
        let id = plugin.id();
        let metadata = Source::metadata(provider, id);
        if let Some(declared) = &metadata.emery_version {
            is_supported(id, declared, &version)?;
        }

        tracing::debug!(
            %adapter,
            %id,
            kind = %metadata.kind,
            "adapter loaded"
        );

        loaded.insert(
            adapter,
            Loaded {
                id: id.to_owned(),
                kind: metadata.kind,
            },
        );
    }

    Ok(loaded)
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

// The loader location a reference names: a declared guest by name, a package
// under the registry its namespace routes to, a local component by its
// project-relative path.
fn location(
    adapter: &AdapterRef, digest: Option<&Digest>, registries: &Registries,
) -> Result<Location, Error> {
    Ok(match adapter {
        AdapterRef::Static(name) => {
            if digest.is_some() {
                return Err(bad_request!(
                    "adapter `{name}` is a guest built into the runtime; it takes no digest"
                ));
            }
            Location::Declared(name.clone())
        }
        // The load names the registry the project's table routes the
        // namespace to, so the deployment needs no routing of its own.
        AdapterRef::Package { namespace, .. } => {
            let endpoint = registries.get(namespace).ok_or_else(|| {
                bad_request!(
                    "no registry routes `{adapter}`: add `{namespace} = \"<registry>\"` under \
                     `[registries]` in emery.toml"
                )
            })?;
            Location::Registry {
                package: adapter.to_string(),
                endpoint: Some(endpoint.to_owned()),
            }
        }
        AdapterRef::File(path) => {
            let local = preopen_path(path)?;
            if !local.is_file() {
                return Err(not_found!("adapter `{adapter}` not found"));
            }
            Location::Path(local.display().to_string())
        }
    })
}

/// A reference to a source adapter.
///
/// Parsing normalises package shorthands and local file prefixes.
/// `intent@1.0.0` becomes `emery:intent@1.0.0`, while
/// `file://./intent.wasm` becomes `./intent.wasm`. Its [`Display`]
/// implementation returns that normalised identity.
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
    /// A registry package, `<namespace>:<name>@<version>`.
    Package {
        /// The namespace `[registries]` routes to a registry, `emery` for a
        /// first-party adapter.
        namespace: String,
        /// The package name, the source key a run derives.
        name: String,
        /// The exact version to fetch.
        version: semver::Version,
    },
    /// A guest the deployment declares, identified by a bare name such as
    /// `intent`.
    Static(String),
}

impl Display for AdapterRef {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Package {
                namespace,
                name,
                version,
            } => write!(f, "{namespace}:{name}@{version}"),
            Self::Static(name) => f.write_str(name),
            Self::File(path) => write!(f, "{}", path.display()),
        }
    }
}

impl FromStr for AdapterRef {
    type Err = Error;

    /// Parses a local path, bare adapter name, or versioned package.
    ///
    /// A package without an explicit namespace uses `emery`.
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
        let (namespace, rest) = value.split_once(':').unwrap_or((FIRST_PARTY.0, value));
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

        Ok(Self::Package {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            version,
        })
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
        adapter.to_string()
    }
}
