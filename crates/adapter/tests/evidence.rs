//! Evidence contract
//!
//! What both the engine and every adapter can rely on from an evidence
//! document: the body a model answers with parses into typed claims that keep
//! their open extras, malformed open fields become absent rather than fatal,
//! and the claim gate refuses an id outside the dotted-kebab grammar or a
//! claim missing an extra its kind requires. Pinning the gate here keeps the
//! two enforcement points from disagreeing.

use emery_adapter::source::{Authority, Backing, ClaimKind, Evidence};

#[test]
fn parse_evidence() {
    let evidence = evidence(
        r#"{
            "authority": "behaviour",
            "claims": [
                {
                    "kind": "example",
                    "id": "password-reset.expiry",
                    "path": "captures/reset.json#L3-L9",
                    "synopsis": "Expired token is rejected.",
                    "backing": {"path": "captures/reset.json"},
                    "replay-digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                    "input": {"token": "stale"},
                    "output": {"status": 410}
                },
                {"kind": "type", "backing": {"payload": "struct ResetToken { expiry: Instant }"}}
            ]
        }"#,
    );

    assert_eq!(evidence.authority, Authority::Behaviour);
    assert_eq!(evidence.claims.len(), 2);
    let example = &evidence.claims[0];
    assert_eq!(example.kind, ClaimKind::Example);
    assert_eq!(example.id.as_deref(), Some("password-reset.expiry"));
    assert_eq!(example.path.as_deref(), Some("captures/reset.json#L3-L9"));
    assert_eq!(example.backing, Some(Backing::Path("captures/reset.json".to_string())));
    // Open per-kind fields are preserved (A8).
    assert_eq!(
        example.extras.get("replay-digest").and_then(serde_json::Value::as_str),
        Some(concat!(
            "sha256:",
            "0000000000000000000000000000000000000000000000000000000000000000"
        )),
    );
    assert_eq!(example.extras["input"], serde_json::json!({"token": "stale"}));
    assert_eq!(example.extras["output"], serde_json::json!({"status": 410}));
    assert!(!example.extras.contains_key("synopsis"), "modeled keys stay typed");
    let claim = &evidence.claims[1];
    assert_eq!(claim.kind, ClaimKind::Type, "`type` deserializes despite being a keyword");
    assert_eq!(
        claim.backing,
        Some(Backing::Payload("struct ResetToken { expiry: Instant }".to_string()))
    );
    assert!(claim.id.is_none() && claim.path.is_none() && claim.synopsis.is_none());
    assert!(claim.extras.is_empty(), "no unmodeled keys, no extras");
}

// Unpinned `synopsis` and `backing` shapes become absent, not fatal.
#[test]
fn open_fields() {
    let evidence = evidence(
        r#"{
            "authority": "documentation",
            "claims": [
                {"kind": "section", "synopsis": {"headline": "structured"}, "backing": "bare string"},
                {"kind": "decision", "synopsis": "kept", "backing": {"payload": "ADR-7"}}
            ]
        }"#,
    );

    let odd = &evidence.claims[0];
    assert!(odd.synopsis.is_none(), "non-string synopsis is dropped");
    assert!(odd.backing.is_none(), "non-variant backing is dropped");
    let clean = &evidence.claims[1];
    assert_eq!(clean.synopsis.as_deref(), Some("kept"), "modeled shapes still parse");
    assert_eq!(clean.backing, Some(Backing::Payload("ADR-7".to_string())));
}

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
    let findings = bare.findings();
    assert_eq!(
        findings.len(),
        2,
        "ids are well-formed; one finding per absent extra: {findings:?}"
    );
    assert!(findings[0].contains("`password-reset.request` is missing extra `statement`"));
    assert!(findings[1].contains("`password-reset.stale` is missing extra `replay-digest`"));
}

fn evidence(json: &str) -> Evidence {
    serde_json::from_str(json).expect("evidence parses")
}
