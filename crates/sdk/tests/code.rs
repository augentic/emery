//! The module an adapter's parser fills is read through its lookups alone;
//! every scenario here builds one by hand, with no parser, no model, and no
//! tree, and reads it under a dialect shaped like each adapter's.

use emery_sdk::survey::code::{
    Arg, Binding, BindingKind, Call, Callee, ClassDecl, Decorated, Export, ExportKind, Import,
    Imported, Init, Invocation, Link, Module, Property, Reexport, Reference, Scope, Use,
};
use emery_sdk::survey::resolve::Target;
use emery_sdk::survey::route::Spelling;
use emery_sdk::survey::{ClassSyntax, Dialect, Lines};

static TYPESCRIPT: Dialect = Dialect {
    self_name: "this",
    generic_stems: &["index", "main"],
    structural: &["use", "then", "forEach", "listen"],
    listeners: &["on", "once"],
    lifecycle_events: &["error", "close", "SIGTERM"],
    mocking: &[],
    lifecycle: &[],
    decorator_noise: &["UseGuards", "Injectable"],
    decorator_noise_prefixes: &["Api"],
    decorator_hooks: &[],
    hook_keywords: &[],
    type_imports_reach: true,
    openers: &['[', '{'],
    comment_prefixes: &["//", "*"],
    globals: &["fetch"],
    options: &[],
    manifests: &["package.json"],
    barrels: &[],
    constructs: Some("new"),
    env_object: Some("process.env"),
    enum_bases: &[],
    class_syntax: ClassSyntax::BRACED,
    described: "a case's title under its suites'",
    route: Spelling {
        pattern: &['*', '{', '(', '['],
        param_name: |segment| segment.trim_start_matches(':'),
    },
};

static PYTHON: Dialect = Dialect {
    self_name: "self",
    generic_stems: &["__init__", "main", "app", "views", "urls"],
    structural: &["map", "partial", "include_router"],
    listeners: &[],
    lifecycle_events: &[],
    mocking: &["patch", "mock.patch"],
    lifecycle: &["atexit.register", "on_event", "site.register", "admin.register"],
    decorator_noise: &["dataclass", "property"],
    decorator_noise_prefixes: &[],
    decorator_hooks: &["receiver", "errorhandler"],
    hook_keywords: &["lifespan", "callback"],
    type_imports_reach: false,
    openers: &['[', '{', '('],
    comment_prefixes: &["#"],
    globals: &["open"],
    options: &["add_argument", "option"],
    manifests: &["pyproject.toml", "setup.cfg"],
    barrels: &["__init__"],
    constructs: None,
    env_object: None,
    enum_bases: &["Enum", "Flag"],
    class_syntax: ClassSyntax::INDENTED,
    described: "a test's docstring or name under its class's",
    route: Spelling {
        pattern: &['*', '{', '(', '[', '<'],
        param_name: |segment| segment.trim_start_matches(':'),
    },
};

const fn span(start: u32, end: u32) -> Lines {
    Lines { start, end }
}

fn import(local: &str, specifier: &str, type_only: bool, target: Target) -> Import {
    Import {
        local: local.to_owned(),
        specifier: specifier.to_owned(),
        imported: Imported::Named(local.to_owned()),
        type_only,
        target: Some(target),
    }
}

fn reexport(specifier: &str, type_only: bool, target: Target) -> Reexport {
    Reexport {
        specifier: specifier.to_owned(),
        names: None,
        type_only,
        target: Some(target),
    }
}

fn binding(name: &str, scope: Scope, kind: BindingKind, lines: Lines) -> Binding {
    Binding {
        name: name.to_owned(),
        scope,
        kind,
        lines,
    }
}

fn path(segments: &[&str]) -> Vec<String> {
    segments.iter().copied().map(str::to_owned).collect()
}

const fn value(root: Option<Vec<String>>, type_path: Option<Vec<String>>) -> BindingKind {
    BindingKind::Value {
        root,
        type_path,
        call: None,
        string: None,
        init: Init::Construction,
        head: None,
    }
}

