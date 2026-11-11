//! Default [`Task::Scope`](super::Task::Scope) rules for target adapters.

use emery_adapter::target::Change;

use crate::unique;

const OVERVIEW: &str = "overview";

/// Returns the conservative part scope for `change`.
///
/// When any design section other than `overview` changed, returns every part
/// in [`Change::parts`]. Otherwise returns the parts whose
/// [`PartReach::reasons`](emery_adapter::target::PartReach::reasons) are
/// non-empty.
///
/// # Examples
///
/// ```
/// use emery_sdk::target::{Change, PartReach, SectionChange, scope};
///
/// let mut change = Change {
///     parts: vec![
///         PartReach {
///             id: "PART-001".into(),
///             name: "auth".into(),
///             requirements: vec!["REQ-001".into()],
///             reasons: Vec::new(),
///         },
///         PartReach {
///             id: "PART-002".into(),
///             name: "orders".into(),
///             requirements: vec!["REQ-002".into()],
///             reasons: vec!["requirements".into()],
///         },
///     ],
///     ..Change::default()
/// };
/// assert_eq!(scope::conservative(&change), ["PART-002"]);
///
/// change.sections.push(SectionChange {
///     kind: "apis".into(),
///     before: Some("Before.".into()),
///     after: Some("After.".into()),
/// });
/// assert_eq!(scope::conservative(&change), ["PART-001", "PART-002"]);
/// ```
#[must_use]
pub fn conservative(change: &Change) -> Vec<String> {
    let widens = change
        .sections
        .iter()
        .any(|section| section.kind != OVERVIEW && section.before != section.after);
    unique(
        change
            .parts
            .iter()
            .filter(|part| widens || !part.reasons.is_empty())
            .map(|part| part.id.clone()),
    )
}
