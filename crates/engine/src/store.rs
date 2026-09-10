//! The revision store
//!
//! Where committed dossiers live. A revision is a dossier — the `spec.md` and
//! `design.md` one `specify` run produced — committed under the id of its
//! content; the store commits a new revision, reads the current one, and
//! reports how it differs from the one it replaced.
//!
//! A revision is identified by the digest of its content, never a sequence
//! number, so the same documents always have the same id and a document that
//! no longer matches its id is recognised as corruption. Only the current
//! revision is kept, which keeps the store small and its meaning simple.

use std::collections::BTreeMap;
use std::fmt::Display;

use anyhow::Context;
use omnia_guest::{BlobStore, Error, StateStore, server_error};
use serde::Serialize;

use crate::artifact::{Design, Document, Dossier, Spec};

/// Keyvalue key holding the current revision id.
pub const CURRENT: &str = "current-revision";

/// Blobstore container holding every revision's documents under `<id>/`.
pub const CONTAINER: &str = "revisions";

/// Commits `dossier` to `store` as a new revision — diff against the readable
/// predecessor, write, swap the current id, prune — returning the id with the
/// diff.
///
/// # Errors
///
/// Fails if another run swapped the id first or storage refuses the write.
pub async fn commit<S: StateStore + BlobStore>(
    store: &S, dossier: &Dossier,
) -> Result<Committed, Error> {
    // One observation feeds both the advisory diff and the CAS.
    let observed = observe(store).await;
    let diff = observed.outgoing.as_ref().map(|outgoing| Diff::between(outgoing, dossier));
    let id = swap(store, dossier, observed).await?;

    Ok(Committed { id, diff })
}

// Writes the documents and swaps the current id against `observed`;
// a lost swap leaves the documents as an inert, unreferenced orphan.
async fn swap<S: StateStore + BlobStore>(
    store: &S, dossier: &Dossier, observed: Observation,
) -> Result<String, Error> {
    if !BlobStore::container_exists(store, CONTAINER).await? {
        BlobStore::create_container(store, CONTAINER).await?;
    }

    let id = dossier.revision();
    for (name, body) in dossier.files() {
        BlobStore::put(store, CONTAINER, &format!("{id}/{name}"), body.as_bytes())
            .await
            .context("writing revision document")?;
    }

    StateStore::cas(store, CURRENT, observed.token.as_deref(), id.as_bytes())
        .await
        .context("swapping current revision")?;

    // The swap landed; prune the previous revision.
    if let Some(previous) = observed.previous().filter(|previous| *previous != id) {
        for (name, _) in dossier.files() {
            let _ = BlobStore::delete(store, CONTAINER, &format!("{previous}/{name}")).await;
        }
    }

    Ok(id)
}

/// Returns the current revision's dossier in `store`, or `None` before the
/// first commit.
///
/// # Errors
///
/// Fails closed for a dangling, incomplete, unreadable, or tampered revision.
pub async fn current<S: StateStore + BlobStore>(store: &S) -> Result<Option<Dossier>, Error> {
    let Some(raw) = StateStore::get(store, CURRENT).await.context("getting current revision id")?
    else {
        return Ok(None);
    };

    let id = String::from_utf8(raw).context("decoding current revision id")?;
    let dossier = load(store, &id).await?;

    Ok(Some(dossier))
}

// Observes the CAS token and outgoing revision without failing; bad state
// suppresses only the advisory diff, never the CAS, which still refuses a
// stale token.
async fn observe<S: StateStore + BlobStore>(store: &S) -> Observation {
    let token = StateStore::get(store, CURRENT).await.ok().flatten();

    let outgoing = if let Some(id) = token.as_deref().and_then(|raw| str::from_utf8(raw).ok()) {
        load(store, id).await.ok()
    } else {
        None
    };

    Observation { token, outgoing }
}

// Loads revision `id` and checks that it still hashes to that id: the
// store is content-addressed, so documents that no longer match the id
// they sit under are corruption, not a revision.
async fn load<S: BlobStore>(store: &S, id: &str) -> Result<Dossier, Error> {
    let spec = read(store, id, Document::Spec.file()).await?;
    let design = read(store, id, Document::Design.file()).await?;
    let dossier = Dossier { spec, design };

    if dossier.revision() != id {
        return Err(server_error!("revision `{id}` does not match its content"));
    }

    Ok(dossier)
}

// Reads one document of revision `id`; a document that is absent or not
// UTF-8 under a named revision is corruption.
async fn read<S: BlobStore>(store: &S, id: &str, name: &str) -> Result<String, Error> {
    let bytes = BlobStore::get(store, CONTAINER, &format!("{id}/{name}"))
        .await
        .context("reading revision document")?
        .ok_or_else(|| server_error!("revision `{id}` does not contain `{name}`"))?;
    let body = String::from_utf8(bytes)
        .with_context(|| format!("revision `{id}`: `{name}` is not UTF-8"))?;
    Ok(body)
}

