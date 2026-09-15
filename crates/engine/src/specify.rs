//! Generates a specification revision from a list of sources.
//!
//! Each source's claims are extracted, the requirements are derived under
//! authority precedence, `spec.md` and `design.md` are synthesised, and the
//! pair is committed as one revision. The result reports the revision id and
//! the diff against the revision it displaced, so a caller can see what
//! changed without reading the documents.
//!
//! A [`SourceConfig`] names one source: the adapter to use, the key the
//! specification cites it by, and a workspace to read or an inline value. The
//! list is per-run input, never stored, so one shape serves the command line,
//! a config file, and any other transport; it is checked whole before any
//! adapter loads.
//!
//! Every source extracts at once, and a run waits for all of them, so every
//! source that fails is reported together rather than only the first. A run
//! starts from its sources alone: nothing of an earlier revision is read into
//! the synthesis.

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
/// The source list is checked whole, the adapters it names are loaded, every
/// source is extracted at once, and the specification and design are
/// synthesised over the claims and committed as one revision.
///
/// # Errors
///
/// - [`Error::BadRequest`] for a source list the rules refuse (code
///   `specify-source-required` when it is empty), an adapter that requires a
///   newer Emery (code `unsupported-version`), or a draft the model could not
///   bring within its rounds.
/// - [`Error::NotFound`] for an adapter path that names no file.
/// - [`Error::ServerError`] when any extraction fails or storage refuses the
///   commit.
/// - [`Error::BadGateway`] for a model failure.
///
/// Adapter load failures pass through with their own class.
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

/// The input to [`specify`]: the sources of one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyInput {
    /// The run's sources, in declaration order.
    pub sources: Vec<SourceConfig>,
}

/// One source of a run: its key, its adapter, and what to read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceConfig {
    /// The kebab-case key the specification cites the source by.
    pub key: String,
    /// The adapter that extracts the source.
    pub adapter: AdapterRef,
    /// What the adapter reads: a project-relative directory (`.` is the
    /// project itself) or an inline value.
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

/// What a successful run committed.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyOutput {
    /// The id of the committed revision.
    pub revision: String,
    /// The diff against the revision this run displaced.
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
