//! Scenario support
//!
//! The shared plumbing behind the root suites: the scripted provider and
//! runner in [`provider`], plus the runners that assert on the response —
//! a success, or the typed failure envelope.

mod provider;

use omnia_guest::api::command::Response;
use omnia_guest::{BlobStore, StateStore};
pub use provider::*;
use serde_json::Value;

/// Runs one CLI invocation and asserts success.
pub async fn cli_ok<S>(provider: &Provider<S>, argv: &[&str]) -> Response
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let resp = cli(provider, argv).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    resp
}

/// Runs `argv` in JSON mode and asserts the typed failure envelope.
pub async fn fail<S>(provider: &Provider<S>, argv: &[&str], exit: u8, code: &str) -> Value
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let mut json = vec!["emery", "--format", "json"];
    json.extend(argv.iter().skip(1).copied());
    let resp = cli(provider, &json).await;
    assert_eq!(resp.exit, exit, "{code}: {}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], code, "{envelope}");
    assert_eq!(envelope["exit-code"], exit, "{envelope}");
    envelope
}
