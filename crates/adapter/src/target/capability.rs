//! Defines the [`Target`] capability and the records supplied to an adapter.
//!
//! The engine uses [`Target`] to query a loaded adapter's metadata and to
//! run one [`Task`] in a workspace: an init, a slice's build, or a wave's
//! verify, each answered by its [`Outcome`]. WebAssembly builds dispatch
//! through the imported adapter interface; native builds require an
//! implementation.

use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::future::Future;

use omnia_sdk::Error;

use crate::target::{Report, Verdict};

/// The engine capability for querying and invoking target adapters.
///
/// Adapter components implement the guest interface rather than this trait.
/// The WebAssembly implementation classifies an input refusal as
/// [`Error::BadRequest`] and any other adapter failure as
/// [`Error::BadGateway`].
///
/// # Examples
///
/// A native host can provide adapters directly:
///
/// ```
/// use std::future::{Future, ready};
///
/// use emery_adapter::target::{Outcome, Report, Target, TargetMetadata, Task, Verdict};
/// use omnia_sdk::Error;
///
/// struct Host;
///
/// impl Target for Host {
///     fn run(
///         &self, _id: &str, _workspace: &str, task: &Task,
///     ) -> impl Future<Output = Result<Outcome, Error>> + Send {
///         ready(Ok(match task {
///             Task::Init => Outcome::Initialised,
///             Task::Build(slice) => Outcome::Built(Report {
///                 covered: slice.requirements.clone(),
///                 written: vec!["src/lib.rs".to_string()],
///             }),
///             Task::Verify => Outcome::Verified(Verdict {
///                 passed: true,
///                 failures: Vec::new(),
///             }),
///         }))
///     }
///
///     fn metadata(&self, _id: &str) -> TargetMetadata {
///         TargetMetadata {
///             emery_version: None,
///             merge_rules: Vec::new(),
///         }
///     }
/// }
/// ```
pub trait Target: Send + Sync {
    /// Runs `task` in the tree at `workspace` using the adapter registered as `id`.
    ///
    /// The outcome answers the task, [`Outcome::answers`]; one that does
    /// not is the adapter's defect, which the caller refuses.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] for any other adapter failure.
    #[cfg(not(target_arch = "wasm32"))]
    fn run(
        &self, id: &str, workspace: &str, task: &Task,
    ) -> impl Future<Output = Result<Outcome, Error>> + Send;

    /// Runs `task` in the tree at `workspace` using the adapter registered as `id`.
    ///
    /// The outcome answers the task, [`Outcome::answers`]; one that does
    /// not is the adapter's defect, which the caller refuses.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] for any other adapter failure.
    #[cfg(target_arch = "wasm32")]
    fn run(
        &self, id: &str, workspace: &str, task: &Task,
    ) -> impl Future<Output = Result<Outcome, Error>> + Send {
        crate::target::bindings::import::run(id, workspace, task)
    }

    /// Returns the metadata the adapter registered as `id` declares.
    #[cfg(not(target_arch = "wasm32"))]
    fn metadata(&self, id: &str) -> TargetMetadata;

    /// Returns the metadata the adapter registered as `id` declares.
    #[cfg(target_arch = "wasm32")]
    fn metadata(&self, id: &str) -> TargetMetadata {
        crate::target::bindings::import::metadata(id)
    }
}

/// What the engine asks a target adapter to do in the lent tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Task {
    /// Lay the layout's initial files the tree lacks; a held file stands.
    ///
    /// Put once per tree a build lands in, before any slice, with no model
    /// turn expected: the engine seals what the tree then holds as its
    /// `Initialise` commit.
    Init,
    /// Build one slice of the plan into the tree.
    Build(Slice),
    /// Run the checks over the integrated tree once a wave has merged into
    /// it, repair what they find, and answer a verdict.
    Verify,
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init => f.write_str("init"),
            Self::Build(slice) => write!(f, "slice `{}`", slice.id),
            Self::Verify => f.write_str("verify"),
        }
    }
}

