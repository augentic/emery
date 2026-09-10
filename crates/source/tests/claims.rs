//! Claim rules contract
//!
//! The rules both the engine and every adapter enforce on a claim set: clean
//! evidence passes, an id outside the dotted-kebab grammar is refused, and a
//! claim missing an extra its kind requires is refused. Pinning these here
//! keeps the two enforcement points from disagreeing.

use emery_source::claims::findings;
use emery_source::types::{ClaimKind, Evidence};

#[test]
fn clean_evidence() {
    let clean = evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"password-reset.request","statement":"Users reset by email."},
            {"kind":"criterion","id":"password-reset.expiry","criterion":"Links expire in 30m."},
            {"kind":"example","id":"password-reset.stale","replay-digest":"sha256:00"},
            {"kind":"decision"}
        ]}"#,
    );
    assert!(findings(&clean.claims).is_empty());
    assert!(clean.findings().is_empty(), "clean evidence passes the gate");
}

#[test]
fn malformed_ids() {
    let malformed = evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","statement":"Unnamed."},
            {"kind":"criterion","id":"Not.Valid","criterion":"Misnamed."},
            {"kind":"section"}
        ]}"#,
    );
    let findings = malformed.findings();
    assert_eq!(findings.len(), 2, "optional-id kinds pass unset; extras are present: {findings:?}");
    let detail = findings.join("\n");
    assert!(detail.contains("claims require an id"), "finding names the missing id: {detail}");
    assert!(detail.contains("`Not.Valid`"), "finding names the malformed id: {detail}");
}

// The closed table is the single A8 rule both gates consume.
#[test]
fn missing_extras() {
    assert_eq!(ClaimKind::Requirement.required_extras(), ["statement"]);
    assert_eq!(ClaimKind::Criterion.required_extras(), ["criterion"]);
    assert_eq!(ClaimKind::Example.required_extras(), ["replay-digest"]);
    assert!(ClaimKind::Decision.required_extras().is_empty());

    let bare = evidence(
        r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"password-reset.request"},
            {"kind":"example","id":"password-reset.stale","input":{}},
            {"kind":"section","synopsis":"no extras required"}
        ]}"#,
    );
    let findings = findings(&bare.claims);
    assert_eq!(
        findings.len(),
        2,
        "ids are well-formed; one finding per absent extra: {findings:?}"
    );
    assert!(findings[0].contains("`password-reset.request` is missing extra `statement`"));
    assert!(findings[1].contains("`password-reset.stale` is missing extra `replay-digest`"));
    assert_eq!(bare.findings(), findings, "the document gate is the claim gate over its claims");
}

fn evidence(json: &str) -> Evidence {
    serde_json::from_str(json).expect("evidence parses")
}
