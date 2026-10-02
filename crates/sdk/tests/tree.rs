//! A tree answers what a survey asks of it as a whole, by code alone; every
//! scenario here builds one by hand and reads it with no model and no parser.

mod support;

use std::path::Path;

use emery_sdk::survey::Lines;
use emery_sdk::survey::code::{
    Arg, BindingKind, Bootstrap, ExportKind, Imported, Init, Listing, Manifest, MemberKind, Parsed,
    Runs, Scope, Surface, TRACE, Use,
};
use emery_sdk::survey::resolve::Target;
use emery_sdk::survey::tests::Statement;
use support::{
    PYTHON, Plain, Stub, TYPESCRIPT, binding, call, callee, class, class_decl, dynamic, export,
    from_module, from_package, function, import, inline, line, literal, member, module, name,
    named_import, path, reference, span, tree, value, write,
};

fn linked(from: &str, to: &[&str]) -> Plain {
    let mut m = module(from, 4);
    m.0.imports =
        to.iter().map(|to| from_module(to.split('/').next_back().unwrap_or(to), to)).collect();
    m
}

fn surface(name: &str, entry: &str, stem: &str, discriminator: Option<&str>) -> Surface {
    Surface {
        name: name.to_owned(),
        entry: entry.to_owned(),
        stem: stem.to_owned(),
        lines: line(1),
        detail: Vec::new(),
        closure: vec![entry.to_owned()],
        discriminator: discriminator.map(str::to_owned),
        methods: Vec::new(),
        ids: Vec::new(),
    }
}

// The seeds lead in their order, then what each reaches breadth-first, once
// each; a module in `stop` is reached and not followed.
#[test]
fn closure_order() {
    let tree = tree(
        Path::new("/svc"),
        &TYPESCRIPT,
        vec![
            linked("a.ts", &["b.ts", "c.ts"]),
            linked("b.ts", &["d.ts"]),
            linked("c.ts", &["a.ts"]),
            linked("d.ts", &[]),
            linked("e.ts", &["a.ts"]),
        ],
        Stub::default(),
    );

    assert_eq!(tree.closure(&["a.ts".to_owned()], &[]), ["a.ts", "b.ts", "c.ts", "d.ts"]);
    assert_eq!(
        tree.closure(&["c.ts".to_owned(), "a.ts".to_owned(), "x.ts".to_owned()], &[]),
        ["c.ts", "a.ts", "b.ts", "d.ts"],
        "the seeds lead in their order; one the tree lacks is skipped"
    );
    assert_eq!(
        tree.closure(&["a.ts".to_owned()], &["b.ts".to_owned()]),
        ["a.ts", "b.ts", "c.ts"],
        "a stopped module is reached and not followed"
    );
    assert_eq!(tree.roots(), ["e.ts"], "the one module nothing imports");
}

#[test]
fn roots_of_a_cycle() {
    let tree = tree(
        Path::new("/svc"),
        &TYPESCRIPT,
        vec![linked("a.ts", &["b.ts"]), linked("b.ts", &["a.ts"])],
        Stub::default(),
    );
    assert_eq!(tree.roots(), ["a.ts", "b.ts"], "every module, where each is imported");
}

// Each shape of import answers the module and the name it exports: a named
// one by name, a default by `default`, a whole module by the local; a star or
// effect import answers nothing.
#[test]
fn exporter_shapes() {
    let mut m = module("src/m.ts", 6);
    m.0.imports = vec![
        from_module("x", "src/lib.ts"),
        import("lib", "./lib", Imported::Whole, Target::Module("src/lib.ts".to_owned())),
        import("d", "./lib", Imported::Default, Target::Module("src/lib.ts".to_owned())),
        import("all", "./lib", Imported::Star, Target::Module("src/lib.ts".to_owned())),
        import("", "./side", Imported::Effect, Target::Module("src/side.ts".to_owned())),
        import(
            "gone",
            "./gone",
            Imported::Named("gone".to_owned()),
            Target::Unresolved("./gone".to_owned()),
        ),
    ];
    let tree = tree(
        Path::new("/svc"),
        &TYPESCRIPT,
        vec![m, module("src/lib.ts", 2), module("src/side.ts", 1)],
        Stub::default(),
    );
    let m = &tree.modules["src/m.ts"];
    let exported =
        |local: &str| tree.exporter(m, local).map(|(target, name)| (target.path.clone(), name));

    assert_eq!(exported("x"), Some(("src/lib.ts".to_owned(), "x".to_owned())));
    assert_eq!(exported("lib"), Some(("src/lib.ts".to_owned(), "lib".to_owned())));
    assert_eq!(exported("d"), Some(("src/lib.ts".to_owned(), "default".to_owned())));
    assert_eq!(exported("all"), None);
    assert_eq!(exported(""), None, "an import binding nothing is never found");
    assert_eq!(exported("gone"), None, "an unresolved specifier leads nowhere");
}

