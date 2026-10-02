//! What a survey derives by code alone is spelled with helpers pure over
//! strings; every scenario here runs with no model and no tree.

use emery_sdk::kebab;
use emery_sdk::survey::Lines;
use emery_sdk::survey::resolve::{Target, normalize};
use emery_sdk::survey::route::{self, Spelling};
use emery_sdk::survey::tests::{Statement, Test, scenarios, stated};

// How Express and Nest spell a route: a parameter leads with a colon.
const COLON: Spelling = Spelling {
    pattern: &['*', '{', '(', '['],
    param_name: |segment| segment.trim_start_matches(':'),
};

// How Flask, Django, and FastAPI spell one: converters, braces, and
// `re_path` groups beside the colon.
const BRACED: Spelling = Spelling {
    pattern: &['*', '{', '(', '[', '<', '?', '\\'],
    param_name,
};

fn param_name(segment: &str) -> &str {
    if let Some(inner) = segment.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
        return inner.split_once(':').map_or(inner, |(name, _)| name);
    }
    if let Some(inner) = segment.strip_prefix('<').and_then(|rest| rest.strip_suffix('>')) {
        return inner.rsplit_once(':').map_or(inner, |(_, name)| name);
    }
    if let Some(start) = segment.find("(?P<") {
        let rest = &segment[start + 4..];
        return rest.split_once('>').map_or(rest, |(name, _)| name);
    }
    segment.trim_start_matches(':')
}

fn statement(text: &str, line: u32) -> Statement {
    Statement {
        text: text.to_owned(),
        line,
    }
}

#[test]
fn kebab_humps() {
    assert_eq!(kebab("OrdersAPI").as_deref(), Some("orders-api"));
    assert_eq!(kebab("HTTPServer").as_deref(), Some("http-server"));
    assert_eq!(kebab("getUserById").as_deref(), Some("get-user-by-id"));
    assert_eq!(kebab("v2Orders").as_deref(), Some("v2-orders"));
    assert_eq!(kebab("ORDERS").as_deref(), Some("orders"));
}

#[test]
fn kebab_runs() {
    assert_eq!(kebab("get_user__by id").as_deref(), Some("get-user-by-id"));
    assert_eq!(kebab("  /api/orders/ ").as_deref(), Some("api-orders"));
    assert_eq!(kebab("@scope/pkg-name").as_deref(), Some("scope-pkg-name"));
    assert_eq!(kebab("orders-api").as_deref(), Some("orders-api"));
    assert_eq!(kebab("--"), None);
    assert_eq!(kebab(""), None);
}

#[test]
fn lines_span() {
    let span = Lines { start: 3, end: 5 };
    assert!(span.holds(3) && span.holds(4) && span.holds(5));
    assert!(!span.holds(2) && !span.holds(6));
    assert!(span.contains(span));
    assert!(span.contains(Lines { start: 4, end: 5 }));
    assert!(!span.contains(Lines { start: 2, end: 4 }));
    assert!(!Lines::default().holds(1), "the default holds no line");
}

// `anchor` is the claim grammar the SDK's gate parses; `Display` is prose.
#[test]
fn lines_rendered() {
    assert_eq!(Lines { start: 3, end: 5 }.anchor(), "L3-L5");
    assert_eq!(Lines { start: 3, end: 5 }.to_string(), "L3–L5");
    assert_eq!(Lines { start: 7, end: 7 }.anchor(), "L7");
    assert_eq!(Lines { start: 7, end: 7 }.to_string(), "L7");
}

#[test]
fn lines_from_cited() {
    assert_eq!(Lines::from((2, 9)), Lines { start: 2, end: 9 });
    assert_eq!(
        Lines::from((1, u64::MAX)),
        Lines {
            start: 1,
            end: u32::MAX
        }
    );
}

#[test]
fn normalize_paths() {
    assert_eq!(normalize("src/routes", "./orders").as_deref(), Some("src/routes/orders"));
    assert_eq!(normalize("src/routes", "../lib/db").as_deref(), Some("src/lib/db"));
    assert_eq!(normalize("src/routes", "../../index").as_deref(), Some("index"));
    assert_eq!(normalize("", "src//./a.ts").as_deref(), Some("src/a.ts"));
    assert_eq!(normalize("", "").as_deref(), Some(""));
    assert_eq!(normalize("src", "../../secret"), None);
    assert_eq!(normalize("", ".."), None);
}

#[test]
fn target_accessors() {
    let targets = [
        Target::Module("src/a.ts".to_owned()),
        Target::Package("express".to_owned()),
        Target::Data("config.json".to_owned()),
        Target::Unresolved("./missing".to_owned()),
    ];
    let read =
        |accessor: fn(&Target) -> Option<&str>| targets.iter().map(accessor).collect::<Vec<_>>();

    assert_eq!(read(Target::module), [Some("src/a.ts"), None, None, None]);
    assert_eq!(read(Target::package), [None, Some("express"), None, None]);
    assert_eq!(read(Target::data), [None, None, Some("config.json"), None]);
    assert_eq!(read(Target::unresolved), [None, None, None, Some("./missing")]);
}

#[test]
fn names_resource() {
    for segment in ["orders", "order-items", "v1beta", "users_v2"] {
        assert!(COLON.names_resource(segment), "{segment}");
    }
    for segment in ["v1", "v22", "api", "rest", "internal", ":id", "*", "{id}", "(.*)", "[slug]"] {
        assert!(!COLON.names_resource(segment), "{segment}");
    }
    for segment in ["<int:pk>", "(?P<pk>\\d+)", "\\d+"] {
        assert!(!BRACED.names_resource(segment), "{segment}");
    }
    assert!(COLON.names_resource("<int:pk>"), "a colon spelling knows no converter");
}

