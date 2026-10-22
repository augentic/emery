//! Tests `target::build` report gating and file writing over a scripted model.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use emery_sdk::target::{
    BuildContext, DesignType, Grant, Guarded, Marker, Report, Requirement, Scaffold, Scenario,
    Slice,
};
use emery_sdk::{Doc, Error};
use omnia_sdk::model::ToolCall;
use omnia_test::guest::Scripted;
use omnia_test::{Seen, SeenFormat, SeenWorker};

const PROSE: &[Doc] = &[
    Doc {
        path: "build.md",
        body: "BUILD",
    },
    Doc {
        path: "references/layout.md",
        body: "One module per requirement.",
    },
];

const VALID: &str = r#"{"covered":["REQ-001","REQ-002"],"written":["src/orders.rs","Cargo.toml"]}"#;

fn requirement(id: &str, subject: &str, statement: &str, when: &str, then: &str) -> Requirement {
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

fn slice() -> Slice {
    Slice {
        id: "SLICE-001".to_owned(),
        name: "orders".to_owned(),
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

async fn ask(model: &Scripted, slice: &Slice, workspace: &str) -> Result<Report, Error> {
    ask_as(model, slice, workspace, &[]).await
}

async fn ask_as(
    model: &Scripted, slice: &Slice, workspace: &str, grants: &[Grant],
) -> Result<Report, Error> {
    ask_checked(model, slice, workspace, grants, |_| Ok(Vec::new())).await
}

async fn ask_checked(
    model: &Scripted, slice: &Slice, workspace: &str, grants: &[Grant],
    check: impl FnMut(&Report) -> Result<Vec<String>, Error> + Send,
) -> Result<Report, Error> {
    ask_scaffolded(model, slice, workspace, grants, None, check).await
}

async fn ask_scaffolded(
    model: &Scripted, slice: &Slice, workspace: &str, grants: &[Grant], brief: Option<&str>,
    check: impl FnMut(&Report) -> Result<Vec<String>, Error> + Send,
) -> Result<Report, Error> {
    let scaffold = brief.map(|brief| Scaffold {
        brief,
        passages: "",
        files: &[],
        types: &[],
    });
    ask_guarded(model, slice, workspace, grants, scaffold, check).await
}

async fn ask_guarded(
    model: &Scripted, slice: &Slice, workspace: &str, grants: &[Grant],
    scaffold: Option<Scaffold<'_>>,
    check: impl FnMut(&Report) -> Result<Vec<String>, Error> + Send,
) -> Result<Report, Error> {
    let ctx = BuildContext {
        adapter_id: "target:probe",
        slice,
        workspace,
        grants,
        model,
        scaffold: None,
    };
    let ctx = match scaffold {
        Some(scaffold) => ctx.with_scaffold(scaffold),
        None => ctx,
    };
    emery_sdk::target::build(&ctx, PROSE, check).await
}

// Writes files beneath `root` with the content given, so a report naming
// them passes the tree rule and a write over them meets what they hold.
fn write<'a>(root: &Path, files: impl IntoIterator<Item = (&'a str, &'a str)>) {
    for (file, content) in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().expect("a file has a parent")).expect("written dir");
        fs::write(path, content).expect("written file");
    }
}

// Writes empty files beneath `root`.
fn seed<'a>(root: &Path, files: impl IntoIterator<Item = &'a str>) {
    write(root, files.into_iter().map(|file| (file, "")));
}

fn write_files(id: &str, files: &[(&str, &str)]) -> ToolCall {
    let files: Vec<_> = files
        .iter()
        .map(|(path, content)| serde_json::json!({ "path": path, "content": content }))
        .collect();
    ToolCall {
        id: id.to_owned(),
        name: "write_files".to_owned(),
        arguments: serde_json::json!({ "files": files }).to_string(),
    }
}

fn edit_files(id: &str, edits: &[(&str, &str, &str)]) -> ToolCall {
    let edits: Vec<_> = edits
        .iter()
        .map(|(path, find, replace)| {
            serde_json::json!({ "path": path, "find": find, "replace": replace })
        })
        .collect();
    ToolCall {
        id: id.to_owned(),
        name: "edit_files".to_owned(),
        arguments: serde_json::json!({ "edits": edits }).to_string(),
    }
}

fn restore_files(id: &str, paths: &[&str]) -> ToolCall {
    ToolCall {
        id: id.to_owned(),
        name: "restore_files".to_owned(),
        arguments: serde_json::json!({ "paths": paths }).to_string(),
    }
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read_dir")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// The turn carries the slice's three documents, the lend, writable, and the
// write tool beside the reference tools; the answer is steered by the report
// schema.
#[tokio::test]
async fn request_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs", "Cargo.toml"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    let slice = slice();

    let report = ask(&model, &slice, root).await.expect("accepted");
    assert_eq!(report.covered, ["REQ-001", "REQ-002"]);
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs"], "`written` is answered sorted");

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("BUILD"));
    assert_eq!(request.workspace.as_deref(), Some(root), "the tree is lent");
    assert_eq!(request.worker, None, "no grants put no worker grant");
    assert_eq!(request.tools, ["list_docs", "read_doc", "write_files", "edit_files"]);
    assert!(request.check, "acceptance is the check");
    let SeenFormat::Schema { name, schema } = &request.format else {
        panic!("the report is steered by schema");
    };
    assert_eq!(name, "build-SLICE-001");
    let schema: serde_json::Value = serde_json::from_str(schema).expect("schema parses");
    assert!(schema.pointer("/properties/covered").is_some(), "{schema}");
    assert!(schema.pointer("/properties/written").is_some(), "{schema}");

    let user = &request.messages[0];
    assert!(
        user.starts_with(
            "Build the slice `orders` (SLICE-001) of the plan, bound to adapter \
             `target:probe`.\n\n`$WORKSPACE` is the project tree, lent writable"
        ),
        "{user}"
    );
    assert!(
        user.contains(
            "The tree sits on commit `1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d`, the integrated \
             head this slice builds over."
        ),
        "{user}"
    );
    assert!(
        user.contains(
            "The slice's entry in the plan:\n\n## SLICE-001 orders\n\nBuild the orders \
             module.\n\nThe specification, cut to the slice's requirements:\n\n## \
             REQ-001\n\nOrders are created.\n\n## REQ-002\n\nOrders are listed.\n\nThe design, \
             cut to this slice:\n\n## Types\n\n`Order`\n\nImplement every requirement"
        ),
        "nothing scaffolded puts no section between the design and the rules: {user}"
    );
    assert!(
        user.contains(
            "Write through this call's two tools alone: `write_files` writes one or more files \
             beneath `$WORKSPACE` per call, each created or replaced whole"
        ),
        "{user}"
    );
    assert!(
        user.contains(
            "`edit_files` changes within files the tree holds, each edit replacing the one \
             place its `find` matches."
        ),
        "{user}"
    );
    assert!(
        user.contains(
            "so is a `write_files` entry that would leave a file empty or under half its size \
             unless it sets `shrink: true`"
        ),
        "{user}"
    );
    assert!(user.contains("— `REQ-001`, `REQ-002` — and no other."), "{user}");
    assert!(
        user.contains(
            "`written` lists the files the tools wrote, once each, as a `/`-separated path \
             relative to `$WORKSPACE`, and only a file the tree now holds; a file you wrote and \
             leave out is added for you."
        ),
        "{user}"
    );
    assert!(!user.contains(root), "the lend carries the root, not the brief: {user}");
    assert!(user.ends_with("rather than claim what the tree does not hold."), "{user}");
    model.assert_exhausted();
}

// What the adapter scaffolded before the turn is carried verbatim, after
// the design and before the rules, so the model edits the files it names
// rather than writing them over; the tools and the lend are the same.
#[tokio::test]
async fn build_scaffolded() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs", "Cargo.toml"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    let scaffold = "- `src/orders.rs`: scaffolded; fill `todo!(\"REQ-001 orders.create: the \
                    handler\")`\n- `Cargo.toml`: held\n\n";

    ask_scaffolded(&model, &slice(), root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].tools, ["list_docs", "read_doc", "write_files", "edit_files"]);
    let user = &seen[0].messages[0];
    assert!(
        user.contains(
            "The design, cut to this slice:\n\n## Types\n\n`Order`\n\nWhat the adapter scaffolded \
             for this slice before this turn:\n\n- `src/orders.rs`: scaffolded; fill \
             `todo!(\"REQ-001 orders.create: the handler\")`\n- `Cargo.toml`: held\n\nImplement \
             every requirement"
        ),
        "{user}"
    );
    assert!(
        !user.contains("filled through `edit_files` alone"),
        "no scaffolded path puts no guard sentence: {user}"
    );
    model.assert_exhausted();
}

// The design the engine renders, with a data type per role the brief's cut
// decides over: one the slice owns, one another slice owns that the
// slice's text names, one nothing of the slice names, and a mechanism.
const DESIGN: &str = "# Design\n\n## Domain model\n\nThe shapes the system stores.\n\nType: \
                      orders.order\n```\ninterface Order {\n  id: string;\n\n  lines: \
                      OrderLine[];\n}\n```\n\nType: orders.order-line\n```\ninterface OrderLine \
                      {\n  sku: string;\n}\n```\n\nType: billing.Invoice (src/billing.ts#L3)\n\
                      ```\ninterface Invoice {\n  total: number;\n}\n```\n\nType: start.Config \
                      (mechanism)\n```\nclass Config {\n  readonly port: number;\n}\n```\n\n## \
                      APIs and integrations\n\nThe store holds every order.\n";

