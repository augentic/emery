//! Driver for [`emery_sdk::target::build`]: reads [`BuildScenario`] JSON from
//! argv and prints a [`Response`] as JSON on stdout.

#![cfg(target_arch = "wasm32")]

use std::path::Path;

use emery_sdk::target::BuildContext;
use test_programs::helpers::{BUILD_PROSE, BUILD_PROSE_OMIT_PROMPT, emit, err, ok, scenario};
use test_programs::protocol::{BuildCheck, BuildOutcome, BuildScenario, WORKSPACE};

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: BuildScenario = scenario();
    let mut ctx = BuildContext::new(scenario.adapter_id, WORKSPACE, scenario.slice)
        .with_grants(scenario.grants);
    if let Some(scaffold) = scenario.scaffold {
        ctx = ctx.with_scaffold(scaffold);
    }
    let docs = if scenario.omit_build_prompt { BUILD_PROSE_OMIT_PROMPT } else { BUILD_PROSE };
    let mut checks = 0_usize;
    let response = match emery_sdk::target::build(&ctx, docs, |_report| {
        checks += 1;
        match scenario.check {
            BuildCheck::None => Ok(Vec::new()),
            BuildCheck::RequireManifest => {
                let held = Path::new(WORKSPACE).join("Cargo.toml").is_file();
                Ok((!held)
                    .then(|| "- the tree lacks `Cargo.toml`; write it".to_owned())
                    .into_iter()
                    .collect())
            }
            BuildCheck::PermissionDenied => {
                Err(omnia_sdk::server_error!("reading `Cargo.toml`: permission denied"))
            }
        }
    })
    .await
    {
        Ok(report) => ok(BuildOutcome { report, checks }),
        Err(error) => err(&error),
    };
    emit(response).await;
}
