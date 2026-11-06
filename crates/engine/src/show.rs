//! Reads documents from the current specification revision, the staged candidate, or the staged amendment.
//!
//! [`show`] returns the selected document as Markdown and structured JSON,
//! together with the identifier of what it was read from. The current
//! revision is read from the target's repository, the staged candidate and
//! the staged amendment from the engine's own storage.

use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;

use anyhow::Context as _;
use emery_adapter::target::Target;
use omnia_sdk::api::Context;
use omnia_sdk::plugins::Digest;
use omnia_sdk::{BlobStore, Error, Plugins, StateStore, Vcs, bad_request};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, EnumString, VariantArray};

use crate::adapter::AdapterRef;
pub use crate::build::{BudgetSource, ScheduledSlice};
use crate::revision::{self, Document, ID, REQUIREMENTS, Revision, TYPES};
pub use crate::revision::{
    Amendment, AmendmentBase, BriefState, Change, ChangeClass, DesignDiff, PartChange, PartClass,
    SliceId, Waves,
};
use crate::vcs::TargetRef;
use crate::{Files, store};

const SLICE_HEADING: &str = "## Slice:";
const PART_KEY: &str = "Part:";
const WAVE_KEY: &str = "Wave:";
const OVER_BUDGET_KEY: &str = "Over budget:";

/// Returns one document from the current revision, the staged candidate, or the staged amendment.
///
/// The current revision is read from the repository [`ShowInput::target`]
/// names, as [`apply`](crate::apply::apply) sealed it; the staged candidate
/// needs no target, and neither does the [`Amendment`](Artifact::Amendment),
/// which stands staged alone until it is sealed. The
/// [`Schedule`](Artifact::Schedule) cuts the current revision's plan into
/// slices under [`ShowInput::budget`], the target adapter's metadata, or
/// [`DEFAULT_BUDGET`](crate::build::DEFAULT_BUDGET), with no model turn.
///
/// # Errors
///
/// - Returns [`Error::NotFound`] with code `spec-not-generated` when the
///   repository holds no revision, naming a staged candidate where one
///   stands, and with code `nothing-staged` when [`ShowInput::staged`] is
///   set and no candidate is staged, or the amendment is asked for and none
///   is staged.
/// - Returns [`Error::BadRequest`] with code `build-target-required` when
///   no target names the repository and the candidate is not asked for,
///   when [`ShowInput::staged`] is set for the
///   [`Schedule`](Artifact::Schedule), with code `amendment-pending` when a
///   revision document is asked for from the staging slot and an amendment
///   stands there, with code `spec-outdated` when the document read uses a
///   different grammar, and the classes [`apply`](crate::apply::apply)
///   reads the repository under otherwise.
/// - Returns [`Error::ServerError`] when storage, validation, or serialisation
///   fails.
pub async fn show<P: Target + Plugins + StateStore + BlobStore + Vcs + Files>(
    input: ShowInput, context: Context<P>,
) -> Result<ShowOutput, Error> {
    let store = context.provider();
    if input.artifact == Artifact::Amendment {
        let staged =
            store::staged_amendment(store).await?.ok_or_else(store::no_amendment_staged)?;
        tracing::debug!(amendment = %staged.id, "amendment read");
        return ShowOutput::new(&staged.amendment, staged.id);
    }
    if input.artifact == Artifact::Schedule && input.staged {
        return Err(bad_request!(
            "`emery show schedule` cuts the sealed plan `emery build` reads, not the staged \
             candidate; inspect the staged parts with `emery show plan --staged`"
        ));
    }

    let (id, revision) = input.revision(store).await?;
    match input.artifact {
        Artifact::Spec => ShowOutput::new(&revision.spec, id),
        Artifact::Design => ShowOutput::new(&revision.design, id),
        Artifact::Plan => ShowOutput::new(&revision.plan, id),
        Artifact::Schedule => {
            let (budget, source) = BudgetSource::resolve_with(
                store,
                input.budget,
                input.budget_source,
                input.adapter.as_ref(),
                input.digest.as_ref(),
            )
            .await?;
            let schedule = Schedule::new(&revision, budget, source);
            ShowOutput::new(&schedule, id)
        }
        Artifact::Amendment => unreachable!("handled above"),
    }
}

/// Selects the document returned by [`show`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// The artifact projected as Markdown and structured JSON.
    pub artifact: Artifact,
    /// Whether to read the staged candidate in place of the current revision.
    ///
    /// Not read for the [`Amendment`](Artifact::Amendment), which is staged
    /// or nothing; refused for the [`Schedule`](Artifact::Schedule), which
    /// cuts the current revision's plan as [`build`](crate::build::build)
    /// reads it.
    #[serde(default)]
    pub staged: bool,
    /// The target whose repository the current revision is read from.
    ///
    /// Required unless [`staged`](Self::staged) is set or the artifact is
    /// the [`Amendment`](Artifact::Amendment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetRef>,
    /// The maximum number of requirements per build slice for the [`Schedule`](Artifact::Schedule).
    ///
    /// `None` falls back to the target adapter's metadata default, or
    /// [`DEFAULT_BUDGET`](crate::build::DEFAULT_BUDGET).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<NonZeroUsize>,
    /// Where [`budget`](Self::budget) was set (`--budget` or `[target] budget`).
    ///
    /// Defaults to [`BudgetSource::Flag`] when [`budget`](Self::budget) is set
    /// and `budget_source` is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_source: Option<BudgetSource>,
    /// The target adapter whose metadata supplies the default budget when [`budget`](Self::budget) is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<AdapterRef>,
    /// The `sha256:` digest the target adapter's component must resolve to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
}

