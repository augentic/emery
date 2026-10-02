//! `survey::seams` lays the facts a tree holds, has the model name the
//! surfaces, holds the answer to the tree, and cuts the seams from what the
//! code derives at the accepted anchors; every scenario here runs it over a
//! hand-built tree under a scripted model.

mod support;

use std::collections::BTreeMap;

use emery_sdk::survey::Survey;
use emery_sdk::survey::code::{
    EnvRead, ExportKind, Init, Listing, Manifest, MemberKind, Parsed, Runs, Tree, Use,
};
use emery_sdk::survey::resolve::Target;
use emery_sdk::{Context, Doc, Error, SourceInput};
use omnia_test::guest::Scripted;
use support::{
    PYTHON, Plain, Stub, TYPESCRIPT, call, callee, class, class_decl, decorated, export,
    from_module, from_package, function, line, literal, member, module, name, named_import,
    reference, span, value, write,
};

const PROSE: &[Doc] = &[
    Doc {
        path: "survey.md",
        body: "SURVEY",
    },
    Doc {
        path: "extract.md",
        body: "SYSTEM",
    },
];

const NO_SURFACE: &str = "No surface was found in this source: its survey named no route, command, \
                          job, consumer, or exported API — no bootstrap the manifest names or a \
                          conventional entry holds, no handler registered with a package, no \
                          function, method, or class under a package's decorator, and no function \
                          or class exported at an entry module for a caller. Read it as a library \
                          is read — for what its exports do for a caller — and claim what the code \
                          exhibits.";

// Two surfaces the facts locate, at their registration and their decorator.
const BOTH: &str = r#"{"surfaces":[
    {"name":"GET /orders","anchor":"src/routes/orders.ts#L3","stem":"orders"},
    {"name":"GET /items","anchor":"src/ctl.ts#L3-L5","stem":"items"}
]}"#;

fn pad(lines: usize) -> String {
    "// pad..\n".repeat(lines)
}

// A service of six modules, a manifest, two data files, and two tests. The
// registration in `src/routes/orders.ts` and the decorator in `src/ctl.ts`
// locate a surface each; `src/ctl.ts` imports what no module answers. A
// large tree pads `src/store.ts` past the inline budget.
fn service(large: bool, bootstrap: Option<Runs>) -> (tempfile::TempDir, SourceInput, Tree<Stub>) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write(root, "package.json", "{\"name\":\"svc\"}\n");
    write(root, "src/main.ts", &pad(4));
    write(root, "src/routes/orders.ts", &pad(6));
    write(root, "src/store.ts", &if large { pad(10_003) } else { pad(3) });
    write(root, "src/ctl.ts", &pad(8));
    write(
        root,
        "src/config.ts",
        "const PORT = process.env.PORT;\nexport const config = { PORT };\nexport const LIMITS = \
         {\n  pageSize: 20,\n};\n",
    );
    write(root, "src/data.ts", &pad(2));
    write(root, "src/jobs/nightly.ts", &pad(2));
    write(root, "data/small.json", "{\"a\":1}\n");
    write(root, "data/big.json", &format!("[{}]", "1,".repeat(4_500)));
    write(root, "tests/orders.test.ts", &pad(2));
    write(root, "tests/loose.test.ts", &pad(1));

    let listing = Listing {
        modules: [
            "src/main.ts",
            "src/routes/orders.ts",
            "src/store.ts",
            "src/ctl.ts",
            "src/config.ts",
            "src/data.ts",
            "src/jobs/nightly.ts",
        ]
        .map(str::to_owned)
        .to_vec(),
        data: ["data/small.json", "data/big.json"].map(str::to_owned).to_vec(),
        tests: ["tests/orders.test.ts", "tests/loose.test.ts"].map(str::to_owned).to_vec(),
    };
    let parsed = Parsed::read(root, &TYPESCRIPT, listing, |path, text| {
        let lines = u32::try_from(text.lines().count()).expect("a short file");
        let mut m = module(path, lines);
        m.0.text = text;
        fill(&mut m);
        m
    });
    let stub = Stub {
        manifest: Manifest {
            name: Some("svc".to_owned()),
            says: vec!["names the package `svc`".to_owned(), "`main` is `src/main.ts`".to_owned()],
            entries: vec!["src/main.ts".to_owned()],
        },
        bootstrap: bootstrap.map(|runs| ("src/main.ts".to_owned(), runs)),
        mounts: BTreeMap::new(),
    };
    let tree = parsed.settle(stub);
    let input = SourceInput::workspace("svc", root.to_str().expect("a UTF-8 scratch root"));
    (tmp, input, tree)
}

