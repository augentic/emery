//! Driver for [`emery_sdk::survey::surfaces`] and [`emery_sdk::survey::seams`]:
//! reads [`SurveyCommand`] JSON from argv and prints a [`Response`] as JSON on stdout.

#![cfg(target_arch = "wasm32")]

use emery_sdk::Context;
use emery_sdk::survey::{Facts, Inventory};
use test_programs::helpers::{
    SURVEY_PROSE, SURVEY_PROSE_OMIT_PROMPT, emit, err, ok, scenario, source_input, survey_check,
};
use test_programs::protocol::{SurveyCommand, SurveyScenario};
use test_programs::seams;

omnia_sdk::command::export!(run);

async fn run() {
    match scenario::<SurveyCommand>() {
        SurveyCommand::Surfaces(scenario) => run_surfaces(&scenario).await,
        SurveyCommand::Seams(scenario) => emit(seams::run(&scenario).await).await,
    }
}

async fn run_surfaces(scenario: &SurveyScenario) {
    let input = source_input(&scenario.input);
    let ctx = Context {
        adapter_id: scenario.adapter_id.as_str(),
        input: &input,
    };
    let facts = Facts {
        modules: &scenario.facts.modules,
        text: &scenario.facts.text,
        files: &scenario.facts.files,
    };
    let docs = if scenario.omit_survey_prompt { SURVEY_PROSE_OMIT_PROMPT } else { SURVEY_PROSE };
    let response =
        match emery_sdk::survey::surfaces(&ctx, docs, &facts, survey_check(scenario.check)).await {
            Ok(inventory) => ok::<Inventory>(inventory),
            Err(error) => err(&error),
        };
    emit(response).await;
}