// A literal, or a name bound to one in the module or in the module of the
// tree it is imported from; a called argument spells nothing.
#[test]
fn constant_led() {
    let mut routes = module("src/routes.ts", 8);
    routes.0.imports =
        vec![from_module("ORDERS", "src/paths.ts"), from_package("express", "express")];
    routes.0.bindings = vec![
        value("LOCAL", None, None, Some("/local"), Init::Literal, line(2)),
        value("app", Some(&["express"]), Some(0), None, Init::Construction, line(1)),
    ];
    routes.0.calls = vec![
        call(callee("express", &[]), Vec::new(), Use::Consumed, line(1)),
        call(
            callee("app", &["get"]),
            vec![name(&["LOCAL"], line(3)), inline(line(3))],
            Use::Discarded,
            line(3),
        ),
        call(
            callee("app", &["get"]),
            vec![name(&["ORDERS"], line(4)), inline(line(4))],
            Use::Discarded,
            line(4),
        ),
        call(
            callee("app", &["get"]),
            vec![literal("/lit", line(5)), inline(line(5))],
            Use::Discarded,
            line(5),
        ),
        call(
            callee("app", &["get"]),
            vec![
                Arg {
                    called: true,
                    ..name(&["join"], line(6))
                },
                inline(line(6)),
            ],
            Use::Discarded,
            line(6),
        ),
    ];
    let mut paths = module("src/paths.ts", 2);
    paths.0.bindings = vec![value("ORDERS", None, None, Some("/orders"), Init::Literal, line(1))];
    paths.0.exports = vec![export("ORDERS", ExportKind::Value, line(1))];
    let tree = tree(Path::new("/svc"), &TYPESCRIPT, vec![routes, paths], Stub::default());
    let routes = &tree.modules["src/routes.ts"];

    let led: Vec<Option<String>> =
        routes.calls[1..].iter().map(|call| tree.led(routes, call)).collect();
    assert_eq!(
        led,
        [Some("/local".to_owned()), Some("/orders".to_owned()), Some("/lit".to_owned()), None]
    );
}

// A receiver is traced through the bindings that construct or type it to the
// package it comes from, and the type it is known as where the code says.
#[test]
fn receiver_traced() {
    let mut m = module("src/m.ts", 12);
    m.0.imports = vec![
        from_package("express", "express"),
        from_package("Queue", "bullmq"),
        from_package("Mailer", "mailer"),
        from_package("Router", "express"),
        from_module("Repo", "src/repo.ts"),
    ];
    m.0.bindings = vec![
        value("app", Some(&["express"]), Some(0), None, Init::Construction, line(1)),
        binding(
            "queue",
            Scope::Module,
            BindingKind::Value {
                root: Some(path(&["Queue"])),
                type_path: None,
                call: Some(1),
                string: None,
                init: Init::Construction,
                head: None,
            },
            line(2),
        ),
        binding(
            "mailer",
            Scope::Function(1),
            BindingKind::Param {
                type_path: Some(path(&["Mailer"])),
            },
            line(3),
        ),
        class("Own", &["Base"], span(5, 9)),
        value("repo", Some(&["Repo"]), Some(2), None, Init::Construction, line(10)),
        value("local", Some(&["Own"]), None, None, Init::Construction, line(11)),
    ];
    m.0.classes = vec![class_decl(
        "Own",
        "class Own extends Base",
        span(5, 9),
        vec![member("declared", MemberKind::Method, "declared()", line(6))],
    )];
    m.0.imports.push(from_package("Base", "framework"));
    let mut constructing = call(callee("Queue", &[]), Vec::new(), Use::Consumed, line(2));
    constructing.constructs = true;
    m.0.calls = vec![
        call(callee("express", &[]), Vec::new(), Use::Consumed, line(1)),
        constructing,
        call(callee("Repo", &[]), Vec::new(), Use::Consumed, line(10)),
        call(callee("app", &["get"]), Vec::new(), Use::Discarded, line(4)),
        call(callee("queue", &["add"]), Vec::new(), Use::Discarded, line(4)),
        {
            let mut c = call(callee("mailer", &["send"]), Vec::new(), Use::Discarded, line(4));
            c.frames = vec![1];
            c
        },
        call(callee("Router", &["get"]), Vec::new(), Use::Discarded, line(4)),
        call(callee("repo", &["find"]), Vec::new(), Use::Discarded, line(4)),
        call(callee("local", &["declared"]), Vec::new(), Use::Discarded, line(4)),
        call(callee("local", &["inherited"]), Vec::new(), Use::Discarded, line(4)),
    ];
    let mut repo = module("src/repo.ts", 2);
    repo.0.bindings = vec![class("Repo", &[], span(1, 2))];
    repo.0.exports = vec![export("Repo", ExportKind::Class, span(1, 2))];
    let tree = tree(Path::new("/svc"), &TYPESCRIPT, vec![m, repo], Stub::default());
    let m = &tree.modules["src/m.ts"];
    let receiver =
        |index: usize| tree.receiver(m, &m.calls[index]).map(|r| (r.package, r.type_name));

    assert_eq!(receiver(3), Some(("express".to_owned(), None)), "a lowercase head names no type");
    assert_eq!(
        receiver(4),
        Some(("bullmq".to_owned(), Some("Queue".to_owned()))),
        "constructed as"
    );
    assert_eq!(receiver(5), Some(("mailer".to_owned(), Some("Mailer".to_owned()))), "typed as");
    assert_eq!(
        receiver(6),
        Some(("express".to_owned(), Some("Router".to_owned()))),
        "a capitalised package head is the type itself"
    );
    assert_eq!(receiver(7), None, "a class of the tree is the tree's own");
    assert_eq!(receiver(8), None, "a member the class declares is its own");
    assert_eq!(
        receiver(9),
        Some(("framework".to_owned(), Some("Base".to_owned()))),
        "a member inherited from a package's base is that package's"
    );
    assert_eq!(
        tree.receiver_of(m, "app").map(|r| r.package),
        Some("express".to_owned()),
        "a decorator's head is traced as a receiver"
    );
    assert_eq!(TRACE, 4);
}

