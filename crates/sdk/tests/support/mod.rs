//! A stub recogniser and hand-built modules for the tree and seam suites.

#![allow(dead_code, reason = "each suite reads the part of the support it needs")]

use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::Path;

use emery_sdk::kebab;
use emery_sdk::survey::code::{
    Arg, Binding, BindingKind, Bootstrap, Call, Callee, ClassDecl, Decorated, Derived, Dynamic,
    Export, ExportKind, Import, Imported, Init, Link, Manifest, Member, MemberKind, Module,
    Recogniser, Reference, Runs, Scope, Tree, Use,
};
use emery_sdk::survey::resolve::Target;
use emery_sdk::survey::route::{self, Spelling};
use emery_sdk::survey::tests::Statement;
use emery_sdk::survey::{ClassSyntax, Dialect, Lines};

pub static TYPESCRIPT: Dialect = Dialect {
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
    hook_keywords: &["callback"],
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

pub static PYTHON: Dialect = Dialect {
    self_name: "self",
    generic_stems: &["__init__", "main", "app", "views", "urls"],
    structural: &["map", "partial", "include_router"],
    listeners: &[],
    lifecycle_events: &[],
    mocking: &["patch", "mock.patch"],
    lifecycle: &["atexit.register", "on_event"],
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

/// The shared module and nothing more, as an adapter with no fields of its own wraps it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Plain(pub Module);

impl Deref for Plain {
    type Target = Module;

    fn deref(&self) -> &Module {
        &self.0
    }
}

/// A recogniser scripted by its fields: the manifest, the bootstrap, and the
/// mounts are what a test says they are, and the lookups follow one plain
/// rule each.
#[derive(Debug, Default)]
pub struct Stub {
    pub manifest: Manifest,
    pub bootstrap: Option<(String, Runs)>,
    pub mounts: BTreeMap<String, String>,
}

impl Stub {
    // Whether `name` within `frames` of `module` is a function or class of
    // the tree, declared there or imported from a module that exports one.
    fn callable(tree: &Tree<Self>, module: &Plain, name: &str, frames: &[u32]) -> bool {
        if let Some(binding) = module.binding(name, frames) {
            return matches!(
                binding.kind,
                BindingKind::Function
                    | BindingKind::Class { .. }
                    | BindingKind::Value {
                        init: Init::Function,
                        ..
                    }
            );
        }
        tree.exporter(module, name)
            .and_then(|(target, exported)| target.export(&exported).map(|export| export.kind))
            .is_some_and(ExportKind::callable)
    }
}

impl Recogniser for Stub {
    type Module = Plain;

    fn settle(&self, _module: &mut Plain) {}

    fn manifest(&self) -> Manifest {
        self.manifest.clone()
    }

    fn statements(&self, module: &Plain) -> Vec<Statement> {
        module
            .references
            .iter()
            .map(|reference| Statement {
                text: format!("states {}", reference.name),
                line: reference.line,
            })
            .collect()
    }

    fn bootstrap<'t>(&self, tree: &'t Tree<Self>) -> Option<Bootstrap<'t, Plain>> {
        let (path, runs) = self.bootstrap.as_ref()?;
        Some(Bootstrap {
            module: tree.modules.get(path)?,
            runs: runs.clone(),
        })
    }

    fn handler(&self, tree: &Tree<Self>, module: &Plain, arg: &Arg, frames: &[u32]) -> bool {
        if arg.function {
            return true;
        }
        if arg.called || arg.literal.is_some() {
            return false;
        }
        match arg.root.as_deref() {
            Some([name]) => Self::callable(tree, module, name, frames),
            _ => false,
        }
    }

    fn class_handed(&self, _tree: &Tree<Self>, module: &Plain, arg: &Arg, frames: &[u32]) -> bool {
        match arg.root.as_deref() {
            Some([name]) if !arg.called => module
                .binding(name, frames)
                .is_some_and(|binding| matches!(binding.kind, BindingKind::Class { .. })),
            _ => false,
        }
    }

    fn handed_class<'t>(
        &self, _tree: &'t Tree<Self>, module: &'t Plain, call: &Call,
    ) -> Option<(&'t Plain, String)> {
        call.args.iter().find_map(|arg| match arg.root.as_deref() {
            Some([name]) if !arg.called => module
                .binding(name, &call.frames)
                .filter(|binding| matches!(binding.kind, BindingKind::Class { .. }))
                .map(|_| (module, name.clone())),
            _ => None,
        })
    }

    fn mounts(&self, _tree: &Tree<Self>) -> BTreeMap<String, String> {
        self.mounts.clone()
    }

    fn declares_data(&self, class: &ClassDecl) -> bool {
        class.bases.iter().any(|base| base == "TypedDict")
    }

    // A registration's stem is what its literal spells under the module's
    // mount, its tell the handler handed by name; a handed class carries its
    // public methods. Anything else is named by the survey alone.
    fn derive(
        &self, tree: &Tree<Self>, module: &Plain, lines: Lines, name: &str, stem: &str,
        mounts: &BTreeMap<String, String>,
    ) -> Derived {
        let Some(call) = tree.registration_at(module, lines) else {
            return Derived::named(tree, module, lines, name, stem);
        };
        let led = tree.led(module, call);
        let mount = mounts.get(&module.path).map_or("", String::as_str);
        let derived = led.as_deref().and_then(|literal| {
            if literal.starts_with('/') {
                tree.dialect.route.stem(&route::join(mount, literal))
            } else {
                route::literal_stem(literal)
            }
        });
        let handler = call
            .args
            .iter()
            .filter(|arg| arg.root.is_some() && !arg.is_hook(tree.dialect))
            .find(|arg| self.handler(tree, module, arg, &call.frames))
            .and_then(|arg| arg.root.as_ref()?.last().and_then(|last| kebab(last)));
        let (methods, handed) = match self.handed_class(tree, module, call) {
            Some((declaring, class)) => {
                let methods = declaring
                    .class(&class)
                    .map(|decl| {
                        decl.members
                            .iter()
                            .filter(|member| !member.private)
                            .map(|member| (member.name.clone(), member.lines))
                            .collect()
                    })
                    .unwrap_or_default();
                (methods, Some((declaring, class)))
            }
            None => (Vec::new(), None),
        };
        let mut closure = tree.reaches(module, call.lines, &call.frames, call.class.as_deref());
        if let Some((declaring, class)) = handed {
            for path in tree.reaches(declaring, declaring.span, &[], Some(&class)) {
                if !closure.contains(&path) {
                    closure.push(path);
                }
            }
        }
        let mut detail = vec![format!("registered {}", call.lines)];
        if let Some(receiver) = tree.receiver(module, call) {
            detail.push(format!("through `{}`", receiver.package));
        }
        let discriminator = handler.filter(|tell| Some(tell.as_str()) != derived.as_deref());
        Derived {
            stem: derived,
            discriminator,
            methods,
            lines: call.lines,
            detail,
            closure,
        }
    }
}