fn fill(m: &mut Plain) {
    let module = &mut m.0;
    match module.path.as_str() {
        "src/main.ts" => {
            module.imports = vec![
                from_module("orders", "src/routes/orders.ts"),
                from_module("config", "src/config.ts"),
            ];
            module.references = vec![reference("orders", 2), reference("config", 3)];
            module.bindings = vec![function("main", line(4))];
            module.exports = vec![export("main", ExportKind::Function, line(4))];
        }
        "src/routes/orders.ts" => {
            module.imports =
                vec![from_package("express", "express"), from_module("Store", "src/store.ts")];
            module.bindings = vec![
                value("app", Some(&["express"]), Some(0), None, Init::Construction, line(2)),
                function("list", span(4, 6)),
            ];
            module.calls = vec![
                call(callee("express", &[]), Vec::new(), Use::Consumed, line(2)),
                call(
                    callee("app", &["get"]),
                    vec![literal("/orders", line(3)), name(&["list"], line(3))],
                    Use::Discarded,
                    line(3),
                ),
            ];
            module.references =
                vec![reference("app", 3), reference("list", 3), reference("Store", 5)];
        }
        "src/store.ts" => {
            module.bindings = vec![class("Store", &[], span(1, 3))];
            module.exports = vec![export("Store", ExportKind::Class, span(1, 3))];
            module.classes = vec![class_decl(
                "Store",
                "export class Store",
                span(1, 3),
                vec![member("find", MemberKind::Method, "find(id: string): Order", line(2))],
            )];
        }
        "src/ctl.ts" => {
            module.imports = vec![
                from_package("Get", "@nestjs/common"),
                from_package("UseGuards", "@nestjs/common"),
                named_import("x", "@alias/x", Target::Unresolved("@alias/x".to_owned())),
            ];
            module.bindings = vec![class("Ctl", &[], span(2, 8))];
            module.exports = vec![export("Ctl", ExportKind::Class, span(2, 8))];
            module.decorated = vec![
                decorated(
                    Some("Ctl"),
                    Some("list"),
                    &["Get"],
                    Some("/items"),
                    span(3, 4),
                    span(3, 5),
                ),
                decorated(Some("Ctl"), Some("other"), &["UseGuards"], None, line(6), span(6, 7)),
            ];
            module.classes = vec![class_decl(
                "Ctl",
                "export class Ctl",
                span(2, 8),
                vec![
                    member("list", MemberKind::Method, "list()", span(4, 5)),
                    member("other", MemberKind::Method, "other()", line(7)),
                ],
            )];
        }
        "src/config.ts" => {
            module.env = vec![EnvRead {
                key: "PORT".to_owned(),
                lines: line(1),
            }];
            module.bindings = vec![value("LIMITS", None, None, None, Init::Literal, span(3, 5))];
            module.exports = vec![export("config", ExportKind::Value, line(2))];
        }
        "src/data.ts" => {
            module.imports = vec![
                named_import(
                    "small",
                    "./data/small.json",
                    Target::Data("data/small.json".to_owned()),
                ),
                named_import("big", "./data/big.json", Target::Data("data/big.json".to_owned())),
            ];
        }
        "tests/orders.test.ts" => {
            module.imports = vec![from_module("list", "src/routes/orders.ts")];
            module.references = vec![reference("lists_orders", 1)];
        }
        "tests/loose.test.ts" => {
            module.references = vec![reference("runs", 1)];
        }
        _ => {}
    }
}

