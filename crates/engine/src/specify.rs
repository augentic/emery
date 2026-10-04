//! Generates specification revisions from configured sources.
//!
//! [`specify`] validates the complete source list before loading adapters.
//! Sources are extracted concurrently, and the first failure among them ends
//! the run; their claims are then reconciled by authority into requirement
//! bases, from which `spec.md` — in chunks of at most [`SPEC_CHUNK`]
//! requirements, grouped by stem — `design.md`, and the `plan.md` slicing are
//! drafted together.
//!
//! The three documents are committed as one content-addressed revision. An
//! earlier revision contributes only the returned [`Diff`]; it is never used
//! as synthesis input.

mod basis;
mod brief;
mod design;
mod plan;
mod shape;
mod spec;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use emery_adapter::is_kebab;
pub use emery_adapter::source::SourceContent;
use emery_adapter::source::{AdapterMetadata, Evidence, Source, SourceInput, SourceKind};
use futures::future;
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, bad_request, server_error};
use serde::{Deserialize, Serialize};

use self::basis::GroupingBrief;
use self::brief::Brief as _;
use self::design::DesignBrief;
use self::plan::SliceBrief;
pub use self::spec::SPEC_CHUNK;
use self::spec::SpecBrief;
use crate::adapter::{self, AdapterRef, Loaded, Registries};
use crate::revision::Revision;
pub use crate::revision::{
    Changed, DesignDiff, Diff, Entry, PlanDiff, ReqId, SectionKind, SliceEntry, SliceId, SpecDiff,
};
use crate::{preopen_path, store};

/// Generates and commits a specification revision from `input`.
///
/// The source list is checked before any adapter loads. Sources are extracted
/// concurrently, and the first failure among them ends the run.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the run is refused:
///   - an empty source list, with code `specify-source-required`;
///   - a malformed or repeated source name;
///   - a workspace path outside the project;
///   - a package no registry routes, or a digest on a declared guest;
///   - two adapters naming one guest, or one naming the engine's own
///     ([`ENGINE`](crate::ENGINE));
///   - an adapter that resolves to other bytes than its digest pin, with code
///     `refused`;
///   - an incompatible adapter, with code `unsupported-version`;
///   - a source that refuses its input;
///   - sources that between them contribute no requirement claim;
///   - a synthesis or slicing answer that cannot be accepted.
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
    let loaded = &adapter::load(
        provider,
        bound.iter().map(|source| (source.adapter, source.digest)),
        &input.registries,
    )
    .await?;

    let extracts =
        future::try_join_all(bound.iter().map(|source| source.extract(provider, loaded))).await?;

    let bases = GroupingBrief::new(&extracts).derive(provider).await?;
    let design = DesignBrief::new(&extracts, &bases);
    let plan = SliceBrief::new(&bases, design.types());
    let chunks = SpecBrief::chunked(&extracts, &bases);
    let (drafts, design, plan) = future::try_join3(
        future::try_join_all(chunks.into_iter().map(|chunk| chunk.judge(provider))),
        design.judge(provider),
        plan.derive(provider),
    )
    .await?;
    let spec = SpecBrief::assemble(drafts);

    let (revision, diff) = store::commit(provider, &Revision { spec, design, plan }).await?;

    Ok(SpecifyOutput { revision, diff })
}

/// The sources used to generate one revision.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyInput {
    /// Sources in declaration order, which reconciliation preserves for
    /// stable requirement numbering.
    pub sources: Vec<SourceConfig>,
    /// The registries package adapters fetch from, by namespace.
    ///
    /// Empty, only the `emery` namespace routes.
    #[serde(default)]
    pub registries: Registries,
}

/// Configuration for one source used by [`specify`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceConfig {
    /// The kebab-case name the specification cites this source by.
    ///
    /// The caller's own name for the source, or the adapter's
    /// ([`AdapterRef::name`]) when the caller gives none.
    pub name: String,
    /// The adapter that extracts the source.
    pub adapter: AdapterRef,
    /// A project-relative workspace or inline text read by the adapter.
    ///
    /// `.` identifies the project root.
    pub content: SourceContent,
    /// The `sha256:` digest the adapter's component must resolve to.
    ///
    /// The run passes it on the load, and the loader holds the resolved
    /// bytes to it. `None` trusts whatever the load resolves. A declared
    /// guest takes none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
}

impl SourceConfig {
    // The one place an operator root meets the guest preopen.
    fn prepare(&self) -> Result<SourceInput, Error> {
        let name = &self.name;
        if !is_kebab(name) {
            return Err(bad_request!("source `{name}` is not a kebab-case name"));
        }

        // spell a lent root beneath the `.` mount: `.` itself, or `./<path>`
        let content = match &self.content {
            SourceContent::Workspace(relative) => {
                let relative = preopen_path(Path::new(relative))?.display().to_string();
                let root = if relative == "." { relative } else { format!("./{relative}") };
                SourceContent::Workspace(root)
            }
            value @ SourceContent::Value(_) => value.clone(),
        };

        Ok(SourceInput {
            name: name.clone(),
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
    digest: Option<&'a Digest>,
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

        let mut names = BTreeSet::new();
        let mut bound = Vec::with_capacity(sources.len());
        for source in sources {
            let input = source.prepare()?;
            if !names.insert(source.name.as_str()) {
                return Err(bad_request!("source `{}` appears twice", source.name));
            }
            bound.push(Self {
                adapter: &source.adapter,
                digest: source.digest.as_ref(),
                input,
            });
        }

        Ok(bound)
    }

    #[tracing::instrument(skip_all, fields(source = %self.input.name, adapter = %self.adapter))]
    async fn extract<S: Source>(
        &self, provider: &S, loaded: &BTreeMap<String, Loaded<AdapterMetadata>>,
    ) -> Result<Extract, Error> {
        let source = &self.input.name;
        let adapter = self.adapter.to_string();

        let Loaded { id, metadata } = loaded
            .get(&adapter)
            .ok_or_else(|| server_error!("adapter `{adapter}` was not loaded"))?;
        let kind = metadata.kind;
        tracing::info!(%source, adapter = %id, %kind, "extracting");
        let evidence = Source::extract(provider, id, &self.input).await?;

        let findings = evidence.findings();
        if !findings.is_empty() {
            return Err(server_error!(
                "`{source}` returned invalid claims:\n{}",
                findings.join("\n")
            ));
        }
        tracing::info!(
            %source,
            claims = evidence.claims.len(),
            kinds = shape::kinds(&evidence),
            stems = shape::stems(&evidence),
            "extracted"
        );

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
    "../prose/slicing.md",
    "../prose/spec-format.md",
    "../prose/synthesise.md",
    "../prose/tags.md",
];

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::brief::Brief as _;
    use super::{DesignBrief, GroupingBrief, SliceBrief, SpecBrief};

    #[test]
    fn corpus() {
        let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("prose");
        let prompts =
            [GroupingBrief::PROSE, SpecBrief::PROSE, DesignBrief::PROSE, SliceBrief::PROSE]
                .concat();
        let findings = emery_prose::check(super::PROSE, &tree, &prompts, &[]);
        assert!(findings.is_empty(), "{}", findings.join("\n"));
    }
}
