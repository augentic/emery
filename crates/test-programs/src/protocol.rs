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
        /// `Error::kind` as lowercase `snake_case`.
        kind: String,
        /// `Error::code`.
        code: String,
        /// `Error::description`.
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

/// Mount path of the writable workspace inside `sdk_verify`.
pub const VERIFY_WORKSPACE: &str = "/lend/workspace";

/// An `extract` driver scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExtractScenario {
    /// The adapter id passed to `Context`.
    pub adapter_id: String,
    /// The source input to mine.
    pub input: InputSpec,
    /// The seams in order.
    pub seams: Vec<SeamSpec>,
    /// Whether the guest omits `extract.md` from embedded prose.
    #[serde(default)]
    pub omit_extract_prompt: bool,
}

/// A `survey::surfaces` driver scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SurveyScenario {
    /// The adapter id passed to `Context`.
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
    /// Optional subpath beneath [`WORKSPACE`] lent as the workspace root.
    #[serde(default)]
    pub subpath: Option<String>,
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
    /// `Seam::whole`.
    Whole,
    /// `Seam::files`.
    Files {
        /// Paths relative to the workspace root.
        files: Vec<String>,
    },
    /// `Seam::note`.
    Note {
        /// Brief text leading the seam.
        text: String,
    },
    /// Full `Seam` fields for turn-shape and gate scenarios.
    Custom {
        /// Brief text leading the seam.
        #[serde(default)]
        text: String,
        /// Paths relative to the workspace root.
        #[serde(default)]
        files: Vec<String>,
        /// Stems subject claim ids must lead with.
        #[serde(default)]
        stems: Vec<String>,
        /// Anchors `requirement` claims must overlap.
        #[serde(default)]
        anchors: Vec<String>,
        /// Package specifiers passed to `Lexicon::from_packages`.
        #[serde(default)]
        packages: Vec<String>,
    },
}

/// A build-driver placeholder scenario (A8 fills behaviour).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildScenario {
    /// The operation the stub understands.
    pub op: String,
}

/// One `Grant` entry passed to a target driver.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct GrantSpec {
    /// Host names granted for egress.
    #[serde(default)]
    pub egress: Vec<String>,
    /// `~/` home-relative read roots.
    #[serde(default)]
    pub read: Vec<String>,
    /// Environment `(name, value)` pairs.
    #[serde(default)]
    pub env: Vec<(String, String)>,
}

/// Which adapter check callback the verify guest applies.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VerifyCheck {
    /// Accept every verdict the gate allows.
    #[default]
    None,
    /// Require `Cargo.toml` to contain `crates/*` when `passed` is true.
    MembersList,
    /// Fail immediately with `bad_request` naming `Cargo.toml`.
    DirectoryAtManifest,
}

/// A `target::verify` driver scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VerifyScenario {
    /// The adapter id passed to `VerifyContext`.
    pub adapter_id: String,
    /// Worker grants attached to the context.
    #[serde(default)]
    pub grants: Vec<GrantSpec>,
    /// Adapter check variant applied to each candidate verdict.
    #[serde(default)]
    pub check: VerifyCheck,
    /// Whether `verify.md` is omitted from embedded prose.
    #[serde(default)]
    pub omit_verify_prompt: bool,
}

impl Default for VerifyScenario {
    fn default() -> Self {
        Self {
            adapter_id: "target:probe".to_owned(),
            grants: Vec::new(),
            check: VerifyCheck::None,
            omit_verify_prompt: false,
        }
    }
}

/// Successful payload returned by `sdk_verify`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VerifyOutcome<V> {
    /// The accepted verdict.
    pub verdict: V,
    /// Number of candidate verdicts handed to the adapter `check`.
    pub checks: usize,
}