async fn run(model: &Scripted, input: &SourceInput, tree: &Tree<Stub>) -> Result<Survey, Error> {
    let ctx = Context {
        adapter_id: "source:probe",
        input,
        model,
    };
    emery_sdk::survey::seams(&ctx, PROSE, tree).await
}

fn files(survey: &Survey, index: usize) -> Vec<&str> {
    survey.seams[index].files.iter().map(String::as_str).collect()
}

// The one survey turn carries what the tree holds — the manifest, the
// bootstrap, the registration, the registering decorator alone, the entry
// modules' exports, the packages, and the import nothing answered — and lays
// the manifest and the bootstrap first.
#[tokio::test]
async fn facts_laid() {
    let (_tmp, input, tree) = service(false, Some(Runs::Load));
    let model = Scripted::answering([BOTH]);

    run(&model, &input, &tree).await.expect("surveyed");

    let user = &model.seen()[0].messages[0];
    for fact in [
        "The manifest names the package `svc`; `main` is `src/main.ts`.",
        "The bootstrap — the entry that runs something — is `src/main.ts`: it runs or constructs \
         the application at load. It is the caller's `start` surface:",
        "Calls that hand a function or a class to something a package provides, each at its \
         lines",
        "- `src/routes/orders.ts#L3` — `app.get` led by `\"/orders\"` handed a function, through \
         `express`\n",
        "Decorators a package provides, each at its lines — where a framework is told what a \
         function, a method, or a class answers",
        "- `src/ctl.ts#L3-L5` — `@Get(\"/items\")` on `Ctl.list`, through `@nestjs/common`\n",
        "What the entry modules — the ones the manifest names, and the ones no other module \
         imports — export, each with its kind and lines, and what a barrel among them \
         re-exports at the module that declares it. In a tree with no bootstrap, or under a \
         framework that routes by file, these are where a caller enters:\n\n- `src/main.ts` \
         exports `main` (function) L4\n- `src/ctl.ts` exports `Ctl` (class) L2–L8\n",
        "Packages these modules import, with the names bound to them.",
        "- `@nestjs/common` — `Get`, `UseGuards` — in `src/ctl.ts`\n- `express` — `express` — in \
         `src/routes/orders.ts`",
        "The caller could not follow every import: `@alias/x` from `src/ctl.ts` names no module \
         of the tree. What these name is in none of the lists above.\n\n`$SOURCE_DIR`",
        "### `package.json` (1 line)",
        "### `src/main.ts` (4 lines)",
    ] {
        assert!(user.contains(fact), "{fact}\n---\n{user}");
    }
    assert!(!user.contains("@UseGuards"), "a shaping decorator is not listed: {user}");
    let laid = user.find("### `package.json`").expect("laid");
    let main = user.find("### `src/main.ts`").expect("laid");
    let ctl = user.find("### `src/ctl.ts`").expect("laid");
    let orders = user.find("### `src/routes/orders.ts`").expect("laid");
    let config = user.find("### `src/config.ts`").expect("laid");
    assert!(
        laid < main && main < ctl && ctl < orders && orders < config,
        "manifest, bootstrap, locating, the rest"
    );
    model.assert_exhausted();
}

