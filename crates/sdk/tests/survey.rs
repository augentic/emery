//! A survey by model is one turn whose answer is held to the tree before the
//! adapter derives anything from it; every scenario here puts one.

use emery_sdk::survey::{Facts, Inventory};
use emery_sdk::{Context, Doc, Error, SourceInput};
use omnia_sdk::model::ToolCall;
use omnia_test::SeenFormat;
use omnia_test::guest::Scripted;

const PROSE: &[Doc] = &[
    Doc {
        path: "survey.md",
        body: "SURVEY",
    },
    Doc {
        path: "extract.md",
        body: "SYSTEM",
    },
    Doc {
        path: "references/surfaces.md",
        body: "A route is a surface.",
    },
];

const VALID: &str = r#"{"surfaces":[
    {"name":"POST /orders","anchor":"src/routes/orders.ts#L2-L3","stem":"orders"},
    {"name":"import command","anchor":"src/cli.ts","stem":"import"}
],"unreached":["src/lib/unused.ts"]}"#;

fn write(root: &std::path::Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(path, body).expect("write");
}

// A tree of three modules and a manifest, lent as the source `svc`.
fn tree() -> (tempfile::TempDir, SourceInput, Vec<String>) {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "package.json", "{\"name\":\"svc\"}\n");
    write(tmp.path(), "src/cli.ts", "import { run } from './lib/run';\nrun();\n");
    write(
        tmp.path(),
        "src/routes/orders.ts",
        "export const r = 1;\napp.post('/orders', h);\napp.get('/orders', l);\n",
    );
    write(tmp.path(), "src/lib/unused.ts", "export const dead = 1;\n");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let input = SourceInput::workspace("svc", root);
    let modules =
        ["src/cli.ts", "src/lib/unused.ts", "src/routes/orders.ts"].map(str::to_owned).to_vec();
    (tmp, input, modules)
}

async fn ask(
    model: &Scripted, input: &SourceInput, facts: &Facts<'_>,
    check: impl FnMut(&Inventory) -> Vec<String> + Send,
) -> Result<Inventory, Error> {
    let ctx = Context {
        adapter_id: "source:probe",
        input,
        model,
    };
    emery_sdk::survey::surfaces(&ctx, PROSE, facts, check).await
}

// The turn carries the adapter's facts, the module list, the laid files, and
// the lend; the answer is steered by the inventory schema.
#[tokio::test]
async fn request_shape() {
    let (_tmp, input, modules) = tree();
    let model = Scripted::answering([VALID]);
    let files = vec!["package.json".to_owned(), "src/cli.ts".to_owned()];
    let facts = Facts {
        modules: &modules,
        text: "The manifest names `svc`; `src/cli.ts` runs at load.\n",
        files: &files,
    };

    let inventory = ask(&model, &input, &facts, |_| Vec::new()).await.expect("accepted");
    assert_eq!(inventory.surfaces.len(), 2);
    assert_eq!(inventory.unreached, ["src/lib/unused.ts"]);

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("SURVEY"));
    let root = match &input.content {
        emery_sdk::SourceContent::Workspace(root) => root.clone(),
        emery_sdk::SourceContent::Value(_) => unreachable!(),
    };
    assert_eq!(request.workspace.as_deref(), Some(root.as_str()), "the tree is lent");
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
    assert!(user.starts_with("Survey the source `svc` bound to adapter `source:probe` before it is mined.\n\nThe manifest names `svc`; `src/cli.ts` runs at load.\n\n`$SOURCE_DIR` is the bound source tree"), "{user}");
    assert!(
        user.contains("- `src/cli.ts`\n- `src/lib/unused.ts`\n- `src/routes/orders.ts`\n\n"),
        "{user}"
    );
    assert!(
        user.contains(
            "These files are laid out here whole, every line led by its number, so cite `#L<n>` \
             from the numbers shown rather than reading them again:\n\n### `package.json` (1 \
             line)\n\n```\n1|{\"name\":\"svc\"}\n```\n\n### `src/cli.ts` (2 lines)\n\n```\n1|import \
             { run } from './lib/run';\n2|run();\n```\n\nName each surface"
        ),
        "{user}"
    );
    assert!(!user.contains(&root), "the lend carries the root, not the brief: {user}");
    assert!(user.ends_with("Answer with one JSON object matching the survey schema."), "{user}");
    model.assert_exhausted();
}