// The slice over `DESIGN`, owning `orders.order` with the design's
// signature, whose `lines` field spells the `order-line` type.
fn designed() -> Slice {
    let mut slice = slice();
    DESIGN.clone_into(&mut slice.design);
    "interface Order {\n  id: string;\n\n  lines: OrderLine[];\n}"
        .clone_into(&mut slice.types[0].signature);
    slice
}

// With nothing scaffolded, the design keeps every paragraph, the owned type
// whole, the type the owned signature names whole, and the mechanism, and
// leaves out the type nothing of the slice spells, listing it by key.
#[tokio::test]
async fn build_design_cut() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs", "Cargo.toml"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    let slice = designed();

    ask(&model, &slice, root).await.expect("accepted");

    let seen = model.seen();
    let user = &seen[0].messages[0];
    assert!(
        user.contains(
            "The design, cut to this slice:\n\n# Design\n\n## Domain model\n\nThe shapes the \
             system stores.\n\nType: orders.order\n```\ninterface Order {\n  id: string;\n\n  \
             lines: OrderLine[];\n}\n```\n\nType: orders.order-line\n```\ninterface OrderLine \
             {\n  sku: string;\n}\n```\n\nType: start.Config (mechanism)\n```\nclass Config {\n  \
             readonly port: number;\n}\n```\n\n## APIs and integrations\n\nThe store holds every \
             order.\n\nDesign types left out, owned by other slices and named by none of this \
             slice's text: `billing.Invoice (src/billing.ts#L3)`.\n\nImplement every requirement"
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// A type among the scaffold's `types` is one line pointing at the scaffold
// section, whatever the slice spells; the scaffold's brief is text of the
// slice, so a type it names stays whole.
#[tokio::test]
async fn build_design_cut_scaffolded() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs", "Cargo.toml"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    let slice = designed();
    let types = ["orders.order".to_owned()];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded, `Order` laid; an `Invoice` is read as a value.\n",
        passages: "",
        files: &[],
        types: &types,
    };

    ask_guarded(&model, &slice, root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");

    let seen = model.seen();
    let user = &seen[0].messages[0];
    assert!(
        user.contains(
            "The shapes the system stores.\n\nType: orders.order — laid in the tree by the \
             adapter before this turn, as the scaffold section says.\n\nType: \
             orders.order-line\n```\ninterface OrderLine {\n  sku: string;\n}\n```\n\nType: \
             billing.Invoice (src/billing.ts#L3)\n```\ninterface Invoice {\n  total: \
             number;\n}\n```\n\nType: start.Config (mechanism)\n```\nclass Config {\n  readonly \
             port: number;\n}\n```\n\n## APIs and integrations\n\nThe store holds every \
             order.\n\nWhat the adapter scaffolded for this slice before this turn:"
        ),
        "{user}"
    );
    assert!(!user.contains("Design types left out"), "{user}");
    model.assert_exhausted();
}

// The scaffold's passages stand after the scaffold section and its guard
// sentence, verbatim, and are no text of the slice: a type a passage alone
// names is left out of the design's cut as if nothing named it.
#[tokio::test]
async fn build_design_cut_passages() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs", "Cargo.toml"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    let slice = designed();
    let files = [Guarded::new("src/orders.rs", ["REQ-001"])];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n",
        passages: "From the references, for this slice:\n\n## A probe\n\nAn `Invoice` is read as \
                   a value and answered whole.\n",
        files: &files,
        types: &[],
    };

    ask_guarded(&model, &slice, root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");

    let seen = model.seen();
    let user = &seen[0].messages[0];
    assert!(
        user.contains(
            "Design types left out, owned by other slices and named by none of this slice's \
             text: `billing.Invoice (src/billing.ts#L3)`.\n\nWhat the adapter scaffolded for \
             this slice before this turn:\n\n- `src/orders.rs`: scaffolded\n\nThe file the \
             adapter scaffolded for this slice is filled in place: change it through \
             `edit_files`, or send the whole file through `write_files` with every marker and \
             name the skeleton holds still in it; an entry that drops one is refused whole \
             naming what it dropped, as a `delete` naming one of them is. `restore_files` puts \
             it back as it was scaffolded, the turn's edits to it discarded: recovery after a \
             refusal, not a step.\n\nFrom the references, for this slice:\n\n## A \
             probe\n\nAn `Invoice` is read as a value and answered whole.\n\nImplement every \
             requirement"
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// A key of several words is spelled only where one text of the slice runs
// them together: `order` in one sentence and `line` in another name no
// `order-line`, while `order lines` does.
#[tokio::test]
async fn build_design_cut_phrase() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs", "Cargo.toml"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let mut apart = designed();
    "interface Order {\n  id: string;\n}".clone_into(&mut apart.types[0].signature);
    "## SLICE-001 orders\n\nBuild the orders module. Each line of the listing is one order.\n"
        .clone_into(&mut apart.plan);
    let mut together = apart.clone();
    "## SLICE-001 orders\n\nBuild the orders module, its order lines included.\n"
        .clone_into(&mut together.plan);

    let model = Scripted::answering([VALID]);
    ask(&model, &apart, root).await.expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "Design types left out, owned by other slices and named by none of this slice's \
             text: `orders.order-line`, `billing.Invoice (src/billing.ts#L3)`."
        ),
        "{user}"
    );
    model.assert_exhausted();

    let model = Scripted::answering([VALID]);
    ask(&model, &together, root).await.expect("accepted");
    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "Type: orders.order-line\n```\ninterface OrderLine {\n  sku: string;\n}\n```"
        ),
        "{user}"
    );
    assert!(
        user.contains(
            "Design types left out, owned by other slices and named by none of this slice's \
             text: `billing.Invoice (src/billing.ts#L3)`."
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// A `write_files` entry naming a file the adapter scaffolded is refused with
// the call when its content drops a token the file keeps, naming the tokens
// and the two ways to change it, never `restore_files`, since a model
// offered the way back takes it and loses its edits; a `delete` naming one
// is refused whatever it keeps; the call lands nothing. An edit within the
// file, and a write beside it, land as they would. The brief says so after
// the scaffold section, and the turn declares `restore_files` beside the
// two tools.
#[tokio::test]
async fn write_scaffolded_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let skeleton = "pub fn create() {\n    todo!(\"REQ-001 orders.create: the handler\")\n}\n";
    write(tmp.path(), [("src/orders.rs", skeleton), ("Cargo.toml", "[package]\n")]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [
                write_files(
                    "1",
                    &[
                        ("src/orders.rs", "pub fn create() {}\n"),
                        ("src/lib.rs", "pub mod orders;\n"),
                    ],
                ),
                ToolCall {
                    id: "2".to_owned(),
                    name: "write_files".to_owned(),
                    arguments: r#"{"delete":["./src/orders.rs"]}"#.to_owned(),
                },
                edit_files(
                    "3",
                    &[("src/orders.rs", "todo!(\"REQ-001 orders.create: the handler\")", "Ok(())")],
                ),
                write_files("4", &[("src/lib.rs", "pub mod orders;\n")]),
            ],
        );
    let files = [
        Guarded::new("src/orders.rs", ["pub fn create", "REQ-001 orders.create"]),
        Guarded::new("Cargo.toml", Vec::<String>::new()),
    ];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n- `Cargo.toml`: held\n",
        passages: "",
        files: &files,
        types: &[],
    };

    let report = ask_guarded(&model, &slice(), root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");
    assert_eq!(report.written, ["src/lib.rs", "src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("edited"),
        "pub fn create() {\n    Ok(())\n}\n",
        "the skeleton is edited, never written over"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 5, "four calls, then the check");
    assert_eq!(
        exchanges[0].outcome,
        Err("write_files: nothing written: `src/orders.rs` was scaffolded before this turn and \
             this content drops what the skeleton holds: `REQ-001 orders.create`; change the \
             file through `edit_files`, or send the whole file with those kept"
            .to_owned()),
        "the token the content keeps is not named"
    );
    assert_eq!(
        exchanges[1].outcome,
        Err("write_files: nothing written: `src/orders.rs` was scaffolded before this turn and \
             is not deleted; change it through `edit_files`, which keeps what the skeleton holds"
            .to_owned()),
        "a `delete` is held to the guard as the tree spells the path"
    );
    assert!(exchanges[2].outcome.is_ok(), "the edit lands: {:?}", exchanges[2].outcome);
    assert!(exchanges[3].outcome.is_ok(), "a write beside lands: {:?}", exchanges[3].outcome);

    let request = &model.seen()[0];
    assert_eq!(
        request.tools,
        ["list_docs", "read_doc", "write_files", "edit_files", "restore_files"],
        "a scaffold declares the third tool"
    );
    let user = &request.messages[0];
    assert!(user.contains("Write through this call's three tools alone:"), "{user}");
    assert!(
        user.contains(
            "What the adapter scaffolded for this slice before this turn:\n\n- `src/orders.rs`: \
             scaffolded\n- `Cargo.toml`: held\n\nThe files the adapter scaffolded for this slice \
             are filled in place: change one through `edit_files`, or send the whole file \
             through `write_files` with every marker and name the skeleton holds still in it; \
             an entry that drops one is refused whole naming what it dropped, as a `delete` \
             naming one of them is. `restore_files` puts one back as it was \
             scaffolded, the turn's edits to it discarded: recovery after a refusal, not a \
             step.\n\nImplement every requirement"
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// A whole write of a scaffolded file that keeps every token lands, and so
// does one of a scaffolded file the tree no longer holds, since nothing is
// left there to keep; a file guarded with no tokens is written as any
// other. A refusal names at most ten dropped tokens and counts the rest.
#[tokio::test]
async fn write_scaffolded_kept() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let skeleton = "pub fn create() {\n    todo!(\"REQ-001 orders.create: the handler\")\n}\n";
    let listing = "pub mod orders;\n// token1 token2 token3 token4 token5 token6 token7 token8 \
                   token9 token10 token11 token12\n";
    write(
        tmp.path(),
        [("src/orders.rs", skeleton), ("Cargo.toml", "[package]\n"), ("src/lib.rs", listing)],
    );
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let filled = "//! Orders.\n\npub fn create() -> Result<(), Error> {\n    // REQ-001 \
                  orders.create: the handler\n    Ok(())\n}\n";
    let tokens: Vec<String> = (1..=12).map(|n| format!("token{n}")).collect();
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [
                write_files("1", &[("src/orders.rs", filled)]),
                write_files("2", &[("src/orders/mod.rs", "pub struct Order;\n")]),
                write_files("3", &[("Cargo.toml", "[package]\nname = \"orders\"\n")]),
                write_files("4", &[("src/lib.rs", "pub mod orders;\npub mod billing;\n")]),
            ],
        );
    let files = [
        Guarded::new("src/orders.rs", ["pub fn create", "REQ-001 orders.create"]),
        Guarded::new("src/orders/mod.rs", ["pub struct Order"]),
        Guarded::new("Cargo.toml", Vec::<String>::new()),
        Guarded::new("src/lib.rs", tokens),
    ];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n",
        passages: "",
        files: &files,
        types: &[],
    };

    let report = ask_guarded(&model, &slice(), root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs", "src/orders/mod.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("written"),
        filled,
        "a whole write keeping the tokens lands"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders/mod.rs")).expect("written"),
        "pub struct Order;\n",
        "a guarded path the tree lacked is written"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/lib.rs")).expect("held"),
        listing,
        "the refused write lands nothing"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 5, "four calls, then the check");
    assert!(exchanges[0].outcome.is_ok(), "{:?}", exchanges[0].outcome);
    assert!(exchanges[1].outcome.is_ok(), "the tree lacked it: {:?}", exchanges[1].outcome);
    assert!(exchanges[2].outcome.is_ok(), "no tokens guard nothing: {:?}", exchanges[2].outcome);
    assert_eq!(
        exchanges[3].outcome,
        Err("write_files: nothing written: `src/lib.rs` was scaffolded before this turn and this \
             content drops what the skeleton holds: `token1`, `token2`, `token3`, `token4`, \
             `token5`, `token6`, `token7`, `token8`, `token9`, `token10`, and 2 more; change the \
             file through `edit_files`, or send the whole file with those kept"
            .to_owned())
    );
    model.assert_exhausted();
}

// A scaffolded file named in `files` and `delete` of one call is replaced
// by the entry, the guard over its content holding and the guard over its
// removal never asked, since a model spells a whole write that way; one
// whose entry drops a token is refused for the content alone.
#[tokio::test]
async fn write_scaffolded_replaced() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let skeleton = "pub fn create() {\n    todo!(\"REQ-001 orders.create: the handler\")\n}\n";
    write(tmp.path(), [("src/orders.rs", skeleton)]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let filled = "//! Orders.\n\npub fn create() -> Result<(), Error> {\n    // REQ-001 \
                  orders.create: the handler\n    Ok(())\n}\n";
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [
                ToolCall {
                    id: "1".to_owned(),
                    name: "write_files".to_owned(),
                    arguments: serde_json::json!({
                        "files": [{ "path": "src/orders.rs", "content": "pub fn create() {}\n" }],
                        "delete": ["src/orders.rs"],
                    })
                    .to_string(),
                },
                ToolCall {
                    id: "2".to_owned(),
                    name: "write_files".to_owned(),
                    arguments: serde_json::json!({
                        "files": [{ "path": "src/orders.rs", "content": filled }],
                        "delete": ["./src/orders.rs"],
                    })
                    .to_string(),
                },
            ],
        );
    let files = [Guarded::new("src/orders.rs", ["pub fn create", "REQ-001 orders.create"])];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n",
        passages: "",
        files: &files,
        types: &[],
    };

    let report = ask_guarded(&model, &slice(), root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");
    assert_eq!(report.written, ["src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("replaced"),
        filled,
        "the entry replaces the file, and the delete removes nothing"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "two calls, then the check");
    assert_eq!(
        exchanges[0].outcome,
        Err("write_files: nothing written: `src/orders.rs` was scaffolded before this turn and \
             this content drops what the skeleton holds: `REQ-001 orders.create`; change the \
             file through `edit_files`, or send the whole file with those kept"
            .to_owned()),
        "the content guard alone refuses the replacement"
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(format!(r#"{{"written":[{{"bytes":{},"path":"src/orders.rs"}}]}}"#, filled.len())
            .as_str()),
        "a replacement reports no deletion"
    );
    model.assert_exhausted();
}

// `restore_files` puts a scaffolded file back as the turn found it, so a
// skeleton edited wrong is recovered whole and the turn no longer counts it
// as written; a path that was not scaffolded, or that the tree lacked when
// the turn began, refuses the call whole and restores nothing.
#[tokio::test]
async fn restore_scaffolded() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let skeleton = "pub fn create() {\n    todo!(\"REQ-001 orders.create: the handler\")\n}\n";
    write(tmp.path(), [("src/orders.rs", skeleton), ("Cargo.toml", "[package]\n")]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["Cargo.toml"]}"#])
        .calling(
            0,
            [
                edit_files("1", &[("src/orders.rs", "the handler", "broken")]),
                restore_files("2", &["src/orders.rs", "Cargo.toml"]),
                restore_files("3", &["src/orders.rs", "src/lib.rs"]),
                restore_files("4", &["src/orders/mod.rs"]),
                ToolCall {
                    id: "5".to_owned(),
                    name: "restore_files".to_owned(),
                    arguments: r#"{"paths":[]}"#.to_owned(),
                },
                restore_files("6", &["./src/orders.rs"]),
                write_files("7", &[("Cargo.toml", "[package]\nname = \"orders\"\n")]),
            ],
        );
    let files = [
        Guarded::new("src/orders.rs", ["REQ-001 orders.create"]),
        Guarded::new("src/orders/mod.rs", ["pub struct Order"]),
        Guarded::new("Cargo.toml", Vec::<String>::new()),
    ];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n",
        passages: "",
        files: &files,
        types: &[],
    };

    let report = ask_guarded(&model, &slice(), root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");
    assert_eq!(report.written, ["Cargo.toml"], "a restored file is no longer counted as written");
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("restored"),
        skeleton,
        "the skeleton is back"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 8, "seven calls, then the check");
    assert!(exchanges[0].outcome.is_ok(), "{:?}", exchanges[0].outcome);
    assert_eq!(
        exchanges[1].outcome,
        Ok(r#"{"restored":[{"bytes":68,"path":"src/orders.rs"},{"bytes":10,"path":"Cargo.toml"}]}"#
            .to_owned()),
        "both scaffolded files are put back, the untouched one included"
    );
    assert_eq!(
        exchanges[2].outcome,
        Err("restore_files: nothing restored: `src/lib.rs` was not scaffolded before this turn; \
             `restore_files` puts back the files the brief names as scaffolded"
            .to_owned())
    );
    assert_eq!(
        exchanges[3].outcome,
        Err("restore_files: nothing restored: `src/orders/mod.rs` was scaffolded for this slice \
             but the tree lacked it before this turn; there is nothing to put back"
            .to_owned())
    );
    assert_eq!(exchanges[4].outcome, Err("restore_files: `paths` names no file".to_owned()));
    assert!(
        exchanges[5].outcome.is_ok(),
        "a path is read as the tree spells it: {:?}",
        exchanges[5].outcome
    );
    assert!(exchanges[6].outcome.is_ok(), "{:?}", exchanges[6].outcome);
    model.assert_exhausted();
}

// The adapter's grants ride the turn as the completion's worker grant,
// united with a root two of them spell carried once, beside the lend it
// reaches beyond; the brief says nothing of it, since the grant is the
// host's to apply.
#[tokio::test]
async fn worker_grant() {
    const CARGO: Grant = Grant {
        egress: &["index.crates.io", "static.crates.io"],
        read: &["~/.cargo/bin"],
        env: &[],
    };
    const RUSTUP: Grant = Grant {
        egress: &[],
        read: &["~/.rustup", "~/.cargo/bin"],
        env: &[("RUSTUP_HOME", "~/.rustup")],
    };
    const GRANTS: &[Grant] = &[CARGO, RUSTUP];
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs", "Cargo.toml"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([VALID]);
    let slice = slice();

    ask_as(&model, &slice, root, GRANTS).await.expect("accepted");

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].workspace.as_deref(), Some(root));
    assert_eq!(
        seen[0].worker,
        Some(SeenWorker {
            egress: vec!["index.crates.io".to_owned(), "static.crates.io".to_owned()],
            read: vec!["~/.cargo/bin".to_owned(), "~/.rustup".to_owned()],
            env: vec![("RUSTUP_HOME".to_owned(), "~/.rustup".to_owned())],
        })
    );
    assert!(!seen[0].messages[0].contains("rustup"), "{}", seen[0].messages[0]);
    model.assert_exhausted();
}

