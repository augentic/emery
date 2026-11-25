//! Runs SDK driver guests under [`Deployment`] and parses their JSON stdout.

use emery_adapter::source::Evidence;
use emery_sdk::Error;
use emery_sdk::survey::Inventory;
use omnia::ExitStatus;
use omnia_sdk::ErrorKind;
use omnia_test::host::{Backends, Deployment, Scratch, scratch};
use omnia_wasi_model::{ModelClient, WasiModel};
use serde::de::DeserializeOwned;
use test_programs::protocol::{
    ExtractScenario, InputSpec, Response, SurveyScenario, WorkspaceInput,
};

/// An [`extract`] run: parsed stdout and the model handle for assertions.
pub struct ExtractRun {
    /// Parsed driver response.
    pub response: Response<Evidence>,
    /// Standard output as UTF-8.
    pub stdout: String,
}

/// A [`survey::surfaces`] run.
pub struct SurveyRun {
    /// Parsed driver response.
    pub response: Response<Inventory>,
    /// Standard output as UTF-8.
    pub stdout: String,
    /// Captured stdout for turn-shape assertions.
    pub raw_stdout: String,
}

/// Runs the extract driver with `scenario` and `model`.
pub async fn extract<M: ModelClient + Clone + 'static>(
    scenario: &ExtractScenario, model: M,
) -> ExtractRun {
    let (response, stdout) =
        drive::<Evidence, _>(test_programs::SDK_EXTRACT, stage_extract(scenario), model).await;
    ExtractRun { response, stdout }
}

/// Runs the survey driver with `scenario` and `model`.
pub async fn survey<M: ModelClient + Clone + 'static>(
    scenario: &SurveyScenario, model: M,
) -> SurveyRun {
    let (response, stdout) =
        drive::<Inventory, _>(test_programs::SDK_SURVEY, stage_survey(scenario), model).await;
    SurveyRun {
        response,
        raw_stdout: stdout.clone(),
        stdout,
    }
}

pub async fn drive<T: DeserializeOwned, M: ModelClient + Clone + 'static>(
    guest: &'static str, (project, arg): (Scratch, String), model: M,
) -> (Response<T>, String) {
    let captured = Deployment::new()
        .guest("driver", guest)
        .command("driver")
        .mount(project.mount(false))
        .args([arg.as_str()])
        .captured();
    let backends = Backends::defaults().await.model(model);
    let run = captured.run_host::<WasiModel, _>(backends).await.expect("driver runs");
    assert_eq!(run.status, ExitStatus::SUCCESS, "driver stdout:\n{}", run.stdout);
    let response = serde_json::from_str(run.stdout.trim()).unwrap_or_else(|err| {
        panic!("driver JSON on stdout: {err}: {}", run.stdout);
    });
    (response, run.stdout)
}

fn stage_extract(scenario: &ExtractScenario) -> (Scratch, String) {
    let project = scratch();
    stage_workspace(&project, workspace_of(&scenario.input));
    (project, serde_json::to_string(scenario).expect("scenario serialises"))
}

fn stage_survey(scenario: &SurveyScenario) -> (Scratch, String) {
    let project = scratch();
    stage_workspace(&project, workspace_of(&scenario.input));
    (project, serde_json::to_string(scenario).expect("scenario serialises"))
}

fn stage_workspace(project: &Scratch, workspace: Option<&WorkspaceInput>) {
    let Some(workspace) = workspace else { return };
    for file in &workspace.files {
        project.write(&file.path, &file.body);
    }
}

const fn workspace_of(input: &InputSpec) -> Option<&WorkspaceInput> {
    match input {
        InputSpec::Workspace(workspace) => Some(workspace),
        InputSpec::Value { .. } => None,
    }
}

impl ExtractRun {
    /// Returns evidence on success.
    #[track_caller]
    pub fn evidence(self) -> Evidence {
        match self.response {
            Response::Ok { value } => value,
            Response::Err {
                code, description, ..
            } => {
                panic!("expected evidence, got `{code}`: {description}");
            }
        }
    }

    /// Returns the error on refusal.
    #[track_caller]
    pub fn error(self) -> Error {
        error_from_response(self.response)
    }
}

impl SurveyRun {
    /// Returns the inventory on success.
    #[track_caller]
    pub fn inventory(self) -> Inventory {
        match self.response {
            Response::Ok { value } => value,
            Response::Err {
                code, description, ..
            } => {
                panic!("expected inventory, got `{code}`: {description}");
            }
        }
    }

    /// Returns the error on refusal.
    #[track_caller]
    pub fn error(self) -> Error {
        error_from_response(self.response)
    }
}

fn error_from_response<T>(response: Response<T>) -> Error {
    match response {
        Response::Err {
            kind,
            code,
            description,
        } => Error::new(parse_kind(&kind), &code, description),
        Response::Ok { .. } => panic!("expected error"),
    }
}

fn parse_kind(kind: &str) -> ErrorKind {
    match kind {
        "badrequest" | "bad_request" | "exhausted" => ErrorKind::BadRequest,
        "notfound" | "not_found" => ErrorKind::NotFound,
        "badgateway" | "bad_gateway" | "unavailable" => ErrorKind::BadGateway,
        "conflict" => ErrorKind::Conflict,
        _ => ErrorKind::ServerError,
    }
}
