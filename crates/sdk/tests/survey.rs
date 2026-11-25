//! `survey::surfaces` on the component rung.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use emery_sdk::survey::Surface;
use omnia_test::SeenFormat;
use support::by_file::single_model;
use support::harness::survey;
use test_programs::protocol::{
    FactsSpec, FileSpec, InputSpec, SurveyCheck, SurveyScenario, WorkspaceInput,
};

const VALID: &str = r#"{"surfaces":[
    {"name":"POST /orders","anchor":"src/routes/orders.ts#L2-L3","stem":"orders"},
    {"name":"import command","anchor":"src/cli.ts","stem":"import"}
],"unreached":["src/lib/unused.ts"]}"#;

fn tree() -> SurveyScenario {
    SurveyScenario {
        adapter_id: "source:probe".to_owned(),
        input: InputSpec::Workspace(WorkspaceInput {
            name: "svc".to_owned(),
            files: vec![
                FileSpec {
                    path: "package.json".to_owned(),
                    body: "{\"name\":\"svc\"}\n".to_owned(),
                },
                FileSpec {
                    path: "src/cli.ts".to_owned(),
                    body: "import { run } from './lib/run';\nrun();\n".to_owned(),
                },
                FileSpec {
                    path: "src/routes/orders.ts".to_owned(),
                    body: "export const r = 1;\napp.post('/orders', h);\napp.get('/orders', l);\n"
                        .to_owned(),
                },
                FileSpec {
                    path: "src/lib/unused.ts".to_owned(),
                    body: "export const dead = 1;\n".to_owned(),
                },
            ],
        }),
        facts: FactsSpec {
            modules: vec![
                "src/cli.ts".to_owned(),
                "src/lib/unused.ts".to_owned(),
                "src/routes/orders.ts".to_owned(),
            ],
            text: String::new(),
            files: Vec::new(),
        },
        check: SurveyCheck::None,
        omit_survey_prompt: false,
    }
}

#[tokio::test]
async fn sdk_extract() {}

#[tokio::test]
async fn sdk_build() {}

#[tokio::test]
async fn sdk_verify() {}

#[tokio::test]
async fn sdk_survey() {
    request_shape().await;
    bare_facts().await;
    inline_value().await;
    doc_refs().await;
    missing_prompt().await;
    gate_findings().await;
    technology_stem().await;
    adapter_check().await;
    rounds_exhausted().await;
}

async fn request_shape() {
    let mut scenario = tree();
    "The manifest names `svc`; `src/cli.ts` runs at load.\n".clone_into(&mut scenario.facts.text);
    scenario.facts.files = vec!["package.json".to_owned(), "src/cli.ts".to_owned()];
    let model = single_model([VALID]);
    let run = survey(&scenario, model.clone()).await;
    let inventory = run.inventory();
    assert_eq!(inventory.surfaces.len(), 2);
    assert_eq!(inventory.unreached, ["src/lib/unused.ts"]);

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("SURVEY"));
    assert!(request.workspace.is_some(), "the tree is lent");
    assert_eq!(request.tools, ["list_docs", "read_doc"]);
    assert!(request.check, "acceptance is the check");
    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("the inventory is steered by schema");
    };
    assert_eq!(name, "survey-svc");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("schema parses");
    assert!(schema.pointer("/properties/surfaces").is_some(), "{schema}");
    assert!(schema.pointer("/properties/unreached").is_some(), "{schema}");
    let user = &request.messages[0];
    assert!(
        user.starts_with(
            "Survey the source `svc` bound to adapter `source:probe` before it is mined.\n\nThe \
             manifest names `svc`; `src/cli.ts` runs at load.\n\n`$SOURCE_DIR` is the bound source \
             tree"
        ),
        "{user}"
    );
    model.assert_exhausted();
}