// A tree within the budget is one seam over every module, the surfaces'
// closures first, its data files and tests among them; the brief lists the
// surfaces with the ids and closures the code derived, then what the modules
// spell; the `type` claims are the reached modules' exports.
#[tokio::test]
async fn whole_tree() {
    let (_tmp, input, tree) = service(false, Some(Runs::Load));
    let model = Scripted::answering([BOTH]);

    let survey = run(&model, &input, &tree).await.expect("surveyed");

    assert_eq!(survey.seams.len(), 1);
    let seam = &survey.seams[0];
    assert_eq!(seam.stems, ["start", "orders", "items"]);
    assert_eq!(
        files(&survey, 0),
        [
            "src/main.ts",
            "src/routes/orders.ts",
            "src/config.ts",
            "src/store.ts",
            "src/ctl.ts",
            "src/data.ts",
            "data/small.json",
            "src/jobs/nightly.ts",
            "data/big.json",
            "tests/orders.test.ts",
            "tests/loose.test.ts",
        ],
        "closures first, a small data file beside its module, a large one after every module, \
         then the tests"
    );
    assert_eq!(
        &seam.anchors[..3],
        ["src/main.ts#L1-L4", "src/routes/orders.ts#L3", "src/ctl.ts#L3-L5"],
        "the surfaces' own lines lead"
    );
    for anchor in
        ["src/routes/orders.ts#L4", "src/ctl.ts#L3-L4", "src/ctl.ts#L4", "src/config.ts#L1"]
    {
        assert!(seam.anchors.contains(&anchor.to_owned()), "{anchor}: {:?}", seam.anchors);
    }
    assert!(!seam.anchors.iter().any(|anchor| anchor.starts_with("tests/")), "a test is no anchor");

    assert!(
        seam.text.starts_with(
            "The surfaces of this source, found by reading its code — where control enters it \
             from outside the process:\n\n- Surface `start` — entry `src/main.ts` — stem `start`: \
             the process bootstrap, run at load: what runs before each handler is registered, \
             what it awaits before serving, and at shutdown — `stop` and what a signal handler \
             calls, wherever declared; id `start`; reaches `src/routes/orders.ts`, \
             `src/config.ts`.\n- Surface `GET /orders` — entry `src/routes/orders.ts` — stem \
             `orders`: registered L3; through `express`; id `orders`; reaches `src/store.ts`.\n- \
             Surface `GET /items` — entry `src/ctl.ts` — stem `items`: named by the survey at \
             L3–L5; id `items`; reaches nothing beyond its entry.\n\nEvery `requirement` and \
             `criterion` belongs to one of these surfaces"
        ),
        "{}",
        seam.text
    );
    for section in [
        "Boundaries the code spells as values of their own, each at its line.",
        "- `src/config.ts#L3-L5` — `LIMITS = { pageSize: 20, }`",
        "- `src/config.ts#L1` — `process.env.PORT` in `const PORT = process.env.PORT;`",
        "Packages these modules import, with the names bound to them.",
        "Data files these modules name by path, each with the modules reading it, laid after the \
         first module naming it — a large one after every module:\n\n- `data/small.json` — named \
         by `src/data.ts`\n- `data/big.json` — named by `src/data.ts`",
        "Behaviours the tree's own tests state, each at its line — a case's title under its \
         suites', a scenario's under its feature's.",
        "- `tests/orders.test.ts#L1` — states lists_orders\n- `tests/loose.test.ts#L1` — states runs",
        "The caller could not follow every import: `@alias/x` from `src/ctl.ts` names no module \
         of the tree. What these name is in none of the lists above.",
    ] {
        assert!(seam.text.contains(section), "{section}\n---\n{}", seam.text);
    }
    assert!(
        !seam.text.contains("Calls these modules make"),
        "nothing left the process: {}",
        seam.text
    );
    assert!(!seam.text.contains("The modules after the closure"), "not widened: {}", seam.text);
    model.assert_exhausted();
}

