//! Holds a parsed tree and answers what a survey asks of it as a whole.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use super::recogniser::{Listing, Manifest, Recogniser};
use super::surface::{Receiver, Surface};
use super::{Arg, BindingKind, Call, Imported, Module};
use crate::kebab;
use crate::survey::tests::{Test, scenarios};
use crate::survey::{Dialect, Lines, push_unique};

/// How many bindings a receiver is traced through before it counts as the tree's own.
pub const TRACE: usize = 4;

/// Every module of a tree parsed, before any import is settled.
///
/// [`Parsed::read`] reads a workspace listing; the adapter builds its
/// [`Recogniser`] from the modules and data files it holds, and
/// [`Parsed::settle`] settles the tree through it.
pub struct Parsed<M> {
    /// The workspace root every path is relative to.
    pub root: PathBuf,
    /// The language the tree is read in.
    pub dialect: &'static Dialect,
    /// Every production module the listing named and the parser read, by
    /// root-relative path.
    pub modules: BTreeMap<String, M>,
    /// Every data file the listing named.
    pub data: Vec<String>,
    tests: Vec<Pending<M>>,
}

enum Pending<M> {
    Module(String, M),
    Feature(Test),
}

impl<M: Deref<Target = Module>> Parsed<M> {
    /// Reads every module and test of `listing` beneath `root`, each parsed by `parse`.
    ///
    /// `parse` is handed each file's root-relative path and its text, and
    /// settles nothing. A file that is not readable text is left out with a
    /// warning, so one broken file never fails a run; a `.feature` file is
    /// read for its scenarios and never parsed.
    pub fn read(
        root: &Path, dialect: &'static Dialect, listing: Listing,
        mut parse: impl FnMut(&str, String) -> M,
    ) -> Self {
        let mut modules = BTreeMap::new();
        for path in listing.modules {
            match std::fs::read_to_string(root.join(&path)) {
                Ok(text) => {
                    modules.insert(path.clone(), parse(&path, text));
                }
                Err(error) => {
                    tracing::warn!(path, %error, "module is not readable text; left out");
                }
            }
        }

        let mut tests = Vec::new();
        for path in listing.tests {
            let text = match std::fs::read_to_string(root.join(&path)) {
                Ok(text) => text,
                Err(error) => {
                    tracing::warn!(path, %error, "test is not readable text; left out");
                    continue;
                }
            };
            if path.ends_with(".feature") {
                tests.push(Pending::Feature(Test {
                    path,
                    imports: Vec::new(),
                    statements: scenarios(&text),
                }));
            } else {
                let module = parse(&path, text);
                tests.push(Pending::Module(path, module));
            }
        }

        Self {
            root: root.to_path_buf(),
            dialect,
            modules,
            data: listing.data,
            tests,
        }
    }

    /// Settles every module and test through `recogniser`, and returns the tree.
    ///
    /// Each module's imports are settled, the manifest read, and each test
    /// read for the modules it imports and the behaviours it states. A
    /// feature file inherits the imports of the test modules under its
    /// directory's parent, and a test stating nothing is dropped.
    pub fn settle<R: Recogniser<Module = M>>(self, recogniser: R) -> Tree<R> {
        let Self {
            root,
            dialect,
            mut modules,
            tests: pending,
            ..
        } = self;
        for module in modules.values_mut() {
            recogniser.settle(module);
        }

        // read each test for what it states and what it imports
        let mut tests: Vec<Test> = Vec::new();
        let mut features: Vec<Test> = Vec::new();
        for test in pending {
            match test {
                Pending::Feature(feature) => features.push(feature),
                Pending::Module(path, mut module) => {
                    recogniser.settle(&mut module);
                    tests.push(Test {
                        path,
                        imports: module.reached(dialect).into_iter().map(str::to_owned).collect(),
                        statements: recogniser.statements(&module),
                    });
                }
            }
        }

        // a feature inherits the imports of the tests under its directory's parent
        for mut feature in features {
            let beside = feature
                .path
                .rsplit_once('/')
                .and_then(|(dir, _)| dir.rsplit_once('/'))
                .map_or_else(String::new, |(parent, _)| format!("{parent}/"));
            for test in tests.iter().filter(|test| test.path.starts_with(&beside)) {
                for import in &test.imports {
                    push_unique(&mut feature.imports, import.clone());
                }
            }
            tests.push(feature);
        }
        tests.retain(|test| !test.statements.is_empty());

        let manifest = recogniser.manifest();
        Tree {
            root,
            dialect,
            modules,
            tests,
            manifest,
            recogniser,
        }
    }
}