/// The answer to a [`Task`].
///
/// [`Initialised`](Self::Initialised) answers [`Task::Init`],
/// [`Built`](Self::Built) answers [`Task::Build`], and
/// [`Verified`](Self::Verified) answers [`Task::Verify`]; any other pairing
/// is the adapter's defect, which [`answers`](Self::answers) tells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The tree holds the layout's initial files.
    Initialised,
    /// The slice is built, and this is what the build reports beyond the tree.
    Built(Report),
    /// The tree is verified, and this is what the checks found.
    Verified(Verdict),
}

impl Outcome {
    /// Returns whether this outcome is the one `task` is answered by.
    #[must_use]
    pub const fn answers(&self, task: &Task) -> bool {
        matches!(
            (self, task),
            (Self::Initialised, Task::Init)
                | (Self::Built(_), Task::Build(_))
                | (Self::Verified(_), Task::Verify)
        )
    }
}

impl Display for Outcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Initialised => "an initialised tree",
            Self::Built(_) => "a report",
            Self::Verified(_) => "a verdict",
        })
    }
}

/// One slice of the build plan, as the engine renders it for a target.
///
/// The documents are the Markdown `emery show` prints, the specification cut
/// to the slice's requirements. `requirements` carries their ids typed, so a
/// [`Report`]'s `covered` is held to them without reading the Markdown back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slice {
    /// The stable slice identifier, `SLICE-001`.
    pub id: String,
    /// The drafted kebab-case name.
    pub name: String,
    /// The commit the lent tree sits on: the integrated head the slice
    /// builds over.
    pub base: String,
    /// The ids of the requirements the slice builds, `REQ-001`.
    pub requirements: Vec<String>,
    /// The specification, cut to the slice.
    pub spec: String,
    /// The whole design.
    pub design: String,
    /// The slice's own plan entry.
    pub plan: String,
}

/// Metadata declared by a target adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetMetadata {
    /// The minimum compatible Emery version, if the adapter declares one.
    pub emery_version: Option<String>,
    /// The rules every slice is merged into the integrated tree under; the
    /// first rule matching a conflicting path applies.
    pub merge_rules: Vec<MergeRule>,
}

/// One rule for merging a slice into the integrated tree.
///
/// An adapter spells its rules as a `const` slice through [`MergeRule::union`],
/// [`MergeRule::ours`], and [`MergeRule::theirs`], so `paths` borrows a
/// literal there and owns what the contract carries across. A conflict no
/// rule resolves is the engine's to resolve with the model.
///
/// # Examples
///
/// ```
/// use emery_adapter::target::{MergeRule, MergeStrategy};
///
/// const RULES: &[MergeRule] =
///     &[MergeRule::ours("Cargo.lock"), MergeRule::union("src/*/index.ts")];
/// assert_eq!(RULES[1].paths, "src/*/index.ts");
/// assert_eq!(RULES[1].strategy, MergeStrategy::Union);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeRule {
    /// A glob over paths relative to the tree root.
    pub paths: Cow<'static, str>,
    /// How a conflict at a matching path is resolved.
    pub strategy: MergeStrategy,
}

impl MergeRule {
    /// Returns a rule keeping both sides' lines at the paths `paths` matches.
    #[must_use]
    pub const fn union(paths: &'static str) -> Self {
        Self::new(paths, MergeStrategy::Union)
    }

    /// Returns a rule keeping the integrated tree's side at the paths `paths` matches.
    #[must_use]
    pub const fn ours(paths: &'static str) -> Self {
        Self::new(paths, MergeStrategy::Ours)
    }

    /// Returns a rule keeping the slice's side at the paths `paths` matches.
    #[must_use]
    pub const fn theirs(paths: &'static str) -> Self {
        Self::new(paths, MergeStrategy::Theirs)
    }

    const fn new(paths: &'static str, strategy: MergeStrategy) -> Self {
        Self {
            paths: Cow::Borrowed(paths),
            strategy,
        }
    }
}

/// How a merge resolves a conflict at a path a [`MergeRule`] matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Both sides' lines kept, each once: declaration and import lists.
    Union,
    /// The integrated tree's side kept whole, for the build to regenerate:
    /// lockfiles.
    Ours,
    /// The slice's side kept whole.
    Theirs,
}