// The `type` claims are the exported classes of the modules the seams reach,
// each anchored and spelled under the dialect's class syntax.
#[tokio::test]
async fn types_anchored() {
    let (_tmp, input, tree) = service(false, Some(Runs::Load));
    let model = Scripted::answering([BOTH]);

    let survey = run(&model, &input, &tree).await.expect("surveyed");

    let types: Vec<(String, String, String)> = survey
        .types
        .iter()
        .map(|claim| {
            (
                claim.extras["name"].as_str().expect("name").to_owned(),
                claim.path.clone().expect("anchored"),
                claim.extras["signature"].as_str().expect("signature").to_owned(),
            )
        })
        .collect();
    assert_eq!(
        types,
        [
            (
                "Store".to_owned(),
                "src/store.ts#L1-L3".to_owned(),
                "export class Store {\n  find(id: string): Order { .. }\n}".to_owned()
            ),
            (
                "Ctl".to_owned(),
                "src/ctl.ts#L2-L8".to_owned(),
                "export class Ctl {\n  list() { .. }\n  other() { .. }\n}".to_owned()
            ),
        ],
        "in the files' order, spelled under the dialect's class syntax"
    );
    assert_eq!(survey.types[0].synopsis.as_deref(), Some("exported class"));
    model.assert_exhausted();
}

// The facts say how the bootstrap runs, in the words of its `Runs`, and the
// `start` surface's note says the same; a tree with none says so and names
// no `start`.
#[tokio::test]
async fn bootstrap_runs() {
    for (runs, said, how) in [
        (
            Runs::Script {
                name: "svc".to_owned(),
                function: "main".to_owned(),
            },
            "the console script `svc` calls its `main()`",
            "run by the console script `svc`, which calls `main()`",
        ),
        (Runs::Guard, "it runs under its `__main__` guard", "run under its `__main__` guard"),
    ] {
        let (_tmp, input, tree) = service(false, Some(runs));
        let model = Scripted::answering([r#"{"surfaces":[],"unreached":["src/ctl.ts"]}"#]);

        let survey = run(&model, &input, &tree).await.expect("surveyed");

        let user = &model.seen()[0].messages[0];
        let sentence = format!("is `src/main.ts`: {said}. It is the caller's `start` surface");
        assert!(user.contains(&sentence), "{sentence}\n---\n{user}");
        assert_eq!(survey.seams[0].stems, ["start"]);
        let note = format!("stem `start`: the process bootstrap, {how}: what runs");
        assert!(survey.seams[0].text.contains(&note), "{note}\n---\n{}", survey.seams[0].text);
        model.assert_exhausted();
    }

    let (_tmp, input, tree) = service(false, None);
    let model = Scripted::answering([
        r#"{"surfaces":[],"unreached":["src/main.ts","src/routes/orders.ts","src/ctl.ts"]}"#,
    ]);

    let survey = run(&model, &input, &tree).await.expect("surveyed");

    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains(
            "No bootstrap runs at load: no module the manifest names, and no conventional entry \
             the tree holds, runs anything when loaded. The surfaces are what the tree exposes \
             without one — what its entry modules export for a caller, or what a convention of \
             the framework it uses makes reachable: a module a setting, a file path, or a \
             directory names; a handler an export names."
        ),
        "{user}"
    );
    assert_eq!(survey.seams.len(), 1);
    let seam = &survey.seams[0];
    assert_eq!(seam.stems, ["svc"], "the manifest's name is the one stem");
    assert_eq!(
        seam.text.split("\n\n").next(),
        Some(
            format!(
                "{NO_SURFACE} The modules below are the whole source, mined under the one stem \
                 `svc`: lead every `requirement` and `criterion` id with it, and name each \
                 behaviour for the export that exhibits it."
            )
            .as_str()
        )
    );
    assert_eq!(seam.anchors, [] as [String; 0], "a tree with no surface has no anchors");
    assert_eq!(
        files(&survey, 0),
        [
            "src/config.ts",
            "src/ctl.ts",
            "src/data.ts",
            "data/small.json",
            "src/jobs/nightly.ts",
            "src/main.ts",
            "src/routes/orders.ts",
            "src/store.ts",
            "data/big.json",
            "tests/orders.test.ts",
            "tests/loose.test.ts",
        ]
    );
    model.assert_exhausted();
}

// An answer leading a surface with `start`, or leaving a module the facts
// locate a surface in neither reached nor `unreached`, comes back with both
// findings for one correction round; the corrected answer is accepted.
#[tokio::test]
async fn check_round() {
    let (_tmp, input, tree) = service(false, Some(Runs::Load));
    let model = Scripted::answering([
        r#"{"surfaces":[{"name":"orders","anchor":"src/routes/orders.ts#L3","stem":"start"}]}"#,
        BOTH,
    ]);

    let survey = run(&model, &input, &tree).await.expect("corrected");

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 2);
    let correction = exchanges[0].outcome.as_ref().expect_err("the first answer is refused");
    for finding in [
        "- surface `orders`: `start` is the bootstrap's stem, which the caller names itself; give \
         the surface the stem of what a caller does through it",
        "- one module the facts list a registration or declaration in is reached by no surface \
         you named and not listed under `unreached`: `src/ctl.ts`; where a caller outside the \
         process reaches what is registered there — a route, a command, a task — name that \
         surface, anchored at the registration or the decorated declaration, so the caller \
         follows its imports to the module; otherwise list it under `unreached` — a plugin, a \
         hook, a signal receiver, a helper is no surface, and none is invented to cover a module",
    ] {
        assert!(correction.contains(finding), "{finding}\n---\n{correction}");
    }
    assert_eq!(exchanges[1].outcome, Ok(String::new()), "the corrected answer is accepted");
    assert_eq!(survey.seams[0].stems, ["start", "orders", "items"]);
    model.assert_exhausted();
}

