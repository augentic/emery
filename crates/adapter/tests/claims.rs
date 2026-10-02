//! An `Evidence` document parses from JSON and its claim gate names each malformed claim.

use emery_adapter::source::{Anchor, Backing, BadAnchor, ClaimKind, Evidence};

#[test]
fn parse_evidence() {
    let evidence = evidence(
        r#"{
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

    assert_eq!(evidence.claims.len(), 2);
    let example = &evidence.claims[0];
    assert_eq!(example.kind, ClaimKind::Example);
    assert_eq!(example.id.as_deref(), Some("password-reset.expiry"));
    assert_eq!(example.path.as_deref(), Some("captures/reset.json#L3-L9"));
    assert_eq!(example.backing, Some(Backing::Path("captures/reset.json".to_string())));
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

// A source kind is not the model's to answer, so the key is a parse failure
// rather than a value to reconcile.
#[test]
fn document_kind() {
    let refused =
        serde_json::from_str::<Evidence>(r#"{"kind":"intent","claims":[{"kind":"decision"}]}"#)
            .expect_err("a document-level kind is refused");
    assert!(refused.to_string().contains("unknown field `kind`"), "{refused}");
}

// An unpinned shape becomes absent, not fatal.
#[test]
fn open_fields() {
    let evidence = evidence(
        r#"{
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
        r#"{"claims":[
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
        r#"{"claims":[
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

#[test]
fn missing_extras() {
    let bare = evidence(
        r#"{"claims":[
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

#[test]
fn anchors() {
    let whole = Anchor::parse("src/orders.ts").expect("a whole-file anchor");
    assert_eq!(whole.path, "src/orders.ts");
    assert_eq!(whole.lines, None);
    let line = Anchor::parse("src/orders.ts#L7").expect("a single line");
    assert_eq!(line.lines, Some((7, 7)));
    let range = Anchor::parse("docs/api/orders.md#L12-L34").expect("a range");
    assert_eq!(range.path, "docs/api/orders.md");
    assert_eq!(range.lines, Some((12, 34)));
    assert_eq!(Anchor::parse("a/.omnia.md").map(|anchor| anchor.path), Ok("a/.omnia.md"));
    assert_eq!(Anchor::parse("a/spec.md.bak").map(|anchor| anchor.path), Ok("a/spec.md.bak"));

    for (anchor, expected) in [
        ("src/orders.ts#12", BadAnchor::Grammar),
        ("src/orders.ts#L", BadAnchor::Grammar),
        ("src/orders.ts#L0", BadAnchor::Grammar),
        ("src/orders.ts#L01", BadAnchor::Grammar),
        ("src/orders.ts#L1-", BadAnchor::Grammar),
        ("src/orders.ts#L1-L2-L3", BadAnchor::Grammar),
        ("src/orders.ts#L1 -L2", BadAnchor::Grammar),
        ("", BadAnchor::Escapes),
        ("#L1", BadAnchor::Escapes),
        ("/etc/passwd", BadAnchor::Escapes),
        ("../secret.ts", BadAnchor::Escapes),
        ("src/../../x.ts#L1", BadAnchor::Escapes),
        (".omnia/storage/x.json", BadAnchor::SkipDir(".omnia".to_string())),
        ("a/.omnia/x.json#L1", BadAnchor::SkipDir(".omnia".to_string())),
        ("spec.md", BadAnchor::SkipFile("spec.md".to_string())),
        ("docs/plan.md#L3", BadAnchor::SkipFile("plan.md".to_string())),
        ("src/orders.ts#L34-L12", BadAnchor::Reversed { start: 34, end: 12 }),
    ] {
        assert_eq!(Anchor::parse(anchor), Err(expected), "{anchor}");
    }
}

// The gate reports the anchor beside the id and the extras, once per claim,
// and `Claim::anchor` gives a caller the parse without re-reading the finding.
#[test]
fn anchored_claims() {
    let anchored = evidence(
        r#"{"claims":[
            {"kind":"requirement","id":"orders.create","path":"src/orders.ts#L3-L9","statement":"Creates."},
            {"kind":"requirement","id":"orders.list","path":"../x.ts","statement":"Lists."},
            {"kind":"call","path":"src/orders.ts#L9-L3"},
            {"kind":"type","path":".omnia/x.json"},
            {"kind":"decision"}
        ]}"#,
    );
    let findings = anchored.findings();
    assert_eq!(findings.len(), 3, "{findings:?}");
    assert!(
        findings[0].contains("claim 1: path `../x.ts` escapes the source root"),
        "{findings:?}"
    );
    assert!(
        findings[1].contains("claim 2: path `src/orders.ts#L9-L3` ends at line 3"),
        "{findings:?}"
    );
    assert!(
        findings[2].contains("claim 3: path `.omnia/x.json` is under the skip root"),
        "{findings:?}"
    );
    assert_eq!(
        anchored.claims[0].anchor(),
        Some(Ok(Anchor {
            path: "src/orders.ts",
            lines: Some((3, 9)),
        }))
    );
    assert_eq!(anchored.claims[1].anchor(), Some(Err(BadAnchor::Escapes)));
    assert_eq!(anchored.claims[4].anchor(), None);
}

fn evidence(json: &str) -> Evidence {
    serde_json::from_str(json).expect("evidence parses")
}
