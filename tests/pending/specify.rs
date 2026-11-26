//! Remainder of `emery specify` scenarios — ported incrementally from this file (A23–A24).
//!
//! Lines 1–2286 are live in `tests/specify.rs` after A22.

// --- triggers ---

// A `contract` claim's route binds to every requirement whose id leads with
// the contract's id, read in the one route grammar: the stored requirement
// carries it as its typed trigger, the projection renders it as its `Route:`
// line, and the drafting turns see it in the requirement outline.
#[tokio::test]
async fn contract_route() {
    let apis = design_with_apis();
    let mut provider = Provider::answering([SPEC_ANSWER, apis.as_str()]);
    let mut contract = claim(ClaimKind::Contract, "greeting", ("route", "get /greeting/"));
    contract.path = Some("src/greeting.ts#L3-L9".to_string());
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            contract,
        ])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let id = staged(&provider.storage);
    let spec: Value = serde_json::from_slice(&staged_document(&provider.storage, &id, "spec.json"))
        .expect("the committed spec is JSON");
    assert_eq!(
        spec["requirements"][0]["trigger"],
        serde_json::json!({"http": {"method": "GET", "path": "/greeting"}}),
        "the route is stored as the typed trigger, normalised: {spec}"
    );
    let shown = shown(&provider, "spec").await;
    assert!(shown.contains("Status: unknown\nRoute: GET /greeting\n\nGET /greeting"), "{shown}");
    let draft = provider.model.seen()[0].messages.join("\n");
    assert!(draft.contains("— Route: GET /greeting\n"), "the outline carries the route: {draft}");
    provider.model.assert_exhausted();
}

// A `contract` claim's message binds the same way, so a consumer's handler
// is laid at the message the source registered it for: stored typed,
// rendered as the `Message:` line, and shown to the drafting turns.
#[tokio::test]
async fn contract_message() {
    let apis = design_with_apis();
    let mut provider = Provider::answering([SPEC_ANSWER, apis.as_str()]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            claim(ClaimKind::Contract, "greeting", ("message", " greeting.requested ")),
        ])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let id = staged(&provider.storage);
    let spec: Value = serde_json::from_slice(&staged_document(&provider.storage, &id, "spec.json"))
        .expect("the committed spec is JSON");
    assert_eq!(
        spec["requirements"][0]["trigger"],
        serde_json::json!({"message": "greeting.requested"}),
        "the message is stored as the typed trigger, trimmed: {spec}"
    );
    let shown = shown(&provider, "spec").await;
    assert!(shown.contains("Status: unknown\nMessage: greeting.requested\n\n"), "{shown}");
    let draft = provider.model.seen()[0].messages.join("\n");
    assert!(
        draft.contains("— Message: greeting.requested\n"),
        "the outline carries the message: {draft}"
    );
    provider.model.assert_exhausted();
}

// The second run binds the same requirement to a message where the first
// bound a route, and nothing else moves: the diff names the trigger alone.
#[tokio::test]
async fn diff_trigger() {
    let apis = design_with_apis();
    let mut provider =
        Provider::answering([SPEC_ANSWER, apis.as_str(), SPEC_ANSWER, apis.as_str()]);
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            claim(ClaimKind::Contract, "greeting", ("route", "GET /greeting")),
        ])),
    );
    cli_ok(
        &provider,
        &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
    )
    .await;
    let first = committed(&provider.storage);

    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            claim(ClaimKind::Contract, "greeting", ("message", "greeting.requested")),
        ])),
    );
    let resp = cli_ok(
        &provider,
        &["emery", "--format", "json", "specify", &reference("docs"), "--config", PROJECT_CONFIG],
    )
    .await;

    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let diff = &envelope["diff"];
    assert_eq!(diff["from"], first, "{envelope}");
    assert_eq!(diff["spec"]["added"], serde_json::json!([]), "{envelope}");
    assert_eq!(diff["spec"]["removed"], serde_json::json!([]), "{envelope}");
    assert_eq!(
        diff["spec"]["changed"],
        serde_json::json!([{"id": "REQ-001", "subject": "greeting.behaviour", "fields": ["trigger"]}]),
        "{envelope}"
    );
    assert_eq!(diff["plan"]["changed"], serde_json::json!([]), "{envelope}");
    provider.model.assert_exhausted();
}

// A `contract` claim is reached through one trigger: a message holding
// whitespace, or a claim carrying both a route and a message, is the
// adapter's defect, named by the claim gate before any turn.
#[tokio::test]
async fn contract_trigger_malformed() {
    let mut provider = Provider::idle();
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            claim(ClaimKind::Contract, "greeting", ("message", "greeting requested")),
        ])),
    );
    let envelope =
        fail(&provider, &["emery", "specify", &reference("docs")], 3, "server_error").await;
    assert_message(
        &envelope,
        "- claim 1: message \"greeting requested\" names the message `greeting requested`, which \
         holds whitespace; a message is one word",
    );

    let mut provider = Provider::idle();
    let mut both = claim(ClaimKind::Contract, "greeting", ("route", "GET /greeting"));
    both.extras.insert("message".to_string(), Value::String("greeting.requested".to_string()));
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            both,
        ])),
    );
    let envelope =
        fail(&provider, &["emery", "specify", &reference("docs")], 3, "server_error").await;
    assert_message(
        &envelope,
        "- claim 1: carries both `route` and `message`; a surface is reached through one",
    );
}

// A `contract` claim whose route the grammar refuses is the adapter's
// defect, named by the claim gate before any turn.
#[tokio::test]
async fn contract_route_malformed() {
    let mut provider = Provider::idle();
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("greeting.behaviour", "GET /greeting returns the static string 'hello'."),
            claim(ClaimKind::Contract, "greeting", ("route", "FETCH greeting")),
        ])),
    );

    let envelope =
        fail(&provider, &["emery", "specify", &reference("docs")], 3, "server_error").await;
    assert_message(
        &envelope,
        "- claim 1: route \"FETCH greeting\" names the method `FETCH`, which is none of GET, \
         POST, PUT, PATCH, DELETE, HEAD, OPTIONS",
    );
}

// Requirements under two stems reached through one route are one part: the
// one not under the contract's id binds where it anchors within the
// contract's lines, the baseline lists the trigger under each stem, and a
// slicing that puts the two apart is refused and corrected.
#[tokio::test]
async fn split_trigger() {
    let split = r#"{"preamble": [], "parts": [
        {"name": "health", "requirements": ["REQ-001"]},
        {"name": "start", "requirements": ["REQ-002"]}
    ]}"#;
    let merged = r#"{"preamble": [], "parts": [
        {"name": "health", "requirements": ["REQ-001", "REQ-002"]}
    ]}"#;
    let subjects = ["health.ready".to_string(), "start.ready-probe".to_string()];
    let spec = spec_draft(&subjects, &["Readiness, stated twice."]);
    let design = design_with_apis();
    let grouping = separate_grouping(2);
    let mut provider =
        Provider::answering([grouping.as_str(), spec.as_str(), design.as_str(), split, merged]);
    let mut contract = claim(ClaimKind::Contract, "health", ("route", "GET /health/ready"));
    contract.path = Some("src/health.ts#L2-L8".to_string());
    let mut probe = requirement("start.ready-probe", "The process reports ready once it listens.");
    probe.path = Some("src/health.ts#L5".to_string());
    provider.source.evidence.insert(
        "docs".to_string(),
        Ok(evidence(vec![
            requirement("health.ready", "GET /health/ready answers 200 once the process listens."),
            probe,
            contract,
        ])),
    );

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let slicing = &provider.model.seen()[3];
    let request = slicing.messages.join("\n");
    assert!(
        request.contains("- `health` — REQ-001 — triggers: `GET /health/ready`\n"),
        "{request}"
    );
    assert!(request.contains("- `start` — REQ-002 — triggers: `GET /health/ready`\n"), "{request}");
    assert!(
        request.contains("Requirements reached through one trigger are one handler"),
        "{request}"
    );
    let check = &provider.model.exchanges()[3];
    assert_eq!(check.tool, "check");
    let correction = check.outcome.as_ref().expect_err("the split slicing is rejected");
    assert!(
        correction.contains(
            "requirements reached through `GET /health/ready` (REQ-001, REQ-002) are split \
             across `health`, `start`: one trigger is one handler, built by one part"
        ),
        "{correction}"
    );
    let spec = shown(&provider, "spec").await;
    assert_eq!(spec.matches("Route: GET /health/ready\n").count(), 2, "{spec}");
    let plan = shown(&provider, "plan").await;
    assert!(plan.contains("Requirements: [REQ-001, REQ-002]\n"), "{plan}");
    provider.model.assert_exhausted();
}

