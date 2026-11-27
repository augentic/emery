//! Runs SDK driver guests under `Deployment` and parses their JSON stdout.

use std::collections::BTreeMap;
use std::sync::Mutex;

use emery_adapter::source::Evidence;
use emery_adapter::target::{Report, Verdict};
use emery_sdk::Error;
use emery_sdk::survey::{Inventory, Survey};
use omnia::{ExitStatus, Mount, SourceSpec};
use omnia_test::host::{Backends, Deployment, Scratch, precompiled, scratch};
use omnia_wasi_model::{ModelClient, WasiModel};
use serde::Serialize;
use serde::de::DeserializeOwned;
use test_programs::protocol::{
    BuildOutcome, BuildScenario, ExtractScenario, Response, SeamsOutcome, SeamsScenario,
    SurveyCommand, SurveyScenario, VERIFY_WORKSPACE, VerifyOutcome, VerifyScenario, WorkspaceInput,
};

pub struct DriverRun<T> {
    pub response: Response<T>,
    pub stdout: String,
}

pub async fn extract<M: ModelClient + Clone + 'static>(
    scenario: &ExtractScenario, model: M,
) -> DriverRun<Evidence> {
    let project = scratch();
    extract_in(&project, scenario, model).await
}

pub async fn extract_in<M: ModelClient + Clone + 'static>(
    project: &Scratch, scenario: &ExtractScenario, model: M,
) -> DriverRun<Evidence> {
    stage_workspace(project, scenario.input.workspace());
    drive_mount(test_programs::SDK_EXTRACT, project.mount(false), scenario, model).await
}

pub async fn survey<M: ModelClient + Clone + 'static>(
    scenario: &SurveyScenario, model: M,
) -> DriverRun<Inventory> {
    let project = scratch();
    stage_workspace(&project, scenario.input.workspace());
    let command = SurveyCommand::Surfaces(scenario.clone());
    drive_mount(test_programs::SDK_SURVEY, project.mount(false), &command, model).await
}

pub async fn seams<M: ModelClient + Clone + 'static>(
    scenario: &SeamsScenario, model: M,
) -> DriverRun<SeamsOutcome> {
    let project = scratch();
    seams_in(&project, scenario, model).await
}

pub async fn seams_in<M: ModelClient + Clone + 'static>(
    project: &Scratch, scenario: &SeamsScenario, model: M,
) -> DriverRun<SeamsOutcome> {
    stage_workspace(project, scenario.input.workspace());
    let command = SurveyCommand::Seams(scenario.clone());
    drive_mount(test_programs::SDK_SURVEY, project.mount(false), &command, model).await
}

pub async fn build<M: ModelClient + Clone + 'static>(
    project: &Scratch, scenario: &BuildScenario, model: M,
) -> DriverRun<BuildOutcome> {
    drive_mount(test_programs::SDK_BUILD, project.mount(true), scenario, model).await
}

pub async fn verify<M: ModelClient + Clone + 'static>(
    project: &Scratch, scenario: &VerifyScenario, model: M,
) -> DriverRun<VerifyOutcome> {
    let mount = project.mount_as(VERIFY_WORKSPACE, true);
    drive_mount(test_programs::SDK_VERIFY, mount, scenario, model).await
}

async fn drive_mount<T, A, M>(guest: &str, mount: Mount, payload: &A, model: M) -> DriverRun<T>
where
    T: DeserializeOwned,
    A: Serialize + Sync + ?Sized,
    M: ModelClient + Clone + 'static,
{
    let arg = serde_json::to_string(payload).expect("scenario serialises");
    let captured = Deployment::new()
        .guest("driver", cached_driver(guest))
        .command("driver")
        .mount(mount)
        .args([arg])
        .captured();
    let backends = Backends::defaults().await.model(model);
    let run = captured.run_host::<WasiModel, _>(backends).await.expect("driver runs");
    assert_eq!(run.status, ExitStatus::SUCCESS, "driver stdout:\n{}", run.stdout);
    let response = serde_json::from_str(run.stdout.trim()).unwrap_or_else(|err| {
        panic!("driver JSON on stdout: {err}: {}", run.stdout);
    });
    DriverRun {
        response,
        stdout: run.stdout,
    }
}

fn cached_driver(guest: &str) -> SourceSpec {
    static DRIVERS: Mutex<BTreeMap<String, SourceSpec>> = Mutex::new(BTreeMap::new());
    let mut drivers = DRIVERS.lock().expect("drivers");
    if let Some(source) = drivers.get(guest) {
        return source.clone();
    }
    let compiled = precompiled(guest).expect("precompiled driver");
    drivers.insert(guest.to_owned(), compiled.clone());
    compiled
}

fn stage_workspace(project: &Scratch, workspace: Option<&WorkspaceInput>) {
    let Some(workspace) = workspace else { return };
    if let Some(subpath) = &workspace.subpath {
        std::fs::create_dir_all(project.path().join(subpath)).expect("subpath dir");
    }
    for file in &workspace.files {
        let rel = workspace
            .subpath
            .as_deref()
            .map_or_else(|| file.path.clone(), |subpath| format!("{subpath}/{}", file.path));
        project.write(&rel, &file.body);
    }
}

impl<T> DriverRun<T> {
    #[track_caller]
    pub fn error(self) -> Error {
        match self.response {
            Response::Err(error) => Error::from(error),
            Response::Ok { .. } => panic!("expected error"),
        }
    }

    #[track_caller]
    fn expect_ok(self, expected: &str) -> T {
        match self.response {
            Response::Ok { value } => value,
            Response::Err(err) => {
                panic!("expected {expected}, got `{}`: {}", err.code, err.description);
            }
        }
    }
}

impl DriverRun<Evidence> {
    #[track_caller]
    pub fn evidence(self) -> Evidence {
        self.expect_ok("evidence")
    }
}

impl DriverRun<Inventory> {
    #[track_caller]
    pub fn inventory(self) -> Inventory {
        self.expect_ok("inventory")
    }
}

impl DriverRun<SeamsOutcome> {
    #[track_caller]
    pub fn survey(self) -> Survey {
        Survey::from(self.expect_ok("survey"))
    }
}

impl DriverRun<BuildOutcome> {
    #[track_caller]
    pub fn outcome(self) -> BuildOutcome {
        self.expect_ok("report")
    }

    #[track_caller]
    pub fn report(self) -> Report {
        self.outcome().report
    }
}

impl DriverRun<VerifyOutcome> {
    #[track_caller]
    pub fn outcome(self) -> VerifyOutcome {
        self.expect_ok("verdict")
    }

    #[track_caller]
    pub fn verdict(self) -> Verdict {
        self.outcome().verdict
    }
}
