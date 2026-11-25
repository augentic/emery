//! Driver for [`emery_sdk::target::build`]: reads [`BuildScenario`] JSON from
//! argv and prints a [`Response`] as JSON on stdout.

#![cfg(target_arch = "wasm32")]

use emery_adapter::target::Report;
use emery_sdk::target::BuildContext;
use test_programs::helpers::{
    BUILD_PROSE, BUILD_PROSE_OMIT_PROMPT, build_slice, emit, err, leak_grants, leak_scaffold, ok,
    scenario, write_files,
};
use test_programs::protocol::{BuildCheck, BuildOutcome, BuildScenario, WORKSPACE};

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: BuildScenario = scenario();
    write_files(&scenario.files);
    let grants = leak_grants(&scenario.grants);
    let slice = build_slice(&scenario.slice);
    let mut ctx = BuildContext {
        adapter_id: scenario.adapter_id.as_str(),
        slice: &slice,
        workspace: WORKSPACE,
        grants,
        scaffold: None,
    };
    if let Some(scaffold) = scenario.scaffold.as_ref() {
        ctx = ctx.with_scaffold(leak_scaffold(scaffold));
    }
    let docs = if scenario.omit_build_prompt { BUILD_PROSE_OMIT_PROMPT } else { BUILD_PROSE };
    let mut checks = 0_usize;
    let response = match emery_sdk::target::build(&ctx, docs, |_report| {
        checks += 1;
        match scenario.check {
            BuildCheck::None => Ok(Vec::new()),
        }
    })
    .await
    {
        Ok(report) => ok::<BuildOutcome<Report>>(BuildOutcome { report, checks }),
        Err(error) => err(&error),
    };
    emit(response).await;
}
