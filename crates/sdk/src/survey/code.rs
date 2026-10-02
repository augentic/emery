//! Holds what an adapter's parser reads of one module, for every rule beneath to read.
//!
//! A [`Module`] is plain data. The adapter's walker fills it, its resolver
//! settles where each import leads, and the lookups here answer what a
//! survey asks of it: which binding a name resolves to, what a module
//! reaches, which calls are structure and which decorators register. A
//! module the parser cannot read is still a `Module`, read as far as the
//! parser got with `parsed` false, so one broken file never fails a run.
//!
//! What a lookup reads of the language — the methods that are structure,
//! the decorators that shape, the stems that name a role — comes from the
//! adapter's [`Dialect`], passed to each lookup that reads one.
//!
//! A [`Tree`] holds every module of a workspace, settled through the
//! adapter's [`Recogniser`], and answers what a survey asks of the tree as
//! a whole; a [`Surface`] is what the code says of each place the survey
//! named.

mod recogniser;
mod surface;
mod tree;

pub use self::recogniser::{Bootstrap, Listing, Manifest, Recogniser, Runs};
pub use self::surface::{Derived, Receiver, Surface};
pub use self::tree::{Parsed, TRACE, Tree};
use super::resolve::Target;
use super::{Dialect, Lines, unique};

// Directories that root a tree rather than name a part of it; a module of a
// generic stem beneath one keeps the stem.
const ROOTS: &[&str] = &["", "src", "lib", "app"];

/// One module of the tree, as the adapter's parser read it.
///
/// Every field is the walker's to fill; the lookups read them. `text` and
/// `span` are set once the module is read, and each import's `target` once
/// the tree is.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Module {
    /// The root-relative path.
    pub path: String,
    /// The source text whole.
    pub text: String,
    /// The first line to the last, one line at the least.
    pub span: Lines,
    /// Whether the parser read the module to its end.
    pub parsed: bool,
    /// Every import, in order.
    pub imports: Vec<Import>,
    /// Every re-export, grouped by specifier.
    pub reexports: Vec<Reexport>,
    /// What the module exports, by name.
    pub exports: Vec<Export>,
    /// Every binding, at module, function, and class scope.
    pub bindings: Vec<Binding>,
    /// Every call, in source order.
    pub calls: Vec<Call>,
    /// Every decorated class and member.
    pub decorated: Vec<Decorated>,
    /// Every type declaration.
    pub types: Vec<TypeDecl>,
    /// Every class declaration.
    pub classes: Vec<ClassDecl>,
    /// Every read of the environment.
    pub env: Vec<EnvRead>,
    /// Every point where the code decides.
    pub decisions: Vec<Decision>,
    /// Every `return` or `yield` of a value.
    pub returns: Vec<Lines>,
    /// Every load of a module by a computed name, which no resolver can follow to a module.
    pub dynamic: Vec<Dynamic>,
    /// Every read of a name, at its line.
    pub references: Vec<Reference>,
}

impl Module {
    /// Returns the root-relative directory the module sits in; empty at the root.
    #[must_use]
    pub fn directory(&self) -> &str {
        self.path.rsplit_once('/').map_or("", |(dir, _)| dir)
    }

    /// Returns the stem the module's path spells, under `dialect`'s generic stems.
    ///
    /// The file's name before its first dot, unless the dialect lists it as
    /// naming a role (`index`, `views`) and the directory names a part of
    /// the tree rather than rooting it, in which case the directory's name.
    #[must_use]
    pub fn stem(&self, dialect: &Dialect) -> &str {
        let (dir, file) = self.path.rsplit_once('/').unwrap_or(("", &self.path));
        let stem = file.split_once('.').map_or(file, |(stem, _)| stem);
        let parent = dir.rsplit_once('/').map_or(dir, |(_, last)| last);
        if dialect.generic_stems.contains(&stem) && !ROOTS.contains(&parent) {
            parent
        } else {
            stem
        }
    }

    /// Returns the import bound as `local`; an import binding nothing is never found.
    #[must_use]
    pub fn import(&self, local: &str) -> Option<&Import> {
        self.imports.iter().find(|import| !import.local.is_empty() && import.local == local)
    }

    /// Returns the package the import bound as `local` leads to.
    #[must_use]
    pub fn package(&self, local: &str) -> Option<&str> {
        self.import(local)?.target.as_ref()?.package()
    }

