//! Declares what an adapter recognises of its own language's surfaces.

use std::collections::BTreeMap;
use std::ops::Deref;

use super::surface::Derived;
use super::tree::Tree;
use super::{Arg, Call, ClassDecl, Module};
use crate::survey::Lines;
use crate::survey::tests::Statement;

/// What an adapter recognises of a tree that the shared pipeline cannot.
///
/// The pipeline reads a tree through the [`Module`] its parser fills and
/// the lookups there; what it asks the adapter is what its language alone
/// decides: where each import leads, what the manifest declares, what a
/// test states, which entry runs when the tree loads, what a call hands,
/// where routes are mounted, and what the code says at an anchor the
/// survey accepted. An adapter implements this for the value that holds
/// its resolver and manifest reader, built from the [`Parsed`] tree, and
/// hands it to [`Parsed::settle`].
///
/// The methods with a default answer are for a language that has no
/// such thing: one that hands no class to a registration leaves
/// [`class_handed`](Self::class_handed) and
/// [`handed_class`](Self::handed_class) alone.
///
/// [`Parsed`]: super::Parsed
/// [`Parsed::settle`]: super::Parsed::settle
pub trait Recogniser: Sized + Sync {
    /// The module the adapter's parser fills: the shared [`Module`], or the
    /// adapter's own type over it.
    type Module: Deref<Target = Module> + Sync;

    /// Settles where each import and re-export of `module` leads.
    ///
    /// Called once per module after every module is parsed, and once per
    /// test module after that.
    fn settle(&self, module: &mut Self::Module);

    /// Returns what the tree's manifest declares.
    ///
    /// The default is a tree with no manifest.
    fn manifest(&self) -> Manifest {
        Manifest::default()
    }

    /// Returns the behaviours a test module states, in file order.
    fn statements(&self, module: &Self::Module) -> Vec<Statement>;

    /// Returns the entry that runs something when the tree loads, and how.
    ///
    /// `None` for a library: a tree whose entries only declare.
    fn bootstrap<'t>(&self, tree: &'t Tree<Self>) -> Option<Bootstrap<'t, Self::Module>>;

    /// Returns whether `arg`, handed within `frames` of `module`, is a handler.
    ///
    /// A handler is a function, or a name bound to a function or class of
    /// the tree, declared in `module` or imported from a module of the tree.
    fn handler(&self, tree: &Tree<Self>, module: &Self::Module, arg: &Arg, frames: &[u32]) -> bool;

    /// Returns whether `arg` hands a class as it is, rather than a function
    /// or an instance.
    ///
    /// A class handed straight to a package's own function is queried or
    /// typed by it, not run, so [`Tree::hands`] discounts it. The default
    /// hands none.
    fn class_handed(
        &self, tree: &Tree<Self>, module: &Self::Module, arg: &Arg, frames: &[u32],
    ) -> bool {
        let _ = (tree, module, arg, frames);
        false
    }

    /// Returns the class `call` hands to be run, with the module declaring it.
    ///
    /// A view handed by its `as_view()`, a view set registered on a router,
    /// a worker class: its methods are ids of the registration, and what
    /// its module references is reached. The default hands none.
    fn handed_class<'t>(
        &self, tree: &'t Tree<Self>, module: &'t Self::Module, call: &Call,
    ) -> Option<(&'t Self::Module, String)> {
        let _ = (tree, module, call);
        None
    }

    /// Returns the route prefix each module's routes sit under, by module path.
    ///
    /// Called once when the tree settles; [`Tree::mounts`] carries the
    /// answer. Empty where the language's frameworks mount nothing under a
    /// prefix.
    fn mounts(&self, tree: &Tree<Self>) -> BTreeMap<String, String> {
        let _ = tree;
        BTreeMap::new()
    }

    /// Returns whether `class` declares data alone, which a caller copies
    /// and never calls.
    ///
    /// The default declares none: every exported class is listed among the
    /// entry modules' exports.
    fn declares_data(&self, class: &ClassDecl) -> bool {
        let _ = class;
        false
    }

    /// Returns what the code says at an anchor the survey accepted.
    ///
    /// `lines` is the anchor within `module`, the whole module where the
    /// survey cited none; `name` and `stem` are the survey's, for the
    /// surface nothing at the anchor derives further than
    /// [`Derived::named`].
    fn derive(
        &self, tree: &Tree<Self>, module: &Self::Module, lines: Lines, name: &str, stem: &str,
    ) -> Derived;
}

/// What a workspace listing offers a tree: its modules, data files, and tests.
///
/// Each path is root-relative, as the adapter's keep policy listed it
/// through [`workspace::list`](crate::workspace::list).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Listing {
    /// Every production module.
    pub modules: Vec<String>,
    /// Every file a module may name as data; only one a module names is
    /// ever laid.
    pub data: Vec<String>,
    /// Every test file: a test module, or a `.feature` file.
    pub tests: Vec<String>,
}

/// What a tree's manifest declares, as the facts say it.
///
/// An absent or unreadable manifest is the default: nothing said, no
/// entry named.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Manifest {
    /// The package's name, as a stem is derived from it: a scoped name
    /// less its scope.
    pub name: Option<String>,
    /// What the manifest says, each clause as the facts render it after
    /// `The manifest`: `` names the package `shop` ``, `` installs the
    /// console scripts `shop` runs `shop.cli:main` ``.
    pub says: Vec<String>,
    /// The modules the manifest's entries, scripts, or binaries name,
    /// root-relative and in manifest order.
    pub entries: Vec<String>,
}

/// The entry that runs something when the tree loads, and how.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bootstrap<'t, M> {
    /// The module that runs.
    pub module: &'t M,
    /// How it comes to run.
    pub runs: Runs,
}

/// How a bootstrap comes to run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Runs {
    /// By a console script the manifest installs, calling a function the
    /// module declares or imports.
    Script {
        /// The script's name.
        name: String,
        /// The function it calls.
        function: String,
    },
    /// Under a `__main__` guard.
    Guard,
    /// At load: a call whose value is discarded, an import for its effect,
    /// or an application constructed at module level.
    Load,
}

impl Runs {
    // The clause after the bootstrap's path in the survey facts.
    pub(crate) fn said(&self) -> String {
        match self {
            Self::Script { name, function } => {
                format!("the console script `{name}` calls its `{function}()`")
            }
            Self::Guard => "it runs under its `__main__` guard".to_owned(),
            Self::Load => "it runs or constructs the application at load".to_owned(),
        }
    }

    // The clause after `the process bootstrap` in the `start` surface's note.
    pub(crate) fn how(&self) -> String {
        match self {
            Self::Script { name, function } => {
                format!("run by the console script `{name}`, which calls `{function}()`")
            }
            Self::Guard => "run under its `__main__` guard".to_owned(),
            Self::Load => "run at load".to_owned(),
        }
    }
}
