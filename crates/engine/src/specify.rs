//! The `specify` operation
//!
//! Emery's central operation: given a list of sources, extract each
//! source's claims, derive the requirement rows under authority precedence,
//! synthesise `spec.md` and `design.md`, and commit the pair as one new
//! revision.
//!
//! A [`SourceConfig`] names one source to extract from: the adapter to use,
//! the key the specification will cite it by, and either a workspace to read
//! or an inline value. The list is per-run input, never stored, so the same
//! shape serves the command line, a config file, and any other transport,
//! and it is checked whole before a single adapter loads.
//!
//! The result reports what was committed — the revision id and the
//! diff against the superseded revision — so a caller can see what
//! changed without reading the documents.

mod brief;
mod provenance;
mod synthesise;

use std::collections::BTreeSet;
use std::path::Path;

use emery_source::Source;
use emery_source::claims::is_kebab;
pub use emery_source::types::SourceContent;
use emery_source::types::{Evidence, SourceInput};
use omnia_guest::api::Context;
use omnia_guest::plugins::Digest;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore, bad_request};
use serde::{Deserialize, Serialize};

use crate::plugin::{AdapterRef, Loader};
use crate::preopen_path;
use crate::store::Store;
pub use crate::store::{Changes, Diff};

/// Runs one `specify` over the context's provider: checks the source list,
/// extracts every source, derives the requirement rows, synthesises the two
/// documents, and commits them as a new revision.
///
/// # Errors
///
/// Returns `BadRequest` for a source the rules refuse or a claim the gate
/// rejects, and passes through the extract, synthesis, and store failures.
pub async fn specify<P: Model + Source + StateStore + BlobStore + Plugins>(
    input: Specify, context: Context<P>,
) -> Result<SpecifyBody, Error> {
    let provider = context.provider();

    // validate the source list
    validate(&input.sources)?;

    // call extract() for every source
    let extracted = extract(provider, &input.sources).await?;

    // synthesise extracted evidence into a single specification set
    let revision = synthesise::synthesise(provider, &extracted).await?;

    // save the specification set as a new revision
    let committed = Store::new(provider).commit(&revision).await?;

    Ok(SpecifyBody {
        revision: committed.id,
        diff: committed.diff,
    })
}

/// Generate a specification revision from sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Specify {
    /// The run's source configurations, in extraction order.
    pub sources: Vec<SourceConfig>,
}

/// A source for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceConfig {
    /// Stable kebab-case source key.
    pub key: String,
    /// The adapter selector.
    pub adapter: AdapterRef,
    /// What the adapter extracts: a project-relative read-only root
    /// (`.` binds the project) or an inline value.
    pub content: SourceContent,
    /// Optional sha256 content pin for a loader-loaded adapter,
    /// verified host-side before validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
    /// Optional registry endpoint override for a package adapter;
    /// `None` selects the acquirer's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

impl SourceConfig {
    // Loads this source's adapter under its pin and registry override.
    async fn load<P: Source + Plugins>(&self, loader: &Loader<'_, P>) -> Result<String, Error> {
        loader.load(&self.adapter, self.digest.as_ref(), self.registry.as_deref()).await
    }

    // Maps this source to the adapter `extract` input; the one place an
    // operator root meets the guest preopen.
    fn input(&self) -> Result<SourceInput, Error> {
        let content = match &self.content {
            // `.` spans the project preopen, including `.emery/`, until
            // guest capability profiles can exclude the revision store.
            SourceContent::Workspace(relative) => {
                let relative = preopen_path(Path::new(relative))?;
                let root = if relative == Path::new(".") {
                    relative
                } else {
                    Path::new(".").join(relative)
                };
                SourceContent::Workspace(root.display().to_string())
            }
            SourceContent::Value(text) => SourceContent::Value(text.clone()),
        };
        Ok(SourceInput {
            key: self.key.clone(),
            content,
        })
    }

    // Checks one source's rules: `registry` only means anything for a package
    // adapter, `digest` only for a loader-acquired one, and the root must pass
    // the rule `input` applies — so a bad list is refused before any load.
    fn validate(&self) -> Result<(), Error> {
        let key = &self.key;
        if self.registry.is_some() && !matches!(self.adapter, AdapterRef::Package { .. }) {
            return Err(bad_request!(
                "source `{key}`: `registry` requires a package adapter \
                 (`<namespace>:<name>@<version>`)"
            ));
        }
        if self.digest.is_some() && matches!(self.adapter, AdapterRef::Bare(_)) {
            return Err(bad_request!(
                "source `{key}`: `digest` requires a `.wasm` path or package adapter, not a bare \
                 name"
            ));
        }

        self.input().map(drop)
    }
}

/// Successful specification result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyBody {
    /// Committed revision id.
    pub revision: String,
    /// Diff from the predecessor; absent on the first run and when the
    /// superseded revision was unreadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
}

// Refuses an empty list (`specify-source-required`), a malformed or repeated
// key, a `digest` on a bare name the loader never acquires, a `registry` on
// a selector the registry never serves, or a root outside the preopen.
fn validate(sources: &[SourceConfig]) -> Result<(), Error> {
    if sources.is_empty() {
        return Err(Error::BadRequest {
            code: "specify-source-required".into(),
            description: "no sources".into(),
        });
    }

    let mut keys = BTreeSet::new();
    for source in sources {
        let key = source.key.as_str();
        if !is_kebab(key) {
            return Err(bad_request!("source `{key}` is not a kebab-case key"));
        }
        if !keys.insert(key) {
            return Err(bad_request!("source `{key}` appears twice"));
        }
        source.validate()?;
    }

    Ok(())
}

// Loads, extracts, and validates every source. Adapters are guests the engine
// did not write, so the contract's claim gate is re-run here (A8) before
// anything downstream trusts their claims; adapter failures arrive classified.
async fn extract<P: Source + Plugins>(
    provider: &P, sources: &[SourceConfig],
) -> Result<Vec<SourceEvidence>, Error> {
    let mut extracted = Vec::with_capacity(sources.len());
    let loader = Loader::new(provider);

    for source in sources {
        let input = source.input()?;
        let id = source.load(&loader).await?;

        let key = &source.key;
        tracing::debug!(source = %key, "extracting");
        let evidence = Source::extract(provider, &id, &input).await?;

        let findings = evidence.findings();
        if !findings.is_empty() {
            let findings = findings.join("\n");
            return Err(bad_request!("source `{key}` returned invalid claims:\n{findings}"));
        }

        extracted.push(SourceEvidence {
            key: key.clone(),
            evidence,
        });
    }

    Ok(extracted)
}

// One source's validated evidence, under the key the documents cite it by.
#[derive(Debug)]
struct SourceEvidence {
    key: String,
    evidence: Evidence,
}