impl ShowInput {
    /// Selects `artifact` from the current revision of `target`.
    #[must_use]
    pub const fn new(artifact: Artifact, target: Option<TargetRef>) -> Self {
        Self {
            artifact,
            staged: false,
            target,
            budget: None,
            budget_source: None,
            adapter: None,
            digest: None,
        }
    }

    // The revision a document of `self.artifact` is read from, with its id:
    // the staged candidate, or the current revision of `self.target`'s
    // repository.
    async fn revision<P: StateStore + BlobStore + Vcs + Files>(
        &self, store: &P,
    ) -> Result<(String, Revision), Error> {
        let (id, revision) = if self.staged {
            let candidate = store::staged(store).await?.ok_or_else(store::nothing_staged)?;
            (candidate.id, candidate.revision)
        } else {
            let target =
                self.target.as_ref().ok_or_else(|| store::target_required("`emery show`"))?;
            let repo = target.repo()?;
            let standing = store::current(store, &repo).await?;
            let Some(current) = standing.current else {
                return Err(store::not_generated(store, &standing).await);
            };
            (current.id, current.revision?)
        };
        tracing::debug!(
            revision = %id,
            staged = self.staged,
            artifact = self.artifact.as_ref(),
            "revision read"
        );
        Ok((id, revision))
    }
}

/// A document available from a specification revision, or the staged amendment.
///
/// String parsing, [`AsRef::as_ref`], and Serde use the lowercase names `spec`,
/// `design`, `plan`, `schedule`, and `amendment`.
///
/// # Examples
///
/// ```
/// use emery_engine::show::Artifact;
///
/// let artifact: Artifact = "spec".parse()?;
/// assert_eq!(artifact, Artifact::Spec);
/// assert_eq!(Artifact::Design.as_ref(), "design");
/// assert_eq!(Artifact::Plan.as_ref(), "plan");
/// assert_eq!(Artifact::Schedule.as_ref(), "schedule");
/// assert_eq!(Artifact::Amendment.as_ref(), "amendment");
/// # Ok::<(), strum::ParseError>(())
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AsRefStr, EnumString, VariantArray,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Artifact {
    /// The behavioural specification, rendered as `spec.md`.
    Spec,
    /// The rebuild design, rendered as `design.md`.
    Design,
    /// The build plan, rendered as `plan.md`.
    Plan,
    /// The build schedule, rendered as `schedule.md`.
    Schedule,
    /// The staged [`Amendment`], rendered as `amendment.md`.
    ///
    /// Read from the staging slot alone: an amendment is never current.
    Amendment,
}

/// The build schedule projected by [`show`] for [`Artifact::Schedule`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Schedule {
    /// The requirement budget per slice the schedule was cut under.
    pub budget: usize,
    /// Where [`budget`](Self::budget) came from.
    pub source: BudgetSource,
    /// Every slice of the schedule, in schedule order.
    pub slices: Vec<ScheduledSlice>,
    /// The schedule's slices grouped into the sets ready to build at once.
    pub waves: Waves<SliceId>,
    /// The slices whose requirements exceed [`budget`](Self::budget), in schedule order.
    pub over_budget: Vec<SliceId>,
}

impl Schedule {
    fn new(revision: &Revision, budget: NonZeroUsize, source: BudgetSource) -> Self {
        let schedule = crate::build::Schedule::for_revision(revision, budget);
        let waves = schedule.waves();
        let over_budget = schedule.over_budget().map(|slice| slice.id).collect();
        Self {
            budget: budget.get(),
            source,
            slices: schedule.into_slices(),
            waves,
            over_budget,
        }
    }
}

impl Document for Schedule {
    const NAME: &'static str = "schedule";
}

impl Display for Schedule {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "# Schedule\n\nBudget: {} ({})", self.budget, self.source.as_ref())?;
        if !self.waves.is_empty() {
            write!(f, "\nWaves: {}", self.waves)?;
        }
        if !self.over_budget.is_empty() {
            f.write_str("\n")?;
            revision::write_list(f, OVER_BUDGET_KEY, &self.over_budget)?;
        }
        for slice in &self.slices {
            write!(f, "\n\n{SLICE_HEADING} {}", slice.name)?;
            if slice.count > 1 {
                write!(f, " ({}/{})", slice.index, slice.count)?;
            }
            let wave = self.waves.wave_of(&slice.id).unwrap_or(0);
            write!(
                f,
                "\n\n{ID} {}\n{PART_KEY} {} ({}/{})\n{WAVE_KEY} {wave}\n",
                slice.id, slice.part, slice.index, slice.count
            )?;
            revision::write_list(f, REQUIREMENTS, &slice.requirements)?;
            if !slice.types.is_empty() {
                f.write_str("\n")?;
                revision::write_list(f, TYPES, &slice.types)?;
            }
            if slice.over_budget {
                write!(f, "\n{OVER_BUDGET_KEY} true")?;
            }
        }
        f.write_str("\n")
    }
}

/// A rendered document and its structured representation.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowOutput {
    /// The identifier of what was read: the current revision, the staged
    /// candidate, or the staged amendment.
    pub revision: String,
    /// The Markdown projection, including revision front matter.
    pub body: String,
    /// The typed document serialised as JSON.
    pub document: Value,
}

impl ShowOutput {
    // The store wrote the document under this derive, so a failure here is
    // the engine's own defect.
    fn new<D: Document>(document: &D, revision: String) -> Result<Self, Error> {
        let value = serde_json::to_value(document)
            .with_context(|| format!("`{}` does not serialise", D::NAME))?;
        Ok(Self {
            body: document.to_markdown(&revision),
            revision,
            document: value,
        })
    }
}
