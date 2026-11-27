//! Guest runtime helpers and prose tables shared by the SDK drivers.

use emery_prose::Doc;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::protocol::Response;
use crate::script::ScriptError;

/// Parses the JSON scenario from argv past the program name.
///
/// # Panics
///
/// Panics when argv is missing, JSON is invalid, or deserialisation fails.
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

/// Maps an SDK error to [`Response::Err`].
#[must_use]
pub fn err<T>(error: &emery_sdk::Error) -> Response<T> {
    Response::Err(ScriptError::from(error))
}

/// Wraps a successful driver value in [`Response::Ok`].
#[must_use]
pub const fn ok<T>(value: T) -> Response<T> {
    Response::Ok { value }
}