/// A parsed tree, every import settled, with the adapter's [`Recogniser`] beside it.
///
/// The lookups here read the tree as a whole: what a module reaches, where
/// a name was bound, what a call hands and through which package. Each
/// takes a module of the tree and reads it through the adapter's
/// recogniser where the language decides.
pub struct Tree<R: Recogniser> {
    /// The workspace root every path is relative to.
    pub root: PathBuf,
    /// The language the tree is read in.
    pub dialect: &'static Dialect,
    /// Every production module, by root-relative path, each import settled.
    pub modules: BTreeMap<String, R::Module>,
    /// The tree's own tests, each stating at least one behaviour.
    pub tests: Vec<Test>,
    /// What the manifest declares; the default where the tree has none.
    pub manifest: Manifest,
    /// The adapter's recogniser.
    pub recogniser: R,
}

impl<R: Recogniser> Tree<R> {
    /// Returns the modules `seeds` reach, breadth-first and once each.
    ///
    /// The seeds the tree holds lead in their order. A module in `stop` is
    /// reached and not followed.
    #[must_use]
    pub fn closure(&self, seeds: &[String], stop: &[String]) -> Vec<String> {
        let mut order: Vec<String> = Vec::new();
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for seed in seeds.iter().filter(|seed| self.modules.contains_key(*seed)) {
            if seen.insert(seed.as_str()) {
                order.push(seed.clone());
            }
        }
        let mut next = 0;
        while next < order.len() {
            let from = self.modules.get(&order[next]).filter(|_| !stop.contains(&order[next]));
            next += 1;
            let Some(module) = from else { continue };
            for path in module.reached(self.dialect) {
                if seen.insert(path) {
                    order.push(path.to_owned());
                }
            }
        }
        order
    }

    /// Returns the modules laid after a seam's `files` when one of them
    /// imports what the resolver could not follow or loads a module by a
    /// computed name: the rest of the tree, in path order.
    ///
    /// Empty when every import of `files` was followed.
    #[must_use]
    pub fn widening(&self, files: &[String]) -> Vec<String> {
        let unfollowed = files
            .iter()
            .filter_map(|path| self.modules.get(path))
            .any(|module| !module.dynamic.is_empty() || !module.unresolved().is_empty());
        if !unfollowed {
            return Vec::new();
        }
        self.modules.keys().filter(|path| !files.contains(*path)).cloned().collect()
    }

    /// Returns the modules no other module imports, each in path order.
    ///
    /// Every module, where each is imported by another.
    #[must_use]
    pub fn roots(&self) -> Vec<String> {
        let imported: BTreeSet<&str> =
            self.modules.values().flat_map(|module| module.reached(self.dialect)).collect();
        let roots: Vec<String> =
            self.modules.keys().filter(|path| !imported.contains(path.as_str())).cloned().collect();
        if roots.is_empty() { self.modules.keys().cloned().collect() } else { roots }
    }

    /// Returns the module exporting what `module` binds as `local`, and the name it exports.
    ///
    /// For an import of a module whole, the module and the local name.
    /// `None` for a star import, an effect import, or a specifier the tree
    /// does not answer.
    #[must_use]
    pub fn exporter(&self, module: &R::Module, local: &str) -> Option<(&R::Module, String)> {
        let import = module.import(local)?;
        let path = import.target.as_ref()?.module()?;
        let name = match &import.imported {
            Imported::Named(name) => name.clone(),
            Imported::Default => "default".to_owned(),
            Imported::Whole => local.to_owned(),
            Imported::Star | Imported::Effect => return None,
        };
        Some((self.modules.get(path)?, name))
    }

