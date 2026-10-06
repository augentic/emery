//! Parses adapter references and loads the adapters a run names.
//!
//! An adapter is an exact package reference, `namespace:name@version`, and
//! nothing else. Every load goes through the deployment loader at the
//! registry location the reference names, and the deployment's grant bounds
//! it: the package store answers a release it holds, and the registry the
//! deployment routes the namespace to answers one it lacks. The engine names
//! no registry on a load, so a project file chooses which package and, by
//! its pin, which bytes, and never where from.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use anyhow::Context;
use emery_adapter::is_kebab;
use emery_adapter::source::{AdapterMetadata, Source};
use emery_adapter::target::{Target, TargetMetadata};
use futures::future;
use omnia_sdk::plugins::{Digest, Location};
use omnia_sdk::{Error, Plugins, bad_request, server_error};
use serde::{Deserialize, Serialize};

/// The two kinds of adapter a run loads, each by the interface it exports.
///
/// A loaded component is held to its axis before any dispatch: one that does
/// not export the axis's interface is refused typed rather than trapping at
/// the first call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// A source adapter, exporting `emery:adapter/source`.
    Source,
    /// A target adapter, exporting `emery:adapter/target`.
    Target,
}

impl Axis {
    /// The interface each axis's component exports, source first.
    pub const INTERFACES: [&'static str; 2] =
        ["emery:adapter/source@0.1.0", "emery:adapter/target@0.1.0"];

    /// Returns the interface a component of this axis exports.
    #[must_use]
    pub const fn interface(self) -> &'static str {
        match self {
            Self::Source => Self::INTERFACES[0],
            Self::Target => Self::INTERFACES[1],
        }
    }

    /// Holds a loaded adapter's `exports` to this axis.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] when `exports` lacks the axis's
    /// [`interface`](Self::interface), naming the reference and the
    /// interface it lacks.
    pub fn require(self, reference: &AdapterRef, exports: &[String]) -> Result<(), Error> {
        let interface = self.interface();
        if exports.iter().any(|export| export == interface) {
            return Ok(());
        }
        Err(bad_request!(
            "adapter `{reference}` is not a {} adapter: it exports no `{interface}`",
            match self {
                Self::Source => "source",
                Self::Target => "target",
            }
        ))
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
/// at the registry location its reference names ([`AdapterRef::location`]),
/// under the guest name that location registers ([`Location::name`]), and
/// all load before any is held to its axis or asked for metadata. The loader
/// holds a pinned adapter to its digest, and a declared minimum Emery version
/// must not exceed the running version.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when a reference is refused:
///   - two digests on one adapter;
///   - two references that name one guest — two versions of one package;
///   - an adapter that does not export the source interface
///     ([`Axis::Source`]);
///   - malformed version metadata;
///   - an incompatible adapter, with code `unsupported-version`;
///   - a release the store lacks whose namespace the deployment routes
///     nowhere, a pre-compiled artifact, or an adapter that resolves to other
///     bytes than its pin, with the loader's code `refused`.
/// - Returns [`Error::BadGateway`] when the registry cannot supply a release
///   the store lacks, with the loader's code `unavailable`.
///
/// A loader's error keeps its class and code; its description names the
/// adapter that failed.
pub async fn load<'a, P: Source + Plugins>(
    provider: &P, adapters: impl IntoIterator<Item = (&'a AdapterRef, Option<&'a Digest>)>,
) -> Result<BTreeMap<AdapterRef, Loaded<AdapterMetadata>>, Error> {
    load_axis(provider, Axis::Source, adapters, |id| Source::metadata(provider, id)).await
}

/// Loads the one target adapter a build names and returns what it loaded as.
///
/// The reference loads as [`load`] loads a source adapter, under the same
/// refusals, held to the target interface ([`Axis::Target`]) instead, and
/// its declared minimum Emery version is gated the same way.
///
/// # Errors
///
/// As [`load`], over the one reference.
pub async fn load_target<P: Target + Plugins>(
    provider: &P, adapter: &AdapterRef, digest: Option<&Digest>,
) -> Result<Loaded<TargetMetadata>, Error> {
    let loaded =
        load_axis(provider, Axis::Target, [(adapter, digest)], |id| Target::metadata(provider, id))
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
    provider: &P, axis: Axis,
    adapters: impl IntoIterator<Item = (&'a AdapterRef, Option<&'a Digest>)>,
    metadata: impl Fn(&str) -> M,
) -> Result<BTreeMap<AdapterRef, Loaded<M>>, Error> {
    let pins = distinct(adapters)?;

    // refuse two references the loader would register as one guest
    let mut names: BTreeMap<String, &AdapterRef> = BTreeMap::new();
    for adapter in pins.keys() {
        let guest = adapter.guest();
        if let Some(first) = names.get(&guest) {
            return Err(bad_request!(
                "adapters `{first}` and `{adapter}` would both register as `{guest}`; a run loads \
                 one adapter per guest name"
            ));
        }
        names.insert(guest, adapter);
    }

    // load all adapters in parallel
    let loaders = pins.iter().map(|(adapter, pin)| async move {
        Plugins::load(provider, &adapter.location(), pin.as_ref())
            .await
            .map_err(|error| at_adapter(error.into(), adapter))
    });
    let handles = future::try_join_all(loaders).await?;

    let version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .with_context(|| format!("issue with emery version `{}`", env!("CARGO_PKG_VERSION")))?;

    // hold each adapter to its axis, gate its version, and record what it loaded as
    let mut loaded = BTreeMap::new();
    for (adapter, plugin) in pins.into_keys().zip(handles) {
        axis.require(adapter, plugin.exports())?;
        let id = plugin.id();
        let metadata = metadata(id);
        if let Some(declared) = metadata.emery_version() {
            require_version(id, declared, &version)?;
        }

        tracing::debug!(%adapter, %id, "adapter loaded");

        loaded.insert(
            adapter.clone(),
            Loaded {
                id: id.to_owned(),
                metadata,
            },
        );
    }

    Ok(loaded)
}

// One load per distinct reference, under one pin, every entry checked.
fn distinct<'a>(
    adapters: impl IntoIterator<Item = (&'a AdapterRef, Option<&'a Digest>)>,
) -> Result<BTreeMap<&'a AdapterRef, Option<Digest>>, Error> {
    let mut pins: BTreeMap<&AdapterRef, Option<Digest>> = BTreeMap::new();
    for (adapter, digest) in adapters {
        match pins.entry(adapter) {
            Entry::Vacant(slot) => {
                slot.insert(digest.cloned());
            }
            Entry::Occupied(mut slot) => {
                if let Some(again) = digest {
                    let first = slot.get_mut().get_or_insert_with(|| again.clone());
                    if first != again {
                        return Err(bad_request!("adapter `{adapter}` is pinned to two digests"));
                    }
                }
            }
        }
    }
    Ok(pins)
}