#[test]
fn route_stem() {
    assert_eq!(COLON.stem("/api/v1/orders/:id").as_deref(), Some("orders"));
    assert_eq!(COLON.stem("/orderItems").as_deref(), Some("order-items"));
    assert_eq!(BRACED.stem("/api/{tenant}/invoices/<int:pk>/").as_deref(), Some("invoices"));
    assert_eq!(COLON.stem("/api/v1/:id"), None);
    assert_eq!(COLON.stem("/"), None);
}

// A route spelling no segment as its stem is relative to a mount the survey
// did not read, so every segment tells.
#[test]
fn route_discriminator() {
    assert_eq!(COLON.discriminator("GET", "/api/orders/:id", "orders").as_deref(), Some("get-id"));
    assert_eq!(COLON.discriminator("get", "/api/orders", "orders").as_deref(), Some("get"));
    assert_eq!(
        COLON.discriminator("POST", "/:id/assign", "tasks").as_deref(),
        Some("post-id-assign")
    );
    assert_eq!(
        BRACED
            .discriminator("GET", "/orders/{order_id}/items/<slug:name>", "orders")
            .as_deref(),
        Some("get-order-id-items-name")
    );
    assert_eq!(
        BRACED.discriminator("DELETE", "orders/(?P<pk>\\d+)/", "orders").as_deref(),
        Some("delete-pk")
    );
}

#[test]
fn normalised_name() {
    assert_eq!(COLON.normalised("GET /api/customers", "customers").as_deref(), Some("get"));
    assert_eq!(
        COLON.normalised("POST /api/v1/orders/:id/cancel", "orders").as_deref(),
        Some("post-id-cancel")
    );
    assert_eq!(COLON.normalised("orders", "orders"), None);
    assert_eq!(COLON.normalised("order items", "order-items"), None, "each word of the stem");
    assert_eq!(COLON.normalised("", "orders"), None);
}

#[test]
fn join_routes() {
    assert_eq!(route::join("/api/", "orders"), "/api/orders");
    assert_eq!(route::join("orders", "/{id}/"), "/orders/{id}");
    assert_eq!(route::join("/a//b", "c"), "/a/b/c");
    assert_eq!(route::join("", "/"), "/");
    assert_eq!(route::join("", ""), "/");
}

#[test]
fn literal_stems() {
    assert_eq!(route::literal_stem("import").as_deref(), Some("import"));
    assert_eq!(route::literal_stem("import <file>").as_deref(), Some("import"));
    assert_eq!(route::literal_stem("importOrders --dry-run").as_deref(), Some("import-orders"));
    assert_eq!(route::literal_stem("invoices.send").as_deref(), Some("invoices"));
    assert_eq!(route::literal_stem("  orders.created.v2 ").as_deref(), Some("orders"));
    for literal in [
        "",
        "   ",
        "*/5 * * * *",
        "0 0 * * *",
        "<file>",
        "orders:created",
        "a/b",
        "[slug]",
        "cron",
        "interval",
        "date",
        "_private",
        "123",
    ] {
        assert_eq!(route::literal_stem(literal), None, "{literal:?}");
    }
}

// An outline's `Examples:` table opens no scenario.
#[test]
fn feature_scenarios() {
    let text = "Feature: Orders\n\n  Background:\n    Given a store\n\n  Scenario: An order is \
                created\n    Given a body\n  Scenario Outline: An order is rejected\n    Given \
                <reason>\n    Examples:\n      | reason |\n  Scenario Template: Another outline\n  \
                Example: A worked example\n\nFeature: Billing\n  Scenario: An invoice is sent\n";

    assert_eq!(
        scenarios(text),
        [
            statement("Orders › An order is created", 6),
            statement("Orders › An order is rejected", 8),
            statement("Orders › Another outline", 12),
            statement("Orders › A worked example", 13),
            statement("Billing › An invoice is sent", 16),
        ]
    );
}

#[test]
fn scenario_without_feature() {
    assert_eq!(scenarios("Scenario: Alone\n"), [statement("Alone", 1)]);
    assert_eq!(scenarios("Given nothing\n"), [] as [Statement; 0]);
}

#[test]
fn stated_section() {
    let tests = [
        Test {
            path: "test/orders.test.ts".to_owned(),
            imports: vec!["src/orders.ts".to_owned()],
            statements: vec![
                statement("orders › creates an order", 4),
                statement("orders › rejects an empty body", 9),
            ],
        },
        Test {
            path: "features/orders.feature".to_owned(),
            imports: Vec::new(),
            statements: vec![statement("Orders › An order is created", 2)],
        },
    ];

    let section = stated(&tests, "a case's title under its suites'").expect("statements");
    assert!(
        section.starts_with(
            "Behaviours the tree's own tests state, each at its line — a case's title under its \
             suites', a scenario's under its feature's. "
        ),
        "{section}"
    );
    assert!(
        section.ends_with(
            "anchor:\n\n- `test/orders.test.ts#L4` — orders › creates an order\n- \
             `test/orders.test.ts#L9` — orders › rejects an empty body\n- \
             `features/orders.feature#L2` — Orders › An order is created"
        ),
        "{section}"
    );

    let python =
        stated(&tests, "a test's docstring or name under its class's").expect("statements");
    assert!(
        python.contains(
            "each at its line — a test's docstring or name under its class's, a scenario's under \
             its feature's. "
        ),
        "{python}"
    );
}

// A test that states nothing lists nothing, so the section is left out.
#[test]
fn stated_nothing() {
    let silent = Test {
        path: "test/empty.test.ts".to_owned(),
        imports: Vec::new(),
        statements: Vec::new(),
    };

    assert_eq!(stated([&silent], "a case's title under its suites'"), None);
    assert_eq!(stated([] as [&Test; 0], "a case's title under its suites'"), None);
}