    /// Returns the string `arg` spells: a literal, or a name bound to one
    /// in `module` within `frames` or in the module of the tree it is
    /// imported from.
    #[must_use]
    pub fn constant(&self, module: &R::Module, arg: &Arg, frames: &[u32]) -> Option<String> {
        if let Some(literal) = &arg.literal {
            return Some(literal.clone());
        }
        if arg.called {
            return None;
        }
        let [name] = arg.root.as_deref()? else { return None };
        if let Some(binding) = module.binding(name, frames) {
            return match &binding.kind {
                BindingKind::Value { string, .. } => string.clone(),
                _ => None,
            };
        }
        let (target, imported) = self.exporter(module, name)?;
        match &target.exported_binding(&imported)?.kind {
            BindingKind::Value { string, .. } => string.clone(),
            _ => None,
        }
    }

    /// Returns the string literal leading `call`'s positional arguments, or
    /// the constant the first of them is bound to.
    #[must_use]
    pub fn led(&self, module: &R::Module, call: &Call) -> Option<String> {
        call.args
            .iter()
            .find(|arg| arg.keyword.is_none())
            .and_then(|arg| self.constant(module, arg, &call.frames))
    }

    /// Returns the package `call`'s receiver comes from, and the type it is
    /// known as.
    ///
    /// The receiver is traced through the bindings that construct or type
    /// it, [`TRACE`] hops at most. A member a class of the tree inherits
    /// from a package's class is that package's; one the class declares is
    /// the tree's own. `None` for a receiver of the tree's own or the
    /// runtime's.
    #[must_use]
    pub fn receiver(&self, module: &R::Module, call: &Call) -> Option<Receiver> {
        let head = call.callee.head.as_str();
        let link = |index: usize| call.callee.links.get(index).map(|link| link.name.as_str());
        if head == self.dialect.self_name {
            let class = call.class.as_deref()?;
            let [field, _, ..] = call.callee.links.as_slice() else { return None };
            let field = module.field(class, &field.name)?;
            return match &field.kind {
                BindingKind::Field {
                    type_path: Some(path),
                    ..
                } => {
                    let package = module.package(path.first()?)?;
                    Some(Receiver {
                        package: package.to_owned(),
                        type_name: path.last().cloned(),
                    })
                }
                BindingKind::Field { root: Some(root), .. } => {
                    let [head, rest @ ..] = root.as_slice() else { return None };
                    let member = rest.first().map(String::as_str).or_else(|| link(1));
                    self.trace(module, head, &[], member, TRACE)
                }
                _ => None,
            };
        }
        self.trace(module, head, &call.frames, link(0), TRACE)
    }

    /// Returns the package a decorator's head comes from, traced as a
    /// call's receiver is.
    ///
    /// `app` in `@app.get(..)` is a binding of the tree (`app = FastAPI()`)
    /// whose constructor the package provides.
    #[must_use]
    pub fn receiver_of(&self, module: &R::Module, name: &str) -> Option<Receiver> {
        self.trace(module, name, &[], None, TRACE)
    }

