//! Script protocol for the scripted source and target adapter guests.
//!
//! The host writes `state.json` under the `script` mount before each engine
//! run; guests read it without modifying it and record dispatches and queue
//! consumption as separate files beneath per-purpose subdirectories.

use std::collections::{BTreeMap, BTreeSet};

use emery_adapter::source::{
    Backing, Claim, ClaimKind, Evidence, SourceInput, SourceKind, SourceMetadata,
};
use emery_adapter::target::{MergeRule, Outcome, Report, Slice, TargetMetadata, Task, Verdict};
use omnia_sdk::{Error, ErrorKind};
use serde::{Deserialize, Serialize};

/// Combined script staged for both adapter guests.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CombinedScript {
    /// Source-side script.
    pub source: SourceScriptState,
    /// Target-side script.
    pub target: TargetScriptState,
}

/// Directory beneath the workspace mount (`.`) holding script state and call logs.
pub const MOUNT: &str = ".emery/script";

/// Relative path of the immutable script state file beneath [`MOUNT`].
pub const STATE: &str = "state.json";
/// Directory holding one JSON file per recorded adapter dispatch beneath [`MOUNT`].
pub const CALLS_DIR: &str = "calls";
/// Directory holding per-slice failure-queue slot markers beneath [`MOUNT`].
pub const FAILURES_DIR: &str = "failures";
/// Directory holding wave verify-queue slot markers beneath [`MOUNT`].
pub const VERIFIES_DIR: &str = "verifies";
/// Directory guests mark so parallel barriers can rendezvous.
pub const ARRIVED: &str = "arrived";

/// Maps a source or slice identifier onto a filesystem-safe path component.
#[must_use]
pub fn sanitise_id(id: &str) -> String {
    id.chars()
        .map(
            |ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') { ch } else { '_' }
            },
        )
        .collect()
}

/// One rendezvous barrier: every `expected` source or slice id must arrive before any proceeds.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RendezvousScript {
    /// Ids that must all be dispatched before any waiting dispatch continues.
    pub expected: BTreeSet<String>,
}

/// One extract outcome for JSON script state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum EvidenceScript {
    /// Successful evidence.
    Ok(Evidence),
    /// Classified refusal or failure.
    Err(ScriptError),
}

/// One build report outcome for JSON script state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ReportScript {
    /// Successful report.
    Ok(Report),
    /// Classified failure.
    Err(ScriptError),
}

/// One verify outcome for JSON script state.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum VerdictScript {
    /// Passing or failing verdict.
    Ok(Verdict),
    /// Classified failure.
    Err(ScriptError),
}

/// Source adapter script the host stages before a run.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SourceScriptState {
    /// Extract outcomes keyed by source name (`SourceInput::name`).
    #[serde(default)]
    pub evidence: BTreeMap<String, EvidenceScript>,
    /// Source names whose `extract` never completes.
    #[serde(default)]
    pub held: BTreeSet<String>,
    /// Minimum `emery` versions keyed by adapter guest id.
    #[serde(default)]
    pub versions: BTreeMap<String, String>,
    /// Kinds of source by adapter guest id.
    #[serde(default)]
    pub kinds: BTreeMap<String, SourceKind>,
    /// Optional parallel-dispatch barrier.
    #[serde(default)]
    pub rendezvous: Option<RendezvousScript>,
}

/// Target adapter script the host stages before a run.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TargetScriptState {
    /// Build outcomes keyed by slice id.
    #[serde(default)]
    pub reports: BTreeMap<String, ReportScript>,
    /// Failures queued per slice id, each consumed before that slice's report.
    #[serde(default)]
    pub failures: BTreeMap<String, Vec<ScriptError>>,
    /// Verify outcomes, consumed in wave order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verdicts: Vec<VerdictScript>,
    /// Minimum `emery` versions keyed by adapter guest id.
    #[serde(default)]
    pub versions: BTreeMap<String, String>,
    /// Merge rules every adapter declares.
    #[serde(default)]
    pub rules: Vec<MergeRule>,
    /// Default slice budget.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<u32>,
    /// Optional parallel-dispatch barrier over slice ids.
    #[serde(default)]
    pub rendezvous: Option<RendezvousScript>,
}

/// Serialisable mirror of [`ErrorKind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorKindWire {
    /// [`ErrorKind::BadRequest`].
    BadRequest,
    /// [`ErrorKind::NotFound`].
    NotFound,
    /// [`ErrorKind::Conflict`].
    Conflict,
    /// [`ErrorKind::ServerError`].
    ServerError,
    /// [`ErrorKind::BadGateway`].
    BadGateway,
}

impl From<ErrorKind> for ErrorKindWire {
    fn from(kind: ErrorKind) -> Self {
        match kind {
            ErrorKind::BadRequest => Self::BadRequest,
            ErrorKind::NotFound => Self::NotFound,
            ErrorKind::Conflict => Self::Conflict,
            ErrorKind::BadGateway => Self::BadGateway,
            _ => Self::ServerError,
        }
    }
}

