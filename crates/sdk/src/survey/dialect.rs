//! Carries what a survey reads of a language, as data.

use super::route::Spelling;

/// The names and spellings a survey's lookups read of one language.
///
/// Two languages' surveys differ in lists of names and a few spellings, so
/// a dialect is a struct of them rather than a trait. An adapter declares
/// one `static` and passes it to every lookup that reads one; a list that
/// does not apply to a language is empty.
///
/// # Examples
///
/// ```
/// use emery_sdk::survey::Dialect;
/// use emery_sdk::survey::route::Spelling;
///
/// static TYPESCRIPT: Dialect = Dialect {
///     self_name: "this",
///     generic_stems: &["index", "main"],
///     structural: &["use", "then", "forEach"],
///     listeners: &["on", "once"],
///     lifecycle_events: &["error", "close"],
///     mocking: &[],
///     lifecycle: &[],
///     decorator_noise: &["UseGuards", "Injectable"],
///     decorator_noise_prefixes: &["Api"],
///     decorator_hooks: &[],
///     hook_keywords: &[],
///     type_imports_reach: true,
///     openers: &['[', '{'],
///     comment_prefixes: &["//", "*"],
///     globals: &["fetch"],
///     route: Spelling {
///         pattern: &['*', '{', '(', '['],
///         param_name: |segment| segment.trim_start_matches(':'),
///     },
/// };
///
/// assert!(TYPESCRIPT.shapes("ApiTags"));
/// assert!(TYPESCRIPT.shapes("UseGuards"));
/// assert!(!TYPESCRIPT.shapes("Get"));
/// assert!(!TYPESCRIPT.hooks("app.listen"));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Dialect {
    /// The receiver a method names itself by: `this`, `self`.
    pub self_name: &'static str,
    /// File stems that name a role rather than a thing, so the module takes
    /// its directory's name: `index`, `main`, `views`.
    pub generic_stems: &'static [&'static str],
    /// Methods whose function arguments are structure, not handlers:
    /// lifecycle, iteration, mounting, scheduling. Matched against a call's
    /// method, or its head when it has none.
    pub structural: &'static [&'static str],
    /// Listener methods whose event decides whether a handler is a surface:
    /// `on`, `once`. Empty where no handler is registered by event.
    pub listeners: &'static [&'static str],
    /// Events a listener hears about the process, a connection, or a
    /// stream, never a caller: `error`, `close`, `SIGTERM`.
    pub lifecycle_events: &'static [&'static str],
    /// Calls that patch rather than register, by their dotted spelling
    /// whole: `mock.patch`.
    pub mocking: &'static [&'static str],
    /// Hooks on the process, a connection, or the application's lifecycle,
    /// matched by the tail of a call's or a decorator's dotted spelling:
    /// `atexit.register`, `on_event`.
    pub lifecycle: &'static [&'static str],
    /// Decorators that shape what they decorate rather than register it:
    /// `UseGuards`, `dataclass`.
    pub decorator_noise: &'static [&'static str],
    /// Prefixes of decorators that document rather than register: `Api`
    /// for `ApiTags`.
    pub decorator_noise_prefixes: &'static [&'static str],
    /// Decorators that hook what they decorate onto a lifecycle rather than
    /// register a surface: `receiver`, `errorhandler`.
    pub decorator_hooks: &'static [&'static str],
    /// Keywords a function is handed under as a hook on the thing being
    /// built, not as the handler it registers: `lifespan`, `callback`.
    pub hook_keywords: &'static [&'static str],
    /// Whether a type-only import reaches the module it names.
    pub type_imports_reach: bool,
    /// The characters an enumeration written out in place opens with: `[`
    /// and `{`, and `(` for a tuple.
    pub openers: &'static [char],
    /// What a comment line leads with: `//` and `*`, or `#`.
    pub comment_prefixes: &'static [&'static str],
    /// Callees the runtime provides with no import, named as themselves:
    /// `fetch`, `open`.
    pub globals: &'static [&'static str],
    /// How the language's frameworks spell a route's parameters and patterns.
    pub route: Spelling,
}

impl Dialect {
    /// Returns whether `decorator` shapes what it decorates rather than registering it.
    #[must_use]
    pub fn shapes(&self, decorator: &str) -> bool {
        self.decorator_noise.contains(&decorator)
            || self.decorator_noise_prefixes.iter().any(|prefix| decorator.starts_with(prefix))
    }

    /// Returns whether `dotted` is a lifecycle hook, whole or by its tail.
    #[must_use]
    pub fn hooks(&self, dotted: &str) -> bool {
        self.lifecycle.iter().any(|tail| {
            dotted.strip_suffix(tail).is_some_and(|rest| rest.is_empty() || rest.ends_with('.'))
        })
    }
}