// A `write_files` entry that would empty a held file, or leave it under half
// its size, is refused with the call, naming both sizes and the ways out;
// the file stands as it was, and the same write with `shrink` set, or an
// entry at or above half, lands.
#[tokio::test]
async fn write_shrink_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let whole = "pub struct Order;\npub struct Line;\npub struct Total;\n";
    write(tmp.path(), [("src/orders.rs", whole), ("src/empty.rs", "")]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [
                write_files("1", &[("src/orders.rs", ""), ("src/lib.rs", "pub mod orders;\n")]),
                write_files("2", &[("src/orders.rs", "pub struct Order;\n")]),
                ToolCall {
                    id: "3".to_owned(),
                    name: "write_files".to_owned(),
                    arguments: r#"{"files":{"path":"src/orders.rs","content":"pub struct Order;\n","shrink":true}}"#.to_owned(),
                },
                write_files("4", &[("src/orders.rs", "pub struct Order; pub struct Line;\n")]),
                write_files("5", &[("src/empty.rs", "")]),
            ],
        );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/empty.rs", "src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("written"),
        "pub struct Order; pub struct Line;\n"
    );
    assert!(!tmp.path().join("src/lib.rs").exists(), "a refused call lands nothing");

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 6, "five calls, then the check");
    assert_eq!(
        exchanges[0].outcome,
        Err("write_files: nothing written: `src/orders.rs` would shrink from 53 to 0 bytes; a \
             file is replaced whole, so send the whole file, use `edit_files` for a change \
             within it, or set `shrink: true` if that is meant"
            .to_owned())
    );
    assert_eq!(
        exchanges[1].outcome,
        Err("write_files: nothing written: `src/orders.rs` would shrink from 53 to 18 bytes; a \
             file is replaced whole, so send the whole file, use `edit_files` for a change \
             within it, or set `shrink: true` if that is meant"
            .to_owned())
    );
    assert_eq!(
        exchanges[2].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":18,"path":"src/orders.rs"}]}"#),
        "`shrink` lets the write through"
    );
    assert_eq!(
        exchanges[3].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":35,"path":"src/orders.rs"}]}"#),
        "a write that grows a file is under no rule"
    );
    assert_eq!(
        exchanges[4].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":0,"path":"src/empty.rs"}]}"#),
        "an empty file written empty does not shrink"
    );
    assert_eq!(exchanges[5].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A write at or above half the held size is no shrink, so an edit that
// trims a file lands without `shrink`.
#[tokio::test]
async fn write_shrink_allowed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), [("src/orders.rs", "pub struct Order;\npub struct Line;\n")]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(0, [write_files("1", &[("src/orders.rs", "pub struct Order;\n")])]);

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("written"),
        "pub struct Order;\n"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "the write, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":18,"path":"src/orders.rs"}]}"#),
        "18 of 35 bytes is half or more"
    );
    model.assert_exhausted();
}

