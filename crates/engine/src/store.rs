//! Stores the current revision and commits the next.
//!
//! A revision is committed as canonical JSON under the digest of its content,
//! so the same revision always has the same id and a document that no longer
//! matches its id is recognised as corruption. Only the current revision is
//! kept: [`commit`] writes the new one, swaps the current id, and prunes the
//! one it displaced; [`current`] reads it back.

use anyhow::Context;
use omnia_sdk::{BlobStore, Error, StateStore, server_error};

use crate::revision::{Design, Diff, Document as _, Revision, Spec};

/// The key-value key holding the current revision id.
pub const CURRENT: &str = "current-revision";

/// The blob container holding each revision's documents under `<id>/`.
pub const CONTAINER: &str = "revisions";

/// Commits `revision` as the current revision.
///
/// Both documents are written, the current id is swapped by compare-and-swap,
/// and the revision it displaced is pruned. Returns the new content id and,
/// when the outgoing revision was readable, the [`Diff`] against it.
///
/// # Errors
///
/// Returns [`Error::ServerError`] when another run swapped the id first or
/// storage refuses a write.
pub async fn commit<S: StateStore + BlobStore>(
    store: &S, revision: &Revision,
) -> Result<(String, Option<Diff>), Error> {
    // One observation feeds both the advisory diff and the CAS.
    let observed = observe(store).await;
    let diff = observed
        .outgoing_id()
        .zip(observed.outgoing.as_ref())
        .map(|(id, outgoing)| Diff::between(id, outgoing, revision));
    let id = swap(store, revision, observed).await?;

    Ok((id, diff))
}

// Writes the documents and swaps the current id against `observed`;
// a lost swap leaves the documents as an inert, unreferenced orphan.
async fn swap<S: StateStore + BlobStore>(
    store: &S, revision: &Revision, observed: Observation,
) -> Result<String, Error> {
    if !BlobStore::container_exists(store, CONTAINER).await? {
        BlobStore::create_container(store, CONTAINER).await?;
    }

    let id = revision.id()?;
    for (name, body) in
        [(Spec::NAME, revision.spec.to_json()?), (Design::NAME, revision.design.to_json()?)]
    {
        BlobStore::put(store, CONTAINER, &key(&id, name), body.as_bytes())
            .await
            .context("writing revision document")?;
    }

    StateStore::cas(store, CURRENT, observed.token.as_deref(), id.as_bytes())
        .await
        .context("swapping current revision")?;

    // The swap landed; prune the outgoing revision.
    if let Some(outgoing) = observed.outgoing_id().filter(|outgoing| *outgoing != id) {
        for name in [Spec::NAME, Design::NAME] {
            let _ = BlobStore::delete(store, CONTAINER, &key(outgoing, name)).await;
        }
    }

    Ok(id)
}

// The blob name a revision's document is stored under.
fn key(id: &str, name: &str) -> String {
    format!("{id}/{name}.json")
}

/// Returns the current revision and its id, or `None` before the first commit.
///
/// # Errors
///
/// Returns [`Error::ServerError`] for a current id that names no complete
/// revision, or a revision whose bytes no longer match its id, and
/// [`Error::BadRequest`] with code `spec-outdated` for a revision written
/// under another grammar.
pub async fn current<S: StateStore + BlobStore>(
    store: &S,
) -> Result<Option<(String, Revision)>, Error> {
    let Some(raw) = StateStore::get(store, CURRENT).await.context("getting current revision id")?
    else {
        return Ok(None);
    };

    let id = String::from_utf8(raw).context("decoding current revision id")?;
    let revision = load(store, &id).await?;

    Ok(Some((id, revision)))
}

// Observes the CAS token and outgoing revision without failing; bad state
// suppresses only the advisory diff, never the CAS, which still refuses a
// stale token.
async fn observe<S: StateStore + BlobStore>(store: &S) -> Observation {
    let token = StateStore::get(store, CURRENT).await.ok().flatten();
    let outgoing = match token.as_deref().and_then(id_of) {
        Some(id) => load(store, id).await.ok(),
        None => None,
    };
    Observation { token, outgoing }
}

// Loads revision `id` from its two documents; the revision itself refuses
// bytes that no longer hash to the id, then another grammar's, then a shape
// this engine did not write.
async fn load<S: BlobStore>(store: &S, id: &str) -> Result<Revision, Error> {
    let spec = read(store, id, Spec::NAME).await?;
    let design = read(store, id, Design::NAME).await?;
    Revision::read(id, &spec, &design)
}

// Reads one document of revision `id`; a document absent under a named
// revision is corruption.
async fn read<S: BlobStore>(store: &S, id: &str, name: &str) -> Result<Vec<u8>, Error> {
    BlobStore::get(store, CONTAINER, &key(id, name))
        .await
        .context("reading revision document")?
        .ok_or_else(|| server_error!("revision `{id}` does not contain `{name}`"))
}

// Reads the revision id a CAS token holds; a non-UTF-8 token names none.
fn id_of(token: &[u8]) -> Option<&str> {
    str::from_utf8(token).ok()
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
    outgoing: Option<Revision>,
}

impl Observation {
    // The outgoing revision's id, the blobs a landed swap prunes.
    fn outgoing_id(&self) -> Option<&str> {
        self.token.as_deref().and_then(id_of)
    }
}

// Keep (entry-point-unreachable): two runs racing one current id cannot
// be arranged through the CLI, whose `commit` observes and swaps as one;
// everything else the store does is owned by the root scenarios.
#[cfg(test)]
mod tests {
    use omnia_test::guest::Memory;

    use super::*;
    use crate::revision::EMERY;

    #[tokio::test]
    async fn commit_conflict() {
        let memory = Memory::default();

        // Both runs observe the empty store; the winner swaps first.
        let stale = observe(&memory).await;
        let observed = observe(&memory).await;
        let winning = revision("winner");
        let winner = swap(&memory, &winning, observed).await.expect("commit");

        let err = swap(&memory, &revision("loser"), stale)
            .await
            .expect_err("a stale observation must never last-write-wins over the swapped id");
        assert_eq!(err.code(), "server_error", "typed failure");
        assert!(
            err.description().contains("swapping current revision"),
            "typed failure: {}",
            err.description()
        );
        let (current, _) = current(&memory).await.expect("current").expect("committed");
        assert_eq!(current, winner, "the current id still names the winner");
        let spec = memory.object(CONTAINER, &key(&winner, Spec::NAME)).expect("winning spec");
        let json = winning.spec.to_json().expect("serialises");
        assert_eq!(spec, json.as_bytes(), "the winning revision is intact");
    }

    fn revision(preamble: &str) -> Revision {
        Revision {
            spec: Spec {
                emery: EMERY,
                preamble: vec![preamble.to_string()],
                requirements: vec![],
            },
            design: Design {
                emery: EMERY,
                preamble: vec![],
                sections: vec![],
            },
        }
    }
}