// A tree past the budget is one seam per stem over what its surfaces reach;
// the seam whose modules import what no module answers lays the rest of the
// tree after its closure and says so, and each test follows the seams whose
// files it imports.
#[tokio::test]
async fn by_stem_widened() {
    let (_tmp, input, tree) = service(true, Some(Runs::Load));
    let model = Scripted::answering([BOTH]);

    let survey = run(&model, &input, &tree).await.expect("surveyed");

    assert_eq!(survey.seams.len(), 3);
    let stems: Vec<&[String]> = survey.seams.iter().map(|seam| seam.stems.as_slice()).collect();
    assert_eq!(stems, [["start"], ["orders"], ["items"]]);
    assert_eq!(
        files(&survey, 0),
        [
            "src/main.ts",
            "src/routes/orders.ts",
            "src/config.ts",
            "tests/orders.test.ts",
            "tests/loose.test.ts"
        ],
        "the bootstrap's closure stops at the other surfaces' entries"
    );
    assert_eq!(
        files(&survey, 1),
        ["src/routes/orders.ts", "src/store.ts", "tests/orders.test.ts", "tests/loose.test.ts"]
    );
    assert_eq!(
        files(&survey, 2),
        [
            "src/ctl.ts",
            "src/config.ts",
            "src/data.ts",
            "data/small.json",
            "src/jobs/nightly.ts",
            "src/main.ts",
            "src/routes/orders.ts",
            "src/store.ts",
            "data/big.json",
            "tests/orders.test.ts",
            "tests/loose.test.ts",
        ],
        "widened to the rest of the tree, in path order"
    );
    assert!(
        survey.seams[1].text.starts_with(
            "This call mines the surface under the stem `orders` alone:\n\n- Surface `GET \
             /orders` — entry `src/routes/orders.ts` — stem `orders`: registered L3; through \
             `express`; id `orders`; reaches `src/store.ts`.\n\nThe files below are what it \
             reaches from its entry, the entry first."
        ),
        "{}",
        survey.seams[1].text
    );
    assert!(
        !survey.seams[1].text.contains("could not follow"),
        "every import of the orders seam was followed: {}",
        survey.seams[1].text
    );
    assert!(
        survey.seams[2].text.contains(
            "The caller could not follow every import: `@alias/x` from `src/ctl.ts` names no \
             module of the tree. What these name is in none of the lists above. The modules after \
             the closure are the rest of the tree, laid so what these name is still within reach; \
             read them for that alone."
        ),
        "{}",
        survey.seams[2].text
    );
    assert_eq!(&survey.seams[2].anchors[0], "src/ctl.ts#L3-L5");
    assert!(
        !survey.seams[1].anchors.iter().any(|anchor| anchor.starts_with("src/ctl.ts")),
        "a seam's anchors are its own files': {:?}",
        survey.seams[1].anchors
    );
    let types: Vec<&str> =
        survey.types.iter().map(|claim| claim.extras["name"].as_str().expect("name")).collect();
    assert_eq!(types, ["Store", "Ctl"]);
    model.assert_exhausted();
}