fn callee(head: &str, links: &[&str]) -> Callee {
    Callee {
        head: head.to_owned(),
        head_call: None,
        links: links
            .iter()
            .map(|&name| Link {
                name: name.to_owned(),
                call: None,
            })
            .collect(),
    }
}

const fn call(callee: Callee, args: Vec<Arg>) -> Call {
    Call {
        callee,
        constructs: false,
        args,
        depth: 0,
        value: Use::Discarded,
        inner: false,
        frames: Vec::new(),
        function: None,
        class: None,
        lines: span(1, 1),
    }
}

fn arg(keyword: Option<&str>, literal: Option<&str>) -> Arg {
    Arg {
        keyword: keyword.map(str::to_owned),
        literal: literal.map(str::to_owned),
        root: None,
        called: false,
        inner: None,
        function: false,
        properties: Vec::new(),
        head: literal.map(|text| format!("\"{text}\"")).unwrap_or_default(),
        lines: span(1, 1),
    }
}

fn decorated(name: &[&str], class: Option<&str>, member: Option<&str>) -> Decorated {
    Decorated {
        class: class.map(str::to_owned),
        member: member.map(str::to_owned),
        name: name.iter().copied().map(str::to_owned).collect(),
        literal: None,
        keywords: vec![("methods".to_owned(), "[\"GET\"]".to_owned())],
        head: span(1, 2),
        lines: span(1, 9),
    }
}

fn reference(name: &str, line: u32) -> Reference {
    Reference {
        name: name.to_owned(),
        line,
    }
}

// A module importing a module of the tree, a type alone, a package, a data
// file, and two specifiers nothing answers, one of them type-only.
fn importing() -> Module {
    Module {
        path: "src/app.ts".to_owned(),
        imports: vec![
            import("db", "./db", false, Target::Module("src/db.ts".to_owned())),
            import("Order", "./types", true, Target::Module("src/types.ts".to_owned())),
            import("express", "express", false, Target::Package("express".to_owned())),
            import("config", "./config.json", false, Target::Data("src/config.json".to_owned())),
            import("missing", "./missing", false, Target::Unresolved("./missing".to_owned())),
            import("Shape", "./shape", true, Target::Unresolved("./shape".to_owned())),
            Import {
                local: String::new(),
                specifier: "./polyfill".to_owned(),
                imported: Imported::Effect,
                type_only: false,
                target: Some(Target::Module("src/polyfill.ts".to_owned())),
            },
        ],
        reexports: vec![
            reexport("./models", true, Target::Module("src/models.ts".to_owned())),
            reexport("./db", false, Target::Module("src/db.ts".to_owned())),
        ],
        ..Module::default()
    }
}

// A type-only import or re-export reaches its module under the dialect that
// follows type imports and not under the one that does not; what nothing
// answers is unresolved under both only where it is not type-only.
#[test]
fn reached_type_only() {
    let module = importing();

    assert_eq!(
        module.reached(&TYPESCRIPT),
        ["src/db.ts", "src/types.ts", "src/polyfill.ts", "src/models.ts"],
        "once each, imports before re-exports"
    );
    assert_eq!(module.reached(&PYTHON), ["src/db.ts", "src/polyfill.ts"]);
    assert_eq!(module.unresolved(), ["./missing"]);
    assert_eq!(module.data(), ["src/config.json"]);
    assert_eq!(module.targets().count(), 9, "every settled target, type-only ones included");
}

#[test]
fn import_lookups() {
    let module = importing();

    assert_eq!(module.import("db").map(|import| import.specifier.as_str()), Some("./db"));
    assert_eq!(module.import(""), None, "an import binding nothing is never found");
    assert_eq!(module.imported("db"), Some("src/db.ts"));
    assert_eq!(module.imported("express"), None, "a package is no module");
    assert_eq!(module.package("express"), Some("express"));
    assert_eq!(module.package("db"), None, "a module is no package");
    assert_eq!(module.package("missing"), None);
}