async fn bare_facts() {
    let scenario = tree();
    let model = single_model([r#"{"surfaces":[]}"#]);
    let inventory = survey(&scenario, model.clone()).await.inventory();
    assert_eq!(inventory.surfaces, [] as [Surface; 0]);
    assert_eq!(inventory.unreached, [] as [String; 0]);
    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains("before it is mined.\n\n`$SOURCE_DIR` is the bound source tree"),
        "{user}"
    );
}

async fn inline_value() {
    let model = single_model(Vec::<String>::new());
    let scenario = SurveyScenario {
        adapter_id: "source:probe".to_owned(),
        input: InputSpec::Value {
            name: "brief".to_owned(),
            text: "Ship it.".to_owned(),
        },
        facts: FactsSpec::default(),
        check: SurveyCheck::None,
        omit_survey_prompt: false,
    };
    let error = survey(&scenario, model.clone()).await.error();
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("needs a workspace input"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

async fn doc_refs() {
    let scenario = tree();
    let model = single_model([VALID]).calling(
        0,
        [
            ("list_docs", "{}"),
            ("read_doc", r#"{"path":"references/surfaces.md"}"#),
            ("read_doc", r#"{"path":"survey.md"}"#),
        ],
    );
    survey(&scenario, model.clone()).await.inventory();
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4);
    model.assert_exhausted();
}

async fn missing_prompt() {
    let mut scenario = tree();
    scenario.omit_survey_prompt = true;
    "source:mute".clone_into(&mut scenario.adapter_id);
    let model = single_model(Vec::<String>::new());
    let error = survey(&scenario, model.clone()).await.error();
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`survey.md` is not embedded"), "{error}");
}

async fn gate_findings() {
    let scenario = tree();
    let model = single_model([
        r#"{"surfaces":[
            {"name":"","anchor":"src/cli.ts","stem":"import"},
            {"name":"dup","anchor":"src/cli.ts#L1","stem":"a"},
            {"name":"dup","anchor":"src/cli.ts#L1","stem":"b"},
            {"name":"grammar","anchor":"src/cli.ts#5","stem":"cli"},
            {"name":"escapes","anchor":"../cli.ts","stem":"cli"},
            {"name":"stranger","anchor":"src/lib/run.ts#L1","stem":"run"},
            {"name":"manifest","anchor":"package.json","stem":"svc"},
            {"name":"long","anchor":"src/cli.ts#L1-L9","stem":"cli"},
            {"name":"Cased","anchor":"src/cli.ts#L2","stem":"Import"}
        ],"unreached":["src/lib/run.ts","src/cli.ts"]}"#,
        r#"{"surfaces":[
            {"name":"import command","anchor":"./src/cli.ts#L2-L2","stem":"import"}
        ],"unreached":["./src/lib/unused.ts","src/routes/orders.ts"]}"#,
    ]);
    let mut scenario = scenario;
    scenario.check = SurveyCheck::RejectStemB;
    let inventory = survey(&scenario, model.clone()).await.inventory();
    assert_eq!(inventory.surfaces.len(), 1);
    assert_eq!(inventory.surfaces[0].anchor, "src/cli.ts#L2");
    assert_eq!(inventory.unreached, ["src/lib/unused.ts", "src/routes/orders.ts"]);
    model.assert_exhausted();
}

async fn technology_stem() {
    let mut scenario = tree();
    scenario.check = SurveyCheck::None;
    let model = single_model([
        r#"{"surfaces":[
            {"name":"secret retriever","anchor":"src/cli.ts#L1","stem":"key-vault"},
            {"name":"POST /orders","anchor":"src/routes/orders.ts#L2","stem":"orders"}
        ]}"#,
        r#"{"surfaces":[
            {"name":"secret retriever","anchor":"src/cli.ts#L1","stem":"secrets"},
            {"name":"POST /orders","anchor":"src/routes/orders.ts#L2","stem":"orders"}
        ]}"#,
    ]);
    let inventory = survey(&scenario, model.clone()).await.inventory();
    assert_eq!(inventory.surfaces[0].stem, "secrets");
    model.assert_exhausted();
}

async fn adapter_check() {
    let mut scenario = tree();
    scenario.check = SurveyCheck::RejectStart;
    let model = single_model([
        r#"{"surfaces":[{"name":"start","anchor":"src/cli.ts","stem":"start"}]}"#,
        VALID,
    ]);
    let inventory = survey(&scenario, model.clone()).await.inventory();
    assert_eq!(inventory.surfaces[0].stem, "orders");
    model.assert_exhausted();
}

async fn rounds_exhausted() {
    let scenario = tree();
    let model = single_model([r#"{"surfaces":[{"name":"x","anchor":"nope.ts","stem":"x"}]}"#]);
    let error = survey(&scenario, model.clone()).await.error();
    assert_eq!(error.code(), "exhausted", "{error}");
    model.assert_exhausted();
}