pub const fn span(start: u32, end: u32) -> Lines {
    Lines { start, end }
}

pub const fn line(at: u32) -> Lines {
    Lines { start: at, end: at }
}

/// A parsed module of `lines` placeholder lines at `path`.
pub fn module(path: &str, lines: u32) -> Plain {
    let text = "line\n".repeat(lines as usize);
    Plain(Module {
        path: path.to_owned(),
        text,
        span: span(1, lines.max(1)),
        parsed: true,
        ..Module::default()
    })
}

pub fn named_import(local: &str, specifier: &str, target: Target) -> Import {
    Import {
        local: local.to_owned(),
        specifier: specifier.to_owned(),
        imported: Imported::Named(local.to_owned()),
        type_only: false,
        target: Some(target),
    }
}

pub fn import(local: &str, specifier: &str, imported: Imported, target: Target) -> Import {
    Import {
        local: local.to_owned(),
        specifier: specifier.to_owned(),
        imported,
        type_only: false,
        target: Some(target),
    }
}

pub fn from_module(local: &str, path: &str) -> Import {
    named_import(
        local,
        &format!("./{}", path.trim_end_matches(".ts")),
        Target::Module(path.to_owned()),
    )
}

pub fn from_package(local: &str, package: &str) -> Import {
    named_import(local, package, Target::Package(package.to_owned()))
}

pub fn binding(name: &str, scope: Scope, kind: BindingKind, lines: Lines) -> Binding {
    Binding {
        name: name.to_owned(),
        scope,
        kind,
        lines,
    }
}

pub fn function(name: &str, lines: Lines) -> Binding {
    binding(name, Scope::Module, BindingKind::Function, lines)
}

pub fn class(name: &str, bases: &[&str], lines: Lines) -> Binding {
    binding(
        name,
        Scope::Module,
        BindingKind::Class {
            bases: bases.iter().map(|base| (*base).to_owned()).collect(),
        },
        lines,
    )
}

