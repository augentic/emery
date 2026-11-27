//! Driver for [`emery_sdk::target::verify`]: reads [`VerifyScenario`] JSON from
//! argv and prints a [`Response`] as JSON on stdout.

#![cfg(target_arch = "wasm32")]

use std::path::Path;

use emery_sdk::target::{Verdict, VerifyContext};
use test_programs::helpers::{
    VERIFY_PROSE, VERIFY_PROSE_OMIT_PROMPT, emit, err, leak_grants, ok, scenario,
};
use test_programs::protocol::{VERIFY_WORKSPACE, VerifyCheck, VerifyOutcome, VerifyScenario};

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: VerifyScenario = scenario();
    let grants = leak_grants(&scenario.grants);
    let ctx = VerifyContext::new(scenario.adapter_id, VERIFY_WORKSPACE).with_grants(grants);
    let docs = if scenario.omit_verify_prompt { VERIFY_PROSE_OMIT_PROMPT } else { VERIFY_PROSE };
    let mut checks = 0_usize;
    let response = match emery_sdk::target::verify(&ctx, docs, |verdict| {
        checks += 1;
        match scenario.check {
            VerifyCheck::None => Ok(Vec::new()),
            VerifyCheck::MembersList => {
                let manifest =
                    std::fs::read_to_string(Path::new(VERIFY_WORKSPACE).join("Cargo.toml"))
                        .map_err(|error| {
                            omnia_sdk::server_error!("reading `Cargo.toml`: {error}")
                        })?;
                let drifted = verdict.passed && !manifest.contains("crates/*");
                Ok(drifted
                    .then(|| "- `Cargo.toml` lacks the members list; put it back".to_owned())
                    .into_iter()
                    .collect())
            }
            VerifyCheck::DirectoryAtManifest => {
                Err(omnia_sdk::bad_request!("a directory stands at `Cargo.toml`"))
            }
        }
    })
    .await
    {
        Ok(verdict) => ok::<VerifyOutcome<Verdict>>(VerifyOutcome { verdict, checks }),
        Err(error) => err(&error),
    };
    emit(response).await;
}
