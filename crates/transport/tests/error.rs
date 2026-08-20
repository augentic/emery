//! Error discriminant, hint, and exit-code contract.

use emery_transport::command::{Format, render_failure};
use omnia_guest::{Error, ErrorKind};

fn envelope(err: &Error) -> serde_json::Value {
    let (stderr, _) = render_failure(Format::Json, err);
    serde_json::from_slice(&stderr).expect("JSON failure envelope")
}

fn validation(code: &'static str, rule: &str, detail: &str) -> Error {
    let body = if rule.is_empty() { detail.to_string() } else { format!("{rule}: {detail}") };
    Error::new(ErrorKind::BadRequest, code, format!("{code}: {body}"))
}

#[test]
fn diag_round_trip() {
    let err = Error::new(ErrorKind::ServerError, "kebab-prefix", "kebab-prefix: specific detail");
    assert_eq!(err.code(), "kebab-prefix");
    assert_eq!(err.description(), "kebab-prefix: specific detail");
    let body = envelope(&err);
    assert_eq!(body["error"], "kebab-prefix");
    assert_eq!(body["message"], "kebab-prefix: specific detail");
    assert_eq!(body["exit-code"], 1);
}

#[test]
fn cli_too_old_display() {
    let err = Error::new(
        ErrorKind::ServerError,
        "emery-version-too-old",
        "emery version 0.9.0 is older than the project floor 1.0.0; upgrade the CLI",
    );
    assert_eq!(err.code(), "emery-version-too-old");
    let msg = err.description();
    assert!(msg.contains("0.9.0") && msg.contains("1.0.0"), "both versions in display: {msg}");
    let (stderr, code) = render_failure(Format::Text, &err);
    let stderr = String::from_utf8(stderr).expect("utf-8");
    assert_eq!(code, 1, "floor failures are exit 1");
    assert!(stderr.contains("brew upgrade emery"), "the hint names the install channel: {stderr}");
}

#[test]
fn adapter_too_old_display() {
    let err = Error::new(
        ErrorKind::ServerError,
        "adapter-cli-too-old",
        "emery version 1.0.0 is older than the floor 2.0.0 required by adapter omnia \
         (omnia@1.0.0.wasm); upgrade the CLI",
    );
    assert_eq!(err.code(), "adapter-cli-too-old");
    let msg = err.description();
    assert!(
        msg.contains("1.0.0") && msg.contains("2.0.0") && msg.contains("omnia"),
        "versions and adapter in display: {msg}"
    );
    let (stderr, code) = render_failure(Format::Text, &err);
    let stderr = String::from_utf8(stderr).expect("utf-8");
    assert_eq!(code, 1, "adapter floor failures are exit 1");
    assert!(stderr.contains("brew upgrade emery"), "the hint names the install channel: {stderr}");
    assert!(
        !stderr.contains("emery adapter upgrade"),
        "the adapter-upgrade fallback is deleted: {stderr}"
    );
}

#[test]
fn validation_code_display() {
    let err = validation("bad-thing", "rule", "detail");
    assert_eq!(err.code(), "bad-thing");
    assert_eq!(err.description(), "bad-thing: rule: detail");
    let body = envelope(&err);
    assert_eq!(body["error"], "bad-thing");
    assert_eq!(body["exit-code"], 2);
}

#[test]
fn empty_rule_omits_prefix() {
    let err = validation("code", "", "just detail");
    assert_eq!(err.description(), "code: just detail");
    assert_eq!(err.code(), "code");
}

#[test]
fn init_source_required_hint() {
    let err = validation(
        "init-source-required",
        "emery init requires at least one source adapter",
        "pass an adapter",
    );
    let (stderr, code) = render_failure(Format::Text, &err);
    let stderr = String::from_utf8(stderr).expect("utf-8");
    assert_eq!(code, 2);
    assert!(stderr.contains("init-source-required"), "{stderr}");
    assert!(stderr.contains("emery init"), "the hint names the recovery gesture: {stderr}");
}

#[test]
fn not_initialized_hint() {
    let err = Error::new(
        ErrorKind::NotFound,
        "not-initialized",
        "not-initialized: .emery/project.yaml not found",
    );
    let (stderr, code) = render_failure(Format::Text, &err);
    let stderr = String::from_utf8(stderr).expect("utf-8");
    assert_eq!(code, 1);
    assert!(stderr.contains("not-initialized"), "{stderr}");
    assert!(stderr.contains("emery init"), "{stderr}");
}
