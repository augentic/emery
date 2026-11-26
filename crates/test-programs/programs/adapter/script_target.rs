//! Scripted target adapter: reads outcomes from the `script` mount.

#![cfg(target_arch = "wasm32")]

use std::future;
use std::num::NonZeroU32;

use emery_adapter::target::{MergeRule, Outcome, Task};
use emery_sdk::target::export::{self, Guest};
use test_programs::script::{CombinedScript, ScriptCall};
use test_programs::script_io::{
    append_call, read_state, rendezvous_wait, write_slice_dispatch, write_state,
};

struct Adapter;

impl Guest for Adapter {
    fn metadata(id: export::AdapterId) -> export::TargetMetadata {
        // Metadata is sync on the host poll; reading the script mount here traps on wasip3.
        let rules = if id.starts_with("acme:builder") || id.starts_with("acme:target") {
            vec![MergeRule::union("src/*/mod.rs"), MergeRule::ours("Cargo.lock")]
        } else {
            Vec::new()
        };
        let mut metadata = emery_sdk::target::metadata(&rules, Some(scripted_budget(&id)));
        metadata.emery_version = scripted_emery_version(&id);
        export::TargetMetadata::from(metadata)
    }

    async fn run(
        id: export::AdapterId, workspace: String, task: export::Task,
    ) -> Result<export::Outcome, export::Error> {
        future::ready(dispatch(&id, &workspace, task)).await
    }
}

fn scripted_emery_version(id: &str) -> Option<String> {
    if id == "acme:builderv100" { Some("100.0.0".into()) } else { None }
}

fn scripted_budget(id: &str) -> NonZeroU32 {
    let default = NonZeroU32::new(12).expect("twelve");
    let Some(rest) = id.strip_prefix("acme:builderb") else {
        return default;
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok().and_then(NonZeroU32::new).unwrap_or(default)
}

fn dispatch(
    id: &export::AdapterId, workspace: &str, task: export::Task,
) -> Result<export::Outcome, export::Error> {
    let mut combined = read_state::<CombinedScript>();
    let task = Task::from(task);
    match &task {
        Task::Init => {
            append_call(&ScriptCall::TargetInit {
                id: id.clone(),
                workspace: workspace.to_owned(),
            });
        }
        Task::Build(slice) => {
            write_slice_dispatch(&slice.id, &slice.base);
            append_call(&ScriptCall::TargetBuild {
                id: id.clone(),
                slice_id: slice.id.clone(),
                workspace: workspace.to_owned(),
                slice: Box::default(),
            });
            if let Some(rendezvous) = &combined.target.rendezvous {
                rendezvous_wait(
                    &slice.id,
                    &rendezvous.expected.iter().cloned().collect::<Vec<_>>(),
                );
            }
        }
        Task::Verify => {
            append_call(&ScriptCall::TargetVerify {
                id: id.clone(),
                workspace: workspace.to_owned(),
            });
        }
    }

    let result = combined
        .target
        .run(id, workspace, &task)
        .map(|outcome| match outcome {
            Outcome::Initialised => export::Outcome::Initialised,
            Outcome::Built(report) => export::Outcome::Built(report.into()),
            Outcome::Verified(verdict) => export::Outcome::Verified(verdict.into()),
        })
        .map_err(Into::into);
    write_state(&combined);
    result
}

emery_sdk::target::export::export!(Adapter with_types_in emery_sdk::target::export);