impl From<ErrorKindWire> for ErrorKind {
    fn from(kind: ErrorKindWire) -> Self {
        match kind {
            ErrorKindWire::BadRequest => Self::BadRequest,
            ErrorKindWire::NotFound => Self::NotFound,
            ErrorKindWire::Conflict => Self::Conflict,
            ErrorKindWire::ServerError => Self::ServerError,
            ErrorKindWire::BadGateway => Self::BadGateway,
        }
    }
}

/// Classified error carried across JSON test boundaries.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ScriptError {
    /// Omnia error class.
    pub kind: ErrorKindWire,
    /// Recovery code.
    pub code: String,
    /// Human-readable description.
    pub description: String,
}

impl ScriptError {
    /// Builds a [`ScriptError`] from a host [`Error`].
    #[must_use]
    pub fn from_adapter(error: &Error) -> Self {
        Self::from(error)
    }
}

impl From<&Error> for ScriptError {
    fn from(error: &Error) -> Self {
        Self {
            kind: error.kind().into(),
            code: error.code().to_owned(),
            description: error.description().to_owned(),
        }
    }
}

impl From<ScriptError> for Error {
    fn from(script: ScriptError) -> Self {
        Self::new(script.kind.into(), script.code, script.description)
    }
}

/// One recorded adapter dispatch.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "axis", rename_all = "lowercase")]
pub enum ScriptCall {
    /// A source `metadata` dispatch.
    SourceMetadata {
        /// Adapter guest id.
        id: String,
    },
    /// A source `extract` dispatch.
    SourceExtract {
        /// Adapter guest id.
        id: String,
        /// The input the engine passed.
        input: SourceInput,
    },
    /// A target `metadata` dispatch.
    TargetMetadata {
        /// Adapter guest id.
        id: String,
    },
    /// A target `run` dispatch for initialisation.
    TargetInit {
        /// Adapter guest id.
        id: String,
        /// Workspace root path.
        workspace: String,
    },
    /// A target `run` dispatch for a slice build.
    TargetBuild {
        /// Adapter guest id.
        id: String,
        /// Workspace root path.
        workspace: String,
        /// The slice as the engine lowered it for the target.
        slice: Box<Slice>,
    },
    /// A target `run` dispatch for verification.
    TargetVerify {
        /// Adapter guest id.
        id: String,
        /// Workspace root path.
        workspace: String,
    },
}

impl SourceScriptState {
    /// Metadata for adapter guest `id`.
    #[must_use]
    pub fn metadata(&self, id: &str) -> SourceMetadata {
        SourceMetadata {
            emery_version: self.versions.get(id).cloned(),
            kind: self.kinds.get(id).copied().unwrap_or(SourceKind::Documentation),
        }
    }

    /// Resolves the scripted extraction outcome for source `name`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] converted from [`ScriptError`] when the script refuses `name`.
    pub fn extract(&self, name: &str) -> Result<Evidence, Error> {
        match self.evidence.get(name) {
            Some(EvidenceScript::Ok(evidence)) => Ok(evidence.clone()),
            Some(EvidenceScript::Err(err)) => Err(err.clone().into()),
            None => Ok(Self::default_evidence()),
        }
    }

    /// Default evidence when nothing is scripted for a source name.
    #[must_use]
    pub fn default_evidence() -> Evidence {
        Evidence {
            claims: vec![
                Claim::new(ClaimKind::Requirement)
                    .with_id("greeting.behaviour")
                    .with_backing(Backing::Payload(
                        "GET /greeting returns the static string 'hello'.".to_owned(),
                    ))
                    .with_extra("statement", "GET /greeting returns the static string 'hello'."),
            ],
        }
    }
}

impl TargetScriptState {
    /// Metadata for adapter guest `id`.
    #[must_use]
    pub fn metadata(&self, id: &str) -> TargetMetadata {
        TargetMetadata {
            emery_version: self.versions.get(id).cloned(),
            merge_rules: self.rules.clone(),
            budget: self.budget,
        }
    }

    /// Resolves the `attempt`-th dispatch of `task` against this immutable script.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] converted from [`ScriptError`] when the script refuses `task`.
    pub fn run_at(&self, task: &Task, attempt: usize) -> Result<Outcome, Error> {
        match task {
            Task::Init => Ok(Outcome::Initialised),
            Task::Build(slice) => {
                if let Some(err) = self.failures.get(&slice.id).and_then(|queue| queue.get(attempt))
                {
                    return Err(err.clone().into());
                }
                let outcome = self.reports.get(&slice.id).cloned().unwrap_or_else(|| {
                    ReportScript::Ok(Report {
                        covered: slice.requirements.iter().map(|r| r.id.clone()).collect(),
                        written: vec![format!("src/{}.rs", slice.name)],
                    })
                });
                match outcome {
                    ReportScript::Ok(report) => Ok(Outcome::Built(report)),
                    ReportScript::Err(err) => Err(err.into()),
                }
            }
            Task::Verify => {
                let outcome = self
                    .verdicts
                    .get(attempt)
                    .cloned()
                    .unwrap_or_else(|| VerdictScript::Ok(Verdict::passed()));
                match outcome {
                    VerdictScript::Ok(verdict) => Ok(Outcome::Verified(verdict)),
                    VerdictScript::Err(err) => Err(err.into()),
                }
            }
        }
    }
}
