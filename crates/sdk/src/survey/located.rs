//! Reads a tree once for the facts the survey turn lays, and holds the
//! answer to them.
//!
//! The parser reads; the model decides what is a surface. The facts laid
//! before it are the manifest, the bootstrap, every call that hands a
//! function to something a package provides, every function, method, and
//! class under a package's decorator, what the entry modules export, and
//! the packages imported. Code holds the answer to the tree and derives the
//! rest from the accepted anchors, never from the model's names, so two
//! runs that accept the same anchors cut the same seams. The bootstrap's
//! `start` is the code's own surface, never the model's.

use std::collections::{BTreeMap, BTreeSet};

use emery_adapter::source::Anchor;

use super::code::{
    Bootstrap, ClassDecl, Decorated, Export, ExportKind, Receiver, Recogniser, Surface, Tree,
};
use super::resolve::Target;
use super::{Inventory, Lines, push_unique, skeleton, unique};
use crate::kebab;

// Read once before the model is asked and held through every correction
// round.
pub(super) struct Located<'t, R: Recogniser> {
    pub(super) tree: &'t Tree<R>,
    pub(super) bootstrap: Option<Bootstrap<'t, R::Module>>,
    entries: Vec<String>,
    roots: Vec<String>,
    handed: BTreeSet<(String, String)>,
    locating: BTreeSet<&'t str>,
    mounts: BTreeMap<String, String>,
}

impl<'t, R: Recogniser> Located<'t, R> {
    pub(super) fn new(tree: &'t Tree<R>) -> Self {
        let handed = tree.handed_classes();
        let locating = tree
            .modules
            .values()
            .filter(|module| Self::locates(tree, &handed, module))
            .map(|module| module.path.as_str())
            .collect();
        Self {
            tree,
            bootstrap: tree.recogniser.bootstrap(tree),
            entries: tree.manifest.entries.clone(),
            roots: tree.roots(),
            handed,
            locating,
            mounts: tree.recogniser.mounts(tree),
        }
    }

    // Whether the facts place a registration or a registering decorator in
    // `module`, so the answer must reach it or list it `unreached`. A method
    // of a class a registration hands is the registration's to locate.
    fn locates(tree: &Tree<R>, handed: &BTreeSet<(String, String)>, module: &R::Module) -> bool {
        module
            .calls
            .iter()
            .any(|call| tree.handed(module, call).is_some() && tree.registers(module, call))
            || module
                .decorated
                .iter()
                .any(|decorated| Self::listed(tree, handed, module, decorated).is_some())
    }

    // The package a decorator the facts list comes from: one that registers
    // a function, a method, or a class through a package, and is not a
    // handed class's.
    fn listed(
        tree: &Tree<R>, handed: &BTreeSet<(String, String)>, module: &R::Module,
        decorated: &Decorated,
    ) -> Option<Receiver> {
        if (decorated.class.is_none() && decorated.member.is_none())
            || !decorated.registering(tree.dialect)
            || Self::of_handed(handed, module, decorated)
        {
            return None;
        }
        tree.receiver_of(module, decorated.name.first()?)
    }

    // A handed class's decorated method is an id of the registration, not a
    // surface.
    fn of_handed(
        handed: &BTreeSet<(String, String)>, module: &R::Module, decorated: &Decorated,
    ) -> bool {
        match (&decorated.class, &decorated.member) {
            (Some(class), Some(_)) => handed.contains(&(module.path.clone(), class.clone())),
            _ => false,
        }
    }

