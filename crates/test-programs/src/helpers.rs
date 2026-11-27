//! Helpers the SDK driver guests share.

use std::path::Path;

use emery_adapter::source::SourceInput;
use emery_adapter::target::{DesignType, Requirement, Scenario, Slice};
use emery_prose::Doc;
use emery_sdk::target::{Grant, Guarded, Marker, Scaffold};
use emery_sdk::{Lexicon, Seam};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::protocol::{
    BuildSliceSpec, DesignTypeSpec, FileSpec, GrantSpec, GuardedSpec, InputSpec, MarkerSpec,
    RequirementSpec, Response, ScaffoldSpec, ScenarioSpec, SeamSpec, SurveyCheck, WORKSPACE,
};

/// Parses the JSON scenario from argv past the program name.
///
/// # Panics
///
/// Panics when argv is missing, JSON is invalid, or deserialization fails.
#[must_use]
pub fn scenario<T: DeserializeOwned>() -> T {
    let args = wasip3::cli::environment::get_arguments();
    let json = args.get(1).expect("one JSON scenario argument");
    serde_json::from_str(json).unwrap_or_else(|err| panic!("scenario JSON: {err}: {json}"))
}

/// Writes one JSON [`Response`] to stdout and waits for it to drain.
///
/// # Panics
///
/// Panics when serialisation, streaming, or draining stdout fails.
pub async fn emit<T: Serialize>(response: Response<T>) {
    let mut text = serde_json::to_string(&response).expect("response serialises");
    text.push('\n');
    let (mut tx, rx) = wasip3::wit_stream::new::<u8>();
    futures::join!(async { wasip3::cli::stdout::write_via_stream(rx).await.unwrap() }, async {
        let remaining = tx.write_all(text.into_bytes()).await;
        assert!(remaining.is_empty(), "stdout buffer accepted the envelope");
        drop(tx);
    });
}

/// Turns a scenario [`InputSpec`] into a [`SourceInput`].
#[must_use]
pub fn source_input(spec: &InputSpec) -> SourceInput {
    match spec {
        InputSpec::Workspace(workspace) => {
            let root = workspace
                .subpath
                .as_deref()
                .map_or_else(|| WORKSPACE.to_owned(), |subpath| format!("{WORKSPACE}/{subpath}"));
            SourceInput::workspace(workspace.name.as_str(), &root)
        }
        InputSpec::Value { name, text } => SourceInput::value(name, text),
    }
}

/// Writes `files` under [`WORKSPACE`].
///
/// # Panics
///
/// Panics when a parent directory cannot be created or a file cannot be written.
pub fn write_files(files: &[FileSpec]) {
    for file in files {
        let path = Path::new(WORKSPACE).join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(path, &file.body).expect("write");
    }
}

