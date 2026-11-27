//! Driver for [`emery_sdk::extract`]: reads [`ExtractScenario`] JSON from argv
//! and prints a [`Response`] as JSON on stdout.

#![cfg(target_arch = "wasm32")]

use emery_sdk::Context;
use test_programs::helpers::{EXTRACT_PROSE, EXTRACT_PROSE_OMIT_PROMPT, emit, err, ok, scenario};
use test_programs::protocol::{ExtractScenario, SeamSpec};

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: ExtractScenario = scenario();
    let input = scenario.input.source_input();
    let ctx = Context::new(scenario.adapter_id, input);
    let seams: Vec<_> = scenario.seams.iter().map(SeamSpec::to_seam).collect();
    let docs = if scenario.omit_extract_prompt { EXTRACT_PROSE_OMIT_PROMPT } else { EXTRACT_PROSE };
    let response = match emery_sdk::extract(&ctx, docs, &seams).await {
        Ok(evidence) => ok(evidence),
        Err(error) => err(&error),
    };
    emit(response).await;
}