    /// Returns the module of the tree the import bound as `local` leads to.
    #[must_use]
    pub fn imported(&self, local: &str) -> Option<&str> {
        self.import(local)?.target.as_ref()?.module()
    }

    /// Returns the binding `name` resolves to within `frames`, else at module scope.
    ///
    /// `frames` are the enclosing functions, outermost first; the innermost
    /// binding wins.
    #[must_use]
    pub fn binding(&self, name: &str, frames: &[u32]) -> Option<&Binding> {
        frames
            .iter()
            .rev()
            .find_map(|frame| {
                self.bindings.iter().find(|b| b.name == name && b.scope == Scope::Function(*frame))
            })
            .or_else(|| self.bindings.iter().find(|b| b.name == name && b.scope == Scope::Module))
    }

    /// Returns the field `name` of `class`.
    #[must_use]
    pub fn field(&self, class: &str, name: &str) -> Option<&Binding> {
        self.bindings
            .iter()
            .find(|b| b.name == name && matches!(&b.scope, Scope::Class(c) if c == class))
    }

    /// Returns the export named `name`.
    #[must_use]
    pub fn export(&self, name: &str) -> Option<&Export> {
        self.exports.iter().find(|export| export.name == name)
    }

    /// Returns the module-level binding the export named `name` exports.
    #[must_use]
    pub fn exported_binding(&self, name: &str) -> Option<&Binding> {
        let export = self.export(name)?;
        self.binding(export.local.as_deref().unwrap_or(&export.name), &[])
    }

    /// Returns where `export`'s binding is declared, else the export's own lines.
    #[must_use]
    pub fn declared_at(&self, export: &Export) -> Lines {
        self.binding(export.local.as_deref().unwrap_or(&export.name), &[])
            .map_or(export.lines, |binding| binding.lines)
    }

    /// Returns the class declared as `name`.
    #[must_use]
    pub fn class(&self, name: &str) -> Option<&ClassDecl> {
        self.classes.iter().find(|class| class.name == name)
    }

    /// Returns the decorators on member `name` of `class`, or on a function for `None`.
    #[must_use]
    pub fn decorators_of(&self, class: Option<&str>, name: &str) -> Vec<&Decorated> {
        self.decorated
            .iter()
            .filter(|d| d.class.as_deref() == class && d.member.as_deref() == Some(name))
            .collect()
    }

    /// Returns each name read within `lines`, once, in first-read order.
    #[must_use]
    pub fn referenced(&self, lines: Lines) -> Vec<&str> {
        self.names(|line| lines.holds(line))
    }

    /// Returns each name read outside every span of `lines`, once, in first-read order.
    #[must_use]
    pub fn referenced_outside(&self, lines: &[Lines]) -> Vec<&str> {
        self.names(|line| !lines.iter().any(|l| l.holds(line)))
    }

    fn names(&self, keep: impl Fn(u32) -> bool) -> Vec<&str> {
        unique(self.references.iter().filter(|r| keep(r.line)).map(|r| r.name.as_str()))
    }

    /// Returns each line reading a name `keep` accepts, once, in first-read order.
    #[must_use]
    pub fn referencing(&self, keep: impl Fn(&str) -> bool) -> Vec<u32> {
        unique(self.references.iter().filter(|r| keep(&r.name)).map(|r| r.line))
    }

    /// Returns what loading the module or constructing its classes reaches.
    ///
    /// The head of every class field's and module-level binding's type and
    /// initializer, once each: `Kafka` for `consumer: Kafka.Consumer`,
    /// `express` for `const app = express()`.
    #[must_use]
    pub fn constructed(&self) -> Vec<&str> {
        unique(self.bindings.iter().flat_map(|binding| {
            let ((Scope::Class(_), BindingKind::Field { type_path, root, .. })
            | (Scope::Module, BindingKind::Value { type_path, root, .. })) =
                (&binding.scope, &binding.kind)
            else {
                return Vec::new();
            };
            type_path
                .iter()
                .chain(root)
                .filter_map(|path| path.first().map(String::as_str))
                .collect()
        }))
    }

    /// Returns every settled target of an import or re-export, type-only ones included.
    pub fn targets(&self) -> impl Iterator<Item = &Target> {
        self.settled(true)
    }