/// A committed revision: its id and the advisory re-mine diff against
/// the revision it superseded, when one was readable.
#[derive(Debug)]
pub struct Committed {
    /// The committed revision id.
    pub id: String,
    /// Absent on the first commit and when the predecessor was unreadable.
    pub diff: Option<Diff>,
}

// The current revision observed before a compare-and-swap; one
// observation drives one CAS and its advisory diff.
#[derive(Debug)]
struct Observation {
    // The raw CAS token exactly as storage holds it. Absent before the
    // first commit; also absent when storage could not be read, so the
    // subsequent CAS fails closed against a present key.
    token: Option<Vec<u8>>,
    // Advisory diff input; absent when no complete revision is readable.
    outgoing: Option<Dossier>,
}

impl Observation {
    // Reads the predecessor's id from the token; a non-UTF-8 token names no
    // blobs.
    fn previous(&self) -> Option<&str> {
        self.token.as_deref().and_then(|raw| str::from_utf8(raw).ok())
    }
}

/// An ephemeral re-mine diff against the superseded revision.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diff {
    /// The outgoing revision id this run superseded.
    pub from: String,
    /// Changed file names in digest order.
    pub artifacts: Vec<String>,
    /// Requirement subjects that changed in `spec.md`.
    pub spec: Changes,
    /// Section headings that changed in `design.md`.
    pub design: Changes,
}

impl Diff {
    // Diffs `incoming` against `outgoing`: the changed files, then requirement
    // subjects and section headings, never positions. The diff is advisory: an
    // outgoing document that fails its grammar leaves its list empty.
    fn between(outgoing: &Dossier, incoming: &Dossier) -> Self {
        let artifacts = outgoing
            .files()
            .zip(incoming.files())
            .filter(|((_, old), (_, new))| old != new)
            .map(|((name, _), _)| name.to_string())
            .collect();

        let spec = match (outgoing.spec.parse::<Spec>(), incoming.spec.parse::<Spec>()) {
            (Ok(old), Ok(new)) => Changes::between(&old.by_subject(), &new.by_subject()),
            _ => Changes::default(),
        };
        let design = match (outgoing.design.parse::<Design>(), incoming.design.parse::<Design>()) {
            (Ok(old), Ok(new)) => Changes::between(&old.by_kind(), &new.by_kind()),
            _ => Changes::default(),
        };

        Self {
            from: outgoing.revision(),
            artifacts,
            spec,
            design,
        }
    }

    /// Returns whether the revisions are byte-identical; identical bytes
    /// cannot yield section differences.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.artifacts.is_empty()
    }
}

/// The sections of one document that differ between two revisions, keyed
/// by heading name.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Changes {
    /// Headings present only in the incoming document.
    pub added: Vec<String>,
    /// Headings present only in the outgoing document.
    pub removed: Vec<String>,
    /// Headings whose sections changed.
    pub changed: Vec<String>,
}

impl Changes {
    // Buckets the headings of `new` against `old` into added, changed, and
    // removed; a section that only moved is not a change.
    fn between<K: Display + Ord, S: PartialEq>(
        old: &BTreeMap<K, &S>, new: &BTreeMap<K, &S>,
    ) -> Self {
        let mut changes = Self::default();
        for (heading, section) in new {
            let bucket = match old.get(heading) {
                None => &mut changes.added,
                Some(previous) if *previous != *section => &mut changes.changed,
                Some(_) => continue,
            };
            bucket.push(heading.to_string());
        }
        changes.removed.extend(
            old.keys().filter(|heading| !new.contains_key(*heading)).map(ToString::to_string),
        );
        changes
    }
}

// Keep (entry-point-unreachable): two runs racing one current id cannot
// be arranged through the CLI, whose `commit` observes and swaps as one;
// everything else the store does is owned by the root scenarios.
#[cfg(test)]
mod tests {
    use omnia_test::guest::Memory;

    use super::*;

    #[tokio::test]
    async fn commit_conflict() {
        let memory = Memory::default();

        // Both runs observe the empty store; the winner swaps first.
        let stale = observe(&memory).await;
        let observed = observe(&memory).await;
        let winner = swap(&memory, &dossier("# Spec winner\n"), observed).await.expect("commit");

        let err = swap(&memory, &dossier("# Spec loser\n"), stale)
            .await
            .expect_err("a stale observation must never last-write-wins over the swapped id");
        assert_eq!(err.code(), "server_error", "typed failure");
        assert!(
            err.description().contains("swapping current revision"),
            "typed failure: {}",
            err.description()
        );
        let committed = current(&memory).await.expect("current").expect("committed");
        assert_eq!(committed.revision(), winner, "the current id still names the winner");
        let spec = memory.object(CONTAINER, &format!("{winner}/spec.md")).expect("winning spec");
        assert_eq!(spec, b"# Spec winner\n", "the winning revision is intact");
    }

    fn dossier(spec: &str) -> Dossier {
        Dossier {
            spec: spec.to_string(),
            design: "# Design\n\n## Overview\n\nOne endpoint.\n".to_string(),
        }
    }
}