// A relative route registered under two non-`start` stems does not merge the
// stems into one part, while requirements of one stem sharing that route are
// still one handler and cannot be split across parts.
#[tokio::test]
async fn trigger_scoped_to_stem() {
    let mut claims = vec![
        requirement("customers.get", "GET / lists the customers."),
        requirement("orders.get", "GET / reads the orders."),
        requirement("orders.list", "GET / lists the orders."),
        requirement("orders.post", "POST / creates an order."),
    ];
    let subjects: Vec<String> = claims.iter().filter_map(|claim| claim.id.clone()).collect();
    claims.push(claim(ClaimKind::Contract, "customers", ("route", "GET /")));
    claims.push(claim(ClaimKind::Contract, "orders.get", ("route", "GET /")));
    claims.push(claim(ClaimKind::Contract, "orders.list", ("route", "GET /")));

    let grouping = separate_grouping(subjects.len());
    let draft = spec_draft(&subjects, &["Customers and orders mounted at relative roots."]);
    let design = design_with_apis();
    let split = separate_slicing(&[
        ("customers", &["REQ-001"]),
        ("orders-main", &["REQ-002", "REQ-004"]),
        ("orders-list", &["REQ-003"]),
    ]);
    let merged = separate_slicing(&[
        ("customers", &["REQ-001"]),
        ("orders", &["REQ-002", "REQ-003", "REQ-004"]),
    ]);
    let mut provider = Provider::answering([
        grouping.as_str(),
        draft.as_str(),
        design.as_str(),
        split.as_str(),
        merged.as_str(),
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(claims)));

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let check = &provider.model.exchanges()[3];
    assert_eq!(check.tool, "check");
    let correction = check.outcome.as_ref().expect_err("the intra-stem trigger split is rejected");
    assert!(
        correction.contains(
            "requirements reached through `GET /` (REQ-002, REQ-003) are split across \
             `orders-list`, `orders-main`: one trigger is one handler, built by one part"
        ),
        "{correction}"
    );
    provider.model.assert_exhausted();
}

// --- slicing ---

// Three stems and two types: the model merges two stems, the engine refuses
// the draft that leaves a requirement out, then numbers the corrected parts by
// their lowest requirement and writes every list in canonical order — the
// requirements the answer listed backwards in id order, the types in key
// order. The slicing follows the design and runs from the bases, so its
// request carries the requirement outline and the design's data type keys,
// never a rendered document.
#[tokio::test]
async fn sliced() {
    let refused = r#"{"preamble": [], "parts": [
        {"name": "authentication", "requirements": ["REQ-001", "REQ-002"]},
        {"name": "orders", "requirements": ["REQ-003"], "types": ["orders.order", "orders.line"], "uses": []}
    ]}"#;
    let grouping = separate_grouping(4);
    let mut provider = Provider::answering([
        grouping.as_str(),
        SLICED_SPEC,
        SLICED_DESIGN,
        refused,
        SLICING_ANSWER,
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(sliced_evidence()));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    // the envelope carries the committed plan's waves
    let committed: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        committed["waves"],
        serde_json::json!([["PART-001", "PART-002"]]),
        "two parts sharing no type are one wave: {committed}"
    );

    // the slicing request carries the stems, the keys, and the requirement outline
    let slicing = &provider.model.seen()[3];
    let SeenFormat::Schema { name, schema } = &slicing.format else {
        panic!("the slicing is steered by schema");
    };
    assert_eq!(name, "slicing");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["parts"]["minItems"], 1);
    assert_eq!(schema["properties"]["parts"]["maxItems"], 3, "one part per stem at most");
    let draft = &schema["$defs"]["Draft"]["properties"];
    assert_eq!(draft["requirements"]["minItems"], 1);
    assert_eq!(
        draft["requirements"]["items"]["enum"],
        serde_json::json!(["REQ-001", "REQ-002", "REQ-003", "REQ-004"]),
        "the run's requirement ids ride the schema as a hint"
    );
    assert_eq!(
        draft["types"]["items"]["enum"],
        serde_json::json!(["orders.line", "orders.order"]),
        "the design's type keys ride the schema in key order"
    );
    assert_eq!(
        draft["uses"]["items"]["enum"],
        serde_json::json!(["orders.line", "orders.order"]),
        "the design's type keys ride `uses` in key order"
    );
    let request = slicing.messages.join("\n");
    assert!(request.contains("- `auth` — REQ-001\n"), "{request}");
    assert!(request.contains("- `orders` — REQ-003, REQ-004\n"), "{request}");
    assert!(request.contains("- `orders.line`\n- `orders.order`\n"), "the keys ride: {request}");
    assert!(
        request.contains("- REQ-004 `orders.cancel` — Status: unknown"),
        "the requirement outline rides: {request}"
    );
    assert!(!request.contains("### Requirement:"), "no rendered spec rides: {request}");
    assert!(!request.contains("## Domain model"), "no rendered design rides: {request}");

    // the design request carries the same outline in place of a rendered spec
    let design = provider.model.seen()[2].messages.join("\n");
    assert!(design.contains("- REQ-004 `orders.cancel` — Status: unknown"), "{design}");
    assert!(!design.contains("### Requirement:"), "no rendered spec rides: {design}");

    // the refusal is the correction; the corrected slicing commits
    let check = &provider.model.exchanges()[3];
    assert_eq!(check.tool, "check");
    let correction = check.outcome.as_ref().expect_err("the first slicing is rejected");
    assert!(correction.contains("`REQ-004` is in no part"), "{correction}");
    let id = staged(&provider.storage);
    assert_eq!(
        String::from_utf8_lossy(&staged_document(&provider.storage, &id, "plan.json")),
        SLICED_REVISION,
        "plan.json is the canonical revision"
    );
    assert_eq!(
        shown(&provider, "plan").await,
        projection(SLICED_RENDERED, &id),
        "show renders plan.md"
    );
    let resp = cli_ok(&provider, &["emery", "--format", "json", "show", "plan", "--staged"]).await;
    let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    let document: Value =
        serde_json::from_str(SLICED_REVISION).expect("the revision fixture is JSON");
    assert_eq!(envelope["document"], document, "the envelope carries the typed plan");
    provider.model.assert_exhausted();
}

// A large stem of 20 requirements is one floor: the brief lists the stem
// whole, the schema admits one part per stem, a draft that splits the stem
// along its sub-stems is refused, and the draft keeping all 20 together
// commits with no cap finding.
#[tokio::test]
async fn large_stem_whole() {
    let verbs: Vec<(&str, usize)> = vec![("get", 8), ("post", 8), ("delete", 4)];
    let mut claims = Vec::new();
    for (verb, count) in &verbs {
        for index in 0..*count {
            let statement = format!("`{verb}` does thing {index}.");
            claims.push(requirement(&format!("orders.{verb}.thing-{index}"), &statement));
        }
    }
    claims.push(requirement("auth.login", "Users sign in with a credential."));
    let subjects: Vec<String> = claims.iter().filter_map(|claim| claim.id.clone()).collect();
    let numbered: Vec<String> = (1..=claims.len()).map(|n| format!("REQ-{n:03}")).collect();
    let ids: Vec<&str> = numbered.iter().map(String::as_str).collect();
    let (orders, auth) = ids.split_at(20);
    let (get, writes) = orders.split_at(8);
    let grouping = separate_grouping(claims.len());
    let draft = spec_draft(&subjects, &["Orders in three verbs, and sign-in."]);
    let refused = separate_slicing(&[
        ("orders-reading", get),
        ("orders-writing", writes),
        ("authentication", auth),
    ]);
    let accepted = separate_slicing(&[("orders", orders), ("authentication", auth)]);
    let mut provider = Provider::answering([
        grouping.as_str(),
        draft.as_str(),
        DESIGN_ANSWER,
        refused.as_str(),
        accepted.as_str(),
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(claims)));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    // the brief lists the 20-requirement stem as one floor and admits one part per stem
    let slicing = &provider.model.seen()[3];
    let SeenFormat::Schema { name, schema } = &slicing.format else {
        panic!("the slicing is steered by schema");
    };
    assert_eq!(name, "slicing");
    let schema: Value = serde_json::from_str(schema).expect("the steering schema is JSON");
    assert_eq!(schema["properties"]["parts"]["maxItems"], 2, "two stems, one floor each");
    let request = slicing.messages.join("\n");
    let baseline = format!("- `orders` — {}\n- `auth` — {}\n", orders.join(", "), auth.join(", "));
    assert!(request.contains(&baseline), "{request}");
    assert!(!request.contains("sub-stem"), "{request}");

    // splitting the stem along its sub-stems is refused; keeping it whole commits
    let check = &provider.model.exchanges()[3];
    assert_eq!(check.tool, "check");
    let correction = check.outcome.as_ref().expect_err("splitting the stem is rejected");
    assert!(
        correction.contains(
            "requirements sharing the stem `orders` are split across `orders-reading`, \
             `orders-writing`"
        ),
        "{correction}"
    );
    let committed: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        committed["waves"],
        serde_json::json!([["PART-001", "PART-002"]]),
        "two parts depending on nothing are one wave: {committed}"
    );
    let id = staged(&provider.storage);
    let plan: Value = serde_json::from_slice(&staged_document(&provider.storage, &id, "plan.json"))
        .expect("the committed plan is JSON");
    let parts: Vec<(&str, &str)> = plan["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .map(|part| (part["id"].as_str().expect("id"), part["name"].as_str().expect("name")))
        .collect();
    assert_eq!(parts, vec![("PART-001", "orders"), ("PART-002", "authentication")]);
    assert_eq!(plan["parts"][0]["requirements"], serde_json::json!(orders));
    provider.model.assert_exhausted();
}