// A stem's one surface is its stem; surfaces sharing one are told apart by
// their discriminator, else their name, else their entry's module stem, and
// two still alike by the nearest segment of their paths; a class carries an
// id per method.
#[test]
fn identify_ids() {
    let tree = tree(
        Path::new("/svc"),
        &TYPESCRIPT,
        vec![
            module("src/orders/index.ts", 2),
            module("src/local/files.ts", 2),
            module("src/s3/files.ts", 2),
            module("src/users.ts", 2),
        ],
        Stub::default(),
    );
    let mut surfaces = vec![
        surface("start", "src/orders/index.ts", "start", None),
        surface("POST /orders", "src/orders/index.ts", "orders", Some("create")),
        surface("GET /orders", "src/orders/index.ts", "orders", Some("list")),
        surface("orders", "src/orders/index.ts", "orders", None),
        surface("files", "src/local/files.ts", "files", None),
        surface("files", "src/s3/files.ts", "files", None),
        Surface {
            methods: vec!["find".to_owned(), "save".to_owned()],
            ..surface("users repo", "src/users.ts", "users", None)
        },
    ];

    tree.identify(&mut surfaces);
    let ids: Vec<Vec<String>> = surfaces.iter().map(|surface| surface.ids.clone()).collect();
    assert_eq!(
        ids,
        [
            vec!["start".to_owned()],
            vec!["orders.create".to_owned()],
            vec!["orders.list".to_owned()],
            vec!["orders.orders".to_owned()],
            vec!["files.files.local".to_owned()],
            vec!["files.files.s3".to_owned()],
            vec!["users".to_owned(), "users.find".to_owned(), "users.save".to_owned()],
        ]
    );
}

