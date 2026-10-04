//! Parses adapter references and loads the adapters a run names.
//!
//! Every load goes through the deployment loader at the location the
//! reference names, and the deployment's grant bounds it: a local component
//! loads through the adapters root the runtime mounts read-only as
//! [`ADAPTERS`], a package from the registry the run's [`Registries`] route
//! its namespace to, and a bare name only where the deployment declares the
//! guest.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use emery_adapter::is_kebab;
use emery_adapter::source::{AdapterMetadata, Source};
use emery_adapter::target::{Target, TargetMetadata};
use futures::future;
use omnia_sdk::plugins::{Digest, Location};
use omnia_sdk::{Error, Plugins, bad_request, not_found, server_error};
use serde::{Deserialize, Serialize};

use crate::preopen_path;

/// The name of the guest the engine itself runs as.
pub const ENGINE: &str = "emery";

/// The name of the mount a local component loads through.
///
/// The runtime mounts the adapters root under it, read-only and apart from
/// the project tree, so a component is never loaded from a directory a run
/// can write. An [`AdapterRef::File`] is a path beneath that root.
pub const ADAPTERS: &str = "adapters";

/// The registry serving each package namespace.
///
/// A map from namespace to registry endpoint, over the one route the engine
/// knows: `emery` resolves to `augentic.io` unless an entry re-routes it. A
/// namespace nothing routes refuses the package before any load.
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

const NAMESPACE: &str = "emery";
const REGISTRY: &str = "augentic.io";

impl Registries {
    /// Returns the registry serving `namespace`, if a project line or the
    /// first-party route names one.
    #[must_use]
    pub fn get(&self, namespace: &str) -> Option<&str> {
        if let Some(registry) = self.0.get(namespace) {
            return Some(registry);
        }
        if namespace == NAMESPACE {
            return Some(REGISTRY);
        }
        None
    }
}

/// What one adapter loaded as.
#[derive(Debug, Clone)]
pub struct Loaded<M> {
    /// The identity the loader registered the adapter under, which every
    /// dispatch names.
    pub id: String,
    /// The metadata the adapter declares for its axis: an
    /// [`AdapterMetadata`], whose kind of source ranks a source adapter's
    /// claims, or a [`TargetMetadata`].
    pub metadata: M,
}

/// Loads each referenced source adapter and returns what it loaded as.
///
/// Duplicate references are loaded once, under one pin. Every adapter loads
/// at the location its reference names ([`AdapterRef::location`]), under the
/// guest name that location registers ([`Location::name`]), and all load
/// before metadata is queried. The loader holds a pinned adapter to its
/// digest, and a declared minimum Emery version must not exceed the running
/// version. The result is keyed by the reference's [`Display`] form.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when a reference is refused:
///   - a path outside the adapters root;
///   - a digest on a declared guest, or two digests on one adapter;
///   - two references that name one guest, such as two components sharing a
///     file stem or two versions of one package;
///   - a reference that names the engine's own guest ([`ENGINE`]);
///   - a package whose namespace `registries` does not route;
///   - malformed version metadata;
///   - an incompatible adapter, with code `unsupported-version`;
///   - an adapter that resolves to other bytes than its pin, with the
///     loader's code `refused`.
/// - Returns [`Error::NotFound`] when a local component does not exist.
///
/// Errors from [`Plugins::load`] are returned unchanged.
pub async fn load<'a, P: Source + Plugins>(
    provider: &P, adapters: impl IntoIterator<Item = (&'a AdapterRef, Option<&'a Digest>)>,
    registries: &Registries,
) -> Result<BTreeMap<String, Loaded<AdapterMetadata>>, Error> {
    load_axis(provider, adapters, registries, |id| Source::metadata(provider, id)).await
}

/// Loads the one target adapter a build names and returns what it loaded as.
///
/// The reference loads as [`load`] loads a source adapter, under the same
/// refusals, and its declared minimum Emery version is gated the same way.
///
/// # Errors
///
/// As [`load`], over the one reference.
pub async fn load_target<P: Target + Plugins>(
    provider: &P, adapter: &AdapterRef, digest: Option<&Digest>, registries: &Registries,
) -> Result<Loaded<TargetMetadata>, Error> {
    let loaded =
        load_axis(provider, [(adapter, digest)], registries, |id| Target::metadata(provider, id))
            .await?;
    loaded.into_values().next().ok_or_else(|| server_error!("adapter `{adapter}` was not loaded"))
}