// The loader's refusal, as the entry that failed: class and code kept, the
// adapter named.
fn at_adapter(error: Error, adapter: &AdapterRef) -> Error {
    let describe = |description: String| format!("adapter `{adapter}`: {description}");
    match error {
        Error::BadRequest { code, description } => Error::BadRequest {
            code,
            description: describe(description),
        },
        Error::NotFound { code, description } => Error::NotFound {
            code,
            description: describe(description),
        },
        Error::ServerError { code, description } => Error::ServerError {
            code,
            description: describe(description),
        },
        Error::BadGateway { code, description } => Error::BadGateway {
            code,
            description: describe(description),
        },
    }
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

/// An adapter reference: an exact package, `namespace:name@version`.
///
/// Parsing accepts that one form and nothing else — no default namespace, no
/// latest, no path, no bare name. [`Display`] is the reference as spelled,
/// which config serialises and a run dedupes loads by; [`name`] is the
/// package name, which a run binds a source under when the operator gives
/// none; [`guest`] is the name the adapter loads and dispatches as, the
/// reference without its version. A malformed reference parses as
/// [`Error::BadRequest`] with code `adapter-reference`.
///
/// [`name`]: Self::name
/// [`guest`]: Self::guest
///
/// # Examples
///
/// ```
/// use emery_engine::AdapterRef;
///
/// let adapter: AdapterRef = "emery:intent@1.0.0".parse()?;
/// assert_eq!(adapter.to_string(), "emery:intent@1.0.0");
/// assert_eq!(adapter.name(), "intent");
/// assert_eq!(adapter.guest(), "emery:intent");
///
/// assert!("intent@1.0.0".parse::<AdapterRef>().is_err());
/// assert!("emery:intent".parse::<AdapterRef>().is_err());
/// assert!("./intent.wasm".parse::<AdapterRef>().is_err());
/// # Ok::<(), omnia_sdk::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AdapterRef {
    /// The namespace the deployment routes to a registry, `emery` for a
    /// first-party adapter.
    pub namespace: String,
    /// The package name, the [`name`](Self::name) a run binds a source under.
    pub name: String,
    /// The exact version the store holds or the registry serves.
    pub version: semver::Version,
}

impl AdapterRef {
    /// Returns the package name, which a run binds a source under when the
    /// operator gives no `name` of their own.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the guest name this adapter loads and dispatches as: the
    /// reference without its version.
    #[must_use]
    pub fn guest(&self) -> String {
        format!("{}:{}", self.namespace, self.name)
    }

    /// Returns the loader location this reference loads at.
    ///
    /// The location names no registry: the deployment's store answers a
    /// release it holds, its routing of the namespace one it lacks, or the
    /// load is refused.
    #[must_use]
    pub fn location(&self) -> Location {
        Location::Registry {
            package: self.to_string(),
            endpoint: None,
        }
    }
}

impl Display for AdapterRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}@{}", self.namespace, self.name, self.version)
    }
}

impl FromStr for AdapterRef {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Error> {
        let value = value.trim();
        if value.is_empty() {
            return Err(malformed("adapter reference is empty".to_owned()));
        }
        if value.starts_with("https://github.com/") {
            return Err(malformed(format!("adapter `{value}`: GitHub URLs are not supported")));
        }
        let Some((package, version)) = value.rsplit_once('@') else {
            return Err(malformed(format!("adapter `{value}` names no version")));
        };
        let Some((namespace, name)) = package.split_once(':') else {
            return Err(malformed(format!("adapter `{value}` names no namespace")));
        };
        if !is_kebab(namespace) || !is_kebab(name) {
            return Err(malformed(format!("adapter `{value}` is not `namespace:name@version`")));
        }
        let version = semver::Version::parse(version).map_err(|err| {
            malformed(format!("adapter `{value}` has an invalid version `{version}`: {err}"))
        })?;

        Ok(Self {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            version,
        })
    }
}

// Every malformed reference shares one recovery: spell an exact package.
fn malformed(description: String) -> Error {
    Error::BadRequest {
        code: "adapter-reference".into(),
        description,
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
