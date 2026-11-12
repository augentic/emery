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
use crate::build::Basis;
pub use crate::build::{BudgetSource, Change as ScheduleChange, ScheduledSlice};
use crate::revision::{self, Document, ID, PartId, REQUIREMENTS, Revision, TYPES};
pub use crate::revision::{
    Amendment, AmendmentBase, BriefState, Change, ChangeClass, DesignDiff, PartChange, PartClass,
    SliceId, Waves,
};
use crate::vcs::{BuildRepo, MergedIds, TargetRef};
use crate::{Files, store};

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
    context: Context<P>, input: ShowInput,
) -> Result<ShowOutput, Error> {
    let store = context.provider();
    if input.artifact == Artifact::Amendment {
        let staged =
            store::staged_amendment(store).await?.ok_or_else(store::no_amendment_staged)?;
        tracing::debug!(amendment = %staged.id, "amendment read");
        return ShowOutput::new(&staged.amendment, staged.id);
    }
    if input.staged {
        if input.artifact == Artifact::Schedule {
            return Err(bad_request!(
                "`emery show schedule` cuts the sealed plan `emery build` reads, not the staged \
                 candidate; inspect the staged parts with `emery show plan --staged`"
            ));
        }
        let candidate = store::staged(store).await?.ok_or_else(store::nothing_staged)?;
        tracing::debug!(
            revision = %candidate.id,
            staged = true,
            artifact = input.artifact.as_ref(),
            "revision read"
        );
        return match input.artifact {
            Artifact::Spec => ShowOutput::new(&candidate.revision.spec, candidate.id),
            Artifact::Design => ShowOutput::new(&candidate.revision.design, candidate.id),
            Artifact::Plan => ShowOutput::new(&candidate.revision.plan, candidate.id),
            Artifact::Schedule | Artifact::Amendment => unreachable!("handled above"),
        };
    }

    let current = input.revision(store).await?;
    match input.artifact {
        Artifact::Spec => ShowOutput::new(&current.revision.spec, current.id),
        Artifact::Design => ShowOutput::new(&current.revision.design, current.id),
        Artifact::Plan => ShowOutput::new(&current.revision.plan, current.id),
        Artifact::Schedule => {
            let schedule = Schedule::resolve(store, &input, &current).await?;
            ShowOutput::new(&schedule, current.id)
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
    /// Whether to cut the [`Schedule`](Artifact::Schedule) for every part of
    /// the plan from the base, ignoring any built ancestor in the lineage
    /// (`--whole`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub whole: bool,
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
            whole: false,
        }
    }

    // The current revision of `self.target`'s repository, with its id,
    // repository, and base commit.
    async fn revision<P: StateStore + BlobStore + Vcs + Files>(
        &self, store: &P,
    ) -> Result<CurrentRevision, Error> {
        let target = self.target.as_ref().ok_or_else(|| store::target_required("`emery show`"))?;
        let repo = target.repo()?;
        let standing = store::current(store, &repo).await?;
        let Some(current) = standing.current else {
            return Err(store::not_generated(store, &standing).await);
        };
        let base = standing.head.unwrap_or(current.commit);
        let id = current.id;
        let revision = current.revision?;
        tracing::debug!(
            revision = %id,
            staged = false,
            artifact = self.artifact.as_ref(),
            "revision read"
        );
        Ok(CurrentRevision {
            id,
            revision,
            repo,
            base,
        })
    }
}

struct CurrentRevision {
    id: String,
    revision: Revision,
    repo: BuildRepo,
    base: String,
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

const SLICE_HEADING: &str = "## Slice:";
const PART_KEY: &str = "Part:";
const WAVE_KEY: &str = "Wave:";
const OVER_BUDGET_KEY: &str = "Over budget:";
const CHANGE_KEY: &str = "Change:";
const DEPARTED_KEY: &str = "Departed:";

/// The build schedule projected by [`show`] for [`Artifact::Schedule`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Schedule {
    /// The requirement budget per slice the schedule was cut under.
    pub budget: usize,
    /// Where [`budget`](Self::budget) came from.
    pub source: BudgetSource,
    /// What a change build would rebuild, carry, and retire against the built
    /// ancestor; `None` when the schedule is for a whole build.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<ScheduleChange>,
    /// Whether [`change`](Self::change) was resolved with the target adapter's
    /// `scope` task (`true`) or from the engine's minimum scope alone because
    /// no target adapter was configured (`false`); `None` for a whole build.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scoped: Option<bool>,
    /// Every slice of the schedule, in schedule order.
    pub slices: Vec<ScheduledSlice>,
    /// The schedule's slices grouped into the sets ready to build at once.
    pub waves: Waves<SliceId>,
    /// The slices whose requirements exceed [`budget`](Self::budget), in schedule order.
    pub over_budget: Vec<SliceId>,
}

impl Schedule {
    async fn resolve<P: Target + Plugins + Vcs + Files>(
        store: &P, input: &ShowInput, current: &CurrentRevision,
    ) -> Result<Self, Error> {
        let load = !input.whole && current.revision.spec.parent.is_some();
        let (budget, source, adapter) = BudgetSource::resolve_with(
            store,
            input.budget,
            input.budget_source,
            input.adapter.as_ref(),
            input.digest.as_ref(),
            load,
        )
        .await?;
        let basis = Basis::settle(
            store,
            &current.repo,
            adapter.as_deref(),
            &current.base,
            &current.revision,
            budget,
            input.whole,
        )
        .await?;
        Ok(Self::new(&current.revision, budget, source, &basis, adapter.is_some()))
    }

    fn new(
        revision: &Revision, budget: NonZeroUsize, source: BudgetSource, basis: &Basis,
        has_adapter: bool,
    ) -> Self {
        let carried = basis.carried();
        let schedule = crate::build::Schedule::new(
            revision,
            budget,
            &MergedIds::default(),
            &[],
            &carried,
            basis.removed_parts(),
        );
        let waves = schedule.waves();
        let over_budget = schedule.over_budget().map(|slice| slice.id).collect();
        let (change, _) = basis.outcome(schedule.retiring().collect());
        let scoped = change.is_some().then_some(has_adapter);
        Self {
            budget: budget.get(),
            source,
            change,
            scoped,
            slices: schedule.into_slices(),
            waves,
            over_budget,
        }
    }

    fn is_carried(&self, part: PartId) -> bool {
        self.change.as_ref().is_some_and(|change| change.carried.contains(&part))
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
        if let Some(change) = &self.change {
            write!(f, "\n{CHANGE_KEY} {} at {}", change.from, change.commit)?;
            if self.scoped == Some(false) {
                f.write_str(" (minimum scope; no target adapter)")?;
            }
        }
        for slice in &self.slices {
            write!(f, "\n\n{SLICE_HEADING} {}", slice.name)?;
            if slice.count > 1 {
                write!(f, " ({}/{})", slice.index, slice.count)?;
            }
            if slice.is_retiring() {
                f.write_str(" [retiring]")?;
            } else if self.is_carried(slice.part) {
                f.write_str(" [carried]")?;
            }
            let wave = self.waves.wave_of(&slice.id).unwrap_or(0);
            write!(
                f,
                "\n\n{ID} {}\n{PART_KEY} {} ({}/{})\n{WAVE_KEY} {wave}\n",
                slice.id, slice.part, slice.index, slice.count
            )?;
            revision::write_list(f, REQUIREMENTS, &slice.requirements)?;
            if !slice.departed.is_empty() {
                f.write_str("\n")?;
                revision::write_list(f, DEPARTED_KEY, &slice.departed)?;
            }
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
