//! Driver for [`emery_sdk::survey::surfaces`]: reads [`SurveyScenario`] JSON from
//! argv and prints a [`Response`] as JSON on stdout.

#![cfg(target_arch = "wasm32")]

use emery_sdk::Context;
use emery_sdk::survey::{Facts, Inventory};
use test_programs::helpers::{
    SURVEY_PROSE, SURVEY_PROSE_OMIT_PROMPT, emit, ok, scenario, source_input, survey_check,
};
use test_programs::protocol::{Response, SurveyScenario};

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: SurveyScenario = scenario();
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
            Err(error) => Response::Err {
                kind: format!("{:?}", error.kind()).to_lowercase(),
                code: error.code().to_owned(),
                description: error.description().to_owned(),
            },
        };
    emit(response).await;
}
