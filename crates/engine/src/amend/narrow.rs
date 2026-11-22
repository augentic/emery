//! Narrows whole-scope behaviour extracts to claims from changed files.

use emery_adapter::source::{Claim, ClaimKind};

use crate::revision;
use crate::specify::{Extract, FileDelta, Snapshot};

pub fn extracts(extracts: Vec<Extract>, base_snapshot: Option<&Snapshot>) -> Vec<Extract> {
    extracts.into_iter().map(|extract| narrow_one(extract, base_snapshot)).collect()
}

fn narrow_one(mut extract: Extract, base_snapshot: Option<&Snapshot>) -> Extract {
    if !extract.is_whole_behaviour() {
        return extract;
    }
    let Some(current_files) = extract.files.as_ref() else {
        tracing::info!(
            source = %extract.source,
            "inline source has no workspace files; keeping all extracted claims"
        );
        return extract;
    };
    let Some(delta) =
        base_snapshot.and_then(|snapshot| snapshot.file_delta(&extract.source, current_files))
    else {
        tracing::info!(
            source = %extract.source,
            "no base source snapshot; keeping all extracted claims"
        );
        return extract;
    };

    let keep = kept_claims(&extract.evidence.claims, &delta);
    let total = extract.evidence.claims.len();
    let claims: Vec<Claim> = std::mem::take(&mut extract.evidence.claims)
        .into_iter()
        .zip(keep)
        .filter_map(|(claim, kept)| kept.then_some(claim))
        .collect();
    let kept = claims.len();
    let dropped = total - kept;
    tracing::info!(
        source = %extract.source,
        changed = delta.changed_count(),
        gone = delta.gone_count(),
        kept,
        dropped,
        "narrowed"
    );
    extract.narrowed(claims, delta)
}

fn kept_claims(claims: &[Claim], delta: &FileDelta) -> Vec<bool> {
    // contracts, types, unanchored claims, and claims in changed files
    let mut keep: Vec<bool> = claims
        .iter()
        .map(|claim| {
            matches!(claim.kind, ClaimKind::Contract | ClaimKind::Type)
                || claim.path.as_deref().is_none_or(|path| delta.is_changed(path))
        })
        .collect();

    // close requirement <-> criterion links under `revision::leads`
    loop {
        let mut progressed = false;
        for (index, claim) in claims.iter().enumerate() {
            if !keep[index] && links_to_kept(claim, claims, &keep) {
                keep[index] = true;
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }

    keep
}

fn links_to_kept(claim: &Claim, claims: &[Claim], keep: &[bool]) -> bool {
    let Some(id) = claim.id.as_deref() else { return false };
    match claim.kind {
        ClaimKind::Requirement => claims.iter().zip(keep).any(|(other, &kept)| {
            kept && other.kind == ClaimKind::Criterion
                && other.id.as_deref().is_some_and(|other_id| revision::leads(id, other_id))
        }),
        ClaimKind::Criterion => claims.iter().zip(keep).any(|(other, &kept)| {
            kept && other.kind == ClaimKind::Requirement
                && other.id.as_deref().is_some_and(|other_id| revision::leads(other_id, id))
        }),
        _ => false,
    }
}
