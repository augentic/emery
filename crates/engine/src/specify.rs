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
//! Every source extracts at once and the run waits for all of them, so a run
//! takes as long as its slowest source, and every source that fails is
//! reported together, in declaration order, rather than only the first.
//!
//! Every run starts from its sources alone: nothing of an earlier revision
//! is read into the synthesis. The result reports what was committed — the
//! revision id and the diff against the revision it displaced — so a caller
//! can see what changed without reading the documents.

mod basis;
mod brief;
mod design;
mod spec;

use std::collections::BTreeSet;
use std::path::Path;

use emery_adapter::is_kebab;
pub use emery_adapter::source::SourceContent;
use emery_adapter::source::{Evidence, Source, SourceInput};
use futures::future;
use omnia_guest::api::Context;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore, bad_request, server_error};
use serde::{Deserialize, Serialize};

use self::basis::GroupingBrief;
use self::brief::Brief as _;
use self::design::DesignBrief;
use self::spec::SpecBrief;
use crate::adapter::{self, AdapterRef};
use crate::revision::Revision;
pub use crate::revision::{Changed, DesignDiff, Diff, Entry, ReqId, SectionKind, SpecDiff};
use crate::{preopen_path, store};

/// Runs `specify` over the context's provider.
///
/// Checks the source list whole, loads the adapters it names, extracts every
/// source's evidence at once, derives the requirement bases, drafts the
/// specification and then the design over them, and commits the pair as one
/// revision.
///
/// # Errors
///
/// Returns `BadRequest` for a source the rules refuse or a draft the model
/// could not bring within the brief's rounds; `ServerError` when one or more
/// source extractions fail; `BadGateway` for a model failure; and passes
/// through load and store failures.
pub async fn specify<P: Model + Source + StateStore + BlobStore + Plugins>(
    input: SpecifyInput, context: Context<P>,
) -> Result<SpecifyOutput, Error> {
    let provider = context.provider();

    // load adapters
    let adapters = input.sources.iter().map(|s| (&s.adapter, s.registry.as_deref()));
    adapter::load(provider, adapters).await?;

    // Extracts every source together and waits for all of them, so a run with
    // several failing sources reports every failure rather than the first one
    // the race happened to surface.
    let inputs = input.to_inputs()?;
    let outcomes = future::join_all(
        input
            .sources
            .iter()
            .zip(inputs)
            .map(|(source, input)| extract(provider, &source.adapter, input)),
    )
    .await;

    let mut extracts = Vec::with_capacity(outcomes.len());
    let mut failures = Vec::new();
    for outcome in outcomes {
        match outcome {
            Ok(extracted) => extracts.push(extracted),
            Err(error) => failures.push(error.description()),
        }
    }
    if !failures.is_empty() {
        return Err(server_error!(failures.join("\n")));
    }

    let bases = GroupingBrief::new(&extracts).derive(provider).await?;
    let spec = SpecBrief::new(&extracts, &bases).judge(provider).await?;
    let design = DesignBrief::new(&extracts, &spec).judge(provider).await?;
    let (id, diff) = store::commit(provider, &Revision { spec, design }).await?;

    Ok(SpecifyOutput { revision: id, diff })
}

/// Generate a specification revision from sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyInput {
    /// The run's source configurations, in declaration order.
    pub sources: Vec<SourceConfig>,
}

impl SpecifyInput {
    // Checks the rules every transport must get — a non-empty list, unique
    // keys, and each source's own — and prepares every guest input, all
    // before any adapter loads.
    fn to_inputs(&self) -> Result<Vec<SourceInput>, Error> {
        if self.sources.is_empty() {
            return Err(Error::BadRequest {
                code: "specify-source-required".into(),
                description: "no sources".into(),
            });
        }

        let mut keys = BTreeSet::new();
        let mut inputs = Vec::with_capacity(self.sources.len());
        for source in &self.sources {
            let input = source.prepare()?;
            if !keys.insert(source.key.as_str()) {
                return Err(bad_request!("source `{}` appears twice", source.key));
            }
            inputs.push(input);
        }

        Ok(inputs)
    }
}

/// A source for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceConfig {
    /// Stable kebab-case source key.
    pub key: String,
    /// Which adapter extracts this source.
    pub adapter: AdapterRef,
    /// What the adapter extracts: a project-relative read-only root
    /// (`.` binds the project) or an inline value.
    pub content: SourceContent,
    /// Optional registry endpoint override for a package adapter;
    /// `None` selects the acquirer's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

impl SourceConfig {
    // Checks this source's rules and maps it to the adapter's `extract`
    // input; the one place an operator root meets the guest preopen.
    fn prepare(&self) -> Result<SourceInput, Error> {
        let key = &self.key;
        if !is_kebab(key) {
            return Err(bad_request!("source `{key}` is not a kebab-case key"));
        }
        if self.registry.is_some() && !matches!(self.adapter, AdapterRef::Package(_)) {
            return Err(bad_request!(
                "source `{key}`: `registry` requires a package adapter \
                 (`<namespace>:<name>@<version>`)"
            ));
        }

        let content = match &self.content {
            // The adapter lends the root to the model by preopen name, so it
            // is spelled beneath the `.` mount: `.` itself, or `./<path>`.
            // `.` spans the whole project, `.omnia/` included, until guest
            // capability profiles can exclude the revision store.
            SourceContent::Workspace(relative) => {
                let relative = preopen_path(Path::new(relative))?.display().to_string();
                let root = if relative == "." { relative } else { format!("./{relative}") };
                SourceContent::Workspace(root)
            }
            value @ SourceContent::Value(_) => value.clone(),
        };

        Ok(SourceInput {
            key: key.clone(),
            content,
        })
    }
}

/// Successful specification result: the revision the store committed.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyOutput {
    /// Committed revision id.
    pub revision: String,
    /// Diff from the displaced revision; absent on the first run and when
    /// the outgoing revision was unreadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
}

// Extracts one source and gates its claims. The adapter checks its own
// answer before returning it, so a document that fails the gate here is
// the engine's failure to report, not the operator's.
async fn extract<P: Source>(
    provider: &P, adapter: &AdapterRef, input: SourceInput,
) -> Result<Extract, Error> {
    let id = adapter.to_string();
    tracing::debug!(source = %input.key, adapter = %id, "extracting");

    let outcome = Source::extract(provider, &id, &input).await.and_then(|evidence| {
        let findings = evidence.findings();
        if findings.is_empty() {
            Ok(evidence)
        } else {
            Err(server_error!("`{}` returned invalid claims:\n{}", input.key, findings.join("\n")))
        }
    });
    if let Err(error) = &outcome {
        tracing::warn!(source = %input.key, adapter = %id, %error, "extract failed");
    }

    outcome.map(|evidence| Extract {
        key: input.key,
        evidence,
    })
}

// One source's validated evidence, under the key the documents cite it by.
#[derive(Debug)]
struct Extract {
    key: String,
    evidence: Evidence,
}
