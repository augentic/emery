//! Spells the stems and discriminators a route or a literal carries.
//!
//! A surface registered at a route takes its stem from the first segment
//! that names a resource, `orders` in `/api/v1/orders/{id}`, and is told
//! from the others under that stem by its verb and the segments past the
//! resource, `get-id`. One led by a literal — a command, a queue, an event —
//! takes the literal's first word. How a language's frameworks spell a
//! parameter or a pattern is the one per-language input, carried as a
//! [`Spelling`].

use crate::kebab;

// Segments that only version or namespace a path, never naming a resource.
const PATH_NOISE: &[&str] = &["api", "rest", "internal"];

// Literals a scheduler is led by that name its trigger, not its job.
const TRIGGERS: &[&str] = &["cron", "interval", "date"];

/// How a language's frameworks spell a route's parameters and patterns.
///
/// The one per-language input to the route helpers: an adapter declares one
/// `const` and reads every stem and discriminator through it.
///
/// # Examples
///
/// ```
/// use emery_sdk::survey::route::Spelling;
///
/// const EXPRESS: Spelling = Spelling {
///     pattern: &['*', '{', '(', '['],
///     param_name: |segment| segment.trim_start_matches(':'),
/// };
///
/// assert_eq!(EXPRESS.stem("/api/v1/orders/:id"), Some("orders".to_owned()));
/// assert_eq!(
///     EXPRESS.discriminator("GET", "/api/orders/:id", "orders"),
///     Some("get-id".to_owned())
/// );
/// assert_eq!(EXPRESS.normalised("GET /api/orders", "orders"), Some("get".to_owned()));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Spelling {
    /// The characters a pattern segment contains; a segment led by `:` is a
    /// pattern regardless.
    pub pattern: &'static [char],
    /// A parameter segment's bare name, `id` for `:id` or `{id}`; any other
    /// segment as written.
    pub param_name: fn(&str) -> &str,
}

impl Spelling {
    /// Returns whether `segment` names a resource.
    ///
    /// A version (`v1`), a namespace (`api`, `rest`, `internal`), a
    /// parameter, and a pattern name none.
    #[must_use]
    pub fn names_resource(&self, segment: &str) -> bool {
        let version = segment
            .strip_prefix('v')
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()));
        let pattern = segment.starts_with(':') || segment.contains(self.pattern);
        !(PATH_NOISE.contains(&segment) || version || pattern)
    }

    /// Returns the stem a route spells: its first segment naming a resource, kebab-cased.
    #[must_use]
    pub fn stem(&self, route: &str) -> Option<String> {
        route
            .split('/')
            .filter(|segment| !segment.is_empty())
            .find(|segment| self.names_resource(segment))
            .and_then(kebab)
    }

    /// Returns what tells a route apart from the others under `stem`.
    ///
    /// The verb, then the segments past the one spelling the stem, a
    /// parameter by its bare name: `GET /api/orders/:id` under `orders` is
    /// `get-id`. A route spelling no segment as its stem is relative to a
    /// mount the survey did not read, so every segment tells: `POST
    /// /:id/assign` under `tasks` is `post-id-assign`.
    #[must_use]
    pub fn discriminator(&self, verb: &str, route: &str, stem: &str) -> Option<String> {
        let segments: Vec<&str> = route.split('/').filter(|segment| !segment.is_empty()).collect();
        let past = segments
            .iter()
            .position(|segment| kebab(segment).is_some_and(|spelled| spelled == stem))
            .map_or(0, |i| i + 1);
        let parts: Vec<&str> = std::iter::once(verb)
            .chain(segments[past..].iter().map(|segment| (self.param_name)(segment)))
            .collect();
        kebab(&parts.join("-"))
    }

    /// Returns a surface's name kebab-cased, less the words of its stem and the noise.
    ///
    /// The words dropped are the stem's own and the segments that only
    /// version or namespace a path: `GET /api/customers` under `customers`
    /// is `get`. `None` when nothing is left.
    #[must_use]
    pub fn normalised(&self, name: &str, stem: &str) -> Option<String> {
        let spelled = kebab(name)?;
        let kept: Vec<&str> = spelled
            .split('-')
            .filter(|word| !stem.split('-').any(|own| own == *word) && self.names_resource(word))
            .collect();
        kebab(&kept.join("-"))
    }
}

/// Returns `/prefix/path`, however either is spelled.
///
/// `orders`, `/orders/`, and `orders/` all join alike: the result leads with
/// one slash and ends with none, `/` for two empty parts.
///
/// # Examples
///
/// ```
/// use emery_sdk::survey::route::join;
///
/// assert_eq!(join("/api/", "orders"), "/api/orders");
/// assert_eq!(join("orders", "/{id}/"), "/orders/{id}");
/// assert_eq!(join("", ""), "/");
/// ```
#[must_use]
pub fn join(prefix: &str, path: &str) -> String {
    let segments: Vec<&str> =
        prefix.split('/').chain(path.split('/')).filter(|segment| !segment.is_empty()).collect();
    format!("/{}", segments.join("/"))
}

/// Returns the stem a literal spells: its first word, a dotted name by its first segment.
///
/// `import-orders` is `import-orders` and `invoices.send` is `invoices`.
/// `None` for a pattern, a schedule, or a scheduler's trigger word (`cron`,
/// `interval`, `date`).
///
/// # Examples
///
/// ```
/// use emery_sdk::survey::route::literal_stem;
///
/// assert_eq!(literal_stem("invoices.send"), Some("invoices".to_owned()));
/// assert_eq!(literal_stem("import <file>"), Some("import".to_owned()));
/// assert_eq!(literal_stem("*/5 * * * *"), None);
/// assert_eq!(literal_stem("cron"), None);
/// ```
#[must_use]
pub fn literal_stem(literal: &str) -> Option<String> {
    let word = literal.split_whitespace().next()?;
    if !word.starts_with(|c: char| c.is_ascii_alphabetic())
        || word.contains(['<', '>', '[', ']', '*', '/', ':'])
        || TRIGGERS.contains(&word)
    {
        return None;
    }
    kebab(word.split('.').next().unwrap_or(word))
}
