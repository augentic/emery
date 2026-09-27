//! The shape of one source's evidence for its `extracted` line: how many
//! claims of each kind, and the id stems its requirements fall under, so a
//! run's log shows what a source yielded without the document.

use std::collections::BTreeMap;

use emery_adapter::source::{ClaimKind, Evidence};

// `requirement=41 type=12`, kinds in taxonomy order
pub fn kinds(evidence: &Evidence) -> String {
    let mut counts: BTreeMap<ClaimKind, usize> = BTreeMap::new();
    for claim in &evidence.claims {
        *counts.entry(claim.kind).or_default() += 1;
    }
    counts.iter().map(|(kind, count)| format!("{kind}={count}")).collect::<Vec<_>>().join(" ")
}

// `start=41 orders=35`: the stem of each requirement id and how many share
// it, largest first
pub fn stems(evidence: &Evidence) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for claim in &evidence.claims {
        if claim.kind != ClaimKind::Requirement {
            continue;
        }
        let Some(id) = claim.id.as_deref() else {
            continue;
        };
        *counts.entry(stem(id)).or_default() += 1;
    }

    let mut stems: Vec<(&str, usize)> = counts.into_iter().collect();
    stems.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    stems.iter().map(|(stem, count)| format!("{stem}={count}")).collect::<Vec<_>>().join(" ")
}

// The first segment of a dotted id: the domain noun the seam led it with.
pub fn stem(id: &str) -> &str {
    id.split_once('.').map_or(id, |(stem, _)| stem)
}

#[cfg(test)]
mod tests {
    use emery_adapter::source::Evidence;

    use super::{kinds, stems};

    fn evidence() -> Evidence {
        serde_json::from_str(
            r#"{ "claims": [
                { "kind": "requirement", "id": "start.listen", "statement": "a" },
                { "kind": "requirement", "id": "start.route", "statement": "b" },
                { "kind": "requirement", "id": "orders.create", "statement": "c" },
                { "kind": "type", "id": "orders.order", "path": "src/orders.ts" },
                { "kind": "type", "path": "src/orders.ts" },
                { "kind": "call", "path": "src/orders.ts#L9" }
            ] }"#,
        )
        .expect("well-formed evidence")
    }

    #[test]
    fn summarised() {
        let evidence = evidence();
        assert_eq!(kinds(&evidence), "requirement=3 type=2 call=1");
        assert_eq!(stems(&evidence), "start=2 orders=1");
        assert_eq!(kinds(&Evidence { claims: vec![] }), "");
        assert_eq!(stems(&Evidence { claims: vec![] }), "");
    }
}