// A tree past the budget with no surface is cut by top-level directory under
// each directory's name, the root's own modules joining the first.
#[tokio::test]
async fn by_directory() {
    let (_tmp, input, tree) = service(true, None);
    let model = Scripted::answering([
        r#"{"surfaces":[],"unreached":["src/main.ts","src/routes/orders.ts","src/ctl.ts"]}"#,
    ]);

    let survey = run(&model, &input, &tree).await.expect("surveyed");

    assert_eq!(survey.seams.len(), 2);
    assert_eq!(survey.seams[0].stems, ["jobs"]);
    assert_eq!(survey.seams[1].stems, ["routes"]);
    assert_eq!(
        files(&survey, 0),
        [
            "src/config.ts",
            "src/ctl.ts",
            "src/data.ts",
            "data/small.json",
            "src/main.ts",
            "src/store.ts",
            "src/jobs/nightly.ts",
            "data/big.json",
            "tests/loose.test.ts",
        ],
        "the root's own modules lead the first directory's"
    );
    assert_eq!(
        files(&survey, 1),
        ["src/routes/orders.ts", "tests/orders.test.ts", "tests/loose.test.ts"]
    );
    assert_eq!(
        survey.seams[0].text.split("\n\n").next(),
        Some(
            format!(
                "{NO_SURFACE} It is past the budget of one call and cut by directory: this call \
                 mines the 6 modules under `src/jobs/`, with the root's own modules alone, under \
                 the stem `jobs` — lead every `requirement` and `criterion` id with it. What \
                 another directory's modules do is another call's to claim, even where these \
                 import them."
            )
            .as_str()
        )
    );
    assert!(
        survey.seams[1].text.contains(
            "this call mines the module under `src/routes/` alone, under the stem `routes`"
        ),
        "{}",
        survey.seams[1].text
    );
    model.assert_exhausted();
}

