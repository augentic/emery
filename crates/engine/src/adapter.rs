//! Parses adapter references and loads the adapters a run names.
//!
//! Every load goes through the deployment loader at the location the
//! reference names, and the deployment's grant bounds it: a local component
//! loads through the project root the runtime mounts read-only, a package
//! from the registry the project's `[registries]` table routes its namespace
//! to, and a bare name only where the deployment declares the guest.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use emery_adapter::is_kebab;
use emery_adapter::source::{Source, SourceKind};
use futures::future;
use omnia_sdk::plugins::{Digest, Location};
use omnia_sdk::{Error, Plugins, bad_request, not_found};
use serde::{Deserialize, Serialize};

use crate::preopen_path;

/// The name of the guest the engine itself runs as.
pub const ENGINE: &str = "emery";

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
/// Duplicate references are loaded once, under one pin. Every adapter loads
/// at the location its reference names, under the guest name it derives
/// ([`AdapterRef::guest`]), and all load before metadata is queried. The
/// loader holds a pinned adapter to its digest, and a declared minimum Emery
/// version must not exceed the running version. The result is keyed by the
/// reference's [`Display`] form.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] for a path outside the project, a digest
///   on a declared guest, two digests on one adapter, two references that
///   name one guest (two components sharing a file stem, two versions of
///   one package), a reference that names the engine's own guest
///   ([`ENGINE`]), a package whose namespace `registries` does not route,
///   malformed version metadata, or an incompatible adapter. Incompatible
///   versions use code `unsupported-version`; an adapter that resolves to
///   other bytes than its pin uses the loader's code `refused`.
/// - Returns [`Error::NotFound`] when a local component does not exist.
///
/// Errors from [`Plugins::load`] are returned unchanged.
pub async fn load<'a, P: Source + Plugins>(
    provider: &P, adapters: impl IntoIterator<Item = (&'a AdapterRef, Option<&'a Digest>)>,
    registries: &Registries,
) -> Result<BTreeMap<String, Loaded>, Error> {
    // one load per distinct reference, under one pin, every entry checked
    let mut locations: BTreeMap<String, (Location, Option<Digest>)> = BTreeMap::new();
    for (adapter, digest) in adapters {
        let location = location(adapter, digest, registries)?;
        match locations.entry(adapter.to_string()) {
            Entry::Vacant(slot) => {
                slot.insert((location, digest.cloned()));
            }
            Entry::Occupied(mut slot) => {
                let (_, pin) = slot.get_mut();
                match (pin.as_ref(), digest) {
                    (Some(first), Some(again)) if first != again => {
                        return Err(bad_request!("adapter `{adapter}` is pinned to two digests"));
                    }
                    (None, Some(again)) => *pin = Some(again.clone()),
                    _ => {}
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
        let metadata = Source::metadata(provider, id);
        if let Some(declared) = &metadata.emery_version {
            is_supported(id, declared, &version)?;
        }

        tracing::debug!(
            adapter = %reference,
            %id,
            kind = %metadata.kind,
            "adapter loaded"
        );

        loaded.insert(
            reference,
            Loaded {
                id: id.to_owned(),
                kind: metadata.kind,
            },
        );
    }

    Ok(loaded)
}

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

// Asked of every source entry rather than each distinct reference, so a
// digest on any entry naming a declared guest is refused.
fn location(
    adapter: &AdapterRef, digest: Option<&Digest>, registries: &Registries,
) -> Result<Location, Error> {
    Ok(match adapter {
        AdapterRef::Static(name) if name == ENGINE => {
            return Err(bad_request!(
                "adapter `{name}` is the engine itself, not a source adapter"
            ));
        }
        AdapterRef::Static(name) => {
            if digest.is_some() {
                return Err(bad_request!(
                    "adapter `{name}` is a guest built into the runtime; it takes no digest"
                ));
            }
            Location::Declared(name.clone())
        }
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
        AdapterRef::File(_) if adapter.guest() == ENGINE => {
            return Err(bad_request!(
                "adapter `{adapter}` would register as `{ENGINE}`, the engine itself; rename the \
                 component"
            ));
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
/// implementation returns that normalised identity, and [`guest`] the name
/// of the guest it loads as. An empty value, a GitHub URL, or a malformed
/// package reference parses as [`Error::BadRequest`].
///
/// [`guest`]: Self::guest
///
/// # Examples
///
/// ```
/// use emery_engine::AdapterRef;
///
/// let package: AdapterRef = "intent@1.0.0".parse()?;
/// assert_eq!(package.to_string(), "emery:intent@1.0.0");
/// assert_eq!(package.guest(), "emery:intent");
///
/// let file: AdapterRef = "file://./adapters/intent.wasm".parse()?;
/// assert_eq!(file.to_string(), "./adapters/intent.wasm");
/// assert_eq!(file.guest(), "intent");
///
/// let declared: AdapterRef = "intent".parse()?;
/// assert_eq!(declared.to_string(), "intent");
/// assert_eq!(declared.guest(), "intent");
/// # Ok::<(), omnia_sdk::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl AdapterRef {
    /// Returns the name of the guest this reference loads and dispatches by.
    ///
    /// A bare name is the guest itself, a package loads as its reference
    /// without the version — `emery:intent@1.0.0` dispatches as
    /// `emery:intent`, so a run holds one version of a package — and a local
    /// component as its file's stem — `./adapters/custom.wasm` dispatches as
    /// `custom`. The deployment declares a component or package under this
    /// name, and the loader admits it by it.
    #[must_use]
    pub fn guest(&self) -> String {
        match self {
            Self::File(path) => path
                .file_stem()
                .and_then(OsStr::to_str)
                .map_or_else(|| path.display().to_string(), str::to_owned),
            Self::Package { namespace, name, .. } => format!("{namespace}:{name}"),
            Self::Static(name) => name.clone(),
        }
    }
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