// The innermost frame's binding wins, then the module's; a class field is
// found through its class alone.
#[test]
fn binding_frames() {
    let module = Module {
        bindings: vec![
            binding("x", Scope::Module, value(None, None), span(1, 1)),
            binding("x", Scope::Function(1), value(None, None), span(3, 3)),
            binding("x", Scope::Function(2), value(None, None), span(5, 5)),
            binding("pool", Scope::Class("Repo".to_owned()), BindingKind::Function, span(8, 8)),
        ],
        ..Module::default()
    };

    let at = |name: &str, frames: &[u32]| module.binding(name, frames).map(|b| b.lines.start);
    assert_eq!(at("x", &[1, 2]), Some(5));
    assert_eq!(at("x", &[1]), Some(3));
    assert_eq!(at("x", &[7]), Some(1), "a frame binding nothing falls to the module's");
    assert_eq!(at("x", &[]), Some(1));
    assert_eq!(at("pool", &[]), None, "a class field is no module binding");
    assert_eq!(module.field("Repo", "pool").map(|b| b.lines.start), Some(8));
    assert_eq!(module.field("Other", "pool"), None);
}

// An export list entry names a local binding; the export's own lines stand
// where the module declares none.
#[test]
fn exports_through_local() {
    let module = Module {
        exports: vec![
            Export {
                name: "handler".to_owned(),
                local: Some("run".to_owned()),
                kind: ExportKind::Function,
                lines: span(20, 20),
            },
            Export {
                name: "main".to_owned(),
                local: None,
                kind: ExportKind::Function,
                lines: span(10, 15),
            },
            Export {
                name: "VERSION".to_owned(),
                local: None,
                kind: ExportKind::Value,
                lines: span(21, 21),
            },
        ],
        bindings: vec![
            binding("run", Scope::Module, BindingKind::Function, span(3, 7)),
            binding("main", Scope::Module, BindingKind::Function, span(10, 15)),
        ],
        ..Module::default()
    };

    let handler = module.export("handler").expect("exported");
    assert_eq!(module.exported_binding("handler").map(|b| b.name.as_str()), Some("run"));
    assert_eq!(module.declared_at(handler), span(3, 7));
    assert_eq!(module.declared_at(module.export("main").expect("exported")), span(10, 15));
    assert_eq!(
        module.declared_at(module.export("VERSION").expect("exported")),
        span(21, 21),
        "no binding: the export's own lines"
    );
    assert_eq!(module.exported_binding("VERSION"), None);
    assert_eq!(module.export("nope"), None);
}

#[test]
fn export_kinds() {
    assert!(ExportKind::Function.callable() && ExportKind::Class.callable());
    assert!(!ExportKind::Value.callable() && !ExportKind::Type.callable());
    assert!(!ExportKind::Unknown.callable());
    assert!(ExportKind::Value.valued() && !ExportKind::Class.valued());

    assert!(Init::Literal.is_value() && Init::Pattern.is_value());
    assert!(Init::Env.is_value() && Init::Definition.is_value());
    assert!(!Init::Construction.is_value() && !Init::Function.is_value());
    assert!(!Init::Other.is_value());
}

// Names are listed once in first-read order, within a span or outside every
// span given; lines likewise for a name.
#[test]
fn referenced_spans() {
    let module = Module {
        references: vec![
            reference("a", 1),
            reference("b", 3),
            reference("a", 4),
            reference("c", 5),
            reference("b", 8),
            reference("d", 9),
        ],
        ..Module::default()
    };

    assert_eq!(module.referenced(span(3, 5)), ["b", "a", "c"]);
    assert_eq!(module.referenced_outside(&[span(3, 5)]), ["a", "b", "d"]);
    assert_eq!(module.referenced_outside(&[span(3, 5), span(8, 8)]), ["a", "d"]);
    assert_eq!(module.referenced_outside(&[]), ["a", "b", "c", "d"]);
    assert_eq!(module.referencing(|name| name == "b"), [3, 8]);
    assert_eq!(module.referencing(|name| name == "a" || name == "d"), [1, 4, 9]);
}