// Under a dialect with barrels, a lone top-level package is cut beneath, as
// `src/` is, and the stem falls back to the root directory's name where the
// manifest names none.
#[tokio::test]
async fn by_directory_package() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("shop");
    std::fs::create_dir_all(&root).expect("mkdir");
    write(&root, "manage.py", &pad(2));
    write(&root, "shop/__init__.py", &pad(1));
    write(&root, "shop/api/views.py", &pad(10_000).replace("//", "#"));
    write(&root, "shop/api/serializers.py", &pad(2));
    write(&root, "shop/jobs/run.py", &pad(2));
    let listing = Listing {
        modules: [
            "manage.py",
            "shop/__init__.py",
            "shop/api/views.py",
            "shop/api/serializers.py",
            "shop/jobs/run.py",
        ]
        .map(str::to_owned)
        .to_vec(),
        ..Listing::default()
    };
    let tree = Parsed::read(&root, &PYTHON, listing, |path, text| {
        let lines = u32::try_from(text.lines().count()).expect("a short file");
        let mut m = module(path, lines);
        m.0.text = text;
        m
    })
    .settle(Stub::default());
    let input = SourceInput::workspace("shop", root.to_str().expect("a UTF-8 scratch root"));
    let model = Scripted::answering([r#"{"surfaces":[]}"#]);

    let survey = run(&model, &input, &tree).await.expect("surveyed");

    let user = &model.seen()[0].messages[0];
    assert!(
        user.contains("The tree has no `pyproject.toml` or `setup.cfg` the parser could read."),
        "{user}"
    );
    assert_eq!(survey.seams.len(), 2);
    assert_eq!(survey.seams[0].stems, ["api"]);
    assert_eq!(
        files(&survey, 0),
        ["manage.py", "shop/__init__.py", "shop/api/serializers.py", "shop/api/views.py"]
    );
    assert_eq!(survey.seams[1].stems, ["jobs"]);
    assert_eq!(files(&survey, 1), ["shop/jobs/run.py"]);
    assert!(
        survey.seams[0]
            .text
            .contains("under `shop/api/`, with the root's own modules alone, under the stem `api`"),
        "{}",
        survey.seams[0].text
    );
    model.assert_exhausted();
}

// A workspace of one module is within the budget however long the module,
// and an inline value is refused before any turn.
#[tokio::test]
async fn one_module_fits() {
    let tmp = tempfile::tempdir().expect("tempdir");
    write(tmp.path(), "main.ts", &pad(10_000));
    let listing = Listing {
        modules: vec!["main.ts".to_owned()],
        ..Listing::default()
    };
    let tree = Parsed::read(tmp.path(), &TYPESCRIPT, listing, |path, text| {
        let mut m = module(path, 10_000);
        m.0.text = text;
        m.0.bindings = vec![function("run", line(1))];
        m.0.exports = vec![export("run", ExportKind::Function, line(1))];
        m
    })
    .settle(Stub::default());
    let input = SourceInput::workspace("one", tmp.path().to_str().expect("a UTF-8 scratch root"));
    let model = Scripted::answering([
        r#"{"surfaces":[{"name":"run","anchor":"main.ts#L1","stem":"run"}]}"#,
    ]);

    let survey = run(&model, &input, &tree).await.expect("surveyed");
    assert_eq!(survey.seams.len(), 1, "one module is one seam past the budget");
    assert_eq!(survey.seams[0].stems, ["run"]);
    assert_eq!(files(&survey, 0), ["main.ts"]);
    assert_eq!(survey.seams[0].anchors[0], "main.ts#L1");
    model.assert_exhausted();

    let value = SourceInput::value("one", "export function run() {}");
    let model = Scripted::answering([] as [&str; 0]);
    let error = run(&model, &value, &tree).await.expect_err("no tree to survey");
    assert_eq!(error.code(), "server_error", "{error}");
    model.assert_exhausted();
}

// An exported class is an `enum` where the dialect's bases say so, and is
// spelled under the dialect's class syntax.
#[test]
fn types_by_dialect() {
    let mut m = module("shop/money.py", 4);
    m.0.classes = vec![class_decl(
        "Currency",
        "class Currency(Enum):",
        span(1, 4),
        vec![member("USD", MemberKind::Field, "USD = \"usd\"", line(2))],
    )];
    let mut empty = module("shop/empty.py", 1);
    empty.0.classes = vec![class_decl("Marker", "class Marker:", line(1), Vec::new())];

    let claims = emery_sdk::survey::types(&PYTHON, [&*m, &*empty], false);
    let rendered: Vec<(Option<String>, Option<String>, String)> = claims
        .iter()
        .map(|claim| {
            (
                claim.path.clone(),
                claim.synopsis.clone(),
                claim.extras["signature"].as_str().expect("signature").to_owned(),
            )
        })
        .collect();
    assert_eq!(
        rendered,
        [
            (
                None,
                Some("exported enum".to_owned()),
                "class Currency(Enum):\n    USD = \"usd\"".to_owned()
            ),
            (None, Some("exported class".to_owned()), "class Marker:\n    ...".to_owned()),
        ],
        "unanchored for an inline value; a body with no member holds `...`"
    );
}