// A new file written empty is no shrink: the rule reads a file the tree
// holds, and a file the call itself wrote earlier is met as the tree stood
// before the call.
#[tokio::test]
async fn write_empty_new_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/mod.rs"]}"#])
        .calling(
            0,
            [write_files(
                "1",
                &[
                    ("src/mod.rs", ""),
                    ("src/draft.rs", "pub struct Draft;\n"),
                    ("src/draft.rs", ""),
                ],
            )],
        );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/draft.rs", "src/mod.rs"]);
    assert_eq!(fs::read_to_string(tmp.path().join("src/draft.rs")).expect("written"), "");

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "the write, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(
            r#"{"written":[{"bytes":0,"path":"src/mod.rs"},{"bytes":18,"path":"src/draft.rs"},{"bytes":0,"path":"src/draft.rs"}]}"#
        )
    );
    model.assert_exhausted();
}

// `edit_files` replaces the one place each `find` matches, a later edit of a
// file meeting it as the earlier left it, the file written once and counted
// as written; a path spelled as `write_files` accepts it is spelled as the
// tree does in the answer.
#[tokio::test]
async fn edit_tool() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(
        tmp.path(),
        [
            ("src/orders.rs", "pub struct Order;\npub fn total(&token: &str) {}\n"),
            ("Cargo.toml", "[package]\nname = \"orders\"\n"),
        ],
    );
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [edit_files(
                "1",
                &[
                    ("./src//orders.rs", "&token", "token"),
                    (
                        "src/orders.rs",
                        "pub fn total(token: &str) {}",
                        "pub fn total(token: &str) -> u32 { 0 }",
                    ),
                    ("Cargo.toml", "name = \"orders\"\n", ""),
                ],
            )],
        );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs"], "an edited file is written");
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("edited"),
        "pub struct Order;\npub fn total(token: &str) -> u32 { 0 }\n"
    );
    assert_eq!(fs::read_to_string(tmp.path().join("Cargo.toml")).expect("edited"), "[package]\n");

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "the edit, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"edited":[{"bytes":10,"path":"Cargo.toml"},{"bytes":57,"path":"src/orders.rs"}]}"#)
    );
    assert_eq!(exchanges[1].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A `find` matching nowhere, or more than once, refuses the call whole,
// naming the path and the count, and no file is written, the edits before it
// in the call included; an empty `find` is refused the same way.
#[tokio::test]
async fn edit_ambiguous() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let before = "pub struct Order;\npub struct Line;\n";
    write(tmp.path(), [("src/orders.rs", before)]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [
            edit_files(
                "1",
                &[
                    ("src/orders.rs", "Order", "Purchase"),
                    ("src/orders.rs", "pub struct", "pub(crate) struct"),
                    ("src/orders.rs", "struct Total", "struct Sum"),
                    ("src/orders.rs", "", "x"),
                ],
            ),
            ToolCall {
                id: "2".to_owned(),
                name: "edit_files".to_owned(),
                arguments: r#"{"edits":[]}"#.to_owned(),
            },
        ],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, Vec::<String>::new());
    assert_eq!(fs::read_to_string(tmp.path().join("src/orders.rs")).expect("as it was"), before);

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "two calls, then the check");
    assert_eq!(
        exchanges[0].outcome,
        Err("edit_files: nothing edited: `src/orders.rs`: `find` matches 2 places in the file; \
             `src/orders.rs`: `find` matches nothing in the file; no line holds its first line; \
             the file has 2 lines and opens with `pub struct Purchase;`; `src/orders.rs`: `find` \
             is empty"
            .to_owned())
    );
    assert_eq!(exchanges[1].outcome, Err("edit_files: `edits` names no edit".to_owned()));
    model.assert_exhausted();
}

// A `find` matching nothing is refused with what the file holds nearest it:
// the line its first line is found on and the line after, where a `find`
// over a doc comment's blank `//!` parts from the file; the lines after the
// first that match, and the line where the file parts from the `find`
// after them, so a `find` of a function whose one line differs comes back
// with that line; the occurrence of the first line the most lines follow,
// where it is found more than once; the found line alone when it is the
// last; a `find` the file ends within; the file's opening line and length
// when no line holds it; and the file's emptiness.
#[tokio::test]
async fn edit_nearest_line() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let create = "pub fn create() {\n    let order = Order::new();\n    order.save()?;\n    \
                  Ok(())\n}\n";
    let twice = "fn a() {\n    x();\n}\nfn b() {\n    x();\n    y();\n}\n";
    write(
        tmp.path(),
        [
            ("src/lib.rs", "//! Orders.\n//!\n//! The crate.\npub mod orders;\n"),
            ("src/create.rs", create),
            ("src/twice.rs", twice),
            ("src/last.rs", "pub struct A;\n    pub struct B;\n"),
            ("src/ends.rs", "pub struct A;\npub struct B;\n"),
            ("src/one.rs", "pub struct One;\n"),
            ("src/empty.rs", ""),
        ],
    );
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [edit_files(
            "1",
            &[
                ("src/lib.rs", "//! Orders.\n\n//! The crate.", "//! Orders.\n//!\n//! A crate."),
                (
                    "src/create.rs",
                    "pub fn create() {\n    let order = Order::new();\n    order.save();\n    \
                     Ok(())\n}",
                    "pub fn create() {}",
                ),
                ("src/twice.rs", "    x();\n    y();\n    z();", "    z();"),
                ("src/last.rs", "  pub struct B;\n}", "pub struct B;"),
                ("src/ends.rs", "pub struct A;\npub struct B;\npub struct C;", "pub struct C;"),
                ("src/one.rs", "pub struct Uno;", "pub struct One;"),
                ("src/empty.rs", "x", "y"),
            ],
        )],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, Vec::<String>::new());

    let exchanges = model.exchanges();
    assert_eq!(
        exchanges[0].outcome,
        Err("edit_files: nothing edited: \
             `src/lib.rs`: `find` matches nothing in the file; its first line is on line 1 of 4, \
             `//! Orders.`, and line 2 is `//!`; \
             `src/create.rs`: `find` matches nothing in the file; its first line is on line 1 \
             of 5, `pub fn create() {`, line 2 matches, and line 3 is `    order.save()?;`; \
             `src/twice.rs`: `find` matches nothing in the file; its first line is on line 5 of \
             7, `    x();`, line 6 matches, and line 7 is `}`; \
             `src/last.rs`: `find` matches nothing in the file; its first line is on line 2 of \
             2, `    pub struct B;`, the last; \
             `src/ends.rs`: `find` matches nothing in the file; its first line is on line 1 of \
             2, `pub struct A;`, line 2 matches, and the file ends there; \
             `src/one.rs`: `find` matches nothing in the file; no line holds its first line; the \
             file's one line is `pub struct One;`; \
             `src/empty.rs`: `find` matches nothing in the file; the file is empty"
            .to_owned())
    );
    model.assert_exhausted();
}

