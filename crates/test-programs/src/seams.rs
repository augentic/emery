//! Guest execution of [`SeamsScenario`] over hand-built trees.

use std::collections::BTreeMap;
use std::path::Path;

use emery_prose::Doc;
use emery_sdk::Context;
use emery_sdk::survey::Lines;
use emery_sdk::survey::code::{
    Arg, Binding, BindingKind, Call, Callee, EnvRead, Export, ExportKind, Init, Listing, Manifest,
    MemberKind, Parsed, Runs, Scope, Tree, Use,
};
use emery_sdk::survey::resolve::Target;

use crate::helpers::{err, ok};
use crate::protocol::{
    BootstrapSpec, Response, SeamsOutcome, SeamsScenario, SeamsTree, SeamsTreeKind,
};
use crate::stub::{
    PYTHON, Plain, Stub, TYPESCRIPT, binding, call, callee, class, class_decl, decorated, dynamic,
    export, from_module, from_package, function, line, literal, member, module, name, named_import,
    path, reference, span, tree, value,
};

const SEAMS_PROSE: &[Doc] = &[
    Doc {
        path: "survey.md",
        body: "SURVEY",
    },
    Doc {
        path: "extract.md",
        body: "SYSTEM",
    },
];

/// Runs [`emery_sdk::survey::seams`] for `scenario` and converts the result to a [`Response`].
pub async fn run(scenario: &SeamsScenario) -> Response<SeamsOutcome> {
    let input = scenario.input.source_input();
    let root = scenario.input.root();
    let settled = build_tree(&scenario.tree, &root);
    let ctx = Context::new(&scenario.adapter_id, input);
    match emery_sdk::survey::seams(&ctx, SEAMS_PROSE, &settled).await {
        Ok(survey) => ok(SeamsOutcome::from(survey)),
        Err(error) => err(&error),
    }
}

fn build_tree(spec: &SeamsTree, root: &Path) -> Tree<Stub> {
    match spec {
        SeamsTree::Service { bootstrap } => {
            service(root, bootstrap.as_ref().map(BootstrapSpec::to_runs))
        }
        SeamsTree::Fixed { kind } => match kind {
            SeamsTreeKind::ClassDecorator => class_decorator(root),
            SeamsTreeKind::ExportsDeclared => exports_declared(root),
            SeamsTreeKind::ByStemLoaded => by_stem_loaded(root),
            SeamsTreeKind::ByDirectoryPackage => by_directory_package(root),
            SeamsTreeKind::OneModule => one_module(root),
            SeamsTreeKind::Handling => handling(root),
        },
    }
}

fn service(root: &Path, bootstrap: Option<Runs>) -> Tree<Stub> {
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
        fill_service(&mut m);
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
    parsed.settle(stub)
}