    pub(super) fn facts(&self) -> String {
        let tree = self.tree;
        let mut sections: Vec<String> = Vec::new();

        // say what the manifest declares
        let manifest = &tree.manifest;
        sections.push(if manifest.says.is_empty() {
            let named: Vec<String> =
                tree.dialect.manifests.iter().map(|name| format!("`{name}`")).collect();
            format!("The tree has no {} the parser could read.", named.join(" or "))
        } else {
            format!("The manifest {}.", manifest.says.join("; "))
        });

        // say which entry runs
        sections.push(self.bootstrap.as_ref().map_or_else(
            || {
                "No bootstrap runs at load: no module the manifest names, and no conventional \
                 entry the tree holds, runs anything when loaded. The surfaces are what the tree \
                 exposes without one — what its entry modules export for a caller, or what a \
                 convention of the framework it uses makes reachable: a module a setting, a file \
                 path, or a directory names; a handler an export names."
                    .to_owned()
            },
            |bootstrap| {
                format!(
                    "The bootstrap — the entry that runs something — is `{}`: {}. It is the \
                     caller's `start` surface: the caller names it itself, so do not list it, \
                     and lead no surface with the stem `start`. What it mounts, registers, \
                     schedules, or subscribes — and what any module it reaches registers — are \
                     the surfaces to name, each anchored where it is registered or declared.",
                    bootstrap.module.path,
                    bootstrap.runs.said()
                )
            },
        ));

        // list the registrations, decorations, exports, and what could not be followed
        sections.extend(self.registrations());
        sections.extend(self.decorated());
        sections.extend(self.exports());
        sections.extend(skeleton::packages(tree.modules.values().map(|module| &**module)));
        sections.extend(skeleton::unfollowed(tree.modules.values().map(|module| &**module), &[]));

        sections.join("\n\n")
    }

    fn registrations(&self) -> Option<String> {
        let tree = self.tree;
        let mut lines: Vec<String> = Vec::new();
        for module in tree.modules.values() {
            for call in &module.calls {
                let Some(receiver) = tree.handed(module, call) else { continue };
                let (_, literal) = call.registered(tree.led(module, call));
                let led =
                    literal.map(|literal| format!(" led by `\"{literal}\"`")).unwrap_or_default();
                let spelled = match tree.dialect.constructs {
                    Some(keyword) if call.constructs => format!("{keyword} {}", call.method()),
                    _ if call.constructs || call.callee.links.is_empty() => {
                        format!("{}(..)", call.method())
                    }
                    _ => format!("{}.{}", call.callee.receiver(tree.dialect), call.method()),
                };
                let typed = receiver
                    .type_name
                    .as_deref()
                    .map(|name| format!(" as `{name}`"))
                    .unwrap_or_default();
                lines.push(format!(
                    "- `{}#{}` — `{spelled}`{led} handed a function, through `{}`{typed}",
                    module.path,
                    call.lines.anchor(),
                    receiver.package
                ));
            }
        }
        if lines.is_empty() {
            return None;
        }
        Some(format!(
            "Calls that hand a function or a class to something a package provides, each at its \
             lines — where the code tells a framework, a queue, a scheduler, or a CLI what to \
             run. A surface is usually registered by one of these; a hook on a surface already \
             registered (an error handler, a signal receiver, a startup event) is not a surface \
             of its own:\n\n{}",
            lines.join("\n")
        ))
    }

    fn decorated(&self) -> Option<String> {
        let tree = self.tree;
        let mut lines: Vec<String> = Vec::new();
        for module in tree.modules.values() {
            for decorated in &module.decorated {
                let Some(receiver) = Self::listed(tree, &self.handed, module, decorated) else {
                    continue;
                };
                let decorator = decorated.name.join(".");
                let argument =
                    decorated.literal.as_ref().map(|l| format!("(\"{l}\")")).unwrap_or_default();
                let on = match (&decorated.class, &decorated.member) {
                    (Some(class), Some(member)) => format!("`{class}.{member}`"),
                    (None, Some(member)) => format!("`{member}`"),
                    (Some(class), None) => format!("class `{class}`"),
                    (None, None) => continue,
                };
                lines.push(format!(
                    "- `{}#{}` — `@{decorator}{argument}` on {on}, through `{}`",
                    module.path,
                    decorated.lines.anchor(),
                    receiver.package
                ));
            }
        }
        if lines.is_empty() {
            return None;
        }
        Some(format!(
            "Decorators a package provides, each at its lines — where a framework is told what a \
             function, a method, or a class answers: a route, a command, a task, a consumer. A \
             decorator that only shapes what it decorates — a dataclass, a property, a cache, a \
             guard — is not listed:\n\n{}",
            lines.join("\n")
        ))
    }

