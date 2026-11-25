//! Placeholder for [`emery_sdk::target::build`] (A8 ports the suite).

#![cfg(target_arch = "wasm32")]

use test_programs::helpers::{emit, ok, scenario};
use test_programs::protocol::BuildScenario;

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: BuildScenario = scenario();
    assert_eq!(scenario.op, "stub", "only the stub op is implemented before A8");
    emit(ok(serde_json::json!({ "ready": false }))).await;
}
