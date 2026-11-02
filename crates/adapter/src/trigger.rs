//! Defines what reaches a requirement: the surface the source registered it at.
//!
//! A [`Trigger`] is the one typed answer every side holds a surface to: the
//! `route` or `message` extra of a `contract` claim, the `Route:` or
//! `Message:` line of a requirement, and the typed trigger a slice carries to
//! a target.

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::{BadRoute, Route};

/// What reaches a requirement: the surface the source registered its behaviour at.
///
/// The variant is closed: a case is added when a recogniser derives it at a
/// registration and a target reads it, so a target matches it whole. Which
/// wire carries a message is the design's to say.
///
/// # Examples
///
/// ```
/// use emery_adapter::Trigger;
///
/// let route = Trigger::http("post /orders")?;
/// assert_eq!(route.to_string(), "POST /orders");
///
/// let message = Trigger::message(" orders.cancel-requested ")?;
/// assert_eq!(message, Trigger::Message("orders.cancel-requested".to_owned()));
/// assert!(Trigger::message("two words").is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    /// An HTTP route, `POST /orders/{id}/pay`.
    Http(Route),
    /// An inbound message by name: a broker topic, a socket event.
    Message(String),
}

impl Trigger {
    /// Parses `route` as an HTTP route, `METHOD /path`.
    ///
    /// # Errors
    ///
    /// Returns [`BadRoute`] describing the first rule the route breaks.
    pub fn http(route: &str) -> Result<Self, BadRoute> {
        Route::parse(route).map(Self::Http)
    }

    /// Reads `name` as a message: one word, trimmed, with no whitespace within.
    ///
    /// # Errors
    ///
    /// Returns [`BadMessage`] for an empty name or one holding whitespace.
    pub fn message(name: &str) -> Result<Self, BadMessage> {
        let name = name.trim();
        if name.is_empty() {
            return Err(BadMessage::Empty);
        }
        if name.chars().any(char::is_whitespace) {
            return Err(BadMessage::Spaced(name.to_owned()));
        }
        Ok(Self::Message(name.to_owned()))
    }
}

impl From<Route> for Trigger {
    fn from(route: Route) -> Self {
        Self::Http(route)
    }
}

impl Display for Trigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(route) => write!(f, "{route}"),
            Self::Message(name) => f.write_str(name),
        }
    }
}

/// The rule a message name breaks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BadMessage {
    /// The name is empty.
    Empty,
    /// The name holds whitespace.
    Spaced(String),
}

impl Display for BadMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("names no message"),
            Self::Spaced(name) => {
                write!(
                    f,
                    "names the message `{name}`, which holds whitespace; a message is one word"
                )
            }
        }
    }
}

impl std::error::Error for BadMessage {}

/// The rule a `contract` claim's trigger breaks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BadTrigger {
    /// The `route` extra is outside the route grammar.
    Route(BadRoute),
    /// The `message` extra is outside the message grammar.
    Message(BadMessage),
    /// The claim carries both a `route` and a `message`.
    Both,
}

impl From<BadRoute> for BadTrigger {
    fn from(bad: BadRoute) -> Self {
        Self::Route(bad)
    }
}

impl From<BadMessage> for BadTrigger {
    fn from(bad: BadMessage) -> Self {
        Self::Message(bad)
    }
}

impl Display for BadTrigger {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Route(bad) => write!(f, "{bad}"),
            Self::Message(bad) => write!(f, "{bad}"),
            Self::Both => {
                f.write_str("carries both `route` and `message`; a surface is reached through one")
            }
        }
    }
}

impl std::error::Error for BadTrigger {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Route(bad) => Some(bad),
            Self::Message(bad) => Some(bad),
            Self::Both => None,
        }
    }
}
