//! Tests `target::build` report gating and file writing over a scripted model.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use std::fs;
use std::num::NonZeroU32;

use emery_sdk::target::{
    Grant, Guarded, MergeRule, Metadata, Scaffold, Slice, TargetMetadata, metadata,
};
use omnia_test::host::{Scratch, ScriptedModel, scratch};
use omnia_test::{SeenFormat, SeenWorker};
use support::build_fixtures::{
    VALID, designed, grants_from, scaffold_spec, scenario_for, scenario_with_grants, slice,
};
use support::harness::build;
use test_programs::protocol::ScaffoldSpec;

async fn ask(
    project: &Scratch, model: &ScriptedModel, slice: &Slice,
) -> emery_adapter::target::Report {
    build(project, &scenario_for(slice), model.clone()).await.report()
}

async fn ask_as(
    project: &Scratch, model: &ScriptedModel, slice: &Slice, grants: &[Grant],
) -> emery_adapter::target::Report {
    let scenario = scenario_with_grants(slice, grants_from(grants));
    build(project, &scenario, model.clone()).await.report()
}

async fn ask_scaffolded(
    project: &Scratch, model: &ScriptedModel, slice: &Slice, brief: &str,
) -> emery_adapter::target::Report {
    let mut scenario = scenario_for(slice);
    scenario.scaffold = Some(ScaffoldSpec {
        brief: brief.to_owned(),
        ..ScaffoldSpec::default()
    });
    build(project, &scenario, model.clone()).await.report()
}

async fn ask_guarded(
    project: &Scratch, model: &ScriptedModel, slice: &Slice, scaffold: Scaffold<'_>,
) -> emery_adapter::target::Report {
    let mut scenario = scenario_for(slice);
    scenario.scaffold = Some(scaffold_spec(scaffold));
    build(project, &scenario, model.clone()).await.report()
}

fn stage<'a>(root: &std::path::Path, files: impl IntoIterator<Item = (&'a str, &'a str)>) {
    for (file, content) in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().expect("a file has a parent")).expect("written dir");
        fs::write(path, content).expect("written file");
    }
}

fn seed<'a>(root: &std::path::Path, files: impl IntoIterator<Item = &'a str>) {
    stage(root, files.into_iter().map(|file| (file, "")));
}

fn write_files(files: &[(&str, &str)]) -> (String, String) {
    let files: Vec<_> = files
        .iter()
        .map(|(path, content)| serde_json::json!({ "path": path, "content": content }))
        .collect();
    ("write_files".to_owned(), serde_json::json!({ "files": files }).to_string())
}

fn edit_files(edits: &[(&str, &str, &str)]) -> (String, String) {
    let edits: Vec<_> = edits
        .iter()
        .map(|(path, find, replace)| {
            serde_json::json!({ "path": path, "find": find, "replace": replace })
        })
        .collect();
    ("edit_files".to_owned(), serde_json::json!({ "edits": edits }).to_string())
}

// The turn carries the slice's three documents, the lend, writable, and the
// write tool beside the reference tools; the answer is steered by the report
// schema.
#[tokio::test]
async fn request_shape() {
    let project = scratch();
    let host_root = project.path().to_str().expect("UTF-8 scratch root");
    seed(project.path(), ["src/orders.rs", "Cargo.toml"]);
    let model = ScriptedModel::answering([VALID]);
    let slice = slice();

    let report = ask(&project, &model, &slice).await;
    assert_eq!(report.covered, ["REQ-001", "REQ-002"]);
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs"], "`written` is answered sorted");

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    let request = &seen[0];
    assert_eq!(request.system.as_deref(), Some("BUILD"));
    assert_eq!(request.workspace.as_deref(), Some(""), "the tree is lent");
    assert_eq!(model.lent(), [Some(project.path().to_path_buf())], "the tree is lent");
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
            "Build slice 1 of 1 of part `orders` (PART-001) as SLICE-001, bound to adapter \
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
    assert!(!user.contains(host_root), "the lend carries the root, not the brief: {user}");
    assert!(user.ends_with("rather than claim what the tree does not hold."), "{user}");
    model.assert_exhausted();
}

// `target::metadata` and `TargetMetadata::from` lower the adapter's merge
// rules and default slice budget beside the SDK's version pin.
#[test]
fn target_metadata() {
    const RULES: &[MergeRule] = &[MergeRule::ours("Cargo.lock")];
    let declared = Metadata {
        rules: RULES,
        grants: &[],
        budget: NonZeroU32::new(12),
    };
    let lowered = TargetMetadata::from(declared);
    assert_eq!(lowered, metadata(RULES, NonZeroU32::new(12)));
    assert_eq!(lowered.emery_version.as_deref(), Some(env!("CARGO_PKG_VERSION")));
    assert_eq!(lowered.merge_rules, [MergeRule::ours("Cargo.lock")]);
    assert_eq!(lowered.budget, Some(12));
    assert_eq!(TargetMetadata::from(Metadata::default()).budget, None);
}

