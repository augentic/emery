//! Artifact types and parsers plus the shared atomic writer.
//!
//! A lifecycle-free leaf: a parser cannot transition engine state.

pub mod atomic;
pub mod evidence;
pub mod spec;

/// Map an I/O failure onto the `io` server-error discriminant.
pub(crate) fn io(err: impl std::fmt::Display) -> omnia_guest::Error {
    omnia_guest::Error::new(omnia_guest::ErrorKind::ServerError, "io", err.to_string())
}

/// Map a YAML (de)serialisation failure onto the `yaml` discriminant.
pub(crate) fn yaml(err: impl std::fmt::Display) -> omnia_guest::Error {
    omnia_guest::Error::new(omnia_guest::ErrorKind::ServerError, "yaml", err.to_string())
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
