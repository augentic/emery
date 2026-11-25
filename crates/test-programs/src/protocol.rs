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

/// How a `SeamsTree::Service` bootstrap runs.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BootstrapSpec {
    /// `Runs::Load`.
    Load,
    /// `Runs::Script`.
    Script {
        /// Console script name.
        name: String,
        /// Entry function name.
        function: String,
    },
    /// `Runs::Guard`.
    Guard,
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

/// Tree fixture configuration for `SeamsScenario`.
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

/// Serialised `Seam` returned by `sdk_survey` in `seams` mode.
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
    /// Extra terms held in the seam's `Lexicon`.
    pub terms: Vec<String>,
}

/// Successful payload returned by `sdk_survey` in `seams` mode.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SeamsOutcome<C> {
    /// Cut seams in order.
    pub seams: Vec<SeamOutcome>,
    /// Parsed `type` claims in order.
    pub types: Vec<C>,
    /// Derived `contract` claims in order.
    pub contracts: Vec<C>,
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

/// Requirement payload in a [`BuildScenario`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RequirementSpec {
    /// The stable requirement identifier.
    pub id: String,
    /// The dotted subject.
    pub subject: String,
    /// Statement paragraphs.
    pub statement: Vec<String>,
    /// Acceptance scenarios.
    pub scenarios: Vec<ScenarioSpec>,
}

/// One scenario in a [`RequirementSpec`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScenarioSpec {
    /// Scenario heading.
    pub name: String,
    /// `GIVEN` lines.
    #[serde(default)]
    pub given: Vec<String>,
    /// `WHEN` line.
    pub when: String,
    /// `THEN` line.
    pub then: String,
    /// `AND` lines.
    #[serde(default)]
    pub and: Vec<String>,
}

/// Owned design type in a [`BuildSliceSpec`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DesignTypeSpec {
    /// Type key.
    pub key: String,
    /// Signature text.
    pub signature: String,
}

/// The slice a build driver runs.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildSliceSpec {
    /// Slice id, `SLICE-001`.
    pub id: String,
    /// Part id.
    pub part: String,
    /// Part name.
    pub name: String,
    /// Slice index.
    pub index: u32,
    /// Slice count in the part.
    pub count: u32,
    /// Base commit id.
    pub base: String,
    /// Requirements to build.
    pub requirements: Vec<RequirementSpec>,
    /// Types the slice owns.
    pub types: Vec<DesignTypeSpec>,
    /// Brief paragraphs.
    pub brief: Vec<String>,
    /// Specification cut.
    pub spec: String,
    /// Design document.
    pub design: String,
    /// Plan entry.
    pub plan: String,
}

/// Marker entry in [`GuardedSpec`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MarkerSpec {
    /// Key an `edit_files` `find` names.
    pub key: String,
    /// Block the scaffold laid.
    pub block: String,
}

/// Guarded file in [`ScaffoldSpec`].
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GuardedSpec {
    /// Root-relative path.
    pub path: String,
    /// Tokens a whole write must keep.
    #[serde(default)]
    pub keeps: Vec<String>,
    /// Fill markers.
    #[serde(default)]
    pub markers: Vec<MarkerSpec>,
}

/// Scaffold the adapter laid before the turn.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScaffoldSpec {
    /// Brief under the scaffold heading.
    #[serde(default)]
    pub brief: String,
    /// Reference passages after the scaffold section.
    #[serde(default)]
    pub passages: String,
    /// Guarded scaffold files.
    #[serde(default)]
    pub files: Vec<GuardedSpec>,
    /// Laid type keys.
    #[serde(default)]
    pub types: Vec<String>,
}

/// Adapter check wired in the build guest.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuildCheck {
    /// Accept every report the gate allows.
    #[default]
    None,
}

/// A `target::build` driver scenario.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildScenario {
    /// Adapter id passed to `BuildContext`.
    pub adapter_id: String,
    /// Slice to build.
    pub slice: BuildSliceSpec,
    /// Worker grants.
    #[serde(default)]
    pub grants: Vec<GrantSpec>,
    /// Optional scaffold.
    #[serde(default)]
    pub scaffold: Option<ScaffoldSpec>,
    /// Files staged under [`WORKSPACE`] before the run.
    #[serde(default)]
    pub files: Vec<FileSpec>,
    /// Per-candidate adapter check.
    #[serde(default)]
    pub check: BuildCheck,
    /// Omits `build.md` from embedded prose.
    #[serde(default)]
    pub omit_build_prompt: bool,
}

/// Successful payload returned by `sdk_build`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BuildOutcome<R> {
    /// Accepted report.
    pub report: R,
    /// Number of candidates the adapter `check` saw.
    pub checks: usize,
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
