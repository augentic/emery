//! Shared test harness and scripted capabilities for the CLI suites.

#![expect(dead_code, reason = "shared by suites that each use a subset")]

pub mod files;
pub mod guest;
pub mod prefix;
pub mod provider;
pub mod scratch;
pub mod storage;
pub mod store;
pub mod vcs;

use omnia_sdk::command::Response;
pub use provider::{Provider, cli};
use serde_json::Value;

/// Runs one CLI invocation and asserts success.
pub async fn cli_ok(provider: &Provider, argv: &[&str]) -> Response {
    let resp = cli(provider, argv).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    resp
}

/// Runs `argv` in JSON mode and asserts the typed failure envelope.
/// Returns the last JSON object in `stderr`, past any log lines the guest may emit.
pub fn envelope_bytes(stderr: &[u8]) -> &[u8] {
    let text = std::str::from_utf8(stderr).expect("utf-8 stderr");
    for marker in ["\n{\n  \"error\"", "\n{\"error\"", "{\n  \"error\""] {
        if let Some(idx) = text.rfind(marker) {
            let start = idx + usize::from(marker.starts_with('\n'));
            return &stderr[start..];
        }
    }
    let Some(start) = text.rfind('{') else {
        return stderr;
    };
    &stderr[start..]
}

pub async fn fail(provider: &Provider, argv: &[&str], exit: u8, code: &str) -> Value {
    let before = provider.storage.snapshot().await;
    let mut json = vec!["emery", "--format", "json"];
    json.extend(argv.iter().skip(1).copied());
    let resp = cli(provider, &json).await;
    assert_eq!(resp.exit, exit, "{code}: {}", String::from_utf8_lossy(&resp.stderr));
    assert!(
        resp.stdout.is_empty(),
        "{code}: a failure writes nothing to stdout: {}",
        String::from_utf8_lossy(&resp.stdout)
    );
    let envelope: Value =
        serde_json::from_slice(envelope_bytes(&resp.stderr)).expect("one JSON envelope");
    assert_eq!(envelope["error"], code, "{envelope}");
    assert_eq!(envelope["exit-code"], exit, "{envelope}");
    assert_eq!(provider.storage.snapshot().await, before, "{code}: a refused run writes nothing");
    envelope
}
