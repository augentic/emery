//! Generates specification candidates from configured sources.
//!
//! [`specify`] extracts claims from each source adapter, reconciles
//! requirements by authority rank, synthesises the specification, design, and
//! build plan, and stages them as one content-addressed candidate for
//! [`apply`](crate::apply::apply) to seal, or seals it in the same run.

mod basis;
mod design;
mod plan;
mod sections;
mod shape;
mod spec;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use emery_adapter::is_kebab;
pub use emery_adapter::source::SourceContent;
use emery_adapter::source::{Evidence, Source, SourceInput, SourceKind, SourceMetadata};
use futures::future;
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, Vcs, bad_request, server_error};
use serde::{Deserialize, Serialize};

pub(crate) use self::basis::GroupingBrief;
pub(crate) use self::design::DesignBrief;
pub use self::plan::SLICE_CAP;
pub(crate) use self::plan::SliceBrief;
pub use self::spec::SPEC_CHUNK;
pub(crate) use self::spec::SpecBrief;
use crate::adapter::{self, AdapterRef, Loaded};
pub use crate::apply::ApplyOutput;
use crate::brief::Brief as _;
use crate::revision::Revision;
pub use crate::revision::{
    BadId, Changed, DesignDiff, Diff, Entry, PlanDiff, ReqId, Role, SectionKind, SliceEntry,
    SliceId, SpecDiff, Waves,
};
use crate::vcs::{SourceClone, TargetRef, WorkingCopy};
use crate::{Files, Rank, apply, preopen_path, store, vcs};

/// Generates and stages a specification candidate from `input`, sealing it when asked.
///
/// The source list is checked before any adapter loads. A source read from a
/// repository is checked out at its revision after the adapters load, each
/// repository fetched or cloned once however many sources name it, and its
/// working copy is removed once every source has answered, whether or not
/// extraction succeeded. Sources are extracted concurrently, and the first
/// failure among them ends the run.
///
/// The drafted revision is staged as the candidate `emery apply` seals,
/// replacing any revision staged before it; a staged amendment is never
/// replaced, and the run is refused before any adapter loads while one
/// stands. Where [`SpecifyInput::target`] names the
/// repository the specification lives in, the current revision is read
/// from it before any adapter loads, to report the diff against it; a run
/// naming no target reads no repository and reports no diff. Under
/// [`SpecifyInput::apply`] the candidate is staged first and sealed in the
/// same run over what was read, so a seal that fails leaves it staged.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] when the run is refused:
///   - an empty source list, with code `specify-source-required`;
///   - an amendment standing staged, with code `amendment-pending`;
///   - [`SpecifyInput::apply`] with no target, with code
///     `build-target-required`;
///   - a target whose name is not kebab-case;
///   - a malformed or repeated source name;
///   - a workspace path outside the project, or outside the repository a
///     source names;
///   - a repository source with inline text in place of a path;
///   - two digests on one adapter, or two adapters naming one guest — two
///     versions of one package;
///   - an adapter that is not a source adapter ([`Axis::Source`](crate::Axis));
///   - a release the store lacks whose namespace the deployment routes
///     nowhere, a pre-compiled artifact, or an adapter that resolves to other
///     bytes than its digest pin, with code `refused`;
///   - an incompatible adapter, with code `unsupported-version`;
///   - a source that refuses its input;
///   - sources that between them contribute no requirement claim;
///   - a synthesis or slicing answer that cannot be accepted.
/// - Returns [`Error::NotFound`] with code `revision-not-found` when a
///   repository, or the revision a source reads it at, does not exist.
/// - Returns [`Error::ServerError`] when evidence has [`Evidence::findings`],
///   or serialisation, storage, or version control fails.
/// - Returns [`Error::BadGateway`] when an adapter, its acquisition, the
///   model, or a repository's remote fails upstream.
/// - Returns what [`apply`](crate::apply::apply) fails with reading the
///   target's repository, before any adapter loads, and sealing the
///   candidate under [`SpecifyInput::apply`], the candidate left staged.
///
/// A loader's or a source's error keeps its own class and code.
pub async fn specify<P: Model + Source + StateStore + BlobStore + Plugins + Vcs + Files>(
    input: SpecifyInput, context: Context<P>,
) -> Result<SpecifyOutput, Error> {
    let provider = context.provider();

    let bound = Bound::all(&input.sources)?;
    store::no_amendment(provider).await?;

    // the repository the specification lives in, read before any load so
    // the diff and the seal land over what it holds
    let repo = match &input.target {
        Some(target) => Some(target.repo()?),
        None if input.apply => return Err(store::target_required("`emery specify --apply`")),
        None => None,
    };
    let standing = match &repo {
        Some(repo) => Some(store::current(provider, repo).await?),
        None => None,
    };

    let loaded =
        &adapter::load(provider, bound.iter().map(|source| (source.adapter, source.digest)))
            .await?;

    // every repository source is read in a working copy that lasts the extraction
    let checkouts = Checkouts::prepare(provider, &bound).await?;
    let extracted =
        future::try_join_all(bound.iter().map(|source| source.extract(provider, loaded))).await;
    let removed = checkouts.remove(provider).await;
    let extracts = extracted?;
    let repositories = removed?;

    let bases = GroupingBrief::new(&extracts).derive(provider).await?;
    let chunks = SpecBrief::chunked(&extracts, &bases);

    // the slicing owns the design's data types alone, so it follows the design
    let (drafts, (design, plan)) = future::try_join(
        future::try_join_all(chunks.into_iter().map(|chunk| chunk.judge(provider))),
        async {
            let design = DesignBrief::new(&extracts, &bases).judge(provider).await?;
            let plan = SliceBrief::new(&bases, design.data_types()).derive(provider).await?;
            Ok::<_, Error>((design, plan))
        },
    )
    .await?;
    let spec = SpecBrief::assemble(drafts);
    let waves = plan.waves();

    // the candidate staged for review, with the diff against the revision a
    // seal would displace; sealed in the same run when asked
    let candidate = Revision { spec, design, plan };
    let diff = standing.as_ref().and_then(|standing| standing.diff(&candidate));
    let revision = store::stage(provider, &candidate).await?;
    tracing::info!(candidate = %revision, slices = candidate.plan.slices.len(), "staged");
    let applied = match (input.apply, repo, standing) {
        (true, Some(repo), Some(standing)) => Some(apply::seal(provider, &repo, standing).await?),
        _ => None,
    };

    Ok(SpecifyOutput {
        revision,
        waves,
        diff,
        repositories,
        staged: applied.is_none(),
        applied,
    })
}