// The metadata an axis declares, gated alike: each carries the minimum
// `emery` version the adapter requires.
trait Declared {
    fn emery_version(&self) -> Option<&str>;
}

impl Declared for AdapterMetadata {
    fn emery_version(&self) -> Option<&str> {
        self.emery_version.as_deref()
    }
}

impl Declared for TargetMetadata {
    fn emery_version(&self) -> Option<&str> {
        self.emery_version.as_deref()
    }
}

async fn load_axis<'a, P: Plugins, M: Declared>(
    provider: &P, adapters: impl IntoIterator<Item = (&'a AdapterRef, Option<&'a Digest>)>,
    registries: &Registries, metadata: impl Fn(&str) -> M,
) -> Result<BTreeMap<String, Loaded<M>>, Error> {
    // one load per distinct reference, under one pin, every entry checked
    let mut locations: BTreeMap<String, (Location, Option<Digest>)> = BTreeMap::new();
    for (adapter, digest) in adapters {
        let location = adapter.location(digest, registries)?;

        match locations.entry(adapter.to_string()) {
            Entry::Vacant(slot) => {
                slot.insert((location, digest.cloned()));
            }
            Entry::Occupied(mut slot) => {
                let (_, pin) = slot.get_mut();
                if let Some(again) = digest {
                    let first = pin.get_or_insert_with(|| again.clone());
                    if first != again {
                        return Err(bad_request!("adapter `{adapter}` is pinned to two digests"));
                    }
                }
            }
        }
    }

    // refuse two references the loader would register as one guest
    let mut names: BTreeMap<&str, &str> = BTreeMap::new();
    for (reference, (location, _)) in &locations {
        if let Some(first) = names.insert(location.name(), reference) {
            return Err(bad_request!(
                "adapters `{first}` and `{reference}` would both register as `{}`; a run loads \
                 one adapter per guest name",
                location.name()
            ));
        }
    }

    // load all adapters in parallel
    let loaders =
        locations.values().map(|(location, pin)| Plugins::load(provider, location, pin.as_ref()));
    let handles = future::try_join_all(loaders).await?;

    let version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .with_context(|| format!("issue with emery version `{}`", env!("CARGO_PKG_VERSION")))?;

    // gate each adapter's version and record what it loaded as
    let mut loaded = BTreeMap::new();
    for (reference, plugin) in locations.into_keys().zip(handles) {
        let id = plugin.id();
        let metadata = metadata(id);
        if let Some(declared) = metadata.emery_version() {
            require_version(id, declared, &version)?;
        }

        tracing::debug!(adapter = %reference, %id, "adapter loaded");

        loaded.insert(
            reference,
            Loaded {
                id: id.to_owned(),
                metadata,
            },
        );
    }

    Ok(loaded)
}

fn require_version(id: &str, declared: &str, running: &semver::Version) -> Result<(), Error> {
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

/// A reference to an adapter, of either axis.
///
/// Parsing normalises package shorthands and local file prefixes.
/// `intent@1.0.0` becomes `emery:intent@1.0.0`, while
/// `file://./intent.wasm` becomes `./intent.wasm`. [`Display`] is that
/// normalised identity, which config serialises and a run dedupes loads by.
/// [`name`] is the kebab-case name it lends a binding that names none. The
/// guest it loads and dispatches as is its [`location`]'s
/// [`Location::name`]. An empty value, a GitHub URL, or a malformed package
/// reference parses as [`Error::BadRequest`].
///
/// [`name`]: Self::name
/// [`location`]: Self::location
///
/// # Examples
///
/// ```
/// use emery_engine::AdapterRef;
///
/// let package: AdapterRef = "intent@1.0.0".parse()?;
/// assert_eq!(package.to_string(), "emery:intent@1.0.0");
/// assert_eq!(package.name()?, "intent");
///
/// let file: AdapterRef = "file://./adapters/my_intent.wasm".parse()?;
/// assert_eq!(file.to_string(), "./adapters/my_intent.wasm");
/// assert_eq!(file.name()?, "my-intent");
///
/// let declared: AdapterRef = "intent".parse()?;
/// assert_eq!(declared.to_string(), "intent");
/// assert_eq!(declared.name()?, "intent");
/// # Ok::<(), omnia_sdk::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum AdapterRef {
    /// A path to a `.wasm` component beneath the adapters root.
    File(PathBuf),
    /// A registry package, `<namespace>:<name>@<version>`.
    Package {
        /// The namespace [`Registries`] routes to a registry, `emery` for a
        /// first-party adapter.
        namespace: String,
        /// The package name, the [`name`](Self::name) a run derives.
        name: String,
        /// The exact version to fetch.
        version: semver::Version,
    },
    /// A guest the deployment declares, identified by a bare name such as
    /// `intent`.
    Static(String),
}

