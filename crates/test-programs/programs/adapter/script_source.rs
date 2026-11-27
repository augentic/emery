//! Scripted source adapter: reads outcomes from the `script` mount.

#![cfg(target_arch = "wasm32")]

use emery_adapter::source::{SourceKind, SourceMetadata};
use emery_sdk::SourceInput;
use emery_sdk::export::{self, Guest};
use test_programs::script::{CombinedScript, ScriptCall, SourceScriptState};
use test_programs::script_io::{append_call, hold_forever, read_state, rendezvous_wait};

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
        let state = read_state::<CombinedScript>().source;
        let source = SourceInput::from(input);
        extract_scripted(&id, &source, &state).await
    }
}

fn scripted_emery_version(id: &str) -> Option<String> {
    (id == "acme:docsv99").then(|| "99.0.0".to_owned())
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
    id: &str, source: &SourceInput, state: &SourceScriptState,
) -> Result<export::Evidence, export::Error> {
    append_call(&ScriptCall::SourceExtract {
        id: id.to_owned(),
        input: source.clone(),
    });

    if let Some(rendezvous) = &state.rendezvous {
        rendezvous_wait(&source.name, &rendezvous.expected).await;
    }
    if state.held.contains(&source.name) {
        hold_forever().await;
    }

    state.extract(&source.name).map(Into::into).map_err(Into::into)
}

emery_sdk::export::export!(Adapter with_types_in emery_sdk::export);