    // A re-export is followed one hop, to the module that declares it.
    fn exports(&self) -> Option<String> {
        let tree = self.tree;
        let entries = unique(self.entries.iter().chain(&self.roots));
        let declared = |module: &R::Module, exports: &mut dyn Iterator<Item = &Export>| {
            declared(module, exports, |class| tree.recogniser.declares_data(class))
        };
        let mut lines: Vec<String> = Vec::new();
        for module in entries.iter().filter_map(|path| tree.modules.get(*path)) {
            let exported = declared(module, &mut module.exports.iter());
            if !exported.is_empty() {
                lines.push(format!("- `{}` exports {}", module.path, exported.join(", ")));
            }
            for reexport in module.reexports.iter().filter(|reexport| !reexport.type_only) {
                let Some(path) = reexport.target.as_ref().and_then(Target::module) else {
                    continue;
                };
                let Some(target) = tree.modules.get(path) else { continue };
                let exported = reexport.names.as_ref().map_or_else(
                    || declared(target, &mut target.exports.iter()),
                    |names| {
                        declared(
                            target,
                            &mut names.iter().filter_map(|(imported, _)| {
                                target.exports.iter().find(|export| {
                                    export.name == *imported
                                        || export.local.as_deref() == Some(imported)
                                })
                            }),
                        )
                    },
                );
                if !exported.is_empty() {
                    lines.push(format!(
                        "- `{}` re-exports from `{}`, where each is declared: {}",
                        module.path,
                        target.path,
                        exported.join(", ")
                    ));
                }
            }
        }
        if lines.is_empty() {
            return None;
        }
        Some(format!(
            "What the entry modules — the ones the manifest names, and the ones no other module \
             imports — export, each with its kind and lines, and what a barrel among them \
             re-exports at the module that declares it. In a tree with no bootstrap, or under a \
             framework that routes by file, these are where a caller enters:\n\n{}",
            lines.join("\n")
        ))
    }

    // Ordered so a tree past the budget lays what locates its surfaces before
    // what does not.
    pub(super) fn laid(&self) -> Vec<String> {
        let tree = self.tree;
        let mut files: Vec<String> = Vec::new();
        for manifest in tree.dialect.manifests {
            if tree.root.join(manifest).is_file() {
                files.push((*manifest).to_owned());
            }
        }
        if let Some(bootstrap) = &self.bootstrap {
            push_unique(&mut files, bootstrap.module.path.clone());
        }
        for path in tree
            .modules
            .keys()
            .filter(|path| self.locating.contains(path.as_str()))
            .chain(&self.entries)
            .chain(&self.roots)
            .chain(tree.modules.keys())
        {
            push_unique(&mut files, path.clone());
        }
        files
    }