/// Maps [`SeamSpec`] to [`Seam`].
#[must_use]
pub fn seam(spec: &SeamSpec) -> Seam {
    match spec {
        SeamSpec::Whole => Seam::whole(),
        SeamSpec::Files { files } => Seam::files(files.clone()),
        SeamSpec::Note { text } => Seam::note(text.clone()),
        SeamSpec::Custom {
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

/// Converts `specs` into owned [`Grant`]s for a driver run.
#[must_use]
pub fn leak_grants(specs: &[GrantSpec]) -> Vec<Grant> {
    specs
        .iter()
        .map(|spec| Grant {
            egress: spec.egress.clone(),
            read: spec.read.clone(),
            env: spec.env.clone(),
        })
        .collect()
}

/// The survey check callback named by the scenario.
pub fn survey_check(
    check: SurveyCheck,
) -> impl FnMut(&emery_sdk::survey::Inventory) -> Vec<String> + Send {
    move |answer| match check {
        SurveyCheck::None => Vec::new(),
        SurveyCheck::RejectStemB => {
            if answer.surfaces.iter().any(|surface| surface.stem == "b") {
                vec!["- the adapter refuses stem `b`".to_owned()]
            } else {
                Vec::new()
            }
        }
        SurveyCheck::RejectStart => answer
            .surfaces
            .iter()
            .filter(|surface| surface.stem == "start")
            .map(|surface| format!("- surface `{}`: `start` is the caller's", surface.name))
            .collect(),
    }
}

/// Embedded prose for [`extract`](emery_sdk::extract) drivers.
pub const EXTRACT_PROSE: &[Doc] = &[
    Doc {
        path: "extract.md",
        body: "SYSTEM",
    },
    Doc {
        path: "references/greeting.md",
        body: "Greet warmly.",
    },
];

/// Empty prose table for missing-prompt refusal scenarios.
pub const EXTRACT_PROSE_OMIT_PROMPT: &[Doc] = &[];

/// Embedded prose for survey drivers.
pub const SURVEY_PROSE: &[Doc] = &[
    Doc {
        path: "survey.md",
        body: "SURVEY",
    },
    Doc {
        path: "extract.md",
        body: "SYSTEM",
    },
    Doc {
        path: "references/surfaces.md",
        body: "A route is a surface.",
    },
];

/// Prose without `survey.md` for refusal scenarios.
pub const SURVEY_PROSE_OMIT_PROMPT: &[Doc] = &[
    Doc {
        path: "extract.md",
        body: "SYSTEM",
    },
    Doc {
        path: "references/surfaces.md",
        body: "A route is a surface.",
    },
];

/// Embedded prose for [`target::build`](emery_sdk::target::build) drivers.
pub const BUILD_PROSE: &[Doc] = &[
    Doc {
        path: "build.md",
        body: "BUILD",
    },
    Doc {
        path: "references/layout.md",
        body: "One module per requirement.",
    },
];

/// Prose without `build.md` for refusal scenarios.
pub const BUILD_PROSE_OMIT_PROMPT: &[Doc] = &[Doc {
    path: "references/layout.md",
    body: "One module per requirement.",
}];

/// Embedded prose for [`target::verify`](emery_sdk::target::verify) drivers.
pub const VERIFY_PROSE: &[Doc] = &[
    Doc {
        path: "build.md",
        body: "BUILD",
    },
    Doc {
        path: "verify.md",
        body: "VERIFY",
    },
    Doc {
        path: "references/checks.md",
        body: "Run `cargo test`.",
    },
];

/// Prose without `verify.md` for refusal scenarios.
pub const VERIFY_PROSE_OMIT_PROMPT: &[Doc] = &[Doc {
    path: "build.md",
    body: "BUILD",
}];

/// Maps a [`BuildSliceSpec`] to a [`Slice`].
#[must_use]
pub fn build_slice(spec: &BuildSliceSpec) -> Slice {
    Slice {
        id: spec.id.clone(),
        part: spec.part.clone(),
        name: spec.name.clone(),
        index: spec.index,
        count: spec.count,
        base: spec.base.clone(),
        requirements: spec.requirements.iter().map(requirement).collect(),
        types: spec.types.iter().map(design_type).collect(),
        brief: spec.brief.clone(),
        spec: spec.spec.clone(),
        design: spec.design.clone(),
        plan: spec.plan.clone(),
    }
}

fn requirement(spec: &RequirementSpec) -> Requirement {
    Requirement {
        id: spec.id.clone(),
        subject: spec.subject.clone(),
        statement: spec.statement.clone(),
        scenarios: spec.scenarios.iter().map(acceptance_scenario).collect(),
        trigger: None,
    }
}

fn acceptance_scenario(spec: &ScenarioSpec) -> Scenario {
    Scenario {
        name: spec.name.clone(),
        given: spec.given.clone(),
        when: spec.when.clone(),
        then: spec.then.clone(),
        and: spec.and.clone(),
    }
}

fn design_type(spec: &DesignTypeSpec) -> DesignType {
    DesignType {
        key: spec.key.clone(),
        signature: spec.signature.clone(),
    }
}

/// Converts a [`ScaffoldSpec`] into an owned [`Scaffold`] for one driver run.
#[must_use]
pub fn leak_scaffold(spec: &ScaffoldSpec) -> Scaffold {
    Scaffold {
        brief: spec.brief.clone(),
        passages: spec.passages.clone(),
        files: spec.files.iter().map(guarded).collect(),
        types: spec.types.clone(),
    }
}

fn guarded(spec: &GuardedSpec) -> Guarded {
    Guarded {
        path: spec.path.clone(),
        keeps: spec.keeps.clone(),
        markers: spec.markers.iter().map(marker).collect(),
    }
}

fn marker(spec: &MarkerSpec) -> Marker {
    Marker::new(spec.key.clone(), spec.block.clone())
}

/// Maps an SDK error to the wire [`Response::Err`].
#[must_use]
pub fn err<T>(error: &emery_sdk::Error) -> Response<T> {
    Response::Err {
        kind: format!("{:?}", error.kind()).to_lowercase(),
        code: error.code().to_owned(),
        description: error.description().to_owned(),
    }
}

/// Serialises a successful value.
#[must_use]
pub const fn ok<T: Serialize>(value: T) -> Response<T> {
    Response::Ok { value }
}
