//! Parses adapter references and loads the adapters required by a run.
//!
//! An [`AdapterRef`] identifies a declared guest, registry package, or local
//! WebAssembly component, and names the guest it loads as
//! ([`AdapterRef::guest`]). [`load`] loads each unique reference through the
//! deployment loader by that name — refusing two that name one guest before
//! either loads — holds each to its digest pin, checks version
//! compatibility, and returns what each adapter [`Loaded`] as: the identity
//! it dispatches by and the source authority it declares. [`Registries`]
//! routes a package reference to the registry that serves its namespace.
//!
//! The deployment's guest list is the loader's allow-list: a local component
//! or a package loads only where the deployment declares it as an on-demand
//! guest under the name the reference derives, which the shipped runtime
//! does for every reference a run names before the engine runs.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use emery_adapter::is_kebab;
use emery_adapter::source::{Source, SourceKind};
use futures::future;
use omnia_sdk::plugins::{self, Digest};
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
/// assert_eq!(registries.routes().len(), 2);
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

    /// Returns every route by namespace, the first-party one included unless
    /// a line re-routes it.
    ///
    /// The routing a deployment installs for the packages a run names.
    #[must_use]
    pub fn routes(&self) -> BTreeMap<&str, &str> {
        let mut routes: BTreeMap<&str, &str> = self
            .0
            .iter()
            .map(|(namespace, registry)| (namespace.as_str(), registry.as_str()))
            .collect();
        routes.entry(FIRST_PARTY.0).or_insert(FIRST_PARTY.1);
        routes
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
/// by the guest name its reference derives ([`AdapterRef::guest`]), and all
/// load before metadata is queried. A pinned adapter must resolve to its
/// digest, and a declared minimum Emery version must not exceed the running
/// version. The result is keyed by the reference's [`Display`] form.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] for a path outside the project, a digest
///   on a declared guest, two digests on one adapter, two references that
///   name one guest, a package whose namespace `registries` does not route,
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
    // one load per distinct reference, under one pin
    let mut guests: BTreeMap<String, (String, Option<Digest>)> = BTreeMap::new();
    for (adapter, digest) in adapters {
        let reference = adapter.to_string();
        if let Some((_, pin)) = guests.get_mut(&reference) {
            match (pin.as_ref(), digest) {
                (Some(first), Some(again)) if first != again => {
                    return Err(bad_request!("adapter `{reference}` is pinned to two digests"));
                }
                (None, Some(again)) => *pin = Some(again.clone()),
                _ => {}
            }
            continue;
        }
        let guest = guest(adapter, digest, registries)?;
        guests.insert(reference, (guest, digest.cloned()));
    }

    // refuse two references the deployment would declare as one guest
    let mut names: BTreeMap<&str, &str> = BTreeMap::new();
    for (reference, (guest, _)) in &guests {
        if let Some(first) = names.insert(guest, reference) {
            return Err(bad_request!(
                "adapters `{first}` and `{reference}` would both register as `{guest}`; rename \
                 one component"
            ));
        }
    }

    // load all adapters in parallel
    let loaders = guests.values().map(|(guest, _)| Plugins::load(provider, guest));
    let handles = future::try_join_all(loaders).await?;

    let version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .with_context(|| format!("issue with emery version `{}`", env!("CARGO_PKG_VERSION")))?;

    // hold each adapter to its pin, gate its version, and record what it loaded as
    let mut loaded = BTreeMap::new();
    for ((reference, (_, pin)), plugin) in guests.into_iter().zip(handles) {
        let id = plugin.id();
        if let Some(pin) = &pin
            && plugin.digest() != Some(pin)
        {
            return Err(unpinned(&reference, plugin.digest(), pin));
        }
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

// The loader's refusal of an adapter whose resolved bytes are not the ones
// its `[[source]] digest` pins — the deployment's answer where it carries the
// pin, and the engine's where it does not.
fn unpinned(reference: &str, resolved: Option<&Digest>, pin: &Digest) -> Error {
    let resolved = resolved.map_or_else(|| "no digest".to_owned(), ToString::to_string);
    plugins::Error::Refused(format!(
        "adapter `{reference}` resolved to {resolved}, not its pinned digest {pin}"
    ))
    .into()
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

// The guest a reference loads once it can load at all: a declared guest is
// the deployment's to pin, a package needs the registry its namespace routes
// to, and a local component must be a file beneath the project root.
fn guest(
    adapter: &AdapterRef, digest: Option<&Digest>, registries: &Registries,
) -> Result<String, Error> {
    match adapter {
        AdapterRef::Static(name) if digest.is_some() => {
            return Err(bad_request!(
                "adapter `{name}` is a guest built into the runtime; it takes no digest"
            ));
        }
        AdapterRef::Package { namespace, .. } if registries.get(namespace).is_none() => {
            return Err(bad_request!(
                "no registry routes `{adapter}`: add `{namespace} = \"<registry>\"` under \
                 `[registries]` in emery.toml"
            ));
        }
        AdapterRef::File(path) if !preopen_path(path)?.is_file() => {
            return Err(not_found!("adapter `{adapter}` not found"));
        }
        _ => {}
    }
    Ok(adapter.guest())
}

/// A reference to a source adapter.
///
/// Parsing normalises package shorthands and local file prefixes.
/// `intent@1.0.0` becomes `emery:intent@1.0.0`, while
/// `file://./intent.wasm` becomes `./intent.wasm`. Its [`Display`]
/// implementation returns that normalised identity, and [`guest`] the name
/// of the guest it loads as.
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
/// assert_eq!(package.guest(), "emery:intent@1.0.0");
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
    /// A bare name is the guest itself, a package loads as its exact
    /// reference, and a local component as its file's stem —
    /// `./adapters/custom.wasm` dispatches as `custom`. The deployment
    /// declares a component or package under this name, and the loader admits
    /// it by it.
    #[must_use]
    pub fn guest(&self) -> String {
        match self {
            Self::File(path) => path
                .file_stem()
                .and_then(OsStr::to_str)
                .map_or_else(|| path.display().to_string(), str::to_owned),
            Self::Package { .. } | Self::Static(_) => self.to_string(),
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