    // Follows `name` through the bindings that initialise it, `budget` hops
    // at most. A capitalised package head is the type itself. A function a
    // package's decorator made into something (a `click` group, a `typer`
    // app) is that package's. `member` tells a member a tree class declares,
    // which is its own, from one it inherits from a package's base.
    fn trace(
        &self, module: &R::Module, name: &str, frames: &[u32], member: Option<&str>, budget: usize,
    ) -> Option<Receiver> {
        if budget == 0 {
            return None;
        }
        if let Some(binding) = module.binding(name, frames) {
            return match &binding.kind {
                BindingKind::Value {
                    type_path: Some(path),
                    ..
                }
                | BindingKind::Param {
                    type_path: Some(path),
                } => {
                    let package = module.package(path.first()?)?;
                    Some(Receiver {
                        package: package.to_owned(),
                        type_name: path.last().cloned(),
                    })
                }
                BindingKind::Value {
                    root: Some(root),
                    call,
                    ..
                } => {
                    let [head, rest @ ..] = root.as_slice() else { return None };
                    let member = rest.first().map(String::as_str).or(member);
                    let traced = self.trace(module, head, frames, member, budget - 1)?;
                    let constructed = call
                        .and_then(|index| module.calls.get(index))
                        .filter(|init| init.constructs)
                        .map(|init| init.method().to_owned());
                    Some(Receiver {
                        package: traced.package,
                        type_name: constructed.or(traced.type_name),
                    })
                }
                BindingKind::Function => {
                    module.decorators_of(None, &binding.name).iter().find_map(|decorator| {
                        let head = decorator.name.first()?;
                        let traced = self.trace(module, head, &[], None, budget - 1)?;
                        let made = decorator
                            .name
                            .last()
                            .filter(|last| last.starts_with(char::is_uppercase))
                            .cloned();
                        Some(Receiver {
                            package: traced.package,
                            type_name: traced.type_name.or(made),
                        })
                    })
                }
                BindingKind::Class { bases } => {
                    let declared = module.classes.iter().any(|class| {
                        class.name == binding.name
                            && class.members.iter().any(|m| Some(m.name.as_str()) == member)
                    });
                    if declared {
                        return None;
                    }
                    bases.iter().find_map(|base| {
                        let traced = self.trace(module, base, &[], None, budget - 1)?;
                        Some(Receiver {
                            type_name: traced.type_name.or_else(|| Some(base.clone())),
                            ..traced
                        })
                    })
                }
                _ => None,
            };
        }
        if let Some(package) = module.package(name) {
            let type_name = name.starts_with(char::is_uppercase).then(|| name.to_owned());
            return Some(Receiver {
                package: package.to_owned(),
                type_name,
            });
        }
        let (target, imported) = self.exporter(module, name)?;
        let binding = target.exported_binding(&imported)?;
        self.trace(target, &binding.name, &[], member, budget - 1)
    }

    /// Returns the modules the code at `lines` of `module` reaches, the module first.
    ///
    /// Through the imports referenced within `lines`, the types of the
    /// bindings referenced there within `frames`, and, for `class`, the
    /// span of its declaration and the types and initializers of its
    /// fields.
    #[must_use]
    pub fn reaches(
        &self, module: &R::Module, lines: Lines, frames: &[u32], class: Option<&str>,
    ) -> Vec<String> {
        let mut seeds = vec![module.path.clone()];
        let span = class.and_then(|class| module.class(class)).map_or(lines, |c| c.lines);
        for name in module.referenced(span) {
            seed(&mut seeds, module, name);
            if let Some(binding) = module.binding(name, frames)
                && let BindingKind::Param {
                    type_path: Some(path),
                }
                | BindingKind::Value {
                    type_path: Some(path),
                    ..
                } = &binding.kind
                && let Some(head) = path.first()
            {
                seed(&mut seeds, module, head);
            }
        }
        if let Some(class) = class {
            let fields = module
                .bindings
                .iter()
                .filter(|b| matches!(&b.scope, super::Scope::Class(c) if c == class));
            for binding in fields {
                if let BindingKind::Field { type_path, root, .. } = &binding.kind {
                    for head in type_path.iter().chain(root).filter_map(|path| path.first()) {
                        seed(&mut seeds, module, head);
                    }
                }
            }
        }
        self.closure(&seeds, &[])
    }

    /// Returns whether `call` hands a handler to what it calls.
    ///
    /// A handler among the arguments, not under a hook keyword, and not to
    /// a structural call. A class handed straight to a package's own
    /// function or constructor is queried or typed by it, not run; one
    /// handed to a receiver the code constructs is run.
    #[must_use]
    pub fn hands(&self, module: &R::Module, call: &Call) -> bool {
        if call.structural(self.dialect) {
            return false;
        }
        let through_package = module.package(&call.callee.head).is_some();
        call.args.iter().any(|arg| {
            !arg.is_hook(self.dialect)
                && self.recogniser.handler(self, module, arg, &call.frames)
                && !(through_package
                    && self.recogniser.class_handed(self, module, arg, &call.frames))
        })
    }

    /// Returns the package `call` hands a handler to, for a call outside any handler.
    ///
    /// Where a framework, a queue, a scheduler, or a CLI is told what to run.
    #[must_use]
    pub fn handed(&self, module: &R::Module, call: &Call) -> Option<Receiver> {
        (call.depth == 0 && self.hands(module, call)).then(|| self.receiver(module, call))?
    }

