//! Helpers the SDK driver guests share.

use std::path::Path;

use emery_adapter::source::SourceInput;
use emery_prose::Doc;
use emery_sdk::Seam;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::protocol::{FileSpec, InputSpec, Response, SeamSpec, SurveyCheck, WORKSPACE};

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
            SourceInput::workspace(workspace.name.as_str(), WORKSPACE)
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
    }
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

/// Embedded prose for [`extract`] drivers.
pub const EXTRACT_PROSE: &[Doc] = &[Doc {
    path: "extract.md",
    body: "SYSTEM",
}];

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

/// Maps an SDK error to the wire [`Response::Err`].
#[must_use]
pub fn err(error: &emery_sdk::Error) -> Response<()> {
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