// A seam widens past an import the resolver could not follow to the
// importing module's own directory, and past a load by a computed name to
// the directory the load spells, or the loader's own where it spells none.
// A directory's modules are those directly beneath it, the root's own at
// the root; a file the survey set aside widens nothing, and what the seam
// lays already is not laid again.
#[test]
fn widening() {
    let mut aliased = module("src/api/a.ts", 3);
    aliased.0.imports =
        vec![named_import("x", "@alias/x", Target::Unresolved("@alias/x".to_owned()))];
    let mut aside = module("src/api/d.ts", 3);
    aside.0.imports = vec![named_import(
        "fake",
        "./fake.test",
        Target::Skipped("src/api/fake.test.ts".to_owned()),
    )];
    let mut spelled = module("src/jobs/b.ts", 3);
    spelled.0.dynamic = vec![dynamic(line(2), Some("src/plugins"))];
    let mut unspelled = module("src/jobs/c.ts", 3);
    unspelled.0.dynamic = vec![dynamic(line(1), None)];
    let mut top = module("main.ts", 3);
    top.0.imports = vec![named_import("y", "./gone", Target::Unresolved("./gone".to_owned()))];
    let tree = tree(
        Path::new("/svc"),
        &TYPESCRIPT,
        vec![
            top,
            module("index.ts", 3),
            aliased,
            aside,
            module("src/api/e.ts", 3),
            module("src/api/sub/f.ts", 3),
            spelled,
            unspelled,
            module("src/jobs/d.ts", 3),
            module("src/plugins/p.ts", 3),
            module("src/plugins/q.ts", 3),
        ],
        Stub::default(),
    );
    let widening =
        |files: &[&str]| tree.widening(&files.iter().map(|f| (*f).to_owned()).collect::<Vec<_>>());

    assert_eq!(
        widening(&["src/api/a.ts"]),
        ["src/api/d.ts", "src/api/e.ts"],
        "the importer's own directory, and nothing beneath it"
    );
    assert_eq!(
        widening(&["src/jobs/b.ts"]),
        ["src/plugins/p.ts", "src/plugins/q.ts"],
        "the directory the load spells"
    );
    assert_eq!(
        widening(&["src/jobs/c.ts"]),
        ["src/jobs/b.ts", "src/jobs/d.ts"],
        "the loader's own where it spells none"
    );
    assert_eq!(
        widening(&["src/jobs/b.ts", "src/jobs/c.ts"]),
        ["src/jobs/d.ts", "src/plugins/p.ts", "src/plugins/q.ts"],
        "both directories in path order, less what the seam lays"
    );
    assert_eq!(widening(&["main.ts"]), ["index.ts"], "the root's own at the root");
    assert_eq!(widening(&["src/api/d.ts"]), [] as [String; 0], "a file set aside widens nothing");
    assert_eq!(widening(&["src/api/e.ts"]), [] as [String; 0]);
}

// A call registers when it hands a handler and is discarded, constructs, or
// is led by a literal, whatever its depth; a structural call or a wrapper
// taken for its value registers nothing; `handed` names the package at
// depth zero alone.
#[test]
fn hands_and_registers() {
    let mut m = module("src/m.ts", 8);
    m.0.imports = vec![from_package("express", "express"), from_package("retrying", "retry")];
    m.0.bindings = vec![
        value("app", Some(&["express"]), Some(0), None, Init::Construction, line(1)),
        function("list", line(2)),
    ];
    let mut deep = call(
        callee("app", &["get"]),
        vec![literal("/deep", line(5)), name(&["list"], line(5))],
        Use::Discarded,
        line(5),
    );
    deep.depth = 1;
    deep.frames = vec![1];
    m.0.calls = vec![
        call(callee("express", &[]), Vec::new(), Use::Consumed, line(1)),
        call(
            callee("app", &["get"]),
            vec![literal("/x", line(3)), name(&["list"], line(3))],
            Use::Discarded,
            line(3),
        ),
        call(
            callee("app", &["use"]),
            vec![name(&["list"], line(4)), inline(line(4))],
            Use::Discarded,
            line(4),
        ),
        deep,
        call(callee("retrying", &[]), vec![name(&["list"], line(6))], Use::Consumed, line(6)),
        call(callee("app", &["get"]), vec![literal("/none", line(7))], Use::Discarded, line(7)),
    ];
    let tree = tree(Path::new("/svc"), &TYPESCRIPT, vec![m], Stub::default());
    let m = &tree.modules["src/m.ts"];

    assert!(tree.registers(m, &m.calls[1]));
    assert_eq!(tree.handed(m, &m.calls[1]).map(|r| r.package), Some("express".to_owned()));
    assert!(!tree.hands(m, &m.calls[2]), "`use` is structure");
    assert!(tree.registers(m, &m.calls[3]), "at any depth");
    assert_eq!(tree.handed(m, &m.calls[3]), None, "`handed` is for a call outside any handler");
    assert!(tree.hands(m, &m.calls[4]));
    assert!(!tree.registers(m, &m.calls[4]), "a wrapper taken for its value registers nothing");
    assert!(!tree.hands(m, &m.calls[5]), "nothing handed");
    assert_eq!(tree.registration_at(m, span(1, 3)).map(|c| c.lines), Some(line(3)));
    assert_eq!(tree.registration_enclosing(m, line(5)).map(|c| c.lines), Some(line(5)));
    assert_eq!(tree.registration_at(m, line(6)), None);
}