// The slicing leg is gated as the drafts are, one finding per case.
#[tokio::test]
async fn invalid_plan() {
    let plan = |parts: &str| format!(r#"{{"preamble": [], "parts": [{parts}]}}"#);
    let all = |extra: &str| {
        plan(&format!(
            r#"{{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]{extra}}}"#
        ))
    };
    let cases: Vec<(String, &str)> = vec![
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002", "REQ-003"]}, {"name": "orders", "requirements": ["REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "requirements sharing the stem `orders` are split across `authentication`, `orders`",
        ),
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002"]}, {"name": "orders", "requirements": ["REQ-003"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "`REQ-004` is in no part",
        ),
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002", "REQ-003"]}, {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "`REQ-003` appears in more than one part",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004", "REQ-009"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "part `all`: `REQ-009` is not a requirement",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}, {"name": "empty", "requirements": []}"#,
            ),
            "part `empty` has no requirement",
        ),
        (
            plan(
                r#"{"name": "All Parts", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "part name `All Parts` is not kebab-case",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002"]}, {"name": "all", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "part `all` is drafted more than once",
        ),
        (
            all(r#", "uses": ["orders.receipt"]"#),
            "part `all` uses `orders.receipt`, which is not a type in the design",
        ),
        (all(r#", "uses": ["orders.order"]"#), "part `all` uses `orders.order`, which it owns"),
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "types": ["orders.line"], "uses": ["orders.order"]}, {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order"], "uses": ["orders.line"]}"#,
            ),
            "parts `authentication`, `orders` cannot be ordered: the edges derived from `orders.line`, `orders.order` form a cycle",
        ),
        (
            plan(
                r#"{"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "types": ["orders.order"]}, {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}"#,
            ),
            "type `orders.order` is owned by 2 parts: `authentication`, `orders`",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.line"]}"#,
            ),
            "type `orders.order` is owned by no part",
        ),
        (
            plan(
                r#"{"name": "all", "requirements": ["REQ-001", "REQ-002", "REQ-003", "REQ-004"], "types": ["orders.order", "orders.line", "orders.receipt"]}"#,
            ),
            "`orders.receipt` is not a type in the design",
        ),
        (
            all(r#", "brief": ["Depends on: nothing"]"#),
            "part `all` brief: a paragraph line opens with the reserved marker `Depends on:`",
        ),
        ("not json".to_string(), "schema and answer type disagree"),
    ];
    for (answer, fragment) in cases {
        let grouping = separate_grouping(4);
        let mut provider = Provider::answering([
            grouping.as_str(),
            SLICED_SPEC,
            SLICED_DESIGN,
            answer.as_str(),
            answer.as_str(),
            answer.as_str(),
        ]);
        provider.source.evidence.insert("docs".to_string(), Ok(sliced_evidence()));
        let envelope =
            fail(&provider, &["emery", "specify", &reference("docs")], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        provider.model.assert_exhausted();
    }
}

// A `uses` entry naming a key absent from the design is refused with the key named.
#[tokio::test]
async fn uses_not_a_type() {
    let refused = r#"{"preamble": [], "parts": [
        {"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "uses": ["orders.receipt"]},
        {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}
    ]}"#;
    let grouping = separate_grouping(4);
    let mut provider = Provider::answering([
        grouping.as_str(),
        SLICED_SPEC,
        SLICED_DESIGN,
        refused,
        SLICING_ANSWER,
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(sliced_evidence()));

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let check = &provider.model.exchanges()[3];
    let correction = check.outcome.as_ref().expect_err("unknown `uses` key is rejected");
    assert!(
        correction.contains(
            "part `authentication` uses `orders.receipt`, which is not a type in the design"
        ),
        "{correction}"
    );
    provider.model.assert_exhausted();
}

// A `uses` entry naming a key the same part owns, or a key no part owns, is refused.
#[tokio::test]
async fn uses_owned() {
    let self_owned = r#"{"preamble": [], "parts": [
        {"name": "authentication", "requirements": ["REQ-001", "REQ-002"]},
        {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"], "uses": ["orders.order"]}
    ]}"#;
    let unowned = r#"{"preamble": [], "parts": [
        {"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "uses": ["orders.order"]},
        {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.line"], "brief": ["Uses: nothing"]}
    ]}"#;
    let grouping = separate_grouping(4);
    let mut provider = Provider::answering([
        grouping.as_str(),
        SLICED_SPEC,
        SLICED_DESIGN,
        self_owned,
        unowned,
        SLICING_ANSWER,
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(sliced_evidence()));

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let first = &provider.model.exchanges()[3];
    let correction = first.outcome.as_ref().expect_err("self-owned `uses` key is rejected");
    assert!(
        correction.contains("part `orders` uses `orders.order`, which it owns"),
        "{correction}"
    );
    let second = &provider.model.exchanges()[4];
    let correction = second.outcome.as_ref().expect_err("unowned `uses` key is rejected");
    assert!(
        correction.contains("part `authentication` uses `orders.order`, which is owned by no part"),
        "{correction}"
    );
    assert!(
        correction.contains(
            "part `orders` brief: a paragraph line opens with the reserved marker `Uses:`"
        ),
        "{correction}"
    );
    provider.model.assert_exhausted();
}

// Two parts whose owned types name each other in their signatures form a cycle:
// the gate quotes both keys and asks that they be owned by one part.
#[tokio::test]
async fn edges_cyclic() {
    let mut claims = sliced_evidence().claims;
    claims[4] =
        claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { line: Line }"));
    claims[5] =
        claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { order: Order }"));
    let refused = r#"{"preamble": [], "parts": [
        {"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "types": ["orders.line"]},
        {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order"]}
    ]}"#;
    let grouping = separate_grouping(4);
    let mut provider = Provider::answering([
        grouping.as_str(),
        SLICED_SPEC,
        SLICED_DESIGN,
        refused,
        SLICING_ANSWER,
    ]);
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(claims)));

    cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;

    let check = &provider.model.exchanges()[3];
    let correction = check.outcome.as_ref().expect_err("mutual type references across parts cycle");
    assert!(
        correction.contains(
            "parts `authentication`, `orders` cannot be ordered: the edges derived from \
             `orders.line`, `orders.order` form a cycle; own types that name each other in one \
             part"
        ),
        "{correction}"
    );
    provider.model.assert_exhausted();
}