// Loading reaches the head of every module-level value's and class field's
// type and initializer, the type first; a function local, a parameter, and
// a function are reached by nothing.
#[test]
fn constructed_heads() {
    let module = Module {
        bindings: vec![
            binding("app", Scope::Module, value(Some(path(&["express"])), None), span(1, 1)),
            binding(
                "consumer",
                Scope::Class("Worker".to_owned()),
                BindingKind::Field {
                    type_path: Some(path(&["Kafka", "Consumer"])),
                    root: Some(path(&["Consumer"])),
                    init: Init::Construction,
                    head: None,
                },
                span(5, 5),
            ),
            binding("local", Scope::Function(1), value(Some(path(&["ignored"])), None), span(8, 8)),
            binding(
                "db",
                Scope::Function(1),
                BindingKind::Param {
                    type_path: Some(path(&["Session"])),
                },
                span(7, 7),
            ),
            binding("start", Scope::Module, BindingKind::Function, span(10, 12)),
            binding(
                "again",
                Scope::Module,
                value(Some(path(&["express", "Router"])), None),
                span(13, 13),
            ),
        ],
        ..Module::default()
    };

    assert_eq!(module.constructed(), ["express", "Kafka", "Consumer"]);
}

// A generic stem takes its directory's name unless that directory only roots
// the tree; which stems are generic is the dialect's.
#[test]
fn module_stems() {
    let stem = |path: &str, dialect: &Dialect| {
        Module {
            path: path.to_owned(),
            ..Module::default()
        }
        .stem(dialect)
        .to_owned()
    };

    assert_eq!(stem("src/routes/index.ts", &TYPESCRIPT), "routes");
    assert_eq!(stem("src/index.ts", &TYPESCRIPT), "index", "`src` roots the tree");
    assert_eq!(stem("index.ts", &TYPESCRIPT), "index");
    assert_eq!(stem("src/orders.service.ts", &TYPESCRIPT), "orders");
    assert_eq!(stem("src/orders/views.ts", &TYPESCRIPT), "views", "generic for python alone");
    assert_eq!(stem("shop/orders/views.py", &PYTHON), "orders");
    assert_eq!(stem("shop/orders/__init__.py", &PYTHON), "orders");
    assert_eq!(stem("app/views.py", &PYTHON), "views", "`app` roots the tree");
    assert_eq!(stem("manage.py", &PYTHON), "manage");
}

// Structure is a listed method, a listener of no event or a lifecycle's, a
// mocking call by its dotted spelling, or a lifecycle hook by its tail; each
// list is the dialect's, and an empty one matches nothing.
#[test]
fn callee_structural() {
    let through = |head: &str, links: &[&str], literal: Option<&str>, dialect: &Dialect| {
        call(callee(head, links), vec![arg(None, literal)]).structural(dialect)
    };

    assert!(through("app", &["use"], None, &TYPESCRIPT));
    assert!(through("app", &["listen"], Some("3000"), &TYPESCRIPT));
    assert!(through("emitter", &["on"], None, &TYPESCRIPT), "a listener of no event");
    assert!(through("process", &["on"], Some("SIGTERM"), &TYPESCRIPT));
    assert!(!through("consumer", &["on"], Some("message"), &TYPESCRIPT), "a caller's event");
    assert!(!through("app", &["get"], Some("/orders"), &TYPESCRIPT));
    assert!(!through("use", &[], None, &PYTHON), "the list is the dialect's");

    assert!(through("items", &["map"], None, &PYTHON));
    assert!(through("mock", &["patch"], Some("x"), &PYTHON), "mocking by its spelling whole");
    assert!(through("patch", &[], Some("x"), &PYTHON));
    assert!(!through("app", &["patch"], Some("/orders"), &PYTHON), "the same name is a verb");
    assert!(through("app", &["on_event"], Some("startup"), &PYTHON), "a lifecycle hook");
    assert!(through("atexit", &["register"], None, &PYTHON));
    assert!(through("admin", &["site", "register"], None, &PYTHON), "by its tail");
    assert!(!through("router", &["register"], Some("orders"), &PYTHON));
    assert!(!through("app", &["on"], None, &PYTHON), "no listener is listed");
}