// A `find` that matches nowhere as written matches with its layout
// whitespace read past: a one-line `todo!("…")` meets the three lines the
// formatter broke it over, the indentation before the opening line is
// replaced with it, a `find` ending in a line break and the next line's
// indentation leaves an empty next line as it stands, a comment line and a
// string literal match as written, and a kept space between two words is
// not read past; a literal whose spaces differ, or a `find` matching two
// places modulo layout, is refused as one as written is.
#[tokio::test]
async fn edit_relaid() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let module = "pub(crate) async fn read(input: ReadInput) -> Output {\n    \
                  let _ = (&input, &context);\n    \
                  todo!(\n        \"REQ-001 orders.read: the handler\"\n    )\n}\n\n\
                  // REQ-002 orders.list: the statement\n\
                  pub fn list() {\n    let store_up = probe(\"SELECT 1\")\n        .await;\n\n    \
                  let _ = store_up;\n}\n";
    let twice = "fn a() {\n    todo!(\n        \"x\"\n    )\n}\nfn b() {\n    todo!(\n        \"x\"\n    \
                 )\n}\n";
    let literal = "fn c() {\n    say(\"two  spaces\")\n}\n";
    let words = "pub struct Order;\npubstruct Line;\n";
    write(
        tmp.path(),
        [
            ("src/orders.rs", module),
            ("src/twice.rs", twice),
            ("src/literal.rs", literal),
            ("src/words.rs", words),
        ],
    );
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [
            edit_files(
                "1",
                &[
                    (
                        "src/orders.rs",
                        "    let _ = (&input, &context);\n    todo!(\"REQ-001 orders.read: the \
                         handler\")",
                        "    let _ = &context;\n    Ok(Output::from(input))",
                    ),
                    (
                        "src/orders.rs",
                        "// REQ-002 orders.list: the statement\npub fn list() {\n    let \
                         store_up = probe(\"SELECT 1\").await;\n    ",
                        "// REQ-002 orders.list: the statement\npub fn list() {\n    let \
                         store_up = probe(\"SELECT 1\").await.is_ok();\n    ",
                    ),
                ],
            ),
            edit_files("2", &[("src/twice.rs", "todo!(\"x\")", "x()")]),
            edit_files("3", &[("src/literal.rs", "say(\"two spaces\")", "say(\"two\")")]),
            edit_files("4", &[("src/words.rs", "pub struct Line;", "pub struct Row;")]),
        ],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("edited"),
        "pub(crate) async fn read(input: ReadInput) -> Output {\n    \
         let _ = &context;\n    Ok(Output::from(input))\n}\n\n\
         // REQ-002 orders.list: the statement\n\
         pub fn list() {\n    let store_up = probe(\"SELECT 1\").await.is_ok();\n    \n    \
         let _ = store_up;\n}\n",
        "the marker's three lines and the statement's two are replaced, the empty line kept"
    );
    for untouched in [("src/twice.rs", twice), ("src/literal.rs", literal), ("src/words.rs", words)]
    {
        assert_eq!(
            fs::read_to_string(tmp.path().join(untouched.0)).expect("as it was"),
            untouched.1
        );
    }

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 5, "four calls, then the check");
    assert!(exchanges[0].outcome.is_ok(), "{:?}", exchanges[0].outcome);
    assert_eq!(
        exchanges[1].outcome,
        Err("edit_files: nothing edited: `src/twice.rs`: `find` matches 2 places in the file"
            .to_owned())
    );
    assert_eq!(
        exchanges[2].outcome,
        Err("edit_files: nothing edited: `src/literal.rs`: `find` matches nothing in the file; \
             no line holds its first line; the file has 3 lines and opens with `fn c() {`"
            .to_owned()),
        "a literal's spaces are its own"
    );
    assert_eq!(
        exchanges[3].outcome,
        Err("edit_files: nothing edited: `src/words.rs`: `find` matches nothing in the file; no \
             line holds its first line; the file has 2 lines and opens with `pub struct Order;`"
            .to_owned()),
        "the space between two words is kept"
    );
    model.assert_exhausted();
}

// A `find` matching nothing whose first line is a call the formatter broke
// after its opener is refused naming the opener's line and the line after
// it, the occurrence whose next line shares the most of the call where
// several are broken; and the lines after a found first line match with
// their indentation read past.
#[tokio::test]
async fn edit_nearest_broken() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let module = "fn a() {\n    todo!(\n        \"REQ-001 orders.create: the handler\"\n    \
                  )\n}\nfn b() {\n    todo!(\n        \"REQ-002 orders.list: the handler\"\n    \
                  )\n}\n";
    let indented = "fn c() {\n        let x = 1;\n        let y = 2;\n}\n";
    write(tmp.path(), [("src/orders.rs", module), ("src/indented.rs", indented)]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [edit_files(
            "1",
            &[
                ("src/orders.rs", "todo!(\"REQ-002 orders.list: the hander\")", "Ok(())"),
                ("src/indented.rs", "fn c() {\n    let x = 1;\n    let y = 3;", "fn c() {}"),
            ],
        )],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, Vec::<String>::new());

    let exchanges = model.exchanges();
    assert_eq!(
        exchanges[0].outcome,
        Err("edit_files: nothing edited: \
             `src/orders.rs`: `find` matches nothing in the file; no line holds its first line \
             whole; its opener `todo!(` is line 7 of 10, the call broken over lines after it, \
             and line 8 is `        \"REQ-002 orders.list: the handler\"`; \
             `src/indented.rs`: `find` matches nothing in the file; its first line is on line 1 \
             of 4, `fn c() {`, line 2 matches, and line 3 is `        let y = 2;`"
            .to_owned())
    );
    model.assert_exhausted();
}

// A marker a guarded file names is filled by an edit whose `find` is the
// marker's key alone, the block the scaffold laid replaced whole where the
// file holds it as written or re-laid by the formatter, never the key's
// own text inside it; a key named again once its block is gone is refused
// naming the marker.
#[tokio::test]
async fn edit_marker_key() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let module = "pub(crate) async fn create(input: CreateInput) -> Output {\n    \
                  let _ = (&input, &context);\n    \
                  todo!(\n        \"REQ-001 orders.create: the handler\"\n    )\n}\n\n\
                  pub fn total() {\n    todo!(\"REQ-002 orders.list: the behaviour\")\n}\n";
    let tests = "#[tokio::test]\n#[ignore = \"REQ-001 create: unfilled\"]\nasync fn create() {\n    \
                 let provider = provider();\n    let _ = &provider;\n    \
                 todo!(\"REQ-001 create: arrange, act, assert\")\n}\n";
    write(tmp.path(), [("src/orders.rs", module), ("tests/orders.rs", tests)]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [
                edit_files(
                    "1",
                    &[
                        (
                            "src/orders.rs",
                            "REQ-001 orders.create: the handler",
                            "    let _ = &context;\n    Ok(Output::from(input))",
                        ),
                        ("src/orders.rs", " REQ-002 orders.list: the behaviour ", "    Ok(())"),
                        (
                            "tests/orders.rs",
                            "REQ-001 create: unfilled",
                            "async fn create() {\n    let (status, _) = send(provider()).await;\n    \
                             assert_eq!(status, 201);\n}",
                        ),
                    ],
                ),
                edit_files("2", &[("src/orders.rs", "REQ-001 orders.create: the handler", "")]),
            ],
        );
    let files = [
        Guarded::new("src/orders.rs", ["REQ-001 orders.create", "REQ-002 orders.list"])
            .with_markers([
                Marker::new(
                    "REQ-001 orders.create: the handler",
                    "    let _ = (&input, &context);\n    todo!(\"REQ-001 orders.create: the \
                     handler\")",
                ),
                Marker::new(
                    "REQ-002 orders.list: the behaviour",
                    "    todo!(\"REQ-002 orders.list: the behaviour\")",
                ),
            ]),
        Guarded::new("tests/orders.rs", ["REQ-001 create"]).with_markers([Marker::new(
            "REQ-001 create: unfilled",
            "#[ignore = \"REQ-001 create: unfilled\"]\nasync fn create() {\n    let provider = \
             provider();\n    let _ = &provider;\n    todo!(\"REQ-001 create: arrange, act, \
             assert\")\n}",
        )]),
    ];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n- `tests/orders.rs`: scaffolded\n",
        passages: "",
        files: &files,
        types: &[],
    };

    let report = ask_guarded(&model, &slice(), root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");
    assert_eq!(report.written, ["src/orders.rs", "tests/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("edited"),
        "pub(crate) async fn create(input: CreateInput) -> Output {\n    \
         let _ = &context;\n    Ok(Output::from(input))\n}\n\n\
         pub fn total() {\n    Ok(())\n}\n",
        "each block is replaced whole, the re-laid one included"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("tests/orders.rs")).expect("edited"),
        "#[tokio::test]\nasync fn create() {\n    let (status, _) = send(provider()).await;\n    \
         assert_eq!(status, 201);\n}\n",
        "the test's block is replaced from its `#[ignore]` line"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "two calls, then the check");
    assert!(exchanges[0].outcome.is_ok(), "{:?}", exchanges[0].outcome);
    assert_eq!(
        exchanges[1].outcome,
        Err("edit_files: nothing edited: `src/orders.rs`: `find` names the marker `REQ-001 \
             orders.create: the handler`, but the file no longer holds the block the scaffold \
             laid for it; copy what stands there into `find`"
            .to_owned())
    );
    model.assert_exhausted();
}

