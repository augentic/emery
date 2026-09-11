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

/// Runs `argv` in JSON mode and asserts the typed failure envelope, and that
/// the refused run left storage exactly as it found it: a refusal never
/// commits, prunes, or writes.
pub async fn fail(provider: &Provider, argv: &[&str], exit: u8, code: &str) -> Value {
    let before = provider.storage.snapshot();
    let mut json = vec!["emery", "--format", "json"];
    json.extend(argv.iter().skip(1).copied());
    let resp = cli(provider, &json).await;
    assert_eq!(resp.exit, exit, "{code}: {}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], code, "{envelope}");
    assert_eq!(envelope["exit-code"], exit, "{envelope}");
    assert_eq!(provider.storage.snapshot(), before, "{code}: a refused run writes nothing");
    envelope
}