// The method is the last name; the receiver is the head, or the first member
// under the dialect's self name.
#[test]
fn callee_names() {
    let chained = callee("this", &["db", "query"]);
    assert_eq!(chained.method(), "query");
    assert_eq!(chained.dotted(), "this.db.query");
    assert_eq!(chained.receiver(&TYPESCRIPT), "db");
    assert_eq!(chained.receiver(&PYTHON), "this", "`this` is no self name in python");
    assert_eq!(chained.path(), ["this", "db", "query"]);

    assert_eq!(callee("this", &[]).receiver(&TYPESCRIPT), "this");
    assert_eq!(callee("pool", &["query"]).receiver(&TYPESCRIPT), "pool");
    assert_eq!(callee("self", &["session", "add"]).receiver(&PYTHON), "session");
    assert_eq!(callee("run", &[]).method(), "run");
}

// The literal is the first positional argument's; a keyword is found by name;
// a chain registers under its first call with a literal.
#[test]
fn call_literal_keyword() {
    let route = call(
        callee("app", &["route"]),
        vec![arg(None, Some("/orders")), arg(Some("methods"), None), arg(None, Some("second"))],
    );
    assert_eq!(route.literal(), Some("/orders"));
    assert!(route.keyword("methods").is_some_and(|arg| arg.literal.is_none()));
    assert_eq!(route.keyword("name"), None);
    assert_eq!(route.method(), "route");
    assert!(route.discarded());

    let keyed =
        call(callee("Field", &[]), vec![arg(Some("default"), Some("x")), arg(None, Some("y"))]);
    assert_eq!(keyed.literal(), Some("y"), "a keyword argument never leads");
    assert_eq!(keyed.keyword("default").and_then(|arg| arg.literal.as_deref()), Some("x"));

    let mut consumed = call(callee("db", &["query"]), Vec::new());
    consumed.value = Use::Consumed;
    assert!(!consumed.discarded());
    assert_eq!(consumed.literal(), None);

    let mut chain = call(callee("program", &["command", "option", "action"]), Vec::new());
    chain.callee.links[0].call = Some(Invocation {
        literal: Some("import".to_owned()),
    });
    chain.callee.links[1].call = Some(Invocation {
        literal: Some("--file".to_owned()),
    });
    assert_eq!(chain.registered(None), ("command", Some("import".to_owned())));
    assert_eq!(
        route.registered(Some("/orders".to_owned())),
        ("route", Some("/orders".to_owned())),
        "no chained literal: the call's own method and lead"
    );
    assert_eq!(route.registered(None), ("route", None));
}

// A decorator registers unless the dialect lists it as shaping, by name or
// prefix, as a hook, or as a lifecycle's by its tail.
#[test]
fn decorated_registering() {
    let registers =
        |name: &[&str], dialect: &Dialect| decorated(name, None, None).registering(dialect);

    assert!(registers(&["Get"], &TYPESCRIPT));
    assert!(registers(&["Controller"], &TYPESCRIPT));
    assert!(!registers(&["UseGuards"], &TYPESCRIPT), "shapes by name");
    assert!(!registers(&["ApiTags"], &TYPESCRIPT), "shapes by prefix");
    assert!(registers(&["dataclass"], &TYPESCRIPT), "the list is the dialect's");

    assert!(registers(&["app", "route"], &PYTHON));
    assert!(registers(&["click", "command"], &PYTHON));
    assert!(!registers(&["dataclass"], &PYTHON));
    assert!(!registers(&["receiver"], &PYTHON), "a hook");
    assert!(!registers(&["app", "errorhandler"], &PYTHON));
    assert!(!registers(&["admin", "register"], &PYTHON), "a lifecycle's by its tail");
    assert!(!registers(&["app", "on_event"], &PYTHON));
    assert!(registers(&["ApiTags"], &PYTHON), "no prefix is listed");
    assert!(!registers(&[], &PYTHON), "no name registers nothing");

    let decorator = decorated(&["app", "route"], Some("Views"), Some("list"));
    assert_eq!(decorator.keyword("methods"), Some("[\"GET\"]"));
    assert_eq!(decorator.keyword("name"), None);
}

