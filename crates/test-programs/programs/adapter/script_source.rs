//! Scripted source adapter: reads outcomes from the `script` mount.

#![cfg(target_arch = "wasm32")]

use std::future::{self, pending};

use emery_adapter::source::{Evidence, SourceMetadata};
use emery_sdk::SourceInput;
use emery_sdk::export::{self, Guest};
use test_programs::script::{CombinedScript, EvidenceScript, ScriptCall, SourceScriptState};
use test_programs::script_io::{append_call, read_state, rendezvous_wait};

struct Adapter;

impl Guest for Adapter {
    fn metadata(id: export::AdapterId) -> export::AdapterMetadata {
        let _ = id;
        export::AdapterMetadata::from(SourceMetadata {
            emery_version: None,
            kind: emery_adapter::source::SourceKind::Documentation,
        })
    }

    async fn extract(
        id: export::AdapterId, input: export::Input,
    ) -> Result<export::Evidence, export::Error> {
        let state: SourceScriptState = read_state::<CombinedScript>().source;
        let source = SourceInput::from(input);
        if state.held.contains(&source.name) {
            pending::<()>().await;
        }
        future::ready(extract_scripted(&id, &source, &state)).await
    }
}

fn extract_scripted(
    id: &export::AdapterId, source: &SourceInput, state: &SourceScriptState,
) -> Result<export::Evidence, export::Error> {
    append_call(&ScriptCall::SourceExtract {
        id: id.clone(),
        input: source.clone(),
    });

    if let Some(rendezvous) = &state.rendezvous {
        rendezvous_wait(&source.name, &rendezvous.expected.iter().cloned().collect::<Vec<_>>());
    }

    let outcome: Result<Evidence, omnia_sdk::Error> = match state.evidence.get(&source.name) {
        Some(EvidenceScript::Ok(evidence)) => Ok(evidence.clone()),
        Some(EvidenceScript::Err(err)) => Err(err.clone().into()),
        None => Ok(SourceScriptState::default_evidence()),
    };

    outcome.map(Into::into).map_err(Into::into)
}

emery_sdk::export::export!(Adapter with_types_in emery_sdk::export);
