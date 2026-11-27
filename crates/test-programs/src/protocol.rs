//! JSON scenario and response shapes shared by the SDK drivers and host suites.

use std::path::PathBuf;

use emery_adapter::source::{Claim, SourceInput};
use emery_adapter::target::{Report, Slice, Verdict};
use emery_sdk::survey::code::Runs;
use emery_sdk::survey::{Inventory, Survey};
use emery_sdk::target::{Grant, Scaffold};
use emery_sdk::{Lexicon, Seam};
use serde::{Deserialize, Serialize};

use crate::script::ScriptError;

/// The mount point of the lent workspace inside a driver guest.
pub const WORKSPACE: &str = ".";

/// Mount path of the writable workspace inside `sdk_verify`.
pub const VERIFY_WORKSPACE: &str = "/lend/workspace";

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
    Err(ScriptError),
}

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

/// Dispatch payload for `sdk_survey`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "mode", content = "scenario", rename_all = "snake_case")]
pub enum SurveyCommand {
    /// Run `survey::surfaces`.
    Surfaces(SurveyScenario),
    /// Run `survey::seams`.
    Seams(SeamsScenario),
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

/// A `survey::seams` driver scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SeamsScenario {
    /// The adapter id passed to `Context`.
    pub adapter_id: String,
    /// The source input passed to `Context`.
    pub input: InputSpec,
    /// Which hand-built tree the guest constructs over the mounted root.
    pub tree: SeamsTree,
}

/// How a [`SeamsTree::Service`] bootstrap runs.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BootstrapSpec {
    /// [`Runs::Load`].
    Load,
    /// [`Runs::Script`].
    Script {
        /// Console script name.
        name: String,
        /// Entry function name.
        function: String,
    },
    /// [`Runs::Guard`].
    Guard,
}

impl BootstrapSpec {
    /// Converts this spec into an SDK [`Runs`] value.
    #[must_use]
    pub fn to_runs(&self) -> Runs {
        match self {
            Self::Load => Runs::Load,
            Self::Script { name, function } => Runs::Script {
                name: name.clone(),
                function: function.clone(),
            },
            Self::Guard => Runs::Guard,
        }
    }
}

/// Named `Tree<Stub>` fixture built inside `sdk_survey`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeamsTreeKind {
    /// Two-module `TypeScript` tree with `@Processor("nightly")` on `Nightly`.
    ClassDecorator,
    /// Single-module `TypeScript` tree exporting a function, a data class, and a plain class.
    ExportsDeclared,
    /// Six-module `TypeScript` tree with an unresolved import and dynamic loads.
    ByStemLoaded,
    /// Six-module Python package under `./shop` with barrels and no manifest.
    ByDirectoryPackage,
    /// Single 10,000-line `TypeScript` module at `main.ts`.
    OneModule,
    /// Two-module Python handler tree exercising bound call steps.
    Handling,
}

/// Tree fixture configuration for [`SeamsScenario`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "fixture", rename_all = "snake_case")]
pub enum SeamsTree {
    /// Six-module `TypeScript` service with manifest, data files, and tests.
    Service {
        /// Optional bootstrap entry on `src/main.ts`.
        #[serde(default)]
        bootstrap: Option<BootstrapSpec>,
    },
    /// One of the fixed tree shapes.
    Fixed {
        /// Fixed tree shape selector.
        kind: SeamsTreeKind,
    },
}

/// Serialised [`Seam`] returned by `sdk_survey` in `seams` mode.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SeamOutcome {
    /// Brief text leading the seam.
    pub text: String,
    /// Workspace-relative file paths in order.
    pub files: Vec<String>,
    /// Stems subject claim ids lead with.
    pub stems: Vec<String>,
    /// Anchors `requirement` claims overlap.
    pub anchors: Vec<String>,
    /// Extra terms held in the seam's [`Lexicon`].
    pub terms: Vec<String>,
}

/// Successful payload returned by `sdk_survey` in `seams` mode.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SeamsOutcome {
    /// Cut seams in order.
    pub seams: Vec<SeamOutcome>,
    /// Parsed `type` claims in order.
    pub types: Vec<Claim>,
    /// Derived `contract` claims in order.
    pub contracts: Vec<Claim>,
}

impl From<Survey> for SeamsOutcome {
    fn from(survey: Survey) -> Self {
        Self {
            seams: survey
                .seams
                .into_iter()
                .map(|seam| SeamOutcome {
                    text: seam.text,
                    files: seam.files,
                    stems: seam.stems,
                    anchors: seam.anchors,
                    terms: seam.lexicon.terms().map(str::to_owned).collect(),
                })
                .collect(),
            types: survey.types,
            contracts: survey.contracts,
        }
    }
}

impl From<SeamsOutcome> for Survey {
    fn from(outcome: SeamsOutcome) -> Self {
        Self {
            seams: outcome
                .seams
                .into_iter()
                .map(|seam| Seam {
                    text: seam.text,
                    files: seam.files,
                    stems: seam.stems,
                    anchors: seam.anchors,
                    lexicon: Lexicon::with(seam.terms),
                })
                .collect(),
            types: outcome.types,
            contracts: outcome.contracts,
        }
    }
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

impl InputSpec {
    /// Borrows the workspace staging config when this input is a workspace.
    #[must_use]
    pub const fn workspace(&self) -> Option<&WorkspaceInput> {
        match self {
            Self::Workspace(workspace) => Some(workspace),
            Self::Value { .. } => None,
        }
    }

