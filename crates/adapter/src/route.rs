//! Defines the route a surface serves: an HTTP method and a path template.
//!
//! [`Route::parse`] is the one grammar every side holds a route to: the
//! `route` extra of a `contract` claim, the `Route:` line of a requirement,
//! and the typed route a slice carries to a target.

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// The HTTP methods a route spells, uppercase.
pub const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// An HTTP route: a method and a path template, `POST /orders/{id}/pay`.
///
/// The method is one of [`METHODS`], uppercase. The path leads with `/`,
/// ends with none, and spells each parameter `{name}`, however the source's
/// framework spelled it.
///
/// # Examples
///
/// ```
/// use emery_adapter::Route;
///
/// let route = Route::parse("post /api/orders/:id/pay/")?;
/// assert_eq!(route.method, "POST");
/// assert_eq!(route.path, "/api/orders/{id}/pay");
/// assert_eq!(route.to_string(), "POST /api/orders/{id}/pay");
/// assert_eq!(route.params().collect::<Vec<_>>(), ["id"]);
///
/// assert_eq!(Route::parse("GET /items/<int:id>")?.path, "/items/{id}");
/// assert!(Route::parse("FETCH /orders").is_err());
/// assert!(Route::parse("GET orders").is_err());
/// # Ok::<(), emery_adapter::BadRoute>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Route {
    /// The method, uppercase.
    pub method: String,
    /// The path template, led by `/`, each parameter `{name}`.
    pub path: String,
}

impl Route {
    /// Parses `text` as `METHOD /path`.
    ///
    /// The method is read without case. A parameter spelled `:name`,
    /// `<name>`, `<type:name>`, or `{name:type}` is written `{name}`; a `*`
    /// segment is the wildcard `{*rest}`. A query string and a trailing `/`
    /// are dropped.
    ///
    /// # Errors
    ///
    /// Returns [`BadRoute`] describing the first rule the text breaks: not two
    /// words, a method outside [`METHODS`], or a path that does not lead
    /// with `/` or holds a segment outside the grammar.
    pub fn parse(text: &str) -> Result<Self, BadRoute> {
        let mut words = text.split_whitespace();
        let (Some(method), Some(path), None) = (words.next(), words.next(), words.next()) else {
            return Err(BadRoute::Grammar);
        };
        let method = METHODS
            .iter()
            .find(|known| known.eq_ignore_ascii_case(method))
            .ok_or_else(|| BadRoute::Method(method.to_owned()))?;
        let Some(rest) = path.strip_prefix('/') else {
            return Err(BadRoute::Path(path.to_owned()));
        };
        let rest = rest.split('?').next().unwrap_or(rest);
        let segments = rest
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| parameter(segment).ok_or_else(|| BadRoute::Path(path.to_owned())))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            method: (*method).to_owned(),
            path: format!("/{}", segments.join("/")),
        })
    }

    /// Returns the names of the path's parameters, in order, a wildcard by
    /// its name.
    pub fn params(&self) -> impl Iterator<Item = &str> {
        self.path
            .split('/')
            .filter_map(|segment| segment.strip_prefix('{')?.strip_suffix('}'))
            .map(|name| name.trim_start_matches('*'))
            .filter(|name| !name.is_empty())
    }
}

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.method, self.path)
    }
}

// One segment as the template spells it: a parameter under any framework's
// spelling as `{name}`, a wildcard as `{*name}`, a literal as written;
// `None` for a segment outside the grammar.
fn parameter(segment: &str) -> Option<String> {
    if let Some(rest) = segment.strip_prefix(':') {
        return param_name_whole(rest).map(|name| format!("{{{name}}}"));
    }
    if let Some(inner) = segment.strip_prefix('<').and_then(|rest| rest.strip_suffix('>')) {
        let last = inner.rsplit(':').next().unwrap_or(inner);
        return param_name_whole(last).map(|name| format!("{{{name}}}"));
    }
    if let Some(inner) = segment.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
        let (star, inner) = inner.strip_prefix('*').map_or(("", inner), |rest| ("*", rest));
        let first = inner.split(':').next().unwrap_or(inner);
        return param_name_whole(first).map(|name| format!("{{{star}{name}}}"));
    }
    if let Some(rest) = segment.strip_prefix('*') {
        let name = if rest.is_empty() {
            "rest".to_owned()
        } else {
            param_name_whole(rest)?
        };
        return Some(format!("{{*{name}}}"));
    }
    segment
        .chars()
        .all(|ch| is_name_char(ch) || matches!(ch, '-' | '.' | '~' | '%' | '@'))
        .then(|| segment.to_owned())
}

const fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn param_name_whole(text: &str) -> Option<String> {
    let name: String = text.chars().take_while(|ch| is_name_char(*ch)).collect();
    let rest = text.get(name.len()..)?;
    rest.is_empty().then_some(name).filter(|name| !name.is_empty())
}

/// The rule a route breaks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BadRoute {
    /// The text is not two words, `METHOD /path`.
    Grammar,
    /// The method is outside [`METHODS`].
    Method(String),
    /// The path does not lead with `/`, or a segment is outside the grammar.
    Path(String),
}

impl Display for BadRoute {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grammar => f.write_str("is not `METHOD /path`"),
            Self::Method(method) => {
                write!(f, "names the method `{method}`, which is none of {}", METHODS.join(", "))
            }
            Self::Path(path) => write!(
                f,
                "names the path `{path}`, which does not lead with `/` or holds a segment that is \
                 neither a literal nor a parameter"
            ),
        }
    }
}

impl std::error::Error for BadRoute {}

#[cfg(test)]
mod tests {
    use super::Route;

    #[test]
    fn parameter_suffix_refused() {
        for text in [
            "GET /items/:id.json",
            "GET /users/{user-id}",
            "GET /orders/:id-pay",
        ] {
            assert!(Route::parse(text).is_err(), "{text}");
        }
    }

    #[test]
    fn distinct_parameter_routes() {
        let id = Route::parse("GET /items/:id").expect("parameter");
        let id_json = Route::parse("GET /items/id.json").expect("literal");
        assert_ne!(id.path, id_json.path);
    }
}