/// The sources used to generate one candidate, and whether to seal it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyInput {
    /// Sources in declaration order, which reconciliation preserves for
    /// stable requirement numbering.
    pub sources: Vec<SourceConfig>,
    /// Whether to seal the staged candidate as the current revision in the
    /// same run, as [`apply`](crate::apply::apply) would.
    #[serde(default)]
    pub apply: bool,
    /// The target whose repository the specification lives in.
    ///
    /// Read for the diff where given, and required under
    /// [`apply`](Self::apply).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetRef>,
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
    /// A workspace path or inline text read by the adapter.
    ///
    /// A path is relative to the project root, or to the clone when
    /// [`repository`](Self::repository) is set; `.` identifies the root.
    pub content: SourceContent,
    /// The repository the source is read from, at a revision.
    ///
    /// `None` reads the project tree. Set, the source's path is a working
    /// copy of the repository at the revision, cut for the run and removed
    /// after it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<SourceRepository>,
    /// The `sha256:` digest the adapter's component must resolve to.
    ///
    /// The run passes it on the load, and the loader holds the resolved
    /// bytes to it, stored or fetched. `None` trusts whatever the load
    /// resolves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
    /// The authority rank the source is reconciled under.
    ///
    /// `None` ranks the source by its adapter's kind ([`Rank::from`]): intent
    /// `1`, documentation `2`, behaviour `3`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<Rank>,
    /// Whether the source states a change alone or its subject whole.
    #[serde(default)]
    pub scope: Scope,
}

/// How much of its subject a [`SourceConfig`] states.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    /// The source states only what changes, and behaviour it is silent on stands.
    #[default]
    Delta,
    /// The source states its subject's behaviour whole.
    Whole,
}

/// A repository a source is read from, at a revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceRepository {
    /// The URL the repository is cloned from.
    pub url: String,
    /// The label, tag, or commit the source is read at, resolved each run.
    pub revision: String,
}

