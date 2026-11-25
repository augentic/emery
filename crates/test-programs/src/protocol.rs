//! JSON scenario and response shapes shared by the drivers and the host suites.

use serde::{Deserialize, Serialize};

/// The mount point of the lent workspace inside a driver guest.
pub const WORKSPACE: &str = ".";

/// What a driver writes to stdout as one JSON value.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response<T> {
    /// A successful run carrying the SDK result.
    Ok {
        /// The successful result.
        value: T,
    },
    /// The SDK error envelope.
    Err {
        /// [`Error::kind`](omnia_sdk::Error::kind) as lowercase `snake_case`.
        kind: String,
        /// [`Error::code`](omnia_sdk::Error::code).
        code: String,
        /// [`Error::description`](omnia_sdk::Error::description).
        description: String,
    },
}

/// Extra fields the host reads after a run (concurrency tests).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExtractMeta {
    /// Peak in-flight completions the host model recorded.
    #[serde(default)]
    pub peak: usize,
    /// Seam-file arrival order the host model recorded.
    #[serde(default)]
    pub arrivals: Vec<String>,
}

/// An [`extract`] driver scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExtractScenario {
    /// The adapter id passed to [`Context`](emery_sdk::Context).
    pub adapter_id: String,
    /// The source input to mine.
    pub input: InputSpec,
    /// The seams in order.
    pub seams: Vec<SeamSpec>,
}

/// An [`survey::surfaces`](emery_sdk::survey::surfaces) driver scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SurveyScenario {
    /// The adapter id passed to [`Context`](emery_sdk::Context).
    pub adapter_id: String,
    /// The source to survey.
    pub input: InputSpec,
    /// Facts put in the survey brief.
    pub facts: FactsSpec,
    /// Which adapter check callback the guest applies.
    pub check: SurveyCheck,
    /// Whether the guest omits `survey.md` from embedded prose.
    #[serde(default)]
    pub omit_survey_prompt: bool,
}

/// The source half of a scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputSpec {
    /// A workspace over [`WORKSPACE`].
    Workspace(WorkspaceInput),
    /// An inline value with no tree to lend.
    Value {
        /// The bound source name.
        name: String,
        /// The inlined text.
        text: String,
    },
}

/// A workspace root named `name` with files written before the guest runs.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct WorkspaceInput {
    /// The bound source name.
    pub name: String,
    /// Root-relative paths written before the run.
    #[serde(default)]
    pub files: Vec<FileSpec>,
}

/// One UTF-8 file staged into the lent tree.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FileSpec {
    /// Root-relative path.
    pub path: String,
    /// File body.
    pub body: String,
}

/// Facts the survey brief carries.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct FactsSpec {
    /// Production module paths listed for the model.
    #[serde(default)]
    pub modules: Vec<String>,
    /// Adapter-authored text before the module list.
    #[serde(default)]
    pub text: String,
    /// Files inlined whole into the brief.
    #[serde(default)]
    pub files: Vec<String>,
}

/// The adapter-specific check wired in the guest for one scenario class.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SurveyCheck {
    /// Accept every answer the tree allows.
    #[default]
    None,
    /// Refuse stem `b` on the first pass (gate test).
    RejectStemB,
    /// Refuse surface `start` as the caller's.
    RejectStart,
}

/// How a driver guest lays one seam.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeamSpec {
    /// [`Seam::whole`].
    Whole,
    /// [`Seam::files`].
    Files {
        /// Paths relative to the workspace root.
        files: Vec<String>,
    },
    /// [`Seam::note`].
    Note {
        /// Brief text leading the seam.
        text: String,
    },
}

/// A build-driver placeholder scenario (A8 fills behaviour).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildScenario {
    /// The operation the stub understands.
    pub op: String,
}

/// A verify-driver placeholder scenario (A6 fills behaviour).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VerifyScenario {
    /// The operation the stub understands.
    pub op: String,
}