    fn settled(&self, types: bool) -> impl Iterator<Item = &Target> {
        let imports = self
            .imports
            .iter()
            .filter(move |import| types || !import.type_only)
            .map(|import| import.target.as_ref());
        let reexports = self
            .reexports
            .iter()
            .filter(move |reexport| types || !reexport.type_only)
            .map(|reexport| reexport.target.as_ref());
        imports.chain(reexports).flatten()
    }

    /// Returns each module of the tree the module reaches, once, in import order.
    ///
    /// A type-only import or re-export reaches its module only where
    /// `dialect` says a type import does.
    #[must_use]
    pub fn reached(&self, dialect: &Dialect) -> Vec<&str> {
        unique(self.settled(dialect.type_imports_reach).filter_map(Target::module))
    }

    /// Returns each data file the module imports, once, in import order.
    #[must_use]
    pub fn data(&self) -> Vec<&str> {
        unique(self.targets().filter_map(Target::data))
    }

    /// Returns each specifier the resolver could not follow, once, in import order.
    ///
    /// A type-only import never counts: it reaches nothing a seam lays.
    #[must_use]
    pub fn unresolved(&self) -> Vec<&str> {
        unique(self.settled(false).filter_map(Target::unresolved))
    }
}

/// One import, and where the resolver settled it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Import {
    /// The name bound in the module; empty for an import binding nothing.
    pub local: String,
    /// The specifier as written: `./db`, `a.b.c`, `..sibling`.
    pub specifier: String,
    /// What the specifier's module is bound as.
    pub imported: Imported,
    /// Whether the import is of types alone, which runs nothing.
    pub type_only: bool,
    /// Where the import leads; `None` until the resolver settles it.
    pub target: Option<Target>,
}

/// What an import binds of the module it names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Imported {
    /// The module's default export.
    Default,
    /// The module itself, under one local name: `import * as m`, `import a.b as m`.
    Whole,
    /// One named export, by the name it is exported as.
    Named(String),
    /// Every export, each under its own name: `from m import *`.
    Star,
    /// Nothing: the module is loaded for its effect, or named by a literal.
    Effect,
}

/// One re-export: a module's exports passed through under this module's name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reexport {
    /// The specifier as written.
    pub specifier: String,
    /// The `(imported, exported)` pairs, or `None` for every export.
    pub names: Option<Vec<(String, String)>>,
    /// Whether the re-export is of types alone.
    pub type_only: bool,
    /// Where the re-export leads; `None` until the resolver settles it.
    pub target: Option<Target>,
}

/// One export of the module, by the name a caller imports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Export {
    /// The exported name.
    pub name: String,
    /// The local binding an `export { local as name }` list names; `None`
    /// where the binding carries the exported name.
    pub local: Option<String>,
    /// What kind of thing is exported.
    pub kind: ExportKind,
    /// The export's own lines.
    pub lines: Lines,
}

/// What kind of thing an export is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportKind {
    /// A function.
    Function,
    /// A class.
    Class,
    /// A value: a constant, an instance, a configuration.
    Value,
    /// A type alone, which a caller copies and never calls.
    Type,
    /// A list entry naming a binding the module does not declare.
    Unknown,
}

impl ExportKind {
    /// Returns whether a caller calls or constructs the export.
    #[must_use]
    pub const fn callable(self) -> bool {
        matches!(self, Self::Function | Self::Class)
    }

    /// Returns whether the export is a value.
    #[must_use]
    pub const fn valued(self) -> bool {
        matches!(self, Self::Value)
    }
}

/// One binding of a name, at the scope that declares it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    /// The bound name.
    pub name: String,
    /// Where the name is bound.
    pub scope: Scope,
    /// What is bound.
    pub kind: BindingKind,
    /// The declaration's lines.
    pub lines: Lines,
}

/// Where a name is bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Scope {
    /// At the module's top level.
    Module,
    /// Within the function frame numbered.
    Function(u32),
    /// As a member of the class named.
    Class(String),
}

