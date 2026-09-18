//! Generates specification revisions from configured sources.
//!
//! [`specify`] validates the complete source list before loading adapters.
//! Sources are extracted concurrently, then their claims are reconciled by
//! authority and synthesised into `spec.md` and `design.md`.
//!
//! The two documents are committed as one content-addressed revision. An
//! earlier revision contributes only the returned [`Diff`]; it is never used
//! as synthesis input.

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
use omnia_sdk::api::Context;
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, bad_request, server_error};
use serde::{Deserialize, Serialize};

use self::basis::GroupingBrief;
use self::brief::Brief as _;
use self::design::DesignBrief;
use self::spec::SpecBrief;
use crate::adapter::{self, AdapterRef};
use crate::revision::Revision;
pub use crate::revision::{Changed, DesignDiff, Diff, Entry, ReqId, SectionKind, SpecDiff};
use crate::{preopen_path, store};

/// Generates and commits a specification revision from `input`.
///
/// The source list is validated before any adapter loads. Extraction runs
/// concurrently and waits for every source, allowing all extraction failures
/// to be reported together.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] for an empty source list, duplicate or
///   malformed source keys, a workspace path outside the project, an adapter
///   input refusal, an incompatible adapter, or a synthesis response that
///   cannot be accepted. An empty list uses code `specify-source-required`;
///   an incompatible adapter uses code `unsupported-version`.
/// - Returns [`Error::NotFound`] when a local adapter does not exist.
/// - Returns [`Error::ServerError`] when extracted evidence has findings from
///   [`Evidence::findings`], or internal validation, serialisation, or storage
///   fails.
/// - Returns [`Error::BadGateway`] when adapter acquisition or extraction, or
///   a model operation, fails upstream.
///
/// Errors from [`Plugins::load`] retain their original class and code.
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

/// The sources used to generate one revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyInput {
    /// Sources in declaration order, which reconciliation preserves for
    /// stable requirement numbering.
    pub sources: Vec<SourceConfig>,
}

/// Configuration for one source used by [`specify`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceConfig {
    /// The kebab-case key used to cite this source.
    pub key: String,
    /// The adapter that extracts the source.
    pub adapter: AdapterRef,
    /// A project-relative workspace or inline text read by the adapter.
    ///
    /// `.` identifies the project root.
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

/// The revision committed by a successful [`specify`] operation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyOutput {
    /// The content identifier of the committed revision.
    pub revision: String,
    /// Changes from the displaced revision.
    ///
    /// Absent on the first run, and when the outgoing revision was unreadable.
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::brief::Brief as _;
    use super::{DesignBrief, GroupingBrief, SpecBrief};

    // Keep (entry-point-unreachable): a synthesis document the list leaves
    // out, or a link no listed document answers, is invisible to every run;
    // one no brief puts to the model is embedded and never read.
    #[test]
    fn corpus() {
        let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("prose");
        let prompts = [GroupingBrief::PROSE, SpecBrief::PROSE, DesignBrief::PROSE].concat();
        let findings = emery_prose::check(crate::PROSE, &tree, &prompts, &[]);
        assert!(findings.is_empty(), "{}", findings.join("\n"));
    }
}
