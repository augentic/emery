//! Scripted source adapter: reads outcomes from the `script` mount.

#![cfg(target_arch = "wasm32")]

use std::future::pending;

use emery_adapter::source::{Evidence, SourceKind, SourceMetadata};
use emery_sdk::SourceInput;
use emery_sdk::export::{self, Guest};
use test_programs::script::{CombinedScript, EvidenceScript, ScriptCall, SourceScriptState};
use test_programs::script_io::{append_call, read_state, rendezvous_wait};

struct Adapter;

impl Guest for Adapter {
    fn metadata(id: export::AdapterId) -> export::AdapterMetadata {
        // Metadata is sync on the host poll; reading the script mount here traps on wasip3.
        export::AdapterMetadata::from(SourceMetadata {
            emery_version: scripted_emery_version(&id),
            kind: kind_for_adapter(&id),
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
        extract_scripted(&id, &source, &state).await
    }
}

fn scripted_emery_version(id: &str) -> Option<String> {
    if id == "acme:docsv99" { Some("99.0.0".into()) } else { None }
}

fn kind_for_adapter(id: &str) -> SourceKind {
    if id.contains("code") {
        SourceKind::Behaviour
    } else if id.contains("intent") {
        SourceKind::Intent
    } else {
        SourceKind::Documentation
    }
}

async fn extract_scripted(
    id: &export::AdapterId, source: &SourceInput, state: &SourceScriptState,
) -> Result<export::Evidence, export::Error> {
    append_call(&ScriptCall::SourceExtract {
        id: id.clone(),
        input: source.clone(),
    });

    if let Some(rendezvous) = &state.rendezvous {
        let expected: Vec<_> = rendezvous.expected.iter().cloned().collect();
        rendezvous_wait(&source.name, &expected).await;
    }

    let outcome: Result<Evidence, omnia_sdk::Error> = match state.evidence.get(&source.name) {
        Some(EvidenceScript::Ok(evidence)) => Ok(evidence.clone()),
        Some(EvidenceScript::Err(err)) => Err(err.clone().into()),
        None => Ok(SourceScriptState::default_evidence()),
    };

    outcome.map(Into::into).map_err(Into::into)
}

emery_sdk::export::export!(Adapter with_types_in emery_sdk::export);