// A part owning a type whose signature names another part's declared type
// derives a dependency onto that owner with no `uses` answered, and the brief
// lists the cross-type reference under `## Type keys`.
#[tokio::test]
async fn edges_derived() {
    let mut claims = sliced_evidence().claims;
    claims[4] = claim(
        ClaimKind::Type,
        "orders.order",
        ("signature", "interface Order { id: string; credential: Credential; lines: Line[] }"),
    );
    claims.push(claim(
        ClaimKind::Type,
        "auth.credential",
        ("signature", "interface Credential { token: string }"),
    ));
    let design = r#"{"preamble": [], "sections": [
        {"kind": "overview", "blocks": [{"text": "Sign-in issues a session; orders are placed and cancelled under it."}]},
        {"kind": "domain-model", "blocks": [
            {"type": {"key": "auth.credential", "role": "data"}},
            {"type": {"key": "orders.order", "role": "data"}},
            {"type": {"key": "orders.line", "role": "data"}}
        ]}
    ]}"#;
    let slicing = r#"{"preamble": [], "parts": [
        {"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "types": ["auth.credential"]},
        {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "types": ["orders.order", "orders.line"]}
    ]}"#;
    let grouping = separate_grouping(4);
    let mut provider = Provider::answering([grouping.as_str(), SLICED_SPEC, design, slicing]);
    provider.source.evidence.insert("docs".to_string(), Ok(evidence(claims)));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    let committed: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(
        committed["waves"],
        serde_json::json!([["PART-001"], ["PART-002"]]),
        "the derived edge serialises `orders` after `authentication`: {committed}"
    );
    let request = provider.model.seen()[3].messages.join("\n");
    assert!(
        request.contains("- `orders.order` — names `auth.credential`, `orders.line`\n"),
        "the brief lists the referenced keys: {request}"
    );
    let id = staged(&provider.storage);
    let plan: Value = serde_json::from_slice(&staged_document(&provider.storage, &id, "plan.json"))
        .expect("the committed plan is JSON");
    assert_eq!(plan["parts"][1]["uses"], serde_json::json!([]));
    assert_eq!(plan["parts"][1]["depends-on"], serde_json::json!(["PART-001"]));
    let rendered = shown(&provider, "plan").await;
    assert!(
        rendered.contains("Types: [orders.line, orders.order]\nDepends on: [PART-001]"),
        "{rendered}"
    );
    provider.model.assert_exhausted();
}

// A part owning no type that lists another part's keys in `uses` stores `uses`
// in design order, derives the dependency onto their owner, and renders `Uses:`
// before `Depends on:`.
#[tokio::test]
async fn edges_used() {
    let slicing = r#"{"preamble": [], "parts": [
        {"name": "authentication", "requirements": ["REQ-001", "REQ-002"], "types": ["orders.order", "orders.line"]},
        {"name": "orders", "requirements": ["REQ-003", "REQ-004"], "uses": ["orders.order", "orders.line"]}
    ]}"#;
    let grouping = separate_grouping(4);
    let mut provider =
        Provider::answering([grouping.as_str(), SLICED_SPEC, SLICED_DESIGN, slicing]);
    provider.source.evidence.insert("docs".to_string(), Ok(sliced_evidence()));

    let resp =
        cli_ok(&provider, &["emery", "--format", "json", "specify", &reference("docs")]).await;

    let committed: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
    assert_eq!(committed["waves"], serde_json::json!([["PART-001"], ["PART-002"]]), "{committed}");
    let id = staged(&provider.storage);
    let plan: Value = serde_json::from_slice(&staged_document(&provider.storage, &id, "plan.json"))
        .expect("the committed plan is JSON");
    assert_eq!(
        plan["parts"][1]["uses"],
        serde_json::json!(["orders.line", "orders.order"]),
        "`uses` is stored in design order"
    );
    assert_eq!(plan["parts"][1]["depends-on"], serde_json::json!(["PART-001"]));
    let rendered = shown(&provider, "plan").await;
    assert!(
        rendered.contains(
            "ID: PART-002\nRequirements: [REQ-003, REQ-004]\nUses: [orders.line, \
             orders.order]\nDepends on: [PART-001]"
        ),
        "{rendered}"
    );
    provider.model.assert_exhausted();
}

// Sign-in, its session, and two order behaviours over two `type` claims: three
// stems, so the slicing is the model's.
fn sliced_evidence() -> Evidence {
    evidence(vec![
        requirement("auth.login", "Users sign in with a credential."),
        requirement("session.timeout", "Sessions expire after an hour of inactivity."),
        requirement("orders.create", "A signed-in user places an order."),
        requirement("orders.cancel", "A signed-in user cancels an open order."),
        claim(ClaimKind::Type, "orders.order", ("signature", "interface Order { id: string }")),
        claim(ClaimKind::Type, "orders.line", ("signature", "interface Line { sku: string }")),
    ])
}

const SLICED_SPEC: &str = r#"{"preamble": ["Sign-in, its session, and orders over both."], "requirements": [
    {"subject": "auth.login", "scenarios": [{"name": "Login", "when": "a valid credential is presented", "then": "the caller is signed in"}]},
    {"subject": "session.timeout", "scenarios": [{"name": "Timeout", "when": "a session is idle for an hour", "then": "it times out"}]},
    {"subject": "orders.create", "scenarios": [{"name": "Create", "when": "a signed-in caller places an order", "then": "the order is created"}]},
    {"subject": "orders.cancel", "scenarios": [{"name": "Cancel", "when": "a signed-in caller cancels an order", "then": "the order is cancelled"}]}
]}"#;

const SLICED_DESIGN: &str = r#"{"preamble": [], "sections": [
    {"kind": "overview", "blocks": [{"text": "Sign-in issues a session; orders are placed and cancelled under it."}]},
    {"kind": "domain-model", "blocks": [
        {"type": {"key": "orders.order", "role": "data"}},
        {"type": {"key": "orders.line", "role": "data"}}
    ]}
]}"#;

// --- config file ---

// The list is checked whole before a single adapter loads.
#[tokio::test]
async fn config_file() {
    let cases: &[(&str, u8, &str, &str)] = &[
        ("not toml [", 1, "bad_request", "TOML parse error"),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\nbranch = \
             \"main\"\n",
            1,
            "bad_request",
            "unknown field `branch`",
        ),
        ("", 1, "specify-source-required", ""),
        (
            "[sources.docs]\nadapter = \"emery:documentation@1.2.0\"\n",
            1,
            "bad_request",
            "unknown field `sources`",
        ),
        (
            "[[source]]\nname = \"intent\"\nadapter = \"emery:intent@1.0.0\"\nvalue = \"text\"\n",
            1,
            "bad_request",
            "unknown field `value`",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
             \"docs\"\ndescription = \"text\"\n",
            1,
            "bad_request",
            "both `path` and `description`",
        ),
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
             [[source]]\nname = \"docs\"\nadapter = \"emery:intent@1.0.0\"\n",
            1,
            "bad_request",
            "appears twice",
        ),
        (
            "[[source]]\nname = \"Docs\"\nadapter = \"emery:documentation@1.2.0\"\n",
            1,
            "bad_request",
            "is not a kebab-case name",
        ),
        (
            "[[source]]\nname = \"local\"\nadapter = \"acme:source@1.0.0\"\n\
             digest = \"sha256:9f2c44aa\"\n",
            1,
            "bad_request",
            "is not 64 hex characters",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\ngit = \
             \"https://github.com/acme/api@v2\"\n",
            1,
            "bad_request",
            "unknown field `git`",
        ),
        (
            "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\nurl = \
             \"https://example.com/openapi.yaml\"\n",
            1,
            "bad_request",
            "unknown field `url`",
        ),
        (
            "[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\nregistry = \
             \"registry.acme.io\"\n",
            1,
            "bad_request",
            "unknown field `registry`",
        ),
        // `path` is relative to the file, which the scratch holds two
        // directories below the project root
        (
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
             \"../../../outside\"\n",
            1,
            "bad_request",
            "../../../outside",
        ),
    ];
    for (body, exit, code, fragment) in cases {
        let scratch = Scratch::new();
        let config = scratch.config(body);
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], *exit, code).await;
        if !fragment.is_empty() {
            assert_message(&envelope, fragment);
        }
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a refused list gates nothing: {body}"
        );
    }

    // an unreadable file is a filesystem error
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--config", "nonexistent/emery.toml"], 3, "server_error")
        .await;

    // host-absolute and escaping paths never reach the guest
    for path in ["/nonexistent/emery.toml", "../emery.toml"] {
        fail(&provider, &["emery", "specify", "--config", path], 1, "bad_request").await;
    }
}

// A file's `adapter` is an exact package reference, refused at the field
// otherwise: no path, no bare name, no unversioned or unnamespaced package.
#[tokio::test]
async fn config_reference() {
    let cases: &[(&str, &str)] = &[
        ("./source.wasm", "adapter `./source.wasm` names no version"),
        ("/tmp/source.wasm", "adapter `/tmp/source.wasm` names no version"),
        ("documentation", "adapter `documentation` names no version"),
        ("documentation@1.2.0", "adapter `documentation@1.2.0` names no namespace"),
        ("emery:documentation", "adapter `emery:documentation` names no version"),
        (
            "emery:documentation@latest",
            "adapter `emery:documentation@latest` has an invalid version `latest`",
        ),
    ];
    for (adapter, fragment) in cases {
        let scratch = Scratch::new();
        let config =
            scratch.config(&format!("[[source]]\nname = \"docs\"\nadapter = \"{adapter}\"\n"));
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a refused reference loads nothing: {adapter}"
        );
    }
}