// A key is a marker of the file it guards alone, plain text in any other;
// and the brief says how a key is used where any scaffolded file carries
// one.
#[tokio::test]
async fn edit_marker_scoped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(
        tmp.path(),
        [
            (
                "src/orders.rs",
                "pub fn total() {\n    todo!(\"REQ-002 orders.list: the behaviour\")\n}\n",
            ),
            ("tests/orders.rs", "#[tokio::test]\nasync fn create() {}\n"),
        ],
    );
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [edit_files("1", &[("tests/orders.rs", "REQ-002 orders.list: the behaviour", "")])],
    );
    let files = [
        Guarded::new("src/orders.rs", ["REQ-002 orders.list"]).with_markers([Marker::new(
            "REQ-002 orders.list: the behaviour",
            "    todo!(\"REQ-002 orders.list: the behaviour\")",
        )]),
        Guarded::new("tests/orders.rs", Vec::<String>::new()),
    ];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n- `tests/orders.rs`: scaffolded\n",
        passages: "",
        files: &files,
        types: &[],
    };

    let report = ask_guarded(&model, &slice(), root, &[], Some(scaffold), |_| Ok(Vec::new()))
        .await
        .expect("accepted");
    assert_eq!(report.written, Vec::<String>::new());

    let exchanges = model.exchanges();
    assert_eq!(
        exchanges[0].outcome,
        Err("edit_files: nothing edited: `tests/orders.rs`: `find` matches nothing in the file; \
             no line holds its first line; the file has 2 lines and opens with `#[tokio::test]`"
            .to_owned())
    );
    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "recovery after a refusal, not a step. A marker the brief quotes a key for is filled \
             by one `edit_files` edit whose `find` is that key alone and whose `replace` is the \
             text that takes the block's place, so its lines are never copied; a `find` copied \
             from the brief matches where the formatter has broken its lines or moved its \
             indentation.\n\nImplement every requirement"
        ),
        "{user}"
    );
    model.assert_exhausted();
}

// An edit of a file the tree does not hold, or at a path the write rule
// refuses, refuses the call whole as a write there would.
#[tokio::test]
async fn edit_unheld() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), [("src/orders.rs", "pub struct Order;\n")]);
    symlink("orders.rs", tmp.path().join("src/link.rs")).expect("file symlink");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [edit_files(
            "1",
            &[
                ("src/orders.rs", "Order", "Purchase"),
                ("src/missing.rs", "Order", "Purchase"),
                ("src/link.rs", "Order", "Purchase"),
                ("../outside.rs", "Order", "Purchase"),
                (".emery/storage/x", "Order", "Purchase"),
            ],
        )],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, Vec::<String>::new());
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("as it was"),
        "pub struct Order;\n"
    );

    let exchanges = model.exchanges();
    assert_eq!(
        exchanges[0].outcome,
        Err("edit_files: nothing edited: `src/missing.rs` names no file in the tree; \
             `src/link.rs` is a symlink; `../outside.rs` escapes the root; `.emery/storage/x` \
             is under the reserved `.emery/`"
            .to_owned())
    );
    model.assert_exhausted();
}

// One `write_files` call lands every file it names beneath the lend, the
// directories above each created; a call naming one path the rule refuses
// writes nothing, every refusal named, as a call naming no file or missing a
// content does; the report naming what was written is accepted.
#[tokio::test]
async fn write_tool() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"covered":["REQ-001"],"written":["src/orders.rs","Cargo.toml"]}"#,
    ])
    .calling(
        0,
        [
            write_files(
                "1",
                &[("./src//orders.rs", "pub struct Order;\n"), ("Cargo.toml", "[package]\n")],
            ),
            write_files(
                "2",
                &[
                    ("src/lib.rs", "pub mod orders;\n"),
                    ("../escape.rs", ""),
                    (".emery/storage/x", ""),
                    ("docs/spec.md", ""),
                ],
            ),
            ToolCall {
                id: "3".to_owned(),
                name: "write_files".to_owned(),
                arguments: r#"{"files":[{"path":"src/lib.rs"}]}"#.to_owned(),
            },
            write_files("4", &[]),
        ],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("written"),
        "pub struct Order;\n"
    );
    assert_eq!(fs::read_to_string(tmp.path().join("Cargo.toml")).expect("written"), "[package]\n");
    assert_eq!(names(tmp.path()), ["Cargo.toml", "src"], "the one accepted call lands alone");
    assert_eq!(names(&tmp.path().join("src")), ["orders.rs"], "the refused call wrote nothing");

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 5, "four tool calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":18,"path":"src/orders.rs"},{"bytes":10,"path":"Cargo.toml"}]}"#),
        "the answer spells each path as the tree does, in call order"
    );
    assert_eq!(
        exchanges[1].outcome,
        Err("write_files: nothing written: `../escape.rs` escapes the root; `.emery/storage/x` \
             is under the reserved `.emery/`; `docs/spec.md` names the engine's own `spec.md`"
            .to_owned())
    );
    let malformed = exchanges[2].outcome.as_deref().expect_err("a file without content");
    assert!(malformed.starts_with("write_files: invalid arguments"), "{malformed}");
    assert_eq!(
        exchanges[3].outcome,
        Err("write_files: `files` names no file and `delete` no path".to_owned())
    );
    assert_eq!(exchanges[4].tool, "check");
    assert_eq!(exchanges[4].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// `files` spelled as a map from path to content is read as the listed shape
// is, in the order the call spells it.
#[tokio::test]
async fn write_map_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"covered":["REQ-001"],"written":["src/orders.rs","Cargo.toml"]}"#,
    ])
    .calling(
        0,
        [ToolCall {
            id: "1".to_owned(),
            name: "write_files".to_owned(),
            arguments:
                r#"{"files":{"src/orders.rs":"pub struct Order;\n","Cargo.toml":"[package]\n"}}"#
                    .to_owned(),
        }],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("written"),
        "pub struct Order;\n"
    );
    assert_eq!(fs::read_to_string(tmp.path().join("Cargo.toml")).expect("written"), "[package]\n");

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "the write, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":18,"path":"src/orders.rs"},{"bytes":10,"path":"Cargo.toml"}]}"#)
    );
    model.assert_exhausted();
}

