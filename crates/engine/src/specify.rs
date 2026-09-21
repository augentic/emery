//! Generates specification revisions from configured sources.
//!
//! [`specify`] validates the complete source list before loading adapters.
//! Sources are extracted concurrently, and the first failure among them ends
//! the run; their claims are then reconciled by authority and synthesised into
//! `spec.md` and `design.md`.
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
/// The source list is checked before any adapter loads. Sources are extracted
/// concurrently, and the first failure among them ends the run.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] for an empty source list (code
///   `specify-source-required`), a malformed or repeated key, a workspace path
///   outside the project, an incompatible adapter (code `unsupported-version`),
///   a source that refuses its input, or a synthesis answer that cannot be
///   accepted.
/// - Returns [`Error::NotFound`] when a local adapter does not exist.
/// - Returns [`Error::ServerError`] when evidence has [`Evidence::findings`],
///   or serialisation or storage fails.
/// - Returns [`Error::BadGateway`] when an adapter, its acquisition, or the
///   model fails upstream.
///
/// A loader's or a source's error keeps its own class and code.
pub async fn specify<P: Model + Source + StateStore + BlobStore + Plugins>(
    input: SpecifyInput, context: Context<P>,
) -> Result<SpecifyOutput, Error> {
    let provider = context.provider();

    let bound = Bound::all(&input.sources)?;
    let kinds = &adapter::load(provider, bound.iter().map(|source| source.adapter)).await?;

    let extracts =
        future::try_join_all(bound.iter().map(|source| source.extract(provider, kinds))).await?;

    let bases = GroupingBrief::new(&extracts).derive(provider).await?;
    let spec = SpecBrief::new(&extracts, &bases).judge(provider).await?;
    let design = DesignBrief::new(&extracts, &spec).judge(provider).await?;

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

struct Bound<'a> {
    adapter: &'a AdapterRef,
    input: SourceInput,
}

impl<'a> Bound<'a> {
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

    #[tracing::instrument(skip_all, fields(source = %self.input.key, adapter = %self.adapter))]
    async fn extract<S: Source>(
        &self, provider: &S, kinds: &BTreeMap<String, SourceKind>,
    ) -> Result<Extract, Error> {
        let source = &self.input.key;
        let adapter = self.adapter.to_string();

        let kind = kinds
            .get(&adapter)
            .copied()
            .ok_or_else(|| server_error!("adapter `{adapter}` was not loaded"))?;
        tracing::info!(%source, %adapter, %kind, "extracting");
        let evidence = Source::extract(provider, &adapter, &self.input).await?;

        let findings = evidence.findings();
        if !findings.is_empty() {
            return Err(server_error!(
                "`{source}` returned invalid claims:\n{}",
                findings.join("\n")
            ));
        }
        tracing::debug!(%source, claims = evidence.claims.len(), "extracted");

        Ok(Extract {
            source: source.clone(),
            kind,
            evidence,
        })
    }
}

#[derive(Debug)]
struct Extract {
    source: String,
    kind: SourceKind,
    evidence: Evidence,
}

static PROSE: &[emery_prose::Doc] = emery_prose::prose![
    "../prose/authority.md",
    "../prose/claim-landing.md",
    "../prose/design-format.md",
    "../prose/grouping.md",
    "../prose/requirement-block.md",
    "../prose/spec-format.md",
    "../prose/synthesise.md",
    "../prose/tags.md",
];

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::brief::Brief as _;
    use super::{DesignBrief, GroupingBrief, SpecBrief};

    #[test]
    fn corpus() {
        let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("prose");
        let prompts = [GroupingBrief::PROSE, SpecBrief::PROSE, DesignBrief::PROSE].concat();
        let findings = emery_prose::check(super::PROSE, &tree, &prompts, &[]);
        assert!(findings.is_empty(), "{}", findings.join("\n"));
    }
}
