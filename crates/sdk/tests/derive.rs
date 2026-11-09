//! Tests pure string, span, route, and test-statement survey helpers.

use emery_sdk::kebab;
use emery_sdk::survey::resolve::normalize;
use emery_sdk::survey::route::{self, Spelling};
use emery_sdk::survey::tests::{Statement, scenarios};

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
fn kebab_cases() {
    assert_eq!(kebab("OrdersAPI").as_deref(), Some("orders-api"));
    assert_eq!(kebab("HTTPServer").as_deref(), Some("http-server"));
    assert_eq!(kebab("getUserById").as_deref(), Some("get-user-by-id"));
    assert_eq!(kebab("v2Orders").as_deref(), Some("v2-orders"));
    assert_eq!(kebab("ORDERS").as_deref(), Some("orders"));
    assert_eq!(kebab("get_user__by id").as_deref(), Some("get-user-by-id"));
    assert_eq!(kebab("  /api/orders/ ").as_deref(), Some("api-orders"));
    assert_eq!(kebab("@scope/pkg-name").as_deref(), Some("scope-pkg-name"));
    assert_eq!(kebab("orders-api").as_deref(), Some("orders-api"));
    assert_eq!(kebab("--"), None);
    assert_eq!(kebab(""), None);
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
        BRACED.discriminator("GET", "/orders/{order_id}/items/<slug:name>", "orders").as_deref(),
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
fn gherkin_scenarios() {
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
    assert_eq!(scenarios("Scenario: Alone\n"), [statement("Alone", 1)]);
    assert_eq!(scenarios("Given nothing\n"), [] as [Statement; 0]);
}
