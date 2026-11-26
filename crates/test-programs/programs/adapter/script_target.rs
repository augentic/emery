//! Scripted target adapter: reads outcomes from the `script` mount.

#![cfg(target_arch = "wasm32")]

use std::future;
use std::num::NonZeroU32;

use emery_adapter::target::{Outcome, Task};
use emery_sdk::target::export::{self, Guest};
use test_programs::script::{CombinedScript, ScriptCall};
use test_programs::script_io::{append_call, read_state, rendezvous_wait};

struct Adapter;

impl Guest for Adapter {
    fn metadata(id: export::AdapterId) -> export::TargetMetadata {
        // Metadata is invoked from the host's async poll: blocking on WASI filesystem here traps.
        // Budget and rules for `show schedule` are covered in run paths; metadata returns defaults.
        let _ = id;
        export::TargetMetadata::from(emery_sdk::target::metadata(&[], NonZeroU32::new(4)))
    }

    async fn run(
        id: export::AdapterId, workspace: String, task: export::Task,
    ) -> Result<export::Outcome, export::Error> {
        future::ready(dispatch(&id, &workspace, task)).await
    }
}

fn dispatch(
    id: &export::AdapterId, workspace: &str, task: export::Task,
) -> Result<export::Outcome, export::Error> {
    let mut state = read_state::<CombinedScript>().target;
    let task = Task::from(task);
    match &task {
        Task::Init => {
            append_call(&ScriptCall::TargetInit {
                id: id.clone(),
                workspace: workspace.to_owned(),
            });
        }
        Task::Build(slice) => {
            append_call(&ScriptCall::TargetBuild {
                id: id.clone(),
                slice_id: slice.id.clone(),
                workspace: workspace.to_owned(),
            });
            if let Some(rendezvous) = &state.rendezvous {
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

    state
        .run(id, workspace, &task)
        .map(|outcome| match outcome {
            Outcome::Initialised => export::Outcome::Initialised,
            Outcome::Built(report) => export::Outcome::Built(report.into()),
            Outcome::Verified(verdict) => export::Outcome::Verified(verdict.into()),
        })
        .map_err(Into::into)
}

emery_sdk::target::export::export!(Adapter with_types_in emery_sdk::target::export);