// `files` sent as a JSON string — the list, the map, or one entry,
// serialised before the call — is read as the shape it spells; a string
// spelling no JSON is refused naming the shapes expected.
#[tokio::test]
async fn write_string_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"covered":["REQ-001"],"written":["src/orders.rs","Cargo.toml","src/lib.rs"]}"#,
    ])
    .calling(
        0,
        [
            ToolCall {
                id: "1".to_owned(),
                name: "write_files".to_owned(),
                arguments: serde_json::json!({
                    "files": r#"[{"path":"src/orders.rs","content":"pub struct Order;\n"}]"#
                })
                .to_string(),
            },
            ToolCall {
                id: "2".to_owned(),
                name: "write_files".to_owned(),
                arguments: serde_json::json!({ "files": r#"{"Cargo.toml":"[package]\n"}"# })
                    .to_string(),
            },
            ToolCall {
                id: "3".to_owned(),
                name: "write_files".to_owned(),
                arguments: serde_json::json!({
                    "files": r#"{"path":"src/lib.rs","content":"pub mod orders;\n"}"#
                })
                .to_string(),
            },
            ToolCall {
                id: "4".to_owned(),
                name: "write_files".to_owned(),
                arguments: serde_json::json!({ "files": "src/lib.rs: pub mod orders;" })
                    .to_string(),
            },
        ],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["Cargo.toml", "src/lib.rs", "src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("written"),
        "pub struct Order;\n"
    );
    assert_eq!(fs::read_to_string(tmp.path().join("Cargo.toml")).expect("written"), "[package]\n");
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/lib.rs")).expect("written"),
        "pub mod orders;\n"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 5, "four calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":18,"path":"src/orders.rs"}]}"#)
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":10,"path":"Cargo.toml"}]}"#)
    );
    assert_eq!(
        exchanges[2].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":16,"path":"src/lib.rs"}]}"#)
    );
    let prose = exchanges[3].outcome.as_deref().expect_err("a string that is not JSON");
    assert!(prose.starts_with("write_files: invalid arguments"), "{prose}");
    assert!(
        prose.contains(
            "a string that is not JSON (expected value at line 1 column 1), where a list of \
             `{path, content}` entries, one such entry, or a map from path to content is expected"
        ),
        "{prose}"
    );
    assert_eq!(exchanges[4].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// `files` spelled as one `{path, content}` object, the shape the schema
// declares for an entry, is that one file, never a map of two files named
// `path` and `content`, and so is that object spelled bare at the root of
// the call, `delete` riding beside it; one missing its content, or carrying
// another key, is malformed as a listed entry would be, and a call spelling
// `files` and a bare entry is refused whole.
#[tokio::test]
async fn write_object_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
        0,
        [
            ToolCall {
                id: "1".to_owned(),
                name: "write_files".to_owned(),
                arguments: r#"{"files":{"path":"src/orders.rs","content":"pub struct Order;\n"}}"#
                    .to_owned(),
            },
            ToolCall {
                id: "2".to_owned(),
                name: "write_files".to_owned(),
                arguments: r#"{"files":{"path":"src/lib.rs"}}"#.to_owned(),
            },
            ToolCall {
                id: "3".to_owned(),
                name: "write_files".to_owned(),
                arguments: r#"{"files":{"path":"src/lib.rs","content":"","mode":"644"}}"#
                    .to_owned(),
            },
            ToolCall {
                id: "4".to_owned(),
                name: "write_files".to_owned(),
                arguments:
                    r#"{"path":"src/lib.rs","content":"pub mod orders;\n","delete":["src/old.rs"]}"#
                        .to_owned(),
            },
            ToolCall {
                id: "5".to_owned(),
                name: "write_files".to_owned(),
                arguments:
                    r#"{"files":[{"path":"src/a.rs","content":""}],"path":"src/b.rs","content":""}"#
                        .to_owned(),
            },
        ],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/lib.rs", "src/orders.rs"]);
    assert_eq!(names(tmp.path()), ["src"], "no file named `path` or `content`");
    assert_eq!(names(&tmp.path().join("src")), ["lib.rs", "orders.rs"]);

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 6, "five calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":18,"path":"src/orders.rs"}]}"#)
    );
    let missing = exchanges[1].outcome.as_deref().expect_err("an entry without content");
    assert!(missing.contains("missing field `content`"), "{missing}");
    let unknown = exchanges[2].outcome.as_deref().expect_err("an entry with a stray key");
    assert!(unknown.contains("unknown field `mode`"), "{unknown}");
    assert_eq!(
        exchanges[3].outcome.as_deref(),
        Ok(r#"{"deleted":["src/old.rs"],"written":[{"bytes":16,"path":"src/lib.rs"}]}"#),
        "the bare entry is the one file, and `delete` rides beside it"
    );
    let both = exchanges[4].outcome.as_deref().expect_err("`files` beside a bare entry");
    assert!(both.contains("cannot both be given"), "{both}");
    assert_eq!(exchanges[5].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// `edits` spelled as one `{path, find, replace}` object, or that object
// spelled bare at the root of the call, is that one edit, as a write's
// one file is; one missing its `find`, or carrying another key, is
// malformed as a listed edit would be, and a call spelling `edits` and a
// bare edit is refused whole.
#[tokio::test]
async fn edit_object_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), [("src/orders.rs", "pub struct Order;\npub struct Line;\n")]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [
                ToolCall {
                    id: "1".to_owned(),
                    name: "edit_files".to_owned(),
                    arguments:
                        r#"{"edits":{"path":"src/orders.rs","find":"Order","replace":"Purchase"}}"#
                            .to_owned(),
                },
                ToolCall {
                    id: "2".to_owned(),
                    name: "edit_files".to_owned(),
                    arguments: r#"{"path":"src/orders.rs","find":"Line","replace":"Item"}"#
                        .to_owned(),
                },
                ToolCall {
                    id: "3".to_owned(),
                    name: "edit_files".to_owned(),
                    arguments: r#"{"edits":{"path":"src/orders.rs","replace":"Row"}}"#.to_owned(),
                },
                ToolCall {
                    id: "4".to_owned(),
                    name: "edit_files".to_owned(),
                    arguments:
                        r#"{"path":"src/orders.rs","find":"Item","replace":"Row","mode":"644"}"#
                            .to_owned(),
                },
                ToolCall {
                    id: "5".to_owned(),
                    name: "edit_files".to_owned(),
                    arguments:
                        r#"{"edits":[],"path":"src/orders.rs","find":"Item","replace":"Row"}"#
                            .to_owned(),
                },
            ],
        );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("edited"),
        "pub struct Purchase;\npub struct Item;\n",
        "the two bare edits landed and the three refused calls changed nothing"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 6, "five calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"edited":[{"bytes":38,"path":"src/orders.rs"}]}"#)
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"edited":[{"bytes":38,"path":"src/orders.rs"}]}"#)
    );
    let missing = exchanges[2].outcome.as_deref().expect_err("an edit without `find`");
    assert!(missing.contains("missing field `find`"), "{missing}");
    let unknown = exchanges[3].outcome.as_deref().expect_err("an edit with a stray key");
    assert!(unknown.contains("unknown field `mode`"), "{unknown}");
    let both = exchanges[4].outcome.as_deref().expect_err("`edits` beside a bare edit");
    assert!(both.contains("cannot both be given"), "{both}");
    assert_eq!(exchanges[5].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// `edits` sent as a JSON string — the list, or one edit — is read as the
// shape it spells; a string spelling no JSON is refused naming the shapes
// expected.
#[tokio::test]
async fn edit_string_shape() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), [("src/orders.rs", "pub struct Order;\npub struct Line;\n")]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [
                ToolCall {
                    id: "1".to_owned(),
                    name: "edit_files".to_owned(),
                    arguments: serde_json::json!({
                        "edits": r#"[{"path":"src/orders.rs","find":"Order","replace":"Purchase"}]"#
                    })
                    .to_string(),
                },
                ToolCall {
                    id: "2".to_owned(),
                    name: "edit_files".to_owned(),
                    arguments: serde_json::json!({
                        "edits": r#"{"path":"src/orders.rs","find":"Line","replace":"Item"}"#
                    })
                    .to_string(),
                },
                ToolCall {
                    id: "3".to_owned(),
                    name: "edit_files".to_owned(),
                    arguments: serde_json::json!({ "edits": "replace Item with Row" }).to_string(),
                },
            ],
        );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("edited"),
        "pub struct Purchase;\npub struct Item;\n"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4, "three calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"edited":[{"bytes":38,"path":"src/orders.rs"}]}"#)
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"edited":[{"bytes":38,"path":"src/orders.rs"}]}"#)
    );
    let prose = exchanges[2].outcome.as_deref().expect_err("a string that is not JSON");
    assert!(prose.starts_with("edit_files: invalid arguments"), "{prose}");
    assert!(
        prose.contains(
            "a string that is not JSON (expected value at line 1 column 1), where a list of \
             `{path, find, replace}` edits, or one such edit is expected"
        ),
        "{prose}"
    );
    assert_eq!(exchanges[3].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A path that leads through a symlink — a directory on the way or the file
// itself — is refused as the lexical rule's refusals are, whatever the link
// points at, since a write or a removal would follow it wherever the mount
// reaches; one through a file, or at a directory, is refused beside it; a
// file under directories not yet made still lands; and a `written` path at
// a link names no regular file.
#[tokio::test]
async fn write_through_symlink() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let outside = tmp.path().join("outside");
    fs::create_dir(&outside).expect("outside dir");
    fs::write(outside.join("config"), "[core]\n").expect("outside file");
    let tree = tmp.path().join("tree");
    seed(&tree, ["src/real.rs"]);
    symlink("../outside", tree.join("link")).expect("dir symlink");
    symlink("../../outside/config", tree.join("src/config")).expect("file symlink");
    let root = tree.to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"covered":["REQ-001"],"written":["src/config"]}"#,
        r#"{"covered":["REQ-001"],"written":["src/new/mod.rs"]}"#,
    ])
    .calling(
        0,
        [
            write_files(
                "1",
                &[
                    ("link/hook", "#!/bin/sh\n"),
                    ("src/config", "[core]\n\thooksPath = hooks\n"),
                    ("src/real.rs/x.rs", ""),
                    ("src", ""),
                ],
            ),
            ToolCall {
                id: "2".to_owned(),
                name: "write_files".to_owned(),
                arguments: r#"{"delete":["link/config","src/config"]}"#.to_owned(),
            },
            write_files("3", &[("src/new/mod.rs", "pub mod new;\n")]),
        ],
    );

    let report = ask(&model, &slice(), root).await.expect("corrected");
    assert_eq!(report.written, ["src/new/mod.rs"]);
    assert_eq!(fs::read_to_string(outside.join("config")).expect("untouched"), "[core]\n");
    assert_eq!(names(&outside), ["config"], "nothing landed outside the tree");
    assert!(
        tree.join("src/config")
            .symlink_metadata()
            .expect("the link stands")
            .file_type()
            .is_symlink()
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 5, "three calls, the refused check, the accepted check");
    assert_eq!(
        exchanges[0].outcome,
        Err("write_files: nothing written: `link/hook` leads through the symlink `link`; \
             `src/config` is a symlink; `src/real.rs/x.rs` leads through `src/real.rs`, which \
             is not a directory; `src` names a directory"
            .to_owned())
    );
    assert_eq!(
        exchanges[1].outcome,
        Err("write_files: nothing written: `link/config` leads through the symlink `link`; \
             `src/config` is a symlink"
            .to_owned())
    );
    assert_eq!(
        exchanges[2].outcome.as_deref(),
        Ok(r#"{"written":[{"bytes":13,"path":"src/new/mod.rs"}]}"#)
    );
    let correction = exchanges[3].outcome.as_ref().expect_err("a link is no regular file");
    assert!(
        correction.contains("- written `src/config` names no regular file under the lent tree"),
        "{correction}"
    );
    assert_eq!(exchanges[4].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// `delete` removes the files it names beside the files written, one the
// tree lacks counting as removed; a path the rule refuses refuses the call
// whole; a call that only deletes is a call; and a file written then
// removed is not filled into `written`.
#[tokio::test]
async fn write_delete() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/old.rs", "src/stale.rs"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [
            ToolCall {
                id: "1".to_owned(),
                name: "write_files".to_owned(),
                arguments: r#"{"files":[{"path":"src/orders.rs","content":"pub struct Order;\n"},{"path":"src/draft.rs","content":""}],"delete":["src/old.rs","src/never.rs"]}"#
                    .to_owned(),
            },
            ToolCall {
                id: "2".to_owned(),
                name: "write_files".to_owned(),
                arguments: r#"{"delete":["src/stale.rs","../escape.rs"]}"#.to_owned(),
            },
            ToolCall {
                id: "3".to_owned(),
                name: "write_files".to_owned(),
                arguments: r#"{"delete":["src/stale.rs","src/draft.rs"]}"#.to_owned(),
            },
        ],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/orders.rs"], "the file written and kept");
    assert_eq!(names(&tmp.path().join("src")), ["orders.rs"]);

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4, "three calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(
            r#"{"deleted":["src/old.rs","src/never.rs"],"written":[{"bytes":18,"path":"src/orders.rs"},{"bytes":0,"path":"src/draft.rs"}]}"#
        )
    );
    assert_eq!(
        exchanges[1].outcome,
        Err("write_files: nothing written: `../escape.rs` escapes the root".to_owned())
    );
    assert_eq!(
        exchanges[2].outcome.as_deref(),
        Ok(r#"{"deleted":["src/stale.rs","src/draft.rs"],"written":[]}"#)
    );
    assert_eq!(exchanges[3].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A path one call names more than once is met as the call left it, not as
// the tree stood before it: one twice in `files` holds the later content,
// and one in `files` and `delete` is the entry's, written and kept, since a
// model spells a whole write as a removal and a write of the path; the
// answer lists it written and not deleted, and a path `delete` alone names
// is removed beside it.
#[tokio::test]
async fn write_path_twice() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/scratch.rs", "src/stale.rs"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [ToolCall {
            id: "1".to_owned(),
            name: "write_files".to_owned(),
            arguments: r#"{"files":[{"path":"src/scratch.rs","content":"// scratch\n"},{"path":"src/orders.rs","content":"pub struct Draft;\n"},{"path":"src/orders.rs","content":"pub struct Order;\n"}],"delete":["src/scratch.rs","src/stale.rs"]}"#
                .to_owned(),
        }],
    );

    let report = ask(&model, &slice(), root).await.expect("accepted");
    assert_eq!(report.written, ["src/orders.rs", "src/scratch.rs"], "the files written and kept");
    assert_eq!(names(&tmp.path().join("src")), ["orders.rs", "scratch.rs"]);
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/orders.rs")).expect("written"),
        "pub struct Order;\n",
        "the later entry replaces the earlier"
    );
    assert_eq!(
        fs::read_to_string(tmp.path().join("src/scratch.rs")).expect("written"),
        "// scratch\n",
        "the entry replaces the file the delete also names"
    );

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2, "one call, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(
            r#"{"deleted":["src/stale.rs"],"written":[{"bytes":11,"path":"src/scratch.rs"},{"bytes":18,"path":"src/orders.rs"},{"bytes":18,"path":"src/orders.rs"}]}"#
        )
    );
    assert_eq!(exchanges[1].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A `written` path the tree does not hold is a finding; a file `write_files`
// wrote that `written` leaves out is none, since the accepted report is
// filled in with it, every path spelled as the tree does, and the write
// stands across rounds.
#[tokio::test]
async fn written_unheld() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"covered":["REQ-001"],"written":["src/missing.rs"]}"#,
        r#"{"covered":["REQ-001"],"written":["./Cargo.toml"]}"#,
    ])
    .calling(
        0,
        [write_files("1", &[("src/orders.rs", "pub struct Order;\n"), ("Cargo.toml", "")])],
    );

    let report = ask(&model, &slice(), root).await.expect("corrected");
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs"], "filled from what was written");

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 3, "the write, the refused check, the accepted check");
    let correction = exchanges[1].outcome.as_ref().expect_err("the first report is refused");
    assert_eq!(
        correction.lines().filter(|line| line.starts_with("- ")).collect::<Vec<_>>(),
        ["- written `src/missing.rs` names no regular file under the lent tree"],
        "{correction}"
    );
    assert_eq!(exchanges[2].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// The build turn offers the adapter's references and the runtime's through
// the same tools a mining turn has, over the adapter's references alone: no
// system document is listed — `build.md` rides this turn's system — and the
// runtime references a source's turns import are neither listed nor read,
// since the claim rules and the reconciliation teach a build nothing; a read
// still answers an unlisted document of the adapter's.
#[tokio::test]
async fn doc_refs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs", "Cargo.toml"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
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
                arguments: r#"{"path":"references/layout.md"}"#.to_string(),
            },
            ToolCall {
                id: "3".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"build.md"}"#.to_string(),
            },
            ToolCall {
                id: "4".to_string(),
                name: "read_doc".to_string(),
                arguments: r#"{"path":"reconciliation.md"}"#.to_string(),
            },
        ],
    );

    ask(&model, &slice(), root).await.expect("accepted");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 5, "four reference calls, then the check");
    assert_eq!(
        exchanges[0].outcome.as_deref(),
        Ok(r#"{"paths":["references/layout.md"]}"#),
        "`list_docs` lists the adapter's references alone"
    );
    assert_eq!(
        exchanges[1].outcome.as_deref(),
        Ok(r#"{"body":"One module per requirement.","path":"references/layout.md"}"#)
    );
    assert_eq!(
        exchanges[2].outcome.as_deref(),
        Ok(r#"{"body":"BUILD","path":"build.md"}"#),
        "`read_doc` still answers an unlisted document"
    );
    assert_eq!(
        exchanges[3].outcome.as_ref().map(String::as_str).map_err(String::as_str),
        Err("no document `reconciliation.md`"),
        "a source's runtime reference is no build's to read"
    );
    assert_eq!(exchanges[4].tool, "check");
    assert_eq!(exchanges[4].outcome, Ok(String::new()));
    model.assert_exhausted();
}

// A corpus without `build.md` is the adapter build's own defect.
#[tokio::test]
async fn missing_prompt() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::default();
    let slice = slice();
    let ctx = BuildContext {
        adapter_id: "target:mute",
        slice: &slice,
        workspace: root,
        grants: &[],
        model: &model,
        scaffold: None,
    };

    let error = emery_sdk::target::build(&ctx, &[], |_| Ok(Vec::new()))
        .await
        .expect_err("no prompt to ask with");
    assert_eq!(error.code(), "server_error");
    assert!(error.description().contains("`build.md` is not embedded"), "{error}");
    assert_eq!(model.seen(), [] as [Seen; 0]);
}

// Every rule the slice can hold the report to is a finding, all returned
// together, and the corrected report is accepted; a requirement left
// uncovered is no finding, since the gate holds what is claimed, not what is
// missing.
#[tokio::test]
async fn gate_findings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"covered":["REQ-001","REQ-009","REQ-001"],
            "written":["src/orders.rs","../escape.rs","./","src/orders.rs",".emery/storage/x"]}"#,
        r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#,
    ]);
    let slice = slice();

    let report = ask(&model, &slice, root).await.expect("corrected");
    assert_eq!(report.covered, ["REQ-001"]);
    assert_eq!(report.uncovered(&slice), ["REQ-002"]);

    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the first report is refused");
    for finding in [
        "- covered `REQ-009` is not a requirement of slice `SLICE-001`; its requirements are \
         REQ-001, REQ-002",
        "- covered `REQ-001` is listed twice",
        "- written `../escape.rs` escapes the root",
        "- written `./` names no file",
        "- written `src/orders.rs` is listed twice",
        "- written `.emery/storage/x` is under the reserved `.emery/`",
    ] {
        assert!(correction.contains(finding), "{finding}: {correction}");
    }
    assert!(exchanges[1].outcome.is_ok(), "the corrected report is accepted");
    model.assert_exhausted();
}