#[test]
fn decorators_of_member() {
    let module = Module {
        decorated: vec![
            decorated(&["Controller"], Some("Orders"), None),
            decorated(&["Get"], Some("Orders"), Some("list")),
            decorated(&["UseGuards"], Some("Orders"), Some("list")),
            decorated(&["Post"], Some("Orders"), Some("create")),
            decorated(&["app", "route"], None, Some("list")),
        ],
        classes: vec![ClassDecl {
            name: "Orders".to_owned(),
            exported: true,
            lines: span(1, 30),
            header: "export class Orders".to_owned(),
            bases: vec!["Base".to_owned()],
            members: Vec::new(),
        }],
        ..Module::default()
    };

    let names = |class: Option<&str>, member: &str| {
        module.decorators_of(class, member).iter().map(|d| d.name.join(".")).collect::<Vec<_>>()
    };
    assert_eq!(names(Some("Orders"), "list"), ["Get", "UseGuards"]);
    assert_eq!(names(Some("Orders"), "create"), ["Post"]);
    assert_eq!(names(None, "list"), ["app.route"], "a function's, not the member's");
    assert_eq!(
        names(Some("Orders"), "Orders"),
        [] as [String; 0],
        "a class decorator is no member's"
    );
    assert_eq!(
        module.class("Orders").map(|class| class.bases.clone()),
        Some(vec!["Base".to_owned()])
    );
    assert_eq!(module.class("Nope"), None);
}

// A hook is a keyword argument under a listed keyword; a property is found
// by its dotted key.
#[test]
fn arg_hooks_properties() {
    assert!(arg(Some("lifespan"), None).is_hook(&PYTHON));
    assert!(arg(Some("callback"), None).is_hook(&PYTHON));
    assert!(!arg(Some("prefix"), Some("/api")).is_hook(&PYTHON));
    assert!(!arg(None, Some("/api")).is_hook(&PYTHON), "a positional argument is no hook");
    assert!(!arg(Some("lifespan"), None).is_hook(&TYPESCRIPT), "no keyword is listed");

    let mut options = arg(None, None);
    options.properties = vec![
        Property {
            key: "dir".to_owned(),
            value: "routes".to_owned(),
            relative: true,
        },
        Property {
            key: "options.prefix".to_owned(),
            value: "/api".to_owned(),
            relative: false,
        },
    ];
    assert_eq!(
        options.property("dir").map(|p| (p.value.as_str(), p.relative)),
        Some(("routes", true))
    );
    assert_eq!(options.property("options.prefix").map(|p| p.value.as_str()), Some("/api"));
    assert_eq!(options.property("prefix"), None, "the key is dotted from the object");
}

// A lifecycle hook matches whole or by a dotted tail, never by a bare suffix;
// a shaping decorator by name or by prefix.
#[test]
fn dialect_hooks() {
    assert!(PYTHON.hooks("on_event"));
    assert!(PYTHON.hooks("app.on_event"));
    assert!(PYTHON.hooks("django.contrib.admin.register"));
    assert!(!PYTHON.hooks("router.register"), "`admin.register` is the tail, not `register`");
    assert!(!PYTHON.hooks("my_on_event"), "a bare suffix is no tail");
    assert!(!PYTHON.hooks(""));
    assert!(!TYPESCRIPT.hooks("app.on_event"), "the list is the dialect's");

    assert!(PYTHON.shapes("dataclass") && !PYTHON.shapes("route"));
    assert!(TYPESCRIPT.shapes("ApiOkResponse") && !TYPESCRIPT.shapes("Get"));
    assert!(TYPESCRIPT.shapes("Api"), "a bare prefix shapes too");
    assert!(!TYPESCRIPT.shapes("OpenApi"), "a prefix, not an infix");
}
