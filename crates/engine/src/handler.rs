//! Shared plumbing for command operations: [`RequestContext`],
//! [`Render`], and [`ReportBody`]. Transports stay out.

mod context;
mod locations;
mod output;
mod paths;

pub use context::RequestContext;
pub use locations::{GUEST_CACHE_MOUNT, Locations};
pub use output::{Render, ReportBody, ReportRow};
pub use paths::ExecutionPaths;

/// Result alias for operation bodies.
pub type Result<T, E = omnia_guest::Error> = std::result::Result<T, E>;

/// Map an I/O failure onto the `io` server-error discriminant.
pub(crate) fn io(err: impl std::fmt::Display) -> omnia_guest::Error {
    omnia_guest::Error::new(omnia_guest::ErrorKind::ServerError, "io", err.to_string())
}

/// Map a YAML (de)serialisation failure onto the `yaml` discriminant.
pub(crate) fn yaml(err: impl std::fmt::Display) -> omnia_guest::Error {
    omnia_guest::Error::new(omnia_guest::ErrorKind::ServerError, "yaml", err.to_string())
}

/// A typed server-error: kebab `code`, description `{code}: {detail}`.
pub(crate) fn diag(code: &'static str, detail: impl std::fmt::Display) -> omnia_guest::Error {
    omnia_guest::Error::new(omnia_guest::ErrorKind::ServerError, code, format!("{code}: {detail}"))
}

/// A typed bad-request: kebab `code`, empty `rule` omits the `{rule}: ` prefix.
pub(crate) fn validation(
    code: &'static str, rule: impl Into<String>, detail: impl Into<String>,
) -> omnia_guest::Error {
    let rule = rule.into();
    let detail = detail.into();
    let body = if rule.is_empty() { detail } else { format!("{rule}: {detail}") };
    omnia_guest::Error::new(omnia_guest::ErrorKind::BadRequest, code, format!("{code}: {body}"))
}