// A report that never lands within the rounds is the operator's `bad_request`,
// as a mining turn's is.
#[tokio::test]
async fn rounds_exhausted() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-404"],"written":[]}"#]);

    let error = ask(&model, &slice(), root).await.expect_err("never accepted");
    assert_eq!(error.code(), "bad_request", "{error}");
    model.assert_exhausted();
}

// The adapter's check reads the tree as the model left it: its finding
// joins the gate's under the one correction, and the candidate that
// repaired the tree is accepted, the check having seen each.
#[tokio::test]
async fn check_findings() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([
        r#"{"covered":["REQ-001","REQ-009"],"written":["src/orders.rs"]}"#,
        r#"{"covered":["REQ-001"],"written":["src/orders.rs","Cargo.toml"]}"#,
    ])
    .calling(1, [write_files("w1", &[("Cargo.toml", "[workspace]\n")])]);
    let slice = slice();
    let mut seen = Vec::new();

    let report = ask_checked(&model, &slice, root, &[], |report| {
        seen.push(report.written.clone());
        let held = Path::new(root).join("Cargo.toml").is_file();
        Ok((!held)
            .then(|| "- the tree lacks `Cargo.toml`; write it".to_owned())
            .into_iter()
            .collect())
    })
    .await
    .expect("corrected");

    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs"]);
    assert_eq!(seen.len(), 2, "the check saw each candidate");
    let exchanges = model.exchanges();
    let correction = exchanges[0].outcome.as_ref().expect_err("the first report is refused");
    assert!(
        correction.contains("- covered `REQ-009` is not a requirement of slice `SLICE-001`"),
        "the gate's finding: {correction}"
    );
    assert!(
        correction.contains("- the tree lacks `Cargo.toml`; write it"),
        "the adapter's finding under the same correction: {correction}"
    );
    assert!(exchanges[2].outcome.is_ok(), "the repaired candidate is accepted");
    model.assert_exhausted();
}

// A check that fails is the run's error as the adapter put it, the
// candidate it failed on ending the turn: one candidate seen, no correction
// put, and the error carried out whole.
#[tokio::test]
async fn check_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    seed(tmp.path(), ["src/orders.rs"]);
    let root = tmp.path().to_str().expect("a UTF-8 scratch root");
    let model = Scripted::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#]);

    let error = ask_checked(&model, &slice(), root, &[], |_| {
        Err(omnia_sdk::server_error!("reading `Cargo.toml`: permission denied"))
    })
    .await
    .expect_err("the check's error is the call's");

    assert_eq!(error.code(), "server_error", "{error}");
    assert!(error.to_string().contains("permission denied"), "{error}");
    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 1, "one candidate, no correction: {exchanges:?}");
    assert!(exchanges[0].outcome.is_ok(), "the candidate ends the turn: {exchanges:?}");
    model.assert_exhausted();
}