fn fill_service(m: &mut Plain) {
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

fn class_decorator(root: &Path) -> Tree<Stub> {
    let mut index = module("src/index.ts", 2);
    index.0.bindings = vec![function("run", line(1))];
    index.0.exports = vec![export("run", ExportKind::Function, line(1))];
    let mut nightly = module("src/jobs/nightly.ts", 6);
    nightly.0.imports = vec![from_package("Processor", "@nestjs/bull")];
    nightly.0.bindings = vec![class("Nightly", &[], span(2, 6))];
    nightly.0.decorated = vec![decorated(
        Some("Nightly"),
        None,
        &["Processor"],
        Some("nightly"),
        span(2, 3),
        span(2, 6),
    )];
    nightly.0.classes = vec![class_decl("Nightly", "export class Nightly", span(2, 6), Vec::new())];
    tree(root, &TYPESCRIPT, vec![index, nightly], Stub::default())
}

fn exports_declared(root: &Path) -> Tree<Stub> {
    let mut index = module("src/index.ts", 5);
    index.0.bindings = vec![
        function("run", line(1)),
        class("Dto", &["TypedDict"], span(2, 3)),
        class("Plain", &[], span(4, 5)),
    ];
    index.0.classes = vec![
        class_decl("Dto", "class Dto extends TypedDict", span(2, 3), Vec::new()),
        class_decl("Plain", "class Plain", span(4, 5), Vec::new()),
    ];
    index.0.exports = vec![
        export("run", ExportKind::Function, line(1)),
        Export {
            local: Some("Dto".to_owned()),
            ..export("OrderDto", ExportKind::Class, line(5))
        },
        Export {
            local: Some("Plain".to_owned()),
            ..export("Entity", ExportKind::Class, line(5))
        },
    ];
    tree(root, &TYPESCRIPT, vec![index], Stub::default())
}

fn by_stem_loaded(root: &Path) -> Tree<Stub> {
    let mut modules = Vec::new();
    for (path, lines) in [
        ("main.ts", 10_003),
        ("index.ts", 2),
        ("src/api/x.ts", 2),
        ("src/jobs/loader.ts", 3),
        ("src/jobs/other.ts", 2),
        ("src/plugins/p.ts", 2),
    ] {
        let mut m = module(path, u32::try_from(lines).expect("a short file"));
        m.0.text = "// pad..\n".repeat(lines);
        modules.push(m);
    }
    modules[0].0.imports =
        vec![named_import("gone", "./gone", Target::Unresolved("./gone".to_owned()))];
    modules[3].0.dynamic = vec![dynamic(line(2), Some("src/plugins")), dynamic(line(3), None)];
    tree(root, &TYPESCRIPT, modules, Stub::default())
}

fn by_directory_package(root: &Path) -> Tree<Stub> {
    let listing = Listing {
        modules: [
            "manage.py",
            "shop/__init__.py",
            "shop/__init__.pyi",
            "shop/api/views.py",
            "shop/api/serializers.py",
            "shop/jobs/run.py",
        ]
        .map(str::to_owned)
        .to_vec(),
        ..Listing::default()
    };
    Parsed::read(root, &PYTHON, listing, |path, text| {
        let lines = u32::try_from(text.lines().count()).expect("a short file");
        let mut m = module(path, lines);
        m.0.text = text;
        m
    })
    .settle(Stub::default())
}

fn one_module(root: &Path) -> Tree<Stub> {
    let listing = Listing {
        modules: vec!["main.ts".to_owned()],
        ..Listing::default()
    };
    Parsed::read(root, &TYPESCRIPT, listing, |path, text| {
        let mut m = module(path, 10_000);
        m.0.text = text;
        m.0.bindings = vec![function("run", line(1))];
        m.0.exports = vec![export("run", ExportKind::Function, line(1))];
        m
    })
    .settle(Stub::default())
}

fn local(name: &str, root: &[&str], index: usize, at: Lines) -> Binding {
    binding(
        name,
        Scope::Function(1),
        BindingKind::Value {
            root: Some(path(root)),
            type_path: None,
            call: Some(index),
            string: None,
            init: Init::Construction,
            head: Some(format!("{name} = ..")),
            table: None,
        },
        at,
    )
}

fn param(name: &str, at: Lines) -> Binding {
    binding(name, Scope::Function(1), BindingKind::Param { type_path: None }, at)
}

fn within(callee: Callee, args: Vec<Arg>, value: Use, at: Lines) -> Call {
    Call {
        frames: vec![1],
        function: Some("run".to_owned()),
        class: Some("Handler".to_owned()),
        ..call(callee, args, value, at)
    }
}

fn handling(root: &Path) -> Tree<Stub> {
    let mut api = module("app/api.py", 16);
    api.0.imports = vec![
        named_import("user_list", ".services", Target::Module("app/services.py".to_owned())),
        named_import("OrdersRepository", ".services", Target::Module("app/services.py".to_owned())),
        from_package("requests", "requests"),
        from_package("psycopg2", "psycopg2"),
    ];
    api.0.bindings = vec![
        class("Handler", &[], span(4, 15)),
        param("customers", line(5)),
        param("limit", line(5)),
        local("users", &["user_list"], 0, line(6)),
        local("repo", &["OrdersRepository"], 1, line(7)),
        local("orders", &["repo", "list"], 2, line(8)),
        local("report", &["self", "reporter", "build"], 3, line(9)),
        local("ok", &["customers", "register"], 4, line(10)),
        local("resp", &["requests", "get"], 6, line(12)),
        local("conn", &["psycopg2", "connect"], 7, line(13)),
        local("cur", &["conn", "cursor"], 8, line(14)),
        value("cache", Some(&["user_list"]), Some(10), None, Init::Construction, line(16)),
    ];
    api.0.calls = vec![
        within(callee("user_list", &[]), vec![name(&["limit"], line(6))], Use::Bound, line(6)),
        Call {
            constructs: true,
            ..within(
                callee("OrdersRepository", &[]),
                vec![name(&["users"], line(7))],
                Use::Bound,
                line(7),
            )
        },
        within(callee("repo", &["list"]), Vec::new(), Use::Bound, line(8)),
        within(
            callee("self", &["reporter", "build"]),
            vec![name(&["orders"], line(9))],
            Use::Bound,
            line(9),
        ),
        within(
            callee("customers", &["register"]),
            vec![name(&["orders"], line(10))],
            Use::Bound,
            line(10),
        ),
        within(callee("user_list", &[]), vec![name(&["limit"], line(11))], Use::Consumed, line(11)),
        within(
            callee("requests", &["get"]),
            vec![literal("https://x", line(12))],
            Use::Bound,
            line(12),
        ),
        within(
            callee("psycopg2", &["connect"]),
            vec![name(&["dsn"], line(13))],
            Use::Bound,
            line(13),
        ),
        within(callee("conn", &["cursor"]), Vec::new(), Use::Bound, line(14)),
        within(
            callee("cur", &["execute"]),
            vec![literal("select 1", line(15))],
            Use::Discarded,
            line(15),
        ),
        call(callee("user_list", &[]), Vec::new(), Use::Bound, line(16)),
    ];
    api.0.classes = vec![class_decl(
        "Handler",
        "class Handler:",
        span(4, 15),
        vec![member("run", MemberKind::Method, "def run(self, customers, limit)", span(5, 15))],
    )];
    let mut services = module("app/services.py", 6);
    services.0.bindings =
        vec![function("user_list", span(1, 2)), class("OrdersRepository", &[], span(4, 6))];
    services.0.exports = vec![
        export("user_list", ExportKind::Function, span(1, 2)),
        export("OrdersRepository", ExportKind::Class, span(4, 6)),
    ];
    tree(root, &PYTHON, vec![api, services], Stub::default())
}