/// What a binding binds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingKind {
    /// A function.
    Function,
    /// A class.
    Class {
        /// The last name of each base, in order.
        bases: Vec<String>,
    },
    /// A value bound at module or function scope.
    ///
    /// The name a `with` or `using` holds is one, its initializer the call
    /// it manages.
    Value {
        /// The identifier path at the head of the initializer: `express`
        /// for `express()`, `click.group` for `click.group()`.
        root: Option<Vec<String>>,
        /// The declared type's path: `Kafka.Consumer` for `x: Kafka.Consumer`.
        type_path: Option<Vec<String>>,
        /// The index in [`Module::calls`] of the initializing call.
        call: Option<usize>,
        /// The value of a plain string literal initializer.
        string: Option<String>,
        /// What the initializer is.
        init: Init,
        /// The initializer's first line, cut as the walker cuts it.
        head: Option<String>,
    },
    /// A class field.
    Field {
        /// The declared type's path.
        type_path: Option<Vec<String>>,
        /// The identifier path at the head of the initializer.
        root: Option<Vec<String>>,
        /// What the initializer is.
        init: Init,
        /// The initializer's first line, cut as the walker cuts it.
        head: Option<String>,
    },
    /// A function parameter.
    Param {
        /// The declared type's path.
        type_path: Option<Vec<String>>,
    },
}

/// What a binding's initializer is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Init {
    /// A literal, or an expression over literals: `5 * 1000`, `["a", "b"]`.
    Literal,
    /// A regular expression over a literal.
    Pattern,
    /// A read of the environment, whatever else it does with it.
    Env,
    /// A call or construction handed something spelled in place and no
    /// function: `z.object({ .. })`, `Field(default=..)`.
    Definition,
    /// A call or construction handed nothing spelled in place, a module
    /// loaded, or an awaited one: `express()`, `create_app()`.
    Construction,
    /// A function.
    Function,
    /// A reference, a member read, or no initializer.
    Other,
}

impl Init {
    /// Returns whether the initializer spells a value a criterion could cite.
    #[must_use]
    pub const fn is_value(self) -> bool {
        matches!(self, Self::Literal | Self::Pattern | Self::Env | Self::Definition)
    }
}

/// What becomes of a call's value.
///
/// A call made for its effect, waited on, or bound to a name is a step the
/// function takes; one passed, returned, or chained on is a value in hand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Use {
    /// Dropped: an expression statement's, awaited or not.
    Discarded,
    /// Waited on, then used.
    Awaited,
    /// Bound to a name: assigned, or held by a `with` or `using`.
    Bound,
    /// Passed, returned, or chained on.
    Consumed,
}

/// One call, with where it stands and what it is handed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Call {
    /// What is called.
    pub callee: Callee,
    /// Whether the call constructs: `new`, or a callee whose last name is capitalised.
    pub constructs: bool,
    /// The arguments, positional then keyword.
    pub args: Vec<Arg>,
    /// How many handler bodies enclose the call.
    pub depth: usize,
    /// What becomes of the value.
    pub value: Use,
    /// Whether a further, non-structural call in the chain is made on this
    /// one's value: `a.b()` in `a.b().c()`.
    pub inner: bool,
    /// The enclosing function frames, outermost first.
    pub frames: Vec<u32>,
    /// The innermost named enclosing function.
    pub function: Option<String>,
    /// The enclosing class.
    pub class: Option<String>,
    /// The call's lines.
    pub lines: Lines,
}

impl Call {
    /// Returns the method called: the callee's last name.
    #[must_use]
    pub fn method(&self) -> &str {
        self.callee.method()
    }

    /// Returns whether the call's value is dropped.
    #[must_use]
    pub fn discarded(&self) -> bool {
        self.value == Use::Discarded
    }

    /// Returns the string literal leading the positional arguments.
    #[must_use]
    pub fn literal(&self) -> Option<&str> {
        self.args.iter().find(|arg| arg.keyword.is_none()).and_then(|arg| arg.literal.as_deref())
    }

    /// Returns the argument handed under the keyword `name`.
    #[must_use]
    pub fn keyword(&self, name: &str) -> Option<&Arg> {
        self.args.iter().find(|arg| arg.keyword.as_deref() == Some(name))
    }

    /// Returns whether the call is structure rather than a registration, under `dialect`.
    #[must_use]
    pub fn structural(&self, dialect: &Dialect) -> bool {
        self.callee.structural(self.literal(), dialect)
    }

    /// Returns the method and literal a chain registers under.
    ///
    /// A chain names itself at its first call with a literal
    /// (`command("x").option(..)` is `command`, `x`); otherwise the call's
    /// own method and `led` stand.
    #[must_use]
    pub fn registered(&self, led: Option<String>) -> (&str, Option<String>) {
        let chained = self.callee.links.iter().find_map(|link| {
            let literal = link.call.as_ref()?.literal.clone()?;
            Some((link.name.as_str(), literal))
        });
        chained.map_or_else(|| (self.method(), led), |(method, literal)| (method, Some(literal)))
    }
}