    // A module the facts say nothing of is not asked after: a framework may
    // load it by a setting's name, and a model made to account for every such
    // module invents surfaces to cover them.
    pub(super) fn check(&self, answer: &Inventory) -> Vec<String> {
        let tree = self.tree;
        let mut findings: Vec<String> = Vec::new();
        if self.bootstrap.is_some() {
            for named in answer.surfaces.iter().filter(|named| named.stem == "start") {
                findings.push(format!(
                    "- surface `{}`: `start` is the bootstrap's stem, which the caller names \
                     itself; give the surface the stem of what a caller does through it",
                    named.name
                ));
            }
        }

        // modules the facts locate a surface in that no named surface reaches;
        // `start` covers its own module alone, since its closure stops only at
        // the entries the answer names and would cover every one it leaves out
        let surfaces = self.named(answer);
        let mut covered: BTreeSet<&str> = surfaces
            .iter()
            .flat_map(|surface| surface.closure.iter().map(String::as_str))
            .collect();
        if let Some(bootstrap) = &self.bootstrap {
            covered.insert(bootstrap.module.path.as_str());
        }
        let unreached: BTreeSet<&str> = answer.unreached.iter().map(String::as_str).collect();
        let missing: Vec<String> = tree
            .modules
            .keys()
            .filter(|path| {
                !covered.contains(path.as_str())
                    && !unreached.contains(path.as_str())
                    && (self.entries.contains(path) || self.locating.contains(path.as_str()))
            })
            .map(|path| format!("`{path}`"))
            .collect();
        if !missing.is_empty() {
            let (these, are) = if missing.len() == 1 {
                ("one module the facts list a registration or declaration in is", "it")
            } else {
                ("these modules the facts list a registration or declaration in are", "each")
            };
            findings.push(format!(
                "- {these} reached by no surface you named and not listed under `unreached`: \
                 {}; where a caller outside the process reaches what is registered there — a \
                 route, a command, a task — name that surface, anchored at the registration or \
                 the decorated declaration, so the caller follows its imports to the module; \
                 otherwise list {are} under `unreached` — a plugin, a hook, a signal receiver, a \
                 helper is no surface, and none is invented to cover a module",
                missing.join(", ")
            ));
        }
        findings
    }

    // `start` sets the bootstrap's own behaviour apart from the surfaces it
    // mounts; where the answer names none, there is nothing to set it apart
    // from, and the tree is cut as one with no surface.
    pub(super) fn build(&self, answer: &Inventory) -> Vec<Surface> {
        let tree = self.tree;
        let mut surfaces = self.named(answer);
        if surfaces.is_empty() {
            return surfaces;
        }
        if let Some(bootstrap) = &self.bootstrap {
            let start = Surface::start(tree, bootstrap, &surfaces);
            surfaces.insert(0, start);
        }
        tree.identify(&mut surfaces);
        surfaces
    }

    // The surfaces the answer names, each derived at its anchor and none
    // identified yet. An anchor the tree does not hold is skipped: the SDK's
    // gate has refused it already.
    fn named(&self, answer: &Inventory) -> Vec<Surface> {
        let tree = self.tree;
        answer
            .surfaces
            .iter()
            .filter_map(|named| {
                let Anchor { path, lines } = Anchor::parse(&named.anchor).ok()?;
                let module = tree.modules.get(path)?;
                let lines = lines.map_or(module.span, Lines::from);
                let derived = tree.recogniser.derive(
                    tree,
                    module,
                    lines,
                    &named.name,
                    &named.stem,
                    &self.mounts,
                );
                Some(Surface {
                    name: named.name.clone(),
                    entry: module.path.clone(),
                    stem: derived.stem.unwrap_or_else(|| named.stem.clone()),
                    lines: derived.lines,
                    detail: derived.detail,
                    closure: derived.closure,
                    discriminator: derived.discriminator,
                    methods: derived.methods.iter().filter_map(|(name, _)| kebab(name)).collect(),
                    ids: Vec::new(),
                })
            })
            .collect()
    }
}

// A type, or a class declaring data alone, is the caller's to copy, not to
// call.
fn declared<'e>(
    module: &super::code::Module, exports: impl Iterator<Item = &'e Export>,
    declares_data: impl Fn(&ClassDecl) -> bool,
) -> Vec<String> {
    exports
        .filter_map(|export| {
            let local = export.local.as_deref().unwrap_or(&export.name);
            let kind = match export.kind {
                ExportKind::Function => "function",
                ExportKind::Class if module.class(local).is_some_and(&declares_data) => {
                    return None;
                }
                ExportKind::Class => "class",
                ExportKind::Value => "value",
                ExportKind::Type | ExportKind::Unknown => return None,
            };
            Some(format!("`{}` ({kind}) {}", export.name, export.lines))
        })
        .collect()
}