    /// Returns whether `call` registers the handler it hands.
    ///
    /// At any depth and whatever the receiver: a call that hands a handler
    /// and is discarded, constructs, or is led by a literal. A wrapper
    /// handed a function for its value defines and registers nothing.
    #[must_use]
    pub fn registers(&self, module: &R::Module, call: &Call) -> bool {
        self.hands(module, call)
            && (call.discarded() || call.constructs || self.led(module, call).is_some())
    }

    /// Returns the first registration starting within `lines` of `module`.
    #[must_use]
    pub fn registration_at<'m>(&self, module: &'m R::Module, lines: Lines) -> Option<&'m Call> {
        module
            .calls
            .iter()
            .filter(|call| self.registers(module, call))
            .find(|call| lines.holds(call.lines.start))
    }

    /// Returns the first registration enclosing the first line of `lines`,
    /// for an anchor within a handler.
    #[must_use]
    pub fn registration_enclosing<'m>(
        &self, module: &'m R::Module, lines: Lines,
    ) -> Option<&'m Call> {
        module
            .calls
            .iter()
            .filter(|call| self.registers(module, call))
            .find(|call| call.lines.holds(lines.start))
    }

    // The classes handed to a registration, by declaring module and name.
    pub(crate) fn handed_classes(&self) -> BTreeSet<(String, String)> {
        self.modules
            .values()
            .flat_map(|module| module.calls.iter().map(move |call| (module, call)))
            .filter(|(module, call)| {
                self.handed(module, call).is_some() && self.registers(module, call)
            })
            .filter_map(|(module, call)| self.recogniser.handed_class(self, module, call))
            .map(|(target, class)| (target.path.clone(), class))
            .collect()
    }

    /// Gives every surface its ids, decided over them all at once.
    ///
    /// - A stem's one surface is its stem.
    /// - Surfaces sharing a stem are `<stem>.<tell>`, the tell its
    ///   discriminator, else its name, else its entry's module stem.
    /// - Two still alike take the nearest segment of their entries' paths
    ///   that spells neither the stem nor the tell.
    /// - A class carries an id per public method beside its own.
    pub fn identify(&self, surfaces: &mut [Surface]) {
        let module_of = |surface: &Surface| self.modules.get(&surface.entry);
        let module_stem = |module: &R::Module| {
            kebab(module.stem(self.dialect)).unwrap_or_else(|| "module".to_owned())
        };
        let mut owned: Vec<String> = surfaces
            .iter()
            .map(|surface| {
                if surfaces.iter().filter(|other| other.stem == surface.stem).count() == 1 {
                    return surface.stem.clone();
                }
                let tell = surface
                    .discriminator
                    .clone()
                    .or_else(|| kebab(&surface.name).filter(|tell| *tell != surface.stem))
                    .or_else(|| module_of(surface).map(module_stem))
                    .unwrap_or_else(|| "module".to_owned());
                format!("{}.{tell}", surface.stem)
            })
            .collect();
        let alike: Vec<bool> = owned
            .iter()
            .map(|own| owned.iter().filter(|other| *other == own).count() > 1)
            .collect();
        for (index, own) in owned.iter_mut().enumerate() {
            if alike[index]
                && let Some(module) = module_of(&surfaces[index])
                && let Some(segment) = self.tells_apart(module, &surfaces[index].stem, own)
            {
                own.push('.');
                own.push_str(&segment);
            }
        }
        for (surface, own) in surfaces.iter_mut().zip(owned) {
            let mut ids = vec![own.clone()];
            ids.extend(surface.methods.iter().map(|method| format!("{own}.{method}")));
            surface.ids = ids;
        }
    }

    // `local/files.py` and `s3/files.py` under `files` are told apart by
    // `local` and `s3`.
    fn tells_apart(&self, module: &R::Module, stem: &str, own: &str) -> Option<String> {
        let dir = module.path.rsplit_once('/').map_or("", |(dir, _)| dir);
        std::iter::once(module.stem(self.dialect))
            .chain(dir.rsplit('/'))
            .filter_map(kebab)
            .find(|segment| segment != stem && !own.ends_with(&format!(".{segment}")))
    }
}

pub fn seed(seeds: &mut Vec<String>, module: &Module, local: &str) {
    if let Some(path) = module.imported(local) {
        push_unique(seeds, path.to_owned());
    }
}
