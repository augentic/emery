//! A seam's anchors are where its survey found a behaviour can start or its
//! result is decided; every scenario here reads them off `survey::seams`
//! over a hand-built tree under a scripted model, with no parser.

mod support;

use emery_sdk::survey::Lines;
use emery_sdk::survey::code::{
    Arg, Binding, BindingKind, Call, Callee, ExportKind, Init, MemberKind, Scope, Use,
};
use emery_sdk::survey::resolve::Target;
use emery_sdk::{Context, Doc, SourceInput};
use omnia_test::guest::Scripted;
use support::{
    PYTHON, Stub, binding, call, callee, class, class_decl, export, from_package, function, line,
    literal, member, module, name, named_import, path, span, tree, value, write,
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

// One surface, named by the survey at the module's first line.
const API: &str = r#"{"surfaces":[{"name":"api","anchor":"app/api.py#L1","stem":"api"}]}"#;

// A local of the method's frame, bound to the call at `index`.
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
        },
        at,
    )
}

fn param(name: &str, at: Lines) -> Binding {
    binding(name, Scope::Function(1), BindingKind::Param { type_path: None }, at)
}

// A call within the method `run` of `Handler`.
fn within(callee: Callee, args: Vec<Arg>, value: Use, at: Lines) -> Call {
    Call {
        frames: vec![1],
        function: Some("run".to_owned()),
        class: Some("Handler".to_owned()),
        ..call(callee, args, value, at)
    }
}

// A handler module whose one method binds what each call answers — a call
// into an imported function, a tree class constructed, a call on what it
// made, on a member of `self`, on an untyped parameter, through two packages,
// and on a cursor a `with` bound — beside the services module it imports.
fn handling() -> Vec<Plain> {
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
    vec![api, services]
}

// A call into the tree whose result is bound to a name is a step the method
// takes, as one for its effect alone or awaited is: into an imported
// function, on a local a tree class constructed, on a member of `self`. The
// construction that made the local only wires, a call on an untyped
// parameter leads nowhere the tree knows, a call passed on is a value in
// hand, and a module-level binding takes no step. A call through a package
// is anchored and listed whatever becomes of its value — the one on a cursor
// a `with` bound among them — and a bare call bound is kept off the list.
#[tokio::test]
async fn bound_steps() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let modules = handling();
    for module in &modules {
        write(root, &module.path, &module.text);
    }
    let tree = tree(root, &PYTHON, modules, Stub::default());
    let input = SourceInput::workspace("svc", root.to_str().expect("a UTF-8 scratch root"));
    let model = Scripted::answering([API]);
    let ctx = Context {
        adapter_id: "source:probe",
        input: &input,
        model: &model,
    };

    let survey = emery_sdk::survey::seams(&ctx, PROSE, &tree).await.expect("surveyed");

    assert_eq!(survey.seams.len(), 1);
    let seam = &survey.seams[0];
    let at = |line: u32| seam.anchors.contains(&format!("app/api.py#L{line}"));
    assert!(at(6), "a call into an imported function, bound: {:?}", seam.anchors);
    assert!(at(8), "a call on a local a tree class constructed, bound: {:?}", seam.anchors);
    assert!(at(9), "a call on a member of `self`, bound: {:?}", seam.anchors);
    assert!(!at(7), "the construction that made the local only wires: {:?}", seam.anchors);
    assert!(!at(10), "a call on an untyped parameter leads nowhere: {:?}", seam.anchors);
    assert!(!at(11), "a call passed on is a value in hand: {:?}", seam.anchors);
    assert!(!at(16), "a module-level binding takes no step: {:?}", seam.anchors);
    for through in [12, 13, 14, 15] {
        assert!(at(through), "every call through a package is anchored: {:?}", seam.anchors);
    }
    for listed in [
        "- `requests:get` in `app/api.py` at L12",
        "- `psycopg2:connect` in `app/api.py` at L13",
        "- `psycopg2:cur.execute` in `app/api.py` at L15",
    ] {
        assert!(seam.text.contains(listed), "{listed}\n---\n{}", seam.text);
    }
    assert!(
        !seam.text.contains("`psycopg2:conn.cursor`"),
        "a bare call bound is kept off the list: {}",
        seam.text
    );
    model.assert_exhausted();
}
