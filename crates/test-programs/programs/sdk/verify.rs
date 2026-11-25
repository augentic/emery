//! Placeholder for [`emery_sdk::target::verify`] (A6 ports the suite).

#![cfg(target_arch = "wasm32")]

use test_programs::helpers::{emit, ok, scenario};
use test_programs::protocol::VerifyScenario;

omnia_sdk::command::export!(run);

async fn run() {
    let scenario: VerifyScenario = scenario();
    assert_eq!(scenario.op, "stub", "only the stub op is implemented before A6");
    emit(ok(serde_json::json!({ "ready": false }))).await;
}