// A file's `rank` is a positive integer, refused at the field otherwise.
#[tokio::test]
async fn config_rank() {
    let cases: &[(&str, &str)] = &[
        ("0", "invalid value: integer `0`, expected a nonzero u32"),
        ("-1", "invalid value: integer `-1`, expected a nonzero u32"),
        ("1.5", "invalid type: floating point `1.5`, expected a nonzero u32"),
        ("\"high\"", "invalid type: string \"high\", expected a nonzero u32"),
    ];
    for (rank, fragment) in cases {
        let scratch = Scratch::new();
        let config = scratch.config(&format!(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\nrank = {rank}\n"
        ));
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a refused rank loads nothing: {rank}"
        );
    }
}

// A file's `scope` is `delta` or `whole`, `delta` when omitted, refused at
// the field otherwise. A specify reads nothing of it, so acceptance is the
// one observable here: a stated or omitted scope runs as before. What an
// amend retires by it is `tests/amend.rs`'s.
#[tokio::test]
async fn config_scope() {
    for scope in ["scope = \"delta\"\n", "scope = \"whole\"\n", ""] {
        let scratch = Scratch::new();
        let config = scratch.config(&format!(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n{scope}"
        ));
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        cli_ok(&provider, &["emery", "specify", "--config", &config]).await;
        provider.model.assert_exhausted();
    }

    let cases: &[(&str, &str)] = &[
        ("\"partial\"", "unknown variant `partial`, expected `delta` or `whole`"),
        ("1", "wanted string or table"),
    ];
    for (scope, fragment) in cases {
        let scratch = Scratch::new();
        let config = scratch.config(&format!(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\nscope = \
             {scope}\n"
        ));
        let provider = Provider::idle();
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
        assert_message(&envelope, fragment);
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a refused scope loads nothing: {scope}"
        );
    }
}

// Path anchoring, inline descriptions, and declaration order, all observed on
// the `SourceInput` the adapter receives.
#[tokio::test]
async fn source_paths() {
    let scratch = Scratch::new();
    let config = scratch.config(
        "[[source]]\nname = \"zulu\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
         \"nested/../docs\"\n\n\
         [[source]]\nname = \"intent\"\nadapter = \"emery:intent@1.0.0\"\ndescription = \"Ship \
         it.\"\n\n\
         [[source]]\nname = \"alpha\"\nadapter = \"acme:local@1.0.0\"\npath = \"./docs\"\n",
    );

    // three sources contribute one id, so a grouping turn is scripted
    let grouping = baseline_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    let calls = provider.source.calls();
    let order: Vec<&str> = calls.iter().map(|(_, input)| input.name.as_str()).collect();
    assert_eq!(order, ["zulu", "intent", "alpha"], "entries extract in declaration order");
    for name in ["zulu", "alpha"] {
        let (_, input) = calls.iter().find(|(_, input)| input.name == name).expect("dispatched");
        let SourceContent::Workspace(root) = &input.content else {
            panic!("a path source lends a workspace");
        };
        assert!(
            !std::path::Path::new(root).is_absolute(),
            "the lend must stay `.`-relative for the guest preopen: {root}"
        );
        assert!(
            root.ends_with("docs") && !root.contains(".."),
            "`.` and `..` fold away lexically against the file's directory: {root}"
        );
    }
    let (_, input) = calls.iter().find(|(_, input)| input.name == "intent").expect("dispatched");
    assert_eq!(
        input.content,
        SourceContent::Value("Ship it.".to_string()),
        "a file `description` entry supplies inline text"
    );
    drop(calls);
    provider.model.assert_exhausted();
}

#[tokio::test]
async fn name_defaulted() {
    let scratch = Scratch::new();
    let config = scratch.config(&format!(
        "[[source]]\nadapter = \"emery:documentation@1.2.0\"\n\n\
         [[source]]\nadapter = \"emery:demo@1.2.0\"\n\n\
         [[source]]\nadapter = \"{SOURCE}\"\n",
    ));
    let grouping = baseline_grouping(3);
    let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

    cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

    assert!(
        shown(&provider, "spec").await.contains(
            "Sources: [documentation:greeting.behaviour, demo:greeting.behaviour, \
             source:greeting.behaviour]"
        ),
        "each entry is named for its adapter's package"
    );
    provider.model.assert_exhausted();
}

// --- repository sources ---

mod repository {
    use omnia_sdk::vcs::Error;

    use super::*;

    const URL: &str = "https://example.com/acme/shop.git";
    // `URL` normalised and hashed, the clone every run of it shares.
    const CLONE: &str = "./.emery/vcs/repos/f4c82940a714c3c5";
    const CHECKOUT: &str = "./.emery/vcs/sources/upstream";

    fn upstream(scratch: &Scratch, body: &str) -> String {
        scratch.config(&format!(
            "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"{URL}\"\n{body}"
        ))
    }

    // A source naming a repository is read in a working copy of it at the
    // revision: the clone made or fetched after the adapters load, the copy cut
    // and lent, at the `path` within it, and removed once the source answered.
    #[tokio::test]
    async fn repository_source() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"v1.2.0\"\npath = \"docs/api\"\n");
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        let resp = cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(
            stdout.contains(&format!("\n  upstream read from {URL} at v1.2.0: v1.2.0-commit\n")),
            "{stdout}"
        );
        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} v1.2.0"),
                format!("add {CLONE} {CHECKOUT} v1.2.0-commit"),
                format!("remove {CHECKOUT}"),
            ]
        );
        let calls = provider.source.calls();
        let (_, input) = calls.first().expect("one extract dispatch");
        assert_eq!(input.name, "upstream");
        assert_eq!(
            input.content,
            SourceContent::Workspace(format!("{CHECKOUT}/docs/api")),
            "the lend is the path within the working copy"
        );
        assert_eq!(
            provider.source.metadata.lock().expect("metadata").len(),
            1,
            "the adapter loads before the repository is touched"
        );
        drop(calls);
        provider.model.assert_exhausted();

        // the JSON envelope names what was read, and a path omitted lends the copy whole
        let config = upstream(&scratch, "revision = \"main\"\n");
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        provider.vcs.clones.script(CLONE, Err(Error::Exists(CLONE.to_owned())));
        let resp =
            cli_ok(&provider, &["emery", "--format", "json", "specify", "--config", &config]).await;
        let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
        assert_eq!(
            envelope["repositories"],
            serde_json::json!([
                {"source": "upstream", "repository": URL, "revision": "main", "commit": "main-commit"}
            ]),
            "{envelope}"
        );
        assert_eq!(
            provider.vcs.calls()[..2],
            [format!("clone {URL} {CLONE}"), format!("fetch {CLONE} origin")],
            "found, so fetched"
        );
        let calls = provider.source.calls();
        assert_eq!(calls[0].1.content, SourceContent::Workspace(CHECKOUT.to_owned()));
        drop(calls);
        provider.model.assert_exhausted();
    }

    // Two sources naming one repository, however they spell its URL, share
    // one clone made once and are read in working copies of their own.
    #[tokio::test]
    async fn repo_shared_clone() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"emery:documentation@1.2.0\"\n\
             repository = \"HTTPS://Example.com/acme/shop/\"\nrevision = \"v2\"\npath = \"api\"\n",
        );
        let grouping = baseline_grouping(2);
        let provider = Provider::answering([grouping.as_str(), SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", "--config", &config]).await;

        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} main"),
                format!("add {CLONE} ./.emery/vcs/sources/docs main-commit"),
                format!("resolve {CLONE} v2"),
                format!("add {CLONE} ./.emery/vcs/sources/api v2-commit"),
                "remove ./.emery/vcs/sources/docs".to_owned(),
                "remove ./.emery/vcs/sources/api".to_owned(),
            ]
        );
        let roots: Vec<SourceContent> =
            provider.source.calls().into_iter().map(|(_, input)| input.content).collect();
        assert_eq!(
            roots,
            [
                SourceContent::Workspace("./.emery/vcs/sources/docs".to_owned()),
                SourceContent::Workspace("./.emery/vcs/sources/api/api".to_owned()),
            ]
        );
        provider.model.assert_exhausted();
    }

    // The working copy goes whether or not the source answered; a stale one
    // from a failed run is removed before the copy is cut again.
    #[tokio::test]
    async fn repo_extract_error() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"main\"\n");
        let mut provider = Provider::idle();
        provider
            .source
            .evidence
            .insert("upstream".to_string(), Err(bad_gateway!("the adapter exploded")));
        provider.vcs.adds.script(CHECKOUT, Err(Error::Exists(CHECKOUT.to_owned())));

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 4, "bad_gateway").await;

        assert_eq!(envelope["message"], "the adapter exploded");
        assert_eq!(
            provider.vcs.calls(),
            [
                format!("clone {URL} {CLONE}"),
                format!("resolve {CLONE} main"),
                format!("add {CLONE} {CHECKOUT} main-commit"),
                format!("remove {CHECKOUT}"),
                format!("add {CLONE} {CHECKOUT} main-commit"),
                format!("remove {CHECKOUT}"),
            ]
        );
        provider.vcs.assert_exhausted();
    }

    // A revision the repository lacks, or a URL that names none, is
    // `revision-not-found` before any source is asked; a remote that cannot be
    // reached is the gateway's.
    #[tokio::test]
    async fn repo_missing_revision() {
        let scratch = Scratch::new();
        let config = upstream(&scratch, "revision = \"v9\"\n");
        let provider = Provider::idle();

        provider.vcs.resolves.script(CLONE, Err(Error::NotFound("v9".to_owned())));
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 2, "revision-not-found")
                .await;
        assert_message(&envelope, &format!("repository `{URL}` has no `v9`"));
        assert!(
            envelope["hint"].as_str().is_some_and(|hint| hint.contains("`revision`")),
            "{envelope}"
        );
        assert!(provider.source.calls().is_empty(), "no source is asked");

        provider.vcs.clones.script(CLONE, Err(Error::Access("authentication failed".to_owned())));
        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 4, "bad_gateway").await;
        assert_message(&envelope, &format!("repository `{URL}`: authentication failed"));
        provider.vcs.assert_exhausted();
    }

    // A repository is read at a revision and at a path within it, never
    // beside inline text or above the clone; the file is refused before any
    // adapter loads.
    #[tokio::test]
    async fn repository_config() {
        let cases: &[(&str, &str)] = &[
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\n",
                "source `upstream` sets `repository` without `revision`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 revision = \"main\"\n",
                "source `upstream` sets `revision` without `repository`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:intent@1.0.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\
                 description = \"Ship it.\"\n",
                "source `upstream` sets both `repository` and `description`",
            ),
            (
                "[[source]]\nname = \"upstream\"\nadapter = \"emery:documentation@1.2.0\"\n\
                 repository = \"https://example.com/acme/shop.git\"\nrevision = \"main\"\n\
                 path = \"../sibling\"\n",
                "source `upstream`: path `../sibling` must lie within the repository",
            ),
        ];
        for (body, fragment) in cases {
            let scratch = Scratch::new();
            let config = scratch.config(body);
            let provider = Provider::idle();
            let envelope =
                fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;
            assert_message(&envelope, fragment);
            assert!(
                provider.source.metadata.lock().expect("metadata").is_empty(),
                "a refused list gates nothing: {body}"
            );
            assert!(provider.vcs.calls().is_empty(), "nothing is asked of version control");
        }
    }
}

