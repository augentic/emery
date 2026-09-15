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

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use emery_adapter::is_kebab;
pub use emery_adapter::source::SourceContent;
use emery_adapter::source::{Evidence, Source, SourceInput, SourceKind};
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

    // load source adapters
    let bound = Bound::all(&input.sources)?;
    let kinds = adapter::load(provider, bound.iter().map(|source| source.adapter)).await?;

    // extract all sources in parallel, each ranked by its adapter's kind
    let outcomes =
        future::join_all(bound.iter().map(|source| source.extract(provider, &kinds))).await;

    // collect extracts or findings for failed extracts
    let mut extracts = Vec::with_capacity(outcomes.len());
    let mut failures = Vec::new();
    for (source, outcome) in bound.iter().zip(outcomes) {
        match outcome {
            Ok(extract) => extracts.push(extract),
            Err(error) => {
                tracing::warn!(source = %source.input.key, %error, "extract failed");
                failures.push(error.description());
            }
        }
    }

    if !failures.is_empty() {
        return Err(server_error!(failures.join("\n")));
    }

    // synthesise extracts into a unified set of specifications
    let bases = GroupingBrief::new(&extracts).derive(provider).await?;
    let spec = SpecBrief::new(&extracts, &bases).judge(provider).await?;
    let design = DesignBrief::new(&extracts, &spec).judge(provider).await?;

    // commit the revision
    let (revision, diff) = store::commit(provider, &Revision { spec, design }).await?;

    Ok(SpecifyOutput { revision, diff })
}

/// Generate a specification revision from sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyInput {
    /// The run's source configurations, in declaration order.
    pub sources: Vec<SourceConfig>,
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
}

impl SourceConfig {
    // Checks this source's rules and maps it to the adapter's `extract`
    // input; the one place an operator root meets the guest preopen.
    fn prepare(&self) -> Result<SourceInput, Error> {
        let key = &self.key;
        if !is_kebab(key) {
            return Err(bad_request!("source `{key}` is not a kebab-case key"));
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

// One source bound to its adapter: the adapter the call routes to and the
// input it carries.
struct Bound<'a> {
    adapter: &'a AdapterRef,
    input: SourceInput,
}

impl<'a> Bound<'a> {
    // Binds every source under the rules every transport must get — a
    // non-empty list, unique keys, and each source's own — before any
    // adapter loads.
    fn all(sources: &'a [SourceConfig]) -> Result<Vec<Self>, Error> {
        if sources.is_empty() {
            return Err(Error::BadRequest {
                code: "specify-source-required".into(),
                description: "no sources".into(),
            });
        }

        let mut keys = BTreeSet::new();
        let mut bound = Vec::with_capacity(sources.len());
        for source in sources {
            let input = source.prepare()?;
            if !keys.insert(source.key.as_str()) {
                return Err(bad_request!("source `{}` appears twice", source.key));
            }
            bound.push(Self {
                adapter: &source.adapter,
                input,
            });
        }

        Ok(bound)
    }

    // Extracts the source under the kind its adapter declared at load.
    async fn extract<P: Source>(
        &self, provider: &P, kinds: &BTreeMap<String, SourceKind>,
    ) -> Result<Extract, Error> {
        let source = &self.input.key;
        let adapter = self.adapter.to_string();
        // The load registered every bound adapter, so an absent kind is the
        // engine's own slip, never the operator's.
        let kind = kinds
            .get(&adapter)
            .copied()
            .ok_or_else(|| server_error!("adapter `{adapter}` was not loaded"))?;
        tracing::debug!(%source, %kind, "extracting");

        let evidence = Source::extract(provider, &adapter, &self.input).await?;

        let findings = evidence.findings();
        if !findings.is_empty() {
            return Err(server_error!(
                "`{source}` returned invalid claims:\n{}",
                findings.join("\n")
            ));
        }

        Ok(Extract {
            source: source.clone(),
            kind,
            evidence,
        })
    }
}

// One source's validated evidence, under the key the documents cite it by
// and the kind its adapter declared, which ranks it against the others.
#[derive(Debug)]
struct Extract {
    source: String,
    kind: SourceKind,
    evidence: Evidence,
}
