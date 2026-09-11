//! The `specify` operation
//!
//! Emery's central operation: given a list of sources, extract each
//! source's claims, derive the requirements under authority precedence,
//! synthesise `spec.md` and `design.md`, and commit the pair as one new
//! revision.
//!
//! A [`SourceConfig`] names one source to extract from: the adapter to use,
//! the key the specification will cite it by, and either a workspace to read
//! or an inline value. The list is per-run input, never stored, so the same
//! shape serves the command line, a config file, and any other transport,
//! and it is checked whole before a single adapter loads.
//!
//! A run may also carry a revision — the documents of an earlier revision that
//! travel beside the code they specify. The carried revision anchors the run:
//! it becomes the current revision, its requirements lend their ids to the
//! requirements that continue them, and only what the evidence changed is
//! drafted again. With nothing carried, the store's current revision anchors
//! the run the same way.
//!
//! The result reports what was committed — the revision id and the
//! diff against the anchoring revision — so a caller can see what
//! changed without reading the documents.

mod basis;
mod brief;
mod synthesis;

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
use serde_json::Value;

use crate::adapter::{AdapterRef, Loader};
use crate::artifact::Revision;
pub use crate::artifact::{ReqId, SectionKind};
pub use crate::store::{Changed, DesignDiff, Diff, Entry, SpecDiff};
use crate::{preopen_path, store};

/// Runs `specify` over the context's provider.
///
/// Checks the source list, anchors the run on the carried or current revision,
/// then extracts each source's evidence, derives the requirements,
/// synthesises the revision, and commits it.
///
/// # Errors
///
/// Returns `BadRequest` for a source the rules refuse, a claim the gate
/// rejects, or a carried revision that is outdated (`spec-outdated`) or not a
/// revision (`revision-invalid`), and passes through the extract, synthesis,
/// and store failures.
pub async fn specify<P: Model + Source + StateStore + BlobStore + Plugins>(
    input: SpecifyInput, context: Context<P>,
) -> Result<SpecifyOutput, Error> {
    let provider = context.provider();

    input.validate()?;

    let prior = input.revision(provider).await?;
    let extracts = input.extract(provider).await?;
    let revision = synthesis::synthesise(provider, &extracts, prior.as_ref()).await?;
    let (id, diff) = store::commit(provider, &revision).await?;

    Ok(SpecifyOutput { revision: id, diff })
}

/// Generate a specification revision from sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecifyInput {
    /// The run's source configurations, in extraction order.
    pub sources: Vec<SourceConfig>,
    /// The revision carried beside the code, when the project has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried: Option<Carried>,
}

/// The documents of an earlier revision left beside the code: the
/// `document` of each `show --format json` envelope, still unread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Carried {
    /// The specification document.
    pub spec: Value,
    /// The design document.
    pub design: Value,
}

impl SpecifyInput {
    fn validate(&self) -> Result<(), Error> {
        if self.sources.is_empty() {
            return Err(Error::BadRequest {
                code: "specify-source-required".into(),
                description: "no sources".into(),
            });
        }

        let mut keys = BTreeSet::new();
        for source in &self.sources {
            source.validate()?;
            if !keys.insert(source.key.as_str()) {
                return Err(bad_request!("source `{}` appears twice", source.key));
            }
        }

        Ok(())
    }

    async fn revision<P: StateStore + BlobStore>(
        &self, provider: &P,
    ) -> Result<Option<Revision>, Error> {
        if let Some(Carried { spec, design }) = &self.carried {
            let revision = Revision::read(spec.clone(), design.clone())?;
            store::adopt(provider, &revision).await?;
            Ok(Some(revision))
        } else {
            Ok(store::current(provider).await.ok().flatten())
        }
    }

    async fn extract<P: Source + Plugins>(&self, provider: &P) -> Result<Vec<Extract>, Error> {
        let mut extracts = Vec::with_capacity(self.sources.len());
        let loader = Loader::new(provider);

        for source in &self.sources {
            let input = source.input()?;
            let id = loader
                .load(&source.adapter, source.digest.as_ref(), source.registry.as_deref())
                .await?;

            let key = &source.key;
            tracing::debug!(source = %key, "extracting");
            let evidence = Source::extract(provider, &id, &input).await?;

            // The adapter is a guest the engine did not write, so the
            // contract's claim gate is re-run here, fail-closed.
            let findings = evidence.findings();
            if !findings.is_empty() {
                let findings = findings.join("\n");
                return Err(bad_request!("source `{key}` returned invalid claims:\n{findings}"));
            }

            extracts.push(Extract {
                key: key.clone(),
                evidence,
            });
        }

        Ok(extracts)
    }
}

// One source's validated evidence, under the key the documents cite it by.
#[derive(Debug)]
struct Extract {
    key: String,
    evidence: Evidence,
}

/// A source for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Stable kebab-case source key.
    pub key: String,
    /// Which adapter extracts this source.
    pub adapter: AdapterRef,
    /// What the adapter extracts: a project-relative read-only root
    /// (`.` binds the project) or an inline value.
    pub content: SourceContent,
    /// Optional sha256 content pin for a loader-loaded adapter,
    /// verified host-side before validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
    /// Optional registry endpoint override for a package adapter;
    /// `None` selects the acquirer's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

impl SourceConfig {
    // Maps this source to the adapter `extract` input; the one place an
    // operator root meets the guest preopen.
    fn input(&self) -> Result<SourceInput, Error> {
        let content = match &self.content {
            // `.` spans the project preopen, including `.omnia/`, until
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
            value @ SourceContent::Value(_) => value.clone(),
        };
        Ok(SourceInput {
            key: self.key.clone(),
            content,
        })
    }

    // Checks one source's rules: the key is kebab-case, `registry` only means
    // anything for a package adapter, `digest` only for a loader-acquired one,
    // and the root must pass the rule `input` applies — before any load.
    fn validate(&self) -> Result<(), Error> {
        let key = &self.key;
        if !is_kebab(key) {
            return Err(bad_request!("source `{key}` is not a kebab-case key"));
        }
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

        self.input()?;
        Ok(())
    }
}

/// Successful specification result: the revision the store committed.
#[derive(Debug, Serialize)]
pub struct SpecifyOutput {
    /// Committed revision id.
    pub revision: String,
    /// Diff from the anchoring revision; absent on the first run and when
    /// the outgoing revision was unreadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
}