impl SourceConfig {
    // The one place an operator root meets the guest preopen.
    fn prepare(&self) -> Result<SourceInput, Error> {
        let name = &self.name;
        if !is_kebab(name) {
            return Err(bad_request!("source `{name}` is not a kebab-case name"));
        }

        // spell a lent root beneath the `.` mount: the project root or the
        // source's working copy itself, or `<root>/<path>`
        let content = match (&self.content, &self.repository) {
            (SourceContent::Workspace(relative), None) => {
                let relative = preopen_path(Path::new(relative))?.display().to_string();
                let root = if relative == "." { relative } else { format!("./{relative}") };
                SourceContent::Workspace(root)
            }
            (SourceContent::Workspace(relative), Some(_)) => {
                let Ok(within) = preopen_path(Path::new(relative)) else {
                    return Err(bad_request!(
                        "source `{name}`: path `{relative}` must lie within the repository"
                    ));
                };
                let checkout = vcs::source_checkout(name);
                let root = if within == Path::new(".") {
                    checkout
                } else {
                    format!("{checkout}/{}", within.display())
                };
                SourceContent::Workspace(root)
            }
            (value @ SourceContent::Value(_), None) => value.clone(),
            (SourceContent::Value(_), Some(_)) => {
                return Err(bad_request!(
                    "source `{name}` names a repository beside inline text; a repository is \
                     read at a path"
                ));
            }
        };

        Ok(SourceInput {
            name: name.clone(),
            content,
        })
    }
}

/// The candidate staged by a successful [`specify`] operation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyOutput {
    /// The content identifier of the staged candidate.
    pub revision: String,
    /// The candidate plan's slices grouped into the sets ready to build at once.
    pub waves: Waves,
    /// Changes from the current revision, which sealing the candidate would displace.
    ///
    /// Absent when the run names no target, and when the target's
    /// repository holds no revision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
    /// The sources read from a repository, in declaration order, each at the
    /// commit its revision resolved to.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub repositories: Vec<RepositorySource>,
    /// Whether the candidate stands staged for `emery apply`: `false` once
    /// the run sealed it under [`SpecifyInput::apply`].
    pub staged: bool,
    /// What the run sealed under [`SpecifyInput::apply`]; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied: Option<ApplyOutput>,
}

/// A source read from a repository, at the commit its revision resolved to.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepositorySource {
    /// The source's name.
    pub source: String,
    /// The repository's URL, as the run named it.
    pub repository: String,
    /// The revision the run named.
    pub revision: String,
    /// The commit the revision resolved to, which the source was read at.
    pub commit: String,
}

struct Bound<'a> {
    adapter: &'a AdapterRef,
    digest: Option<&'a Digest>,
    repository: Option<&'a SourceRepository>,
    rank: Option<Rank>,
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
                repository: source.repository.as_ref(),
                rank: source.rank,
                input,
            });
        }

        Ok(bound)
    }

    #[tracing::instrument(skip_all, fields(source = %self.input.name, adapter = %self.adapter))]
    async fn extract<S: Source>(
        &self, provider: &S, loaded: &BTreeMap<AdapterRef, Loaded<SourceMetadata>>,
    ) -> Result<Extract, Error> {
        let source = &self.input.name;
        let adapter = self.adapter;

        let Loaded { id, metadata } = loaded
            .get(adapter)
            .ok_or_else(|| server_error!("adapter `{adapter}` was not loaded"))?;
        let kind = metadata.kind;
        let rank = self.rank.unwrap_or_else(|| Rank::from(kind));
        tracing::info!(%source, adapter = %id, %kind, %rank, "extracting");
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
            rank,
            evidence,
        })
    }
}

// `pub(crate)`: the briefs over it are named by the corpus check at the
// crate root.
#[derive(Debug)]
pub(crate) struct Extract {
    source: String,
    kind: SourceKind,
    rank: Rank,
    evidence: Evidence,
}

// The working copies the repository sources are read in, one per source,
// cut before extraction and removed after it.
struct Checkouts {
    copies: Vec<WorkingCopy>,
    read: Vec<RepositorySource>,
}

impl Checkouts {
    async fn prepare<V: Vcs>(vcs: &V, bound: &[Bound<'_>]) -> Result<Self, Error> {
        let mut ensured = BTreeSet::new();
        let mut copies = Vec::new();
        let mut read = Vec::new();
        for source in bound {
            let Some(repository) = source.repository else { continue };
            let name = &source.input.name;
            let repo = SourceClone::new(&repository.url);
            if ensured.insert(repo.path()) {
                repo.ensure(vcs).await?;
            }
            let commit = repo.resolve(vcs, &repository.revision).await?;
            let at = vcs::source_checkout(name);
            copies.push(WorkingCopy::cut(vcs, &repo.path(), &at, &commit).await?);
            tracing::info!(
                source = %name,
                repository = %repo.url(),
                revision = %repository.revision,
                %commit,
                "checked out"
            );
            read.push(RepositorySource {
                source: name.clone(),
                repository: repo.url().to_owned(),
                revision: repository.revision.clone(),
                commit,
            });
        }
        Ok(Self { copies, read })
    }

    async fn remove<V: Vcs>(self, vcs: &V) -> Result<Vec<RepositorySource>, Error> {
        for copy in self.copies {
            copy.remove(vcs).await?;
        }
        Ok(self.read)
    }
}