// --- adapter references ---

mod reference {
    use std::fs;

    use super::{
        DESIGN_ANSWER, Provider, SPEC_ANSWER, Scratch, assert_message, cli_ok, fail, guest, shown,
    };

    // The exact reference loads as the guest it names without its version,
    // and the source is named for the package.
    #[tokio::test]
    async fn exact_ref() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

        cli_ok(&provider, &["emery", "specify", "emery:demo@1.2.0"]).await;

        let gated = provider.source.metadata.lock().expect("metadata").clone();
        assert_eq!(gated, ["emery:demo"], "the package is gated without its version");
        let calls = provider.source.calls();
        let (id, input) = calls.first().expect("one extract dispatch");
        assert_eq!(id, "emery:demo", "the adapter id is the registered guest");
        assert_eq!(input.name, "demo", "the source name is the package name");
        drop(calls);
        provider.model.assert_exhausted();
    }

    // A reference parses whole or not at all: no default namespace, nothing
    // resolving to a latest, no path, no bare name, no source checkout.
    #[tokio::test]
    async fn malformed_ref() {
        let cases: &[(&str, &str)] = &[
            ("emery:demo", "adapter `emery:demo` names no version"),
            ("demo@1.2.0", "adapter `demo@1.2.0` names no namespace"),
            ("demo", "adapter `demo` names no version"),
            ("emery:demo@main", "adapter `emery:demo@main` has an invalid version `main`"),
            ("emery:@1.2.0", "adapter `emery:@1.2.0` is not `namespace:name@version`"),
            ("Emery:demo@1.2.0", "adapter `Emery:demo@1.2.0` is not `namespace:name@version`"),
            ("./demo.wasm", "adapter `./demo.wasm` names no version"),
            ("/tmp/demo.wasm", "adapter `/tmp/demo.wasm` names no version"),
            ("https://github.com/acme/api", "GitHub URLs are not supported"),
        ];
        for (reference, fragment) in cases {
            let provider = Provider::idle();
            let envelope =
                fail(&provider, &["emery", "specify", reference], 1, "adapter-reference").await;
            assert_message(&envelope, fragment);
            assert_eq!(
                envelope["hint"],
                "an adapter is an exact package reference, `namespace:name@version`",
                "{envelope}"
            );
            assert!(
                provider.source.metadata.lock().expect("metadata").is_empty(),
                "a malformed reference loads nothing: {reference}"
            );
        }
    }

    // Routing is the deployment's, so the file reserves no table for it.
    #[tokio::test]
    async fn registries_table() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"ledger\"\nadapter = \"acme:ledger@2.1.0\"\n\n\
             [registries]\nacme = \"registry.acme.io\"\n",
        );
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(&envelope, "unknown field `registries`");
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a file that does not parse loads nothing"
        );
    }

    // Two versions of one package would register as one guest, the second
    // load attesting the first's bytes; the list is refused before either
    // loads.
    #[tokio::test]
    async fn version_collision() {
        let scratch = Scratch::new();
        let config = scratch.config(
            "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"emery:documentation@1.3.0\"\n",
        );
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(
            &envelope,
            "adapters `emery:documentation@1.2.0` and `emery:documentation@1.3.0`",
        );
        assert_message(&envelope, "would both register as `emery:documentation`");
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a colliding list loads nothing"
        );

        // the same two on the command line are two sources of one package name
        let envelope = fail(
            &provider,
            &["emery", "specify", "emery:documentation@1.2.0", "emery:documentation@1.3.0"],
            1,
            "bad_request",
        )
        .await;
        assert_message(&envelope, "source `documentation` appears twice");
    }

    // An argv run reads no project-root file: one that does not parse, or
    // whose source is located outside its root, refuses the discovered run
    // alone. The CWD move is hermetic under nextest's process-per-test
    // isolation.
    #[tokio::test]
    async fn argv_with_bad_config() {
        let project = tempfile::TempDir::new().expect("project dir");
        std::env::set_current_dir(project.path()).expect("enter project");
        let argv = ["emery", "specify", "acme:ledger@2.1.0"];

        for (entry, refusal) in [
            (
                "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\nbranch = \
                 \"main\"\n",
                "unknown field `branch`",
            ),
            (
                "[[source]]\nname = \"docs\"\nadapter = \"emery:documentation@1.2.0\"\npath = \
                 \"../../outside\"\n",
                "must be relative to the project root",
            ),
            ("[[source]]\nname = \"local\"\nadapter = \"/tmp/source.wasm\"\n", "names no version"),
        ] {
            fs::write(project.path().join("emery.toml"), entry).expect("write emery.toml");
            let envelope = fail(&Provider::idle(), &["emery", "specify"], 1, "bad_request").await;
            assert_message(&envelope, refusal);
            let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);

            cli_ok(&provider, &argv).await;

            let gated = provider.source.metadata.lock().expect("metadata").clone();
            assert_eq!(gated, [guest("ledger")], "argv names the run's only source: {entry}");
            assert!(
                shown(&provider, "spec").await.contains("Sources: [ledger:greeting.behaviour]"),
                "the file's `[[source]]` entries stay out of an argv run"
            );
            provider.model.assert_exhausted();
        }
    }
}

mod digest {
    use super::{Provider, SOURCE, Scratch, assert_message, digest, fail};

