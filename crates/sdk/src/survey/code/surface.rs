//! Carries what the code says of each surface the survey named.

use super::recogniser::{Bootstrap, Recogniser};
use super::tree::{Tree, seed};
use crate::survey::{Lines, unique};

/// One surface of a tree: where control enters it from outside the process.
///
/// The survey names a surface at the lines that register or declare it;
/// the code derives the rest from those lines, so two runs that accept the
/// same anchors lead their ids the same way and reach the same modules.
/// The bootstrap's [`start`](Self::start) is the code's own surface, never
/// the survey's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Surface {
    /// What a caller does through the surface, as the survey named it.
    pub name: String,
    /// The module the surface is registered or declared in.
    pub entry: String,
    /// What every `requirement` and `criterion` id of the surface leads with.
    pub stem: String,
    /// The registration or declaration.
    pub lines: Lines,
    /// Notes for the brief, each a clause.
    pub detail: Vec<String>,
    /// The modules the surface reaches, its entry first.
    pub closure: Vec<String>,
    /// What tells the surface from the others under its stem: its handler's
    /// name, its verb and the path past the resource, its method's name.
    pub discriminator: Option<String>,
    /// The public methods of an exported or handed class, kebab-cased.
    pub methods: Vec<String>,
    /// The stem alone for a stem's one surface, `<stem>.<discriminator>`
    /// otherwise, and one more per method of a class; set by
    /// [`Tree::identify`].
    pub ids: Vec<String>,
}

impl Surface {
    /// Returns the bootstrap's `start` surface: what its module does outside
    /// the handlers it registers.
    ///
    /// Another surface's entry is what the bootstrap mounts, not what it
    /// does, so it is reached and not followed; what loading that entry
    /// constructs runs before any handler and is the bootstrap's to reach.
    /// `registered` are the surfaces the survey named.
    #[must_use]
    pub fn start<R: Recogniser>(
        tree: &Tree<R>, bootstrap: &Bootstrap<'_, R::Module>, registered: &[Self],
    ) -> Self {
        let module = bootstrap.module;
        let registrations: Vec<Lines> = registered
            .iter()
            .filter(|surface| surface.entry == module.path)
            .map(|surface| surface.lines)
            .collect();
        let mut seeds = vec![module.path.clone()];
        for name in module.referenced_outside(&registrations) {
            seed(&mut seeds, module, name);
        }
        let entries = unique(
            registered
                .iter()
                .map(|surface| surface.entry.clone())
                .filter(|entry| *entry != module.path),
        );

        // what each mounted entry constructs at load
        for entry in &entries {
            let Some(target) = tree.modules.get(entry) else { continue };
            for local in target.constructed() {
                seed(&mut seeds, target, local);
            }
        }

        Self {
            name: "start".to_owned(),
            entry: module.path.clone(),
            stem: "start".to_owned(),
            lines: module.span,
            detail: vec![format!(
                "the process bootstrap, {}: what runs before each handler is registered, what it \
                 awaits before serving, and at shutdown — `stop` and what a signal handler calls, \
                 wherever declared",
                bootstrap.runs.how()
            )],
            closure: tree.closure(&seeds, &entries),
            discriminator: None,
            methods: Vec::new(),
            ids: vec!["start".to_owned()],
        }
    }
}

/// The package a call's receiver comes from, and the type it is known as.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receiver {
    /// The package, by top-level name as imported.
    pub package: String,
    /// The class the receiver is typed or constructed as, where the code says.
    pub type_name: Option<String>,
}

/// What the code says at an anchor the survey accepted.
///
/// An adapter's [`Recogniser::derive`] reads the registration, decorator,
/// or export at the anchor and answers one of these; [`Derived::named`] is
/// the answer where nothing at the anchor derives further.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Derived {
    /// The stem the code spells at the anchor; `None` leaves the survey's
    /// standing.
    pub stem: Option<String>,
    /// What tells the surface from the others under its stem.
    pub discriminator: Option<String>,
    /// The public methods of the class declared, decorated, or handed at
    /// the anchor, each with its lines.
    pub methods: Vec<(String, Lines)>,
    /// The registration or declaration whole where the survey cited one
    /// line of several, else the anchor.
    pub lines: Lines,
    /// Notes for the brief, each a clause.
    pub detail: Vec<String>,
    /// The modules reached, the anchor's module first.
    pub closure: Vec<String>,
}

impl Derived {
    /// Returns what the survey's `name` and `stem` alone say at `lines` of `module`.
    ///
    /// The name less its stem tells the surface apart and the lines are read
    /// as they are, reaching what the code there references.
    #[must_use]
    pub fn named<R: Recogniser>(
        tree: &Tree<R>, module: &R::Module, lines: Lines, name: &str, stem: &str,
    ) -> Self {
        Self {
            stem: None,
            discriminator: tree.dialect.route.normalised(name, stem),
            methods: Vec::new(),
            lines,
            detail: vec![format!("named by the survey at {lines}")],
            closure: tree.reaches(module, lines, &[], None),
        }
    }
}
