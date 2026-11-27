//! Slice fixtures for the build suite.

use emery_sdk::target::{DesignType, Requirement, Scenario, Slice};

pub const VALID: &str =
    r#"{"covered":["REQ-001","REQ-002"],"written":["src/orders.rs","Cargo.toml"]}"#;

pub const DESIGN: &str = "# Design\n\n## Domain model\n\nThe shapes the system stores.\n\nType: \
                      orders.order\n```\ninterface Order {\n  id: string;\n\n  lines: \
                      OrderLine[];\n}\n```\n\nType: orders.order-line\n```\ninterface OrderLine \
                      {\n  sku: string;\n}\n```\n\nType: billing.Invoice (src/billing.ts#L3)\n\
                      ```\ninterface Invoice {\n  total: number;\n}\n```\n\nType: start.Config \
                      (mechanism)\n```\nclass Config {\n  readonly port: number;\n}\n```\n\n## \
                      APIs and integrations\n\nThe store holds every order.\n";

pub fn requirement(
    id: &str, subject: &str, statement: &str, when: &str, then: &str,
) -> Requirement {
    Requirement {
        id: id.to_owned(),
        subject: subject.to_owned(),
        statement: vec![statement.to_owned()],
        scenarios: vec![Scenario {
            name: subject.rsplit('.').next().unwrap_or(subject).to_owned(),
            given: Vec::new(),
            when: when.to_owned(),
            then: then.to_owned(),
            and: Vec::new(),
        }],
        trigger: None,
    }
}

pub fn slice() -> Slice {
    Slice {
        id: "SLICE-001".to_owned(),
        part: "PART-001".to_owned(),
        name: "orders".to_owned(),
        index: 1,
        count: 1,
        base: "1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d".to_owned(),
        requirements: vec![
            requirement(
                "REQ-001",
                "orders.create",
                "Orders are created.",
                "an order is placed",
                "it is created",
            ),
            requirement(
                "REQ-002",
                "orders.list",
                "Orders are listed.",
                "the orders are asked for",
                "each is listed",
            ),
        ],
        types: vec![DesignType {
            key: "orders.order".to_owned(),
            signature: "interface Order { id: string }".to_owned(),
        }],
        brief: vec!["Build the orders module.".to_owned()],
        spec: "## REQ-001\n\nOrders are created.\n\n## REQ-002\n\nOrders are listed.\n".to_owned(),
        design: "## Types\n\n`Order`\n".to_owned(),
        plan: "## SLICE-001 orders\n\nBuild the orders module.\n".to_owned(),
    }
}

pub fn designed() -> Slice {
    let mut slice = slice();
    DESIGN.clone_into(&mut slice.design);
    "interface Order {\n  id: string;\n\n  lines: OrderLine[];\n}"
        .clone_into(&mut slice.types[0].signature);
    slice
}
