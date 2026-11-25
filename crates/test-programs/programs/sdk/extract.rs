//! Driver for [`emery_sdk::extract`]: reads [`ExtractScenario`] JSON from argv
//! and prints a [`Response`] as JSON on stdout.

#![cfg(target_arch = "wasm32")]

use emery_sdk::{Context, Evidence};
use test_programs::helpers::{EXTRACT_PROSE, emit, ok, scenario, seam, source_input};
use test_programs::protocol::{ExtractScenario, Response};

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: ExtractScenario = scenario();
    let ctx = Context {
        adapter_id: scenario.adapter_id.as_str(),
        input: &source_input(&scenario.input),
    };
    let seams: Vec<_> = scenario.seams.iter().map(seam).collect();
    let response = match emery_sdk::extract(&ctx, EXTRACT_PROSE, &seams).await {
        Ok(evidence) => ok::<Evidence>(evidence),
        Err(error) => Response::Err {
            kind: format!("{:?}", error.kind()).to_lowercase(),
            code: error.code().to_owned(),
            description: error.description().to_owned(),
        },
    };
    emit(response).await;
}
