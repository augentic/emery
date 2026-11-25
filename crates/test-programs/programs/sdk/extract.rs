//! Driver for [`emery_sdk::extract`]: reads [`ExtractScenario`] JSON from argv
//! and prints a [`Response`] as JSON on stdout.

#![cfg(target_arch = "wasm32")]

use emery_sdk::{Context, Evidence};
use test_programs::helpers::{
    EXTRACT_PROSE, EXTRACT_PROSE_OMIT_PROMPT, emit, err, ok, scenario, seam, source_input,
};
use test_programs::protocol::ExtractScenario;

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: ExtractScenario = scenario();
    let input = source_input(&scenario.input);
    let ctx = Context {
        adapter_id: scenario.adapter_id.as_str(),
        input: &input,
    };
    let seams: Vec<_> = scenario.seams.iter().map(seam).collect();
    let docs = if scenario.omit_extract_prompt { EXTRACT_PROSE_OMIT_PROMPT } else { EXTRACT_PROSE };
    let response = match emery_sdk::extract(&ctx, docs, &seams).await {
        Ok(evidence) => ok::<Evidence>(evidence),
        Err(error) => err(&error),
    };
    emit(response).await;
}