/// What a call calls: a head and the members chained from it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Callee {
    /// The identifier the callee starts from, or the dialect's self name.
    pub head: String,
    /// The head's own call, where the head is called first: `Router().get`.
    pub head_call: Option<Invocation>,
    /// The members chained from the head, in order.
    pub links: Vec<Link>,
}

impl Callee {
    /// Returns the last link's name, or the head when there is none.
    #[must_use]
    pub fn method(&self) -> &str {
        self.links.last().map_or(&self.head, |link| &link.name)
    }

    /// Returns the head and every link, dotted: `app.router.get`.
    #[must_use]
    pub fn dotted(&self) -> String {
        std::iter::once(self.head.as_str())
            .chain(self.links.iter().map(|link| link.name.as_str()))
            .collect::<Vec<_>>()
            .join(".")
    }

    /// Returns the head and every link as a path.
    #[must_use]
    pub fn path(self) -> Vec<String> {
        std::iter::once(self.head).chain(self.links.into_iter().map(|link| link.name)).collect()
    }

    /// Returns the head, or the first link's name under the dialect's self name.
    #[must_use]
    pub fn receiver(&self, dialect: &Dialect) -> &str {
        if self.head == dialect.self_name {
            self.links.first().map_or(dialect.self_name, |link| link.name.as_str())
        } else {
            &self.head
        }
    }

    /// Returns whether a call through the callee is structure, under `dialect`.
    ///
    /// Structure is a method the dialect lists as such, a listener whose
    /// event (`literal`) is none or a lifecycle's, a mocking call by its
    /// dotted spelling, or a lifecycle hook by the tail of it. A function
    /// handed to structure runs at its caller's depth and registers nothing.
    #[must_use]
    pub fn structural(&self, literal: Option<&str>, dialect: &Dialect) -> bool {
        let method = self.method();
        let listener = dialect.listeners.contains(&method)
            && literal.is_none_or(|event| dialect.lifecycle_events.contains(&event));
        let dotted = self.dotted();
        dialect.structural.contains(&method)
            || listener
            || dialect.mocking.contains(&dotted.as_str())
            || dialect.hooks(&dotted)
    }
}

/// One member of a callee's chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Link {
    /// The member's name.
    pub name: String,
    /// The member's own call, where it is called within the chain.
    pub call: Option<Invocation>,
}

/// A call made within a chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invocation {
    /// The string literal leading its arguments.
    pub literal: Option<String>,
}

/// One argument of a call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Arg {
    /// The keyword the argument is handed under; `None` for a positional one.
    pub keyword: Option<String>,
    /// The string literal the argument is, if one.
    pub literal: Option<String>,
    /// The identifier path the argument starts from: `controller.list`,
    /// `OrderList.as_view`.
    pub root: Option<Vec<String>>,
    /// Whether the argument is itself a call.
    pub called: bool,
    /// The string literal leading a called argument's own arguments, or a
    /// list's elements: `orders.urls` for `include("orders.urls")`.
    pub inner: Option<String>,
    /// Whether the argument is a function, an object holding one, or a call passing one.
    pub function: bool,
    /// The string-valued properties of an object written in place, nested
    /// objects flattened.
    pub properties: Vec<Property>,
    /// The argument's first line, cut as an initializer's head is.
    pub head: String,
    /// The argument's lines.
    pub lines: Lines,
}

impl Arg {
    /// Returns the property at the dotted `key`: `options.prefix`.
    #[must_use]
    pub fn property(&self, key: &str) -> Option<&Property> {
        self.properties.iter().find(|property| property.key == key)
    }

    /// Returns whether the argument is handed as a hook under `dialect`'s keywords.
    #[must_use]
    pub fn is_hook(&self, dialect: &Dialect) -> bool {
        self.keyword.as_deref().is_some_and(|keyword| dialect.hook_keywords.contains(&keyword))
    }
}

/// One string-valued property of an object written in place.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Property {
    /// The static keys from the object down to the property, dotted: `options.prefix`.
    pub key: String,
    /// The string the property holds, or the path it spells.
    pub value: String,
    /// Whether the value is spelled from the module's own directory
    /// (`path.join(__dirname, "routes")`), so it names a directory of the
    /// tree relative to the module's.
    pub relative: bool,
}