/// A module-level value: `const <name> = <root>(..)`, or a string literal.
pub fn value(
    name: &str, root: Option<&[&str]>, call: Option<usize>, string: Option<&str>, init: Init,
    lines: Lines,
) -> Binding {
    binding(
        name,
        Scope::Module,
        BindingKind::Value {
            root: root.map(path),
            type_path: None,
            call,
            string: string.map(str::to_owned),
            init,
            head: Some(format!("{name} = ..")),
        },
        lines,
    )
}

pub fn path(segments: &[&str]) -> Vec<String> {
    segments.iter().map(|segment| (*segment).to_owned()).collect()
}

pub fn callee(head: &str, links: &[&str]) -> Callee {
    Callee {
        head: head.to_owned(),
        head_call: None,
        links: links
            .iter()
            .map(|name| Link {
                name: (*name).to_owned(),
                call: None,
            })
            .collect(),
    }
}

pub const fn call(callee: Callee, args: Vec<Arg>, value: Use, lines: Lines) -> Call {
    Call {
        callee,
        constructs: false,
        args,
        depth: 0,
        value,
        inner: false,
        frames: Vec::new(),
        function: None,
        class: None,
        lines,
    }
}

pub fn literal(text: &str, lines: Lines) -> Arg {
    Arg {
        keyword: None,
        literal: Some(text.to_owned()),
        root: None,
        called: false,
        inner: None,
        function: false,
        properties: Vec::new(),
        head: format!("\"{text}\""),
        lines,
    }
}

pub fn name(root: &[&str], lines: Lines) -> Arg {
    Arg {
        keyword: None,
        literal: None,
        root: Some(path(root)),
        called: false,
        inner: None,
        function: false,
        properties: Vec::new(),
        head: root.join("."),
        lines,
    }
}

pub fn inline(lines: Lines) -> Arg {
    Arg {
        keyword: None,
        literal: None,
        root: None,
        called: false,
        inner: None,
        function: true,
        properties: Vec::new(),
        head: "() => ..".to_owned(),
        lines,
    }
}

/// An exported class declaration, `header` as written, over `members`.
pub fn class_decl(name: &str, header: &str, lines: Lines, members: Vec<Member>) -> ClassDecl {
    ClassDecl {
        name: name.to_owned(),
        exported: true,
        lines,
        header: header.to_owned(),
        bases: header
            .split_once('(')
            .map(|(_, bases)| bases.trim_end_matches([')', ':']))
            .or_else(|| header.split_once(" extends ").map(|(_, base)| base))
            .map(|bases| bases.split(',').map(|base| base.trim().to_owned()).collect())
            .unwrap_or_default(),
        members,
    }
}

pub fn member(name: &str, kind: MemberKind, signature: &str, lines: Lines) -> Member {
    Member {
        name: name.to_owned(),
        kind,
        private: false,
        lines,
        signature: signature.to_owned(),
    }
}

pub fn export(name: &str, kind: ExportKind, lines: Lines) -> Export {
    Export {
        name: name.to_owned(),
        local: None,
        kind,
        lines,
    }
}

pub fn decorated(
    class: Option<&str>, member: Option<&str>, name: &[&str], literal: Option<&str>, head: Lines,
    lines: Lines,
) -> Decorated {
    Decorated {
        class: class.map(str::to_owned),
        member: member.map(str::to_owned),
        name: path(name),
        literal: literal.map(str::to_owned),
        keywords: Vec::new(),
        head,
        lines,
    }
}

pub fn reference(name: &str, line: u32) -> Reference {
    Reference {
        name: name.to_owned(),
        line,
    }
}

/// A load by a computed name at `lines`, settled to the directory `scope` where it spells one.
pub fn dynamic(lines: Lines, scope: Option<&str>) -> Dynamic {
    Dynamic {
        lines,
        specifier: None,
        scope: scope.map(str::to_owned),
    }
}

pub fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(path, body).expect("write");
}

/// A tree over `modules` at `root`, with no tests and the stub's manifest.
pub fn tree(root: &Path, dialect: &'static Dialect, modules: Vec<Plain>, stub: Stub) -> Tree<Stub> {
    Tree {
        root: root.to_path_buf(),
        dialect,
        modules: modules.into_iter().map(|module| (module.path.clone(), module)).collect(),
        tests: Vec::new(),
        manifest: stub.manifest.clone(),
        recogniser: stub,
    }
}