// The bootstrap's `start` reaches what its module references outside the
// registrations, stops at another surface's entry, and reaches what that
// entry constructs; its detail says how the bootstrap runs.
#[test]
fn start_surface() {
    let mut main = module("src/main.ts", 6);
    main.0.imports =
        vec![from_module("orders", "src/orders.ts"), from_module("config", "src/config.ts")];
    main.0.references = vec![reference("config", 2), reference("orders", 4)];
    let mut orders = module("src/orders.ts", 6);
    orders.0.imports =
        vec![from_module("Store", "src/store.ts"), from_module("helper", "src/helper.ts")];
    orders.0.bindings =
        vec![value("store", Some(&["Store"]), None, None, Init::Construction, line(1))];
    let tree = tree(
        Path::new("/svc"),
        &TYPESCRIPT,
        vec![
            main,
            orders,
            module("src/config.ts", 2),
            module("src/store.ts", 2),
            module("src/helper.ts", 2),
        ],
        Stub::default(),
    );
    let bootstrap = Bootstrap {
        module: &tree.modules["src/main.ts"],
        runs: Runs::Script {
            name: "svc".to_owned(),
            function: "main".to_owned(),
        },
    };
    let registered = [Surface {
        lines: span(3, 5),
        ..surface("orders", "src/orders.ts", "orders", None)
    }];

    let start = Surface::start(&tree, &bootstrap, &registered);
    assert_eq!(start.ids, ["start"]);
    assert_eq!(
        start.closure,
        ["src/main.ts", "src/config.ts", "src/orders.ts", "src/store.ts"],
        "another surface's entry is reached and not followed; what it constructs is"
    );
    assert_eq!(
        start.detail,
        [
            "the process bootstrap, run by the console script `svc`, which calls `main()`: what runs \
          before each handler is registered, what it awaits before serving, and at shutdown — \
          `stop` and what a signal handler calls, wherever declared"
        ]
    );
    assert_eq!(Runs::Guard.clone(), Runs::Guard);
}

// A listing is read from disk: a module the parser is handed, a test module
// read for what it imports and states, a feature inheriting the imports of
// the tests beside it, an unreadable file left out, and a test stating
// nothing dropped.
#[test]
fn parsed_read() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    write(root, "src/a.py", "import b\n");
    write(root, "src/b.py", "x = 1\n");
    write(root, "tests/test_a.py", "def test_x():\n    pass\n");
    write(root, "tests/test_empty.py", "\n");
    write(root, "tests/features/a.feature", "Feature: A\n  Scenario: lists\n    Given x\n");
    std::fs::write(root.join("src/bad.py"), [0xff, 0xfe]).expect("write");
    let listing = Listing {
        modules: vec!["src/a.py".to_owned(), "src/b.py".to_owned(), "src/bad.py".to_owned()],
        data: vec!["data.json".to_owned()],
        tests: vec![
            "tests/test_a.py".to_owned(),
            "tests/test_empty.py".to_owned(),
            "tests/features/a.feature".to_owned(),
        ],
    };

    let parsed = Parsed::read(root, &PYTHON, listing, |path, text| {
        let mut m = module(path, 1);
        m.0.text = text;
        if path == "tests/test_a.py" {
            m.0.imports = vec![from_module("a", "src/a.py")];
            m.0.references = vec![reference("x", 1)];
        }
        if path == "src/a.py" {
            m.0.imports = vec![from_module("b", "src/b.py")];
        }
        m
    });
    assert_eq!(
        parsed.modules.keys().collect::<Vec<_>>(),
        ["src/a.py", "src/b.py"],
        "the unreadable one is left out"
    );
    assert_eq!(parsed.data, ["data.json"]);
    assert_eq!(parsed.modules["src/a.py"].text, "import b\n");

    let tree = parsed.settle(Stub {
        manifest: Manifest {
            name: Some("svc".to_owned()),
            ..Manifest::default()
        },
        ..Stub::default()
    });
    assert_eq!(tree.manifest.name.as_deref(), Some("svc"));
    assert_eq!(tree.tests.len(), 2, "the test stating nothing is dropped");
    assert_eq!(tree.tests[0].path, "tests/test_a.py");
    assert_eq!(tree.tests[0].imports, ["src/a.py"]);
    assert_eq!(
        tree.tests[0].statements,
        [Statement {
            text: "states x".to_owned(),
            line: 1
        }]
    );
    assert_eq!(tree.tests[1].path, "tests/features/a.feature");
    assert_eq!(tree.tests[1].imports, ["src/a.py"], "inherited from the tests under the parent");
    assert_eq!(tree.tests[1].statements[0].text, "A › lists");
    assert_eq!(tree.closure(&["src/a.py".to_owned()], &[]), ["src/a.py", "src/b.py"]);
}

#[test]
fn lines_of_a_module() {
    let m = module("x.ts", 0);
    assert_eq!(m.span, Lines { start: 1, end: 1 }, "one line at the least");
}