/// One decoration: a decorator applied to a class or a member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decorated {
    /// The class a decorated member belongs to, or the class decorated;
    /// `None` for a decorated function.
    pub class: Option<String>,
    /// The member decorated; `None` for a class decorator.
    pub member: Option<String>,
    /// The decorator's path: `Get`, `app.route`.
    pub name: Vec<String>,
    /// The string literal it is handed first, or under its path keyword.
    pub literal: Option<String>,
    /// Each keyword argument with its value's head: `methods`, `["GET", "POST"]`.
    pub keywords: Vec<(String, String)>,
    /// From the first decorator through the declaration line.
    pub head: Lines,
    /// From the first decorator to the end of what it decorates.
    pub lines: Lines,
}

impl Decorated {
    /// Returns the head of the value handed under the keyword `name`.
    #[must_use]
    pub fn keyword(&self, name: &str) -> Option<&str> {
        self.keywords.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    /// Returns whether the decorator registers what it decorates, under `dialect`.
    ///
    /// One the dialect lists as shaping, as a hook, or as a lifecycle's
    /// registers nothing.
    #[must_use]
    pub fn registering(&self, dialect: &Dialect) -> bool {
        self.name.last().is_some_and(|name| {
            !dialect.shapes(name) && !dialect.decorator_hooks.contains(&name.as_str())
        }) && !dialect.hooks(&self.name.join("."))
    }
}

/// One type declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeDecl {
    /// The declared name.
    pub name: String,
    /// What kind of declaration it is.
    pub kind: TypeKind,
    /// Whether the module exports it.
    pub exported: bool,
    /// The declaration's lines.
    pub lines: Lines,
    /// The declaration as written.
    pub text: String,
}

/// What kind of type a declaration is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeKind {
    /// An interface.
    Interface,
    /// An alias: `type X = ..`, `X: TypeAlias = ..`, a `TypeVar` or `NewType`.
    Alias,
    /// An enumeration declared as a type.
    Enum,
    /// A type built by call: a functional `TypedDict`, `NamedTuple`, or `Enum`.
    Functional,
}

/// One class declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassDecl {
    /// The declared name.
    pub name: String,
    /// Whether the module exports it.
    pub exported: bool,
    /// The declaration's lines.
    pub lines: Lines,
    /// The header as written: `export class Main`, `class Money:`.
    pub header: String,
    /// The last name of each base, in order.
    pub bases: Vec<String>,
    /// The members, in order.
    pub members: Vec<Member>,
}

/// One member of a class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Member {
    /// The member's name.
    pub name: String,
    /// What kind of member it is.
    pub kind: MemberKind,
    /// Whether the member is private to the class.
    pub private: bool,
    /// The member's lines.
    pub lines: Lines,
    /// The declaration up to its body or initializer.
    pub signature: String,
}

/// What kind of member a class member is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemberKind {
    /// The constructor.
    Constructor,
    /// A method.
    Method,
    /// A getter.
    Getter,
    /// A setter.
    Setter,
    /// A field.
    Field,
    /// A class nested in the body: a `Meta`, a `Config`.
    Nested,
}

/// One read of the environment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvRead {
    /// The key read.
    pub key: String,
    /// The read's lines.
    pub lines: Lines,
}

/// One point where the code decides.
///
/// A guard, a switch or match, a conditional, a throw or raise, a catch
/// or except, a loop condition, an assertion, or a timer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    /// The decision's lines.
    pub lines: Lines,
    /// The innermost named enclosing function.
    pub function: Option<String>,
    /// The enclosing class.
    pub class: Option<String>,
    /// The first line, cut as an initializer's head is: `if (<test>)`,
    /// `match <subject>`, `except <type>`, the throw statement.
    pub text: String,
}

/// One load of a module by a computed name, which no resolver can follow to a module.
///
/// A seam past the load widens to the directory it leads into, where the
/// load spells one, and to the loading module's own where it spells none.
/// See [`Tree::widening`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Dynamic {
    /// The load's lines.
    pub lines: Lines,
    /// What the load spells of where it leads, as written: the literal a
    /// computed name leads with (`app.plugins.`, `./plugins/`), or the
    /// package whose path is walked (`plugins`). `None` where it spells
    /// nothing.
    pub specifier: Option<String>,
    /// The root-relative directory the load leads into; `None` until the
    /// resolver settles it, and where the load spells nothing.
    pub scope: Option<String>,
}

/// One read of a name, at its line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reference {
    /// The name read.
    pub name: String,
    /// The line it is read at.
    pub line: u32,
}