    // One adapter loads once under one pin, so two entries pinning it differently
    // cannot both be honoured; the list is refused before either is asked for.
    #[tokio::test]
    async fn conflict() {
        let scratch = Scratch::new();
        let config = scratch.config(&format!(
            "[[source]]\nname = \"docs\"\nadapter = \"{SOURCE}\"\ndigest = \"{}\"\n\n\
             [[source]]\nname = \"api\"\nadapter = \"{SOURCE}\"\ndigest = \"{}\"\n",
            digest("cd"),
            digest("ab")
        ));
        let provider = Provider::idle();

        let envelope =
            fail(&provider, &["emery", "specify", "--config", &config], 1, "bad_request").await;

        assert_message(&envelope, &format!("adapter `{SOURCE}` is pinned to two digests"));
        assert!(
            provider.source.metadata.lock().expect("metadata").is_empty(),
            "a conflicting pin loads nothing"
        );
    }
}

mod store {
    use std::sync::Arc;

    use emery_engine::vcs::{REVISION, RevisionMessage, SPECIFICATION};
    use emery_engine::{EntryKind, REVISION_KEY, STAGED_CONTAINER, STAGED_KEY};
    use omnia_test::guest::{Memory, Namespaced};
    use serde_json::Value;

    use super::support::vcs::HEAD;
    use super::{
        DESIGN_ANSWER, PROJECT_CONFIG, Provider, SOURCE, SPEC_ANSWER, SPEC_RENDERED, SPEC_REVISION,
        assert_message, cli_ok, committed, fail, project_staged, projection, reference, seed,
        shown, shown_sealed, staged,
    };

    const SPEC: &[u8] = include_bytes!("build/spec.json");
    const DESIGN: &[u8] = include_bytes!("build/design.json");
    const PLAN: &[u8] = include_bytes!("build/plan.json");

    const LABELLED: &str = "e1f2a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4";

    // The pointer is a cache over the repository: bytes at it that name no
    // label the repository holds, or that decode to no id, are passed over
    // and the head's tree read, so `show` refuses typed where that holds no
    // revision, the next `specify` stages with the diff suppressed, and
    // `apply` swaps over them.
    #[tokio::test]
    async fn corrupt_current() {
        for bytes in [&b"0123456789abcdef"[..], &b"\xff\xfe"[..]] {
            let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
            provider.storage.insert_state(REVISION_KEY, bytes);
            fail(
                &provider,
                &["emery", "show", "spec", "--config", PROJECT_CONFIG],
                2,
                "spec-not-generated",
            )
            .await;

            let resp = cli_ok(
                &provider,
                &["emery", "specify", &reference("docs"), "--config", PROJECT_CONFIG],
            )
            .await;
            let stdout = String::from_utf8_lossy(&resp.stdout);
            assert!(!stdout.contains("diff vs"), "a corrupt pointer yields no diff: {stdout}");
            let id = staged(&provider.storage);
            assert!(
                provider.storage.object(STAGED_CONTAINER, &format!("{id}/spec.json")).is_some()
            );

            cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
            assert_eq!(committed(&provider.storage), id, "the seal swaps over the corruption");
            cli_ok(&provider, &["emery", "show", "spec", "--config", PROJECT_CONFIG]).await;
            provider.model.assert_exhausted();
        }
    }

    // Bytes at the staged key that name no candidate are corruption: `show
    // --staged` and `apply` fail closed, and the next `specify` stages over
    // them.
    #[tokio::test]
    async fn corrupt_staged() {
        for bytes in [&b"0123456789abcdef"[..], &b"\xff\xfe"[..]] {
            let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
            provider.storage.insert_state(STAGED_KEY, bytes);
            fail(&provider, &["emery", "show", "spec", "--staged"], 3, "server_error").await;
            fail(&provider, &["emery", "apply", "--config", PROJECT_CONFIG], 3, "server_error")
                .await;

            cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
            let id = staged(&provider.storage);
            assert_ne!(id.as_bytes(), bytes, "the candidate stands in the slot");
            cli_ok(&provider, &["emery", "show", "spec", "--staged"]).await;
            provider.model.assert_exhausted();
        }
    }

    // A document edited under the label no longer hashes to the revision the
    // label names, so every verb that reads the current revision refuses
    // typed, nothing is staged over it, and the hint names the restore;
    // once the tree holds the revision again, `show` renders it.
    #[tokio::test]
    async fn tampered_revision() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        cli_ok(
            &provider,
            &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
        )
        .await;
        let id = committed(&provider.storage);
        let sealed = super::committed_document(&provider, "spec");
        provider.files.holds(REVISION, &format!("{SPECIFICATION}/spec.json"), "{}\n");
        for _ in 0..3 {
            provider.vcs.labelleds.script(".", Ok(LABELLED.to_owned()));
        }

        let envelope = fail(
            &provider,
            &["emery", "show", "spec", "--config", PROJECT_CONFIG],
            1,
            "spec-mismatch",
        )
        .await;
        assert_message(
            &envelope,
            &format!("labelled `emery/{id}`, does not match revision `{id}`"),
        );
        assert!(
            envelope["hint"]
                .as_str()
                .unwrap_or("")
                .contains("git checkout emery/<revision> -- spec/"),
            "{envelope}"
        );
        fail(
            &provider,
            &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
            1,
            "spec-mismatch",
        )
        .await;
        assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged over it");