// With nothing to lay and no facts text, the brief is the module list alone
// between the opening and the instructions.
#[tokio::test]
async fn bare_facts() {
    let (_tmp, input, modules) = tree();
    let model = Scripted::answering([r#"{"surfaces":[]}"#]);
    let facts = Facts {
        modules: &modules,
        text: "",
        files: &[],
    };

    let inventory = ask(&model, &input, &facts, |_| Vec::new()).await.expect("none is valid");
    assert!(inventory.surfaces.is_empty());
    assert!(inventory.unreached.is_empty(), "`unreached` defaults to none");

    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains("before it is mined.\n\n`$SOURCE_DIR` is the bound source tree"),
        "{user}"
    );
    assert!(!user.contains("laid out here whole"), "{user}");
}

// An inline value has no tree to survey: the adapter's own defect.
#[tokio::test]
async fn inline_value() {
    let model = Scripted::default();
    let input = SourceInput::value("brief", "Ship it.");
    let facts = Facts::default();

    let error = ask(&model, &input, &facts, |_| Vec::new()).await.expect_err("no tree");
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("needs a workspace input"), "{error}");
    assert!(model.seen().is_empty(), "no turn was spent");
}

// The survey turn offers the adapter's references and the runtime's through
// the same tools a mining turn has; no system document is listed — the
// survey prompt rides this turn's system, and a mining turn's prompt has
// nothing to teach a survey — and a read still answers each.
#[tokio::test]
async fn doc_refs() {
    let (_tmp, input, modules) = tree();
    let model = Scripted::answering([VALID]).calling(
        0,
        [
            ToolCall {
                id: "1".to_string(),
                name: "list_docs".to_string(),
                arguments: "{}".to_string(),
            },
            ToolCall {
                id: "2".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"references/surfaces.md"}"#.to_string(),
            },
            ToolCall {
                id: "3".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"survey.md"}"#.to_string(),
            },
        ],
    );
    let facts = Facts {
        modules: &modules,
        text: "",
        files: &[],
    };

    ask(&model, &input, &facts, |_| Vec::new()).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4, "three reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"paths":["references/surfaces.md","reconciliation.md"]}"#),
        "`list_docs` lists no system document"
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"A route is a surface.","path":"references/surfaces.md"}"#)
    );
    assert_eq!(
        exchanges[2].outcome.as_deref(),
        Ok(r#"{"body":"SURVEY","path":"survey.md"}"#),
        "`read_doc` still answers an unlisted document"
    );
    assert_eq!(exchanges[3].tool, "check");
    assert_eq!(exchanges[3].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A corpus without `survey.md` is the adapter build's own defect.
#[tokio::test]
async fn missing_prompt() {
    let (_tmp, input, modules) = tree();
    let model = Scripted::default();
    let facts = Facts {
        modules: &modules,
        text: "",
        files: &[],
    };
    let ctx = Context {
        adapter_id: "source:mute",
        input: &input,
        model: &model,
    };

    let error = emery_sdk::survey::surfaces(&ctx, &[], &facts, |_| Vec::new())
        .await
        .expect_err("no prompt to ask with");
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`survey.md` is not embedded"), "{error}");
    assert!(model.seen().is_empty());
}

// Every rule the tree can hold the answer to is a finding, all returned
// together with the adapter's own, and the corrected answer is accepted with
// its anchors normalised — the spelling the adapter's check read on every
// round, so what it derives from rests on the paths the tree spells.
#[tokio::test]
async fn gate_findings() {
    let (_tmp, input, modules) = tree();
    let model = Scripted::answering([
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
    let facts = Facts {
        modules: &modules,
        text: "",
        files: &[],
    };

    let mut checked: Vec<Inventory> = Vec::new();
    let inventory = ask(&model, &input, &facts, |answer| {
        checked.push(answer.clone());
        if answer.surfaces.iter().any(|surface| surface.stem == "b") {
            vec!["- the adapter refuses stem `b`".to_owned()]
        } else {
            Vec::new()
        }
    })
    .await
    .expect("corrected");
    assert_eq!(inventory.surfaces.len(), 1);
    assert_eq!(inventory.surfaces[0].anchor, "src/cli.ts#L2", "normalised root-relative");
    assert_eq!(inventory.unreached, ["src/lib/unused.ts", "src/routes/orders.ts"]);
    assert_eq!(checked.len(), 2, "the adapter's check ran on both rounds");
    assert_eq!(checked[1], inventory, "the check read the spelling that is returned");
    assert_eq!(
        checked[0].surfaces[4].anchor, "../cli.ts",
        "an anchor that escapes the root is left as answered for its finding"
    );
    let anchor = inventory.surfaces[0].anchor().expect("an accepted anchor parses");
    assert_eq!((anchor.path, anchor.lines), ("src/cli.ts", Some((2, 2))));

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the first answer is refused");
    for finding in [
        "- surface 0: has no name",
        "- surface `dup` is listed twice",
        "- surface `grammar`: anchor `src/cli.ts#5` is not `<path>`, `<path>#L<n>`, or \
         `<path>#L<n>-L<n>`",
        "- surface `escapes`: anchor `../cli.ts` escapes the source root",
        "- surface `stranger`: anchor `src/lib/run.ts#L1` names no module of this source; anchor \
         within the modules listed",
        "- surface `manifest`: anchor `package.json` names no module of this source",
        "- surface `long`: anchor `src/cli.ts#L1-L9` cites line 9, but `src/cli.ts` has 2 lines",
        "- surface `Cased`: stem `Import` is not lowercase kebab-case",
        "- unreached `src/lib/run.ts` names no module of this source",
        "- unreached `src/cli.ts` is a surface's entry; a module is one or the other",
        "- the adapter refuses stem `b`",
    ] {
        assert!(correction.contains(finding), "{finding}: {correction}");
    }
    assert!(exchanges[1].outcome.is_ok(), "the corrected answer is accepted");
    model.assert_exhausted();
}

// The adapter's check runs on an answer the tree accepts, and its findings
// alone drive the correction.
#[tokio::test]
async fn adapter_check() {
    let (_tmp, input, modules) = tree();
    let model = Scripted::answering([
        r#"{"surfaces":[{"name":"start","anchor":"src/cli.ts","stem":"start"}]}"#,
        VALID,
    ]);
    let facts = Facts {
        modules: &modules,
        text: "",
        files: &[],
    };

    let inventory = ask(&model, &input, &facts, |answer| {
        answer
            .surfaces
            .iter()
            .filter(|surface| surface.stem == "start")
            .map(|surface| format!("- surface `{}`: `start` is the caller's", surface.name))
            .collect()
    })
    .await
    .expect("corrected");
    assert_eq!(inventory.surfaces[0].stem, "orders");

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("refused by the adapter");
    assert!(correction.contains("- surface `start`: `start` is the caller's"), "{correction}");
    assert!(!correction.contains("names no module"), "the tree held nothing: {correction}");
    model.assert_exhausted();
}

// A rejected answer that never lands within the rounds is the operator's
// `bad_request`, as a mining turn's is.
#[tokio::test]
async fn rounds_exhausted() {
    let (_tmp, input, modules) = tree();
    let model =
        Scripted::answering([r#"{"surfaces":[{"name":"x","anchor":"nope.ts","stem":"x"}]}"#]);
    let facts = Facts {
        modules: &modules,
        text: "",
        files: &[],
    };

    let error = ask(&model, &input, &facts, |_| Vec::new()).await.expect_err("never accepted");
    assert_eq!(error.code(), "bad_request", "{error}");
    model.assert_exhausted();
}