    /// Resolves the guest root directory for this input beneath [`WORKSPACE`].
    #[must_use]
    pub fn root(&self) -> PathBuf {
        match self {
            Self::Workspace(workspace) => workspace.subpath.as_deref().map_or_else(
                || PathBuf::from(WORKSPACE),
                |subpath| PathBuf::from(format!("{WORKSPACE}/{subpath}")),
            ),
            Self::Value { .. } => PathBuf::from(WORKSPACE),
        }
    }

    /// Converts this spec into an adapter [`SourceInput`].
    #[must_use]
    pub fn source_input(&self) -> SourceInput {
        match self {
            Self::Workspace(workspace) => {
                let root = workspace.subpath.as_deref().map_or_else(
                    || WORKSPACE.to_owned(),
                    |subpath| format!("{WORKSPACE}/{subpath}"),
                );
                SourceInput::workspace(&workspace.name, root)
            }
            Self::Value { name, text } => SourceInput::value(name, text),
        }
    }
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

impl SurveyCheck {
    /// Returns the check closure passed to [`emery_sdk::survey::surfaces`].
    pub fn into_fn(self) -> impl FnMut(&Inventory) -> Vec<String> + Send {
        move |answer| match self {
            Self::None => Vec::new(),
            Self::RejectStemB => {
                if answer.surfaces.iter().any(|surface| surface.stem == "b") {
                    vec!["- the adapter refuses stem `b`".to_owned()]
                } else {
                    Vec::new()
                }
            }
            Self::RejectStart => answer
                .surfaces
                .iter()
                .filter(|surface| surface.stem == "start")
                .map(|surface| format!("- surface `{}`: `start` is the caller's", surface.name))
                .collect(),
        }
    }
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
    /// Full [`Seam`] fields for turn-shape and gate scenarios.
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
        /// Package specifiers passed to [`Lexicon::from_packages`].
        #[serde(default)]
        packages: Vec<String>,
    },
}

impl SeamSpec {
    /// Converts this spec into an SDK [`Seam`].
    #[must_use]
    pub fn to_seam(&self) -> Seam {
        match self {
            Self::Whole => Seam::whole(),
            Self::Files { files } => Seam::files(files.clone()),
            Self::Note { text } => Seam::note(text.clone()),
            Self::Custom {
                text,
                files,
                stems,
                anchors,
                packages,
            } => Seam {
                text: text.clone(),
                files: files.clone(),
                stems: stems.clone(),
                anchors: anchors.clone(),
                lexicon: if packages.is_empty() {
                    Lexicon::default()
                } else {
                    Lexicon::from_packages(packages)
                },
            },
        }
    }
}

/// Adapter check wired in the build guest.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildCheck {
    /// Accept every report the gate allows.
    #[default]
    None,
    /// Refuse until `Cargo.toml` exists under the workspace.
    RequireManifest,
    /// Fail the check with a simulated permission error on `Cargo.toml`.
    PermissionDenied,
}

/// A `target::build` driver scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildScenario {
    /// Adapter id passed to `BuildContext`.
    pub adapter_id: String,
    /// Slice to build.
    pub slice: Slice,
    /// Worker grants.
    #[serde(default)]
    pub grants: Vec<Grant>,
    /// Optional scaffold.
    #[serde(default)]
    pub scaffold: Option<Scaffold>,
    /// Per-candidate adapter check.
    #[serde(default)]
    pub check: BuildCheck,
    /// Omits `build.md` from embedded prose.
    #[serde(default)]
    pub omit_build_prompt: bool,
}

impl BuildScenario {
    /// Returns a scenario for `target:probe` building `slice`.
    #[must_use]
    pub fn new(slice: Slice) -> Self {
        Self {
            adapter_id: "target:probe".to_owned(),
            slice,
            grants: Vec::new(),
            scaffold: None,
            check: BuildCheck::None,
            omit_build_prompt: false,
        }
    }

    /// Attaches worker `grants` to the scenario.
    #[must_use]
    pub fn with_grants(mut self, grants: impl Into<Vec<Grant>>) -> Self {
        self.grants = grants.into();
        self
    }

    /// Attaches `scaffold` to the scenario.
    #[must_use]
    pub fn with_scaffold(mut self, scaffold: Scaffold) -> Self {
        self.scaffold = Some(scaffold);
        self
    }

    /// Sets the per-candidate `check` variant.
    #[must_use]
    pub const fn with_check(mut self, check: BuildCheck) -> Self {
        self.check = check;
        self
    }
}

/// Successful payload returned by `sdk_build`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildOutcome {
    /// Accepted report.
    pub report: Report,
    /// Number of candidates the adapter `check` saw.
    pub checks: usize,
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
    pub grants: Vec<Grant>,
    /// Adapter check variant applied to each candidate verdict.
    #[serde(default)]
    pub check: VerifyCheck,
    /// Whether `verify.md` is omitted from embedded prose.
    #[serde(default)]
    pub omit_verify_prompt: bool,
}

impl VerifyScenario {
    /// Attaches worker `grants` to the scenario.
    #[must_use]
    pub fn with_grants(mut self, grants: impl Into<Vec<Grant>>) -> Self {
        self.grants = grants.into();
        self
    }

    /// Sets the per-candidate `check` variant.
    #[must_use]
    pub const fn with_check(mut self, check: VerifyCheck) -> Self {
        self.check = check;
        self
    }
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
pub struct VerifyOutcome {
    /// The accepted verdict.
    pub verdict: Verdict,
    /// Number of candidate verdicts handed to the adapter `check`.
    pub checks: usize,
}