// What the adapter scaffolded before the turn is carried verbatim, after
// the design and before the rules, so the model edits the files it names
// rather than writing them over; the tools and the lend are the same.
#[tokio::test]
async fn build_scaffolded() {
    let project = scratch();
    seed(project.path(), ["src/orders.rs", "Cargo.toml"]);
    let model = ScriptedModel::answering([VALID]);
    let scaffold = "- `src/orders.rs`: scaffolded; fill `todo!(\"REQ-001 orders.create: the \
                    handler\")`\n- `Cargo.toml`: held\n\n";
    let mut slice = slice();
    slice.id = "SLICE-002".to_owned();
    slice.index = 2;
    slice.count = 3;

    ask_scaffolded(&project, &model, &slice, scaffold).await;

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].tools, ["list_docs", "read_doc", "write_files", "edit_files"]);
    let user = &seen[0].messages[0];
    assert!(
        user.starts_with(
            "Build slice 2 of 3 of part `orders` (PART-001) as SLICE-002, bound to adapter \
             `target:probe`.\n\n"
        ),
        "{user}"
    );
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

// With nothing scaffolded, the design keeps every paragraph, the owned type
// whole, the type the owned signature names whole, and the mechanism, and
// leaves out the type nothing of the slice spells, listing it by key.
#[tokio::test]
async fn design_cut() {
    let project = scratch();
    seed(project.path(), ["src/orders.rs", "Cargo.toml"]);
    let model = ScriptedModel::answering([VALID]);
    let slice = designed();

    ask(&project, &model, &slice).await;

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

// A data type among the scaffold's `types` is one line pointing at the
// scaffold section, whatever the slice spells; a mechanism type listed there
// stays whole, label and signature, since the cut keeps every mechanism as
// the design holds it; the scaffold's brief is text of the slice, so a type
// it names stays whole.
#[tokio::test]
async fn design_cut_scaffolded() {
    let project = scratch();
    seed(project.path(), ["src/orders.rs", "Cargo.toml"]);
    let model = ScriptedModel::answering([VALID]);
    let slice = designed();
    let types = ["orders.order".to_owned(), "start.Config".to_owned()];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded, `Order` laid; an `Invoice` is read as a value.\n",
        passages: "",
        files: &[],
        types: &types,
    };

    ask_guarded(&project, &model, &slice, scaffold).await;

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
async fn design_cut_passages() {
    let project = scratch();
    seed(project.path(), ["src/orders.rs", "Cargo.toml"]);
    let model = ScriptedModel::answering([VALID]);
    let slice = designed();
    let files = [Guarded::new("src/orders.rs", ["REQ-001"])];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n",
        passages: "From the references, for this slice:\n\n## A probe\n\nAn `Invoice` is read as \
                   a value and answered whole.\n",
        files: &files,
        types: &[],
    };

    ask_guarded(&project, &model, &slice, scaffold).await;

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
             naming what it dropped, as a `delete` naming one of them is.\n\nFrom the \
             references, for this slice:\n\n## A \
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
async fn design_cut_phrase() {
    let project = scratch();
    seed(project.path(), ["src/orders.rs", "Cargo.toml"]);
    let mut apart = designed();
    "interface Order {\n  id: string;\n}".clone_into(&mut apart.types[0].signature);
    "## SLICE-001 orders\n\nBuild the orders module. Each line of the listing is one order.\n"
        .clone_into(&mut apart.plan);
    let mut together = apart.clone();
    "## SLICE-001 orders\n\nBuild the orders module, its order lines included.\n"
        .clone_into(&mut together.plan);

    let model = ScriptedModel::answering([VALID]);
    ask(&project, &model, &apart).await;
    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "Design types left out, owned by other slices and named by none of this slice's \
             text: `orders.order-line`, `billing.Invoice (src/billing.ts#L3)`."
        ),
        "{user}"
    );
    model.assert_exhausted();

    let model = ScriptedModel::answering([VALID]);
    ask(&project, &model, &together).await;
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
// and the two ways to change it; a `delete` naming one is refused whatever
// it keeps; the call lands nothing. An edit within the file, and a write
// beside it, land as they would. The brief says so after the scaffold
// section.
#[tokio::test]
async fn write_scaffolded_drop() {
    let project = scratch();
    let skeleton = "pub fn create() {\n    todo!(\"REQ-001 orders.create: the handler\")\n}\n";
    stage(project.path(), [("src/orders.rs", skeleton), ("Cargo.toml", "[package]\n")]);
    let model =
        ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
            .calling(
                0,
                [
                    write_files(&[
                        ("src/orders.rs", "pub fn create() {}\n"),
                        ("src/lib.rs", "pub mod orders;\n"),
                    ]),
                    ("write_files".to_owned(), r#"{"delete":["./src/orders.rs"]}"#.to_owned()),
                    edit_files(&[(
                        "src/orders.rs",
                        "todo!(\"REQ-001 orders.create: the handler\")",
                        "Ok(())",
                    )]),
                    write_files(&[("src/lib.rs", "pub mod orders;\n")]),
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

    let report = ask_guarded(&project, &model, &slice(), scaffold).await;
    assert_eq!(report.written, ["src/lib.rs", "src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(project.path().join("src/orders.rs")).expect("edited"),
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
        ["list_docs", "read_doc", "write_files", "edit_files"],
        "a scaffold declares the two write tools"
    );
    let user = &request.messages[0];
    assert!(user.contains("Write through this call's two tools alone:"), "{user}");
    assert!(
        user.contains(
            "What the adapter scaffolded for this slice before this turn:\n\n- `src/orders.rs`: \
             scaffolded\n- `Cargo.toml`: held\n\nThe files the adapter scaffolded for this slice \
             are filled in place: change one through `edit_files`, or send the whole file \
             through `write_files` with every marker and name the skeleton holds still in it; \
             an entry that drops one is refused whole naming what it dropped, as a `delete` \
             naming one of them is.\n\nImplement every requirement"
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
    let project = scratch();
    let skeleton = "pub fn create() {\n    todo!(\"REQ-001 orders.create: the handler\")\n}\n";
    let listing = "pub mod orders;\n// token1 token2 token3 token4 token5 token6 token7 token8 \
                   token9 token10 token11 token12\n";
    stage(
        project.path(),
        [("src/orders.rs", skeleton), ("Cargo.toml", "[package]\n"), ("src/lib.rs", listing)],
    );
    let filled = "//! Orders.\n\npub fn create() -> Result<(), Error> {\n    // REQ-001 \
                  orders.create: the handler\n    Ok(())\n}\n";
    let tokens: Vec<String> = (1..=12).map(|n| format!("token{n}")).collect();
    let model =
        ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
            .calling(
                0,
                [
                    write_files(&[("src/orders.rs", filled)]),
                    write_files(&[("src/orders/mod.rs", "pub struct Order;\n")]),
                    write_files(&[("Cargo.toml", "[package]\nname = \"orders\"\n")]),
                    write_files(&[("src/lib.rs", "pub mod orders;\npub mod billing;\n")]),
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

    let report = ask_guarded(&project, &model, &slice(), scaffold).await;
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs", "src/orders/mod.rs"]);
    assert_eq!(
        fs::read_to_string(project.path().join("src/orders.rs")).expect("written"),
        filled,
        "a whole write keeping the tokens lands"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("src/orders/mod.rs")).expect("written"),
        "pub struct Order;\n",
        "a guarded path the tree lacked is written"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("src/lib.rs")).expect("held"),
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
async fn write_scaffolded_edit() {
    let project = scratch();
    let skeleton = "pub fn create() {\n    todo!(\"REQ-001 orders.create: the handler\")\n}\n";
    stage(project.path(), [("src/orders.rs", skeleton)]);
    let filled = "//! Orders.\n\npub fn create() -> Result<(), Error> {\n    // REQ-001 \
                  orders.create: the handler\n    Ok(())\n}\n";
    let model =
        ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
            .calling(
            0,
            [
                (
                    "write_files".to_owned(),
                    serde_json::json!({
                        "files": [{ "path": "src/orders.rs", "content": "pub fn create() {}\n" }],
                        "delete": ["src/orders.rs"],
                    })
                    .to_string(),
                ),
                (
                    "write_files".to_owned(),
                    serde_json::json!({
                        "files": [{ "path": "src/orders.rs", "content": filled }],
                        "delete": ["./src/orders.rs"],
                    })
                    .to_string(),
                ),
            ],
        );
    let files = [Guarded::new("src/orders.rs", ["pub fn create", "REQ-001 orders.create"])];
    let scaffold = Scaffold {
        brief: "- `src/orders.rs`: scaffolded\n",
        passages: "",
        files: &files,
        types: &[],
    };

    let report = ask_guarded(&project, &model, &slice(), scaffold).await;
    assert_eq!(report.written, ["src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(project.path().join("src/orders.rs")).expect("replaced"),
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
    let project = scratch();

    seed(project.path(), ["src/orders.rs", "Cargo.toml"]);
    let model = ScriptedModel::answering([VALID]);
    let slice = slice();

    ask_as(&project, &model, &slice, GRANTS).await;

    let seen = model.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].workspace.as_deref(), Some(""));
    assert_eq!(model.lent(), [Some(project.path().to_path_buf())], "the tree is lent");
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
async fn write_halved_file() {
    let project = scratch();
    let whole = "pub struct Order;\npub struct Line;\npub struct Total;\n";
    stage(project.path(), [("src/orders.rs", whole), ("src/empty.rs", "")]);
    let model = ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
        .calling(
            0,
            [
                write_files(&[("src/orders.rs", ""), ("src/lib.rs", "pub mod orders;\n")]),
                write_files(&[("src/orders.rs", "pub struct Order;\n")]),
                (
                    "write_files".to_owned(),
                    r#"{"files":{"path":"src/orders.rs","content":"pub struct Order;\n","shrink":true}}"#
                        .to_owned(),
                ),
                write_files(&[("src/orders.rs", "pub struct Order; pub struct Line;\n")]),
                write_files(&[("src/empty.rs", "")]),
            ],
        );

    let report = ask(&project, &model, &slice()).await;
    assert_eq!(report.written, ["src/empty.rs", "src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(project.path().join("src/orders.rs")).expect("written"),
        "pub struct Order; pub struct Line;\n"
    );
    assert!(!project.path().join("src/lib.rs").exists(), "a refused call lands nothing");

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
async fn write_minor_shrink() {
    let project = scratch();
    stage(project.path(), [("src/orders.rs", "pub struct Order;\npub struct Line;\n")]);
    let model =
        ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
            .calling(0, [write_files(&[("src/orders.rs", "pub struct Order;\n")])]);

    let report = ask(&project, &model, &slice()).await;
    assert_eq!(report.written, ["src/orders.rs"]);
    assert_eq!(
        fs::read_to_string(project.path().join("src/orders.rs")).expect("written"),
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
    let project = scratch();
    let model = ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":["src/mod.rs"]}"#])
        .calling(
            0,
            [write_files(&[
                ("src/mod.rs", ""),
                ("src/draft.rs", "pub struct Draft;\n"),
                ("src/draft.rs", ""),
            ])],
        );

    let report = ask(&project, &model, &slice()).await;
    assert_eq!(report.written, ["src/draft.rs", "src/mod.rs"]);
    assert_eq!(fs::read_to_string(project.path().join("src/draft.rs")).expect("written"), "");

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
    let project = scratch();
    stage(
        project.path(),
        [
            ("src/orders.rs", "pub struct Order;\npub fn total(&token: &str) {}\n"),
            ("Cargo.toml", "[package]\nname = \"orders\"\n"),
        ],
    );
    let model =
        ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":["src/orders.rs"]}"#])
            .calling(
                0,
                [edit_files(&[
                    ("./src//orders.rs", "&token", "token"),
                    (
                        "src/orders.rs",
                        "pub fn total(token: &str) {}",
                        "pub fn total(token: &str) -> u32 { 0 }",
                    ),
                    ("Cargo.toml", "name = \"orders\"\n", ""),
                ])],
            );

    let report = ask(&project, &model, &slice()).await;
    assert_eq!(report.written, ["Cargo.toml", "src/orders.rs"], "an edited file is written");
    assert_eq!(
        fs::read_to_string(project.path().join("src/orders.rs")).expect("edited"),
        "pub struct Order;\npub fn total(token: &str) -> u32 { 0 }\n"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("Cargo.toml")).expect("edited"),
        "[package]\n"
    );

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
    let project = scratch();
    let before = "pub struct Order;\npub struct Line;\n";
    stage(project.path(), [("src/orders.rs", before)]);
    let model = ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [
            edit_files(&[
                ("src/orders.rs", "Order", "Purchase"),
                ("src/orders.rs", "pub struct", "pub(crate) struct"),
                ("src/orders.rs", "struct Total", "struct Sum"),
                ("src/orders.rs", "", "x"),
            ]),
            ("edit_files".to_owned(), r#"{"edits":[]}"#.to_owned()),
        ],
    );

    let report = ask(&project, &model, &slice()).await;
    assert_eq!(report.written, Vec::<String>::new());
    assert_eq!(
        fs::read_to_string(project.path().join("src/orders.rs")).expect("as it was"),
        before
    );

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
    let project = scratch();
    let create = "pub fn create() {\n    let order = Order::new();\n    order.save()?;\n    \
                  Ok(())\n}\n";
    let twice = "fn a() {\n    x();\n}\nfn b() {\n    x();\n    y();\n}\n";
    stage(
        project.path(),
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
    let model = ScriptedModel::answering([r#"{"covered":["REQ-001"],"written":[]}"#]).calling(
        0,
        [edit_files(&[
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
        ])],
    );

    let report = ask(&project, &model, &slice()).await;
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