        provider.files.holds_bytes(REVISION, &format!("{SPECIFICATION}/spec.json"), &sealed);
        assert!(shown_sealed(&provider, "spec").await.contains("hello"), "show renders it again");
        provider.model.assert_exhausted();
    }

    // A `spec/` of the operator's own at the head, holding anything but the
    // three documents, is where the specification cannot live: every verb
    // over the current revision refuses typed and nothing is sealed there.
    #[tokio::test]
    async fn spec_occupied() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        provider.files.lists(REVISION, ".", &[(SPECIFICATION, EntryKind::Directory)]);
        provider.files.lists(REVISION, SPECIFICATION, &[("openapi.yaml", EntryKind::File)]);

        let envelope = fail(
            &provider,
            &["emery", "show", "spec", "--config", PROJECT_CONFIG],
            1,
            "spec-mismatch",
        )
        .await;
        assert_message(&envelope, "holds `openapi.yaml`, where the three documents");
        fail(
            &provider,
            &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
            1,
            "spec-mismatch",
        )
        .await;
        assert!(provider.vcs.messages().is_empty(), "nothing is sealed");
        assert!(provider.files.writes().is_empty(), "nothing is written");

        // a bare specify needs no repository and stages regardless
        cli_ok(&provider, &["emery", "specify", &reference("docs")]).await;
        assert!(provider.storage.state(STAGED_KEY).is_some());
        provider.model.assert_exhausted();
    }

    // Outdated is not corrupt: `show` refuses typed, and the next `specify`
    // regenerates over it with the diff suppressed.
    #[tokio::test]
    async fn spec_outdated() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        let outdated = seed(
            &provider.files,
            br#"{"emery": 1, "requirements": []}"#,
            br#"{"emery": 1, "sections": []}"#,
            br#"{"emery": 1, "slices": []}"#,
        );

        let envelope = fail(
            &provider,
            &["emery", "show", "spec", "--config", PROJECT_CONFIG],
            1,
            "spec-outdated",
        )
        .await;
        assert_message(&envelope, "grammar 1");
        assert!(
            envelope["hint"].as_str().unwrap_or("").contains("emery specify"),
            "the hint names the way out: {envelope}"
        );

        let resp = cli_ok(
            &provider,
            &["emery", "specify", &reference("docs"), "--apply", "--config", PROJECT_CONFIG],
        )
        .await;
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(
            !stdout.contains("diff vs"),
            "an outdated outgoing revision yields no diff: {stdout}"
        );
        let id = committed(&provider.storage);
        assert_ne!(id, outdated, "the regenerated revision is current");
        cli_ok(&provider, &["emery", "show", "spec", "--config", PROJECT_CONFIG]).await;
        provider.model.assert_exhausted();
    }

    // A revision id no repository holds, for a parent the lineage names.
    const EARLIER: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    // `tests/build/spec.json` as an amendment sealed it, its `parent` naming
    // `EARLIER`, so a repository holding it holds a lineage.
    fn lineage() -> Vec<u8> {
        let spec = std::str::from_utf8(SPEC).expect("spec.json is UTF-8");
        spec.replace("\"parent\": null", &format!("\"parent\": \"{EARLIER}\"")).into_bytes()
    }

    // A specify that knows the target reads the current revision before any
    // load, and one that amends an earlier revision refuses the run there:
    // nothing loads, nothing is staged, and the hint names `emery amend` and
    // `--fresh` as the ways out. The parent is read off the specification's
    // own key, so a lineage under an outdated grammar refuses the same, and
    // a run naming no target reads no repository and stages regardless.
    #[tokio::test]
    async fn existing_lineage() {
        let provider = Provider::idle();
        let held = seed(&provider.files, &lineage(), DESIGN, PLAN);

        for argv in [
            vec!["emery", "specify", SOURCE, "--config", PROJECT_CONFIG],
            vec!["emery", "specify", SOURCE, "--apply", "--config", PROJECT_CONFIG],
        ] {
            let envelope = fail(&provider, &argv, 1, "lineage-exists").await;
            assert_message(&envelope, &format!("the current revision `{held}` amends `{EARLIER}`"));
            let hint = envelope["hint"].as_str().expect("a hint");
            assert!(hint.contains("emery amend"), "{argv:?}: {envelope}");
            assert!(hint.contains("emery specify --fresh"), "{argv:?}: {envelope}");
        }
        assert!(provider.source.metadata.lock().expect("metadata").is_empty(), "nothing loads");
        assert!(provider.storage.state(STAGED_KEY).is_none(), "nothing is staged");

        // a lineage under an outdated grammar is a lineage still
        let provider = Provider::idle();
        seed(
            &provider.files,
            format!(r#"{{"emery": 1, "parent": "{EARLIER}", "requirements": []}}"#).as_bytes(),
            br#"{"emery": 1, "sections": []}"#,
            br#"{"emery": 1, "slices": []}"#,
        );
        fail(
            &provider,
            &["emery", "specify", SOURCE, "--config", PROJECT_CONFIG],
            1,
            "lineage-exists",
        )
        .await;

        // a run with no target reads no repository
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        seed(&provider.files, &lineage(), DESIGN, PLAN);
        cli_ok(&provider, &["emery", "specify", SOURCE]).await;
        assert!(provider.vcs.calls().is_empty(), "a run with no target reads no repository");
        provider.model.assert_exhausted();
    }

    // `--fresh` starts the history over: the run proceeds over the lineage,
    // the candidate is staged with its record marked fresh, the text and
    // the envelope say so, and `apply` seals it with no parent over the
    // head, the diff still reported against the revision it supersedes.
    #[tokio::test]
    async fn specify_fresh() {
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        let outgoing = seed(&provider.files, &lineage(), DESIGN, PLAN);

        let resp =
            cli_ok(&provider, &["emery", "specify", SOURCE, "--fresh", "--config", PROJECT_CONFIG])
                .await;

        let id = staged(&provider.storage);
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(stdout.starts_with(&format!("staged candidate {id} (fresh)\n")), "{stdout}");
        assert!(stdout.contains(&format!("  diff vs {outgoing}: ")), "{stdout}");
        assert!(stdout.ends_with("  nothing sealed; `emery apply` seals it\n"), "{stdout}");
        let record: Value =
            serde_json::from_slice(&provider.storage.state(STAGED_KEY).expect("staged record"))
                .expect("staged record JSON");
        assert_eq!(record["fresh"], true, "the record marks the candidate fresh: {record}");

        cli_ok(&provider, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
        assert_eq!(committed(&provider.storage), id);
        let message =
            RevisionMessage::parse(&provider.vcs.messages()[0]).expect("a revision message");
        assert_eq!(message.parent, None, "a fresh seal records no parent");
        assert_eq!(message.base.as_deref(), Some(HEAD), "sealed over the head");
        provider.model.assert_exhausted();

        // the envelope carries the flag
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        seed(&provider.files, &lineage(), DESIGN, PLAN);
        let resp = cli_ok(
            &provider,
            &[
                "emery",
                "--format",
                "json",
                "specify",
                SOURCE,
                "--fresh",
                "--config",
                PROJECT_CONFIG,
            ],
        )
        .await;
        let envelope: Value = serde_json::from_slice(&resp.stdout).expect("one JSON envelope");
        assert_eq!(envelope["revision"], id, "{envelope}");
        assert_eq!(envelope["fresh"], true, "{envelope}");
        assert_eq!(envelope["staged"], true, "{envelope}");
        assert_eq!(envelope["diff"]["from"], outgoing, "{envelope}");
        provider.model.assert_exhausted();

        // a run over no lineage takes the flag too, staging as any other
        let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
        let resp = cli_ok(&provider, &["emery", "specify", SOURCE, "--fresh"]).await;
        let stdout = String::from_utf8_lossy(&resp.stdout);
        assert!(stdout.starts_with(&format!("staged candidate {id} (fresh)\n")), "{stdout}");
        provider.model.assert_exhausted();
    }

    // Isolation is host policy over the engine's flat keys.
    #[tokio::test]
    async fn multi_project() {
        // one shared store, two project-scoped views
        let shared = Memory::default();
        let alpha = Provider::over(
            Arc::new(Namespaced::new("alpha", shared.clone())),
            [SPEC_ANSWER, DESIGN_ANSWER],
        );
        let beta_spec = SPEC_ANSWER.replace("hello", "howdy");
        let beta_design = DESIGN_ANSWER.replace("hello", "howdy");
        let beta = Provider::over(
            Arc::new(Namespaced::new("beta", shared.clone())),
            [beta_spec, beta_design],
        );

        cli_ok(&alpha, &["emery", "specify", SOURCE]).await;
        cli_ok(&beta, &["emery", "specify", SOURCE]).await;

        // every write landed under its project prefix
        assert!(shared.state(STAGED_KEY).is_none(), "no unprefixed staged id exists");
        assert!(shared.objects(STAGED_CONTAINER).is_empty(), "no unprefixed candidate exists");

        let id_alpha = project_staged(&shared, "alpha");
        let id_beta = project_staged(&shared, "beta");
        assert_ne!(id_alpha, id_beta, "distinct documents stage distinct candidates");

        // each project shows its own candidate
        let spec_alpha = shared
            .object(&format!("alpha/{STAGED_CONTAINER}"), &format!("{id_alpha}/spec.json"))
            .expect("spec.json");
        let spec_beta = shared
            .object(&format!("beta/{STAGED_CONTAINER}"), &format!("{id_beta}/spec.json"))
            .expect("spec.json");
        assert_eq!(
            String::from_utf8_lossy(&spec_alpha),
            SPEC_REVISION,
            "alpha staged the revision"
        );
        assert!(String::from_utf8_lossy(&spec_beta).contains("howdy"));
        assert_eq!(
            shown(&alpha, "spec").await,
            projection(SPEC_RENDERED, &id_alpha),
            "alpha shows its own revision"
        );
        assert!(shown(&beta, "spec").await.contains("howdy"), "beta shows its own revision");

        // a seal's pointer lands under its prefix too
        cli_ok(&alpha, &["emery", "apply", "--config", PROJECT_CONFIG]).await;
        assert!(shared.state(REVISION_KEY).is_none(), "no unprefixed current id exists");
        assert_eq!(
            shared.state(&format!("alpha/{REVISION_KEY}")).as_deref(),
            Some(id_alpha.as_bytes()),
            "alpha sealed its own candidate"
        );
        assert_eq!(project_staged(&shared, "beta"), id_beta, "beta's candidate stands");

        alpha.model.assert_exhausted();
        beta.model.assert_exhausted();
    }
}

// --- helpers ---

fn assert_message(envelope: &Value, fragment: &str) {
    let message = envelope["message"].as_str().unwrap_or("");
    assert!(message.contains(fragment), "expected `{fragment}` in: {envelope}");
}

fn committed(storage: &Memory) -> String {
    String::from_utf8(storage.state(REVISION_KEY).expect("current")).expect("utf-8 revision id")
}

fn project_staged(shared: &Memory, project: &str) -> String {
    staged_at(shared, &format!("{project}/{STAGED_KEY}"))
}

fn staged_document(storage: &Memory, id: &str, name: &str) -> Vec<u8> {
    storage
        .object(STAGED_CONTAINER, &format!("{id}/{name}"))
        .unwrap_or_else(|| panic!("staged {name}"))
}

fn committed_document(provider: &Provider, name: &str) -> Vec<u8> {
    written(&provider.files, name)
}

fn projection(fixture: &str, id: &str) -> String {
    fixture.replace("<revision>", id)
}

async fn shown<S>(provider: &Provider<S>, document: &str) -> String
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let resp = cli_ok(provider, &["emery", "show", document, "--staged"]).await;
    String::from_utf8(resp.stdout).expect("utf-8 document")
}

async fn shown_sealed<S>(provider: &Provider<S>, document: &str) -> String
where
    S: StateStore + BlobStore + Send + Sync + 'static,
{
    let resp = cli_ok(provider, &["emery", "show", document, "--config", PROJECT_CONFIG]).await;
    String::from_utf8(resp.stdout).expect("utf-8 document")
}