impl AdapterRef {
    /// Returns the kebab-case name this adapter lends what it is bound to.
    ///
    /// A bare name or a package is its name; a local component is its file's
    /// stem with `_` read as `-`, so `./adapters/my_tool.wasm` is `my-tool`.
    /// A run uses it where an operator gives no `name` of their own.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] when the derived name is not kebab-case,
    /// as a component's stem need not be; the binding then needs an explicit
    /// `name`.
    pub fn name(&self) -> Result<String, Error> {
        let name = match self {
            Self::Static(name) | Self::Package { name, .. } => name.clone(),
            Self::File(path) => {
                let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or_default();
                stem.replace('_', "-")
            }
        };
        if !is_kebab(&name) {
            return Err(bad_request!(
                "adapter `{self}` derives the name `{name}`, which is not kebab-case; set `name` \
                 explicitly"
            ));
        }
        Ok(name)
    }

    /// Returns the loader location this reference loads at.
    ///
    /// A local component loads at its path beneath the [`ADAPTERS`] mount,
    /// wherever the reference was written.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] for a digest on a declared guest, a
    ///   package whose namespace `registries` does not route, a local
    ///   component whose path escapes the adapters root, or a reference that
    ///   would register as [`ENGINE`].
    /// - Returns [`Error::NotFound`] when a local component does not exist.
    pub fn location(
        &self, digest: Option<&Digest>, registries: &Registries,
    ) -> Result<Location, Error> {
        let location = match self {
            Self::Static(name) => {
                if digest.is_some() {
                    return Err(bad_request!(
                        "adapter `{name}` is a guest built into the runtime; it takes no digest"
                    ));
                }
                Location::Declared(name.clone())
            }
            Self::Package { namespace, .. } => {
                let endpoint = registries.get(namespace).ok_or_else(|| {
                    bad_request!(
                        "no registry routes `{self}`: `registries` names no route for namespace \
                         `{namespace}`"
                    )
                })?;
                Location::Registry {
                    package: self.to_string(),
                    endpoint: Some(endpoint.to_owned()),
                }
            }
            Self::File(path) => {
                let Ok(relative) = preopen_path(path) else {
                    return Err(bad_request!(
                        "adapter `{self}` escapes the adapters root; a local component is a \
                         relative path beneath it"
                    ));
                };
                let local = Path::new(ADAPTERS).join(relative);
                if !local.is_file() {
                    return Err(not_found!("adapter `{self}` not found beneath the adapters root"));
                }
                Location::Path(local.display().to_string())
            }
        };

        // refuse the engine's own guest name, which the loader would attest
        if location.name() == ENGINE {
            return Err(bad_request!(
                "adapter `{self}` would register as `{ENGINE}`, the engine itself; a run loads no \
                 adapter under that name"
            ));
        }

        Ok(location)
    }
}

impl Display for AdapterRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package {
                namespace,
                name,
                version,
            } => write!(f, "{namespace}:{name}@{version}"),
            Self::Static(name) => f.write_str(name),
            Self::File(path) => path.display().fmt(f),
        }
    }
}

impl FromStr for AdapterRef {
    type Err = Error;

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

        // parse `<namespace>:<name>@<version>`, the namespace defaulting to `emery`
        let (namespace, rest) = value.split_once(':').unwrap_or((NAMESPACE, value));
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

// A decoder reports the refusal at the offending field, where the error's
// class and code would be noise.
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
