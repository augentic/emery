//! Persists content-addressed revisions and tracks the current revision.
//!
//! [`commit`] stores both documents, atomically updates the current revision
//! identifier, and removes the displaced revision. [`current`] verifies and
//! returns the stored revision. Content is checked against its identifier when
//! read, allowing corruption to be detected.

use anyhow::Context;
use omnia_sdk::{BlobStore, Error, StateStore, server_error};

use crate::revision::{Design, Diff, Document as _, Revision, Spec};

/// The state-store key containing the current revision identifier.
pub const REVISION_KEY: &str = "current-revision";

/// The blob container containing revision documents under `<id>/`.
pub const CONTAINER: &str = "revisions";

/// Commits `revision` as the current revision.
///
/// Both documents are written, the current id is swapped by compare-and-swap,
/// and the revision it displaced is pruned. Returns the new content id and,
/// when the outgoing revision was readable, the [`Diff`] against it.
///
/// # Errors
///
/// Returns [`Error::ServerError`] when serialisation or storage fails,
/// including when another writer updates the current identifier first.
pub async fn commit<S: StateStore + BlobStore>(
    store: &S, revision: &Revision,
) -> Result<(String, Option<Diff>), Error> {
    // observe once for the diff and the CAS
    let observed = observe(store).await;
    let diff = observed
        .id()
        .zip(observed.revision.as_ref())
        .map(|(from, outgoing)| Diff::between(from, outgoing, revision));
    let id = swap(store, revision, observed).await?;

    Ok((id, diff))
}

// A lost swap leaves the written documents as an inert, unreferenced orphan.
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

    StateStore::cas(store, REVISION_KEY, observed.token.as_deref(), id.as_bytes())
        .await
        .context("swapping current revision")?;
    tracing::debug!(%id, outgoing = ?observed.id(), "revision committed");

    // prune the outgoing revision
    if let Some(outgoing) = observed.id().filter(|outgoing| *outgoing != id) {
        for name in [Spec::NAME, Design::NAME] {
            let _ = BlobStore::delete(store, CONTAINER, &key(outgoing, name)).await;
        }
    }

    Ok(id)
}

fn key(id: &str, name: &str) -> String {
    format!("{id}/{name}.json")
}

/// Returns the current revision and its id, or `None` before the first commit.
///
/// # Errors
///
/// - Returns [`Error::BadRequest`] with code `spec-outdated` when the stored
///   revision uses a different grammar.
/// - Returns [`Error::ServerError`] when storage fails, the current revision
///   is incomplete, or its content no longer matches its identifier.
pub async fn current<S: StateStore + BlobStore>(
    store: &S,
) -> Result<Option<(String, Revision)>, Error> {
    let Some(raw) =
        StateStore::get(store, REVISION_KEY).await.context("getting current revision id")?
    else {
        return Ok(None);
    };

    let id = String::from_utf8(raw).context("decoding current revision id")?;
    let revision = load(store, &id).await?;

    Ok(Some((id, revision)))
}

// Bad state suppresses only the advisory diff; the CAS still refuses a stale
// token. Each suppressed failure is logged, since the CAS that follows can
// only report it as a conflict.
async fn observe<S: StateStore + BlobStore>(store: &S) -> Observation {
    let nothing = Observation {
        token: None,
        revision: None,
    };

    // the current id, or nothing when there is none or it cannot be read
    let token = match StateStore::get(store, REVISION_KEY).await {
        Ok(Some(token)) => token,
        Ok(None) => return nothing,
        Err(error) => {
            tracing::warn!(%error, "revision id is unreadable");
            return nothing;
        }
    };

    // the revision it names, or nothing when it cannot be read
    let Ok(id) = str::from_utf8(&token) else {
        return Observation {
            token: Some(token),
            revision: None,
        };
    };

    let revision = match load(store, id).await {
        Ok(revision) => Some(revision),
        Err(error) => {
            tracing::warn!(%id, %error, "revision is unreadable");
            None
        }
    };

    Observation {
        token: Some(token),
        revision,
    }
}

async fn load<S: BlobStore>(store: &S, id: &str) -> Result<Revision, Error> {
    let spec = read(store, id, Spec::NAME).await?;
    let design = read(store, id, Design::NAME).await?;
    Revision::read(id, &spec, &design)
}

async fn read<S: BlobStore>(store: &S, id: &str, name: &str) -> Result<Vec<u8>, Error> {
    BlobStore::get(store, CONTAINER, &key(id, name))
        .await
        .context("reading revision document")?
        .ok_or_else(|| server_error!("revision `{id}` does not contain `{name}`"))
}

#[derive(Debug)]
struct Observation {
    // Absent when storage could not be read too, so the CAS fails closed
    // against a present key.
    token: Option<Vec<u8>>,
    revision: Option<Revision>,
}

impl Observation {
    fn id(&self) -> Option<&str> {
        self.token.as_deref().and_then(|token| str::from_utf8(token).ok())
    }
}

// Two runs racing one current id cannot be arranged through the CLI, whose
// `commit` observes and swaps as one.
#[cfg(test)]
mod tests {
    use omnia_test::guest::Memory;

    use super::*;
    use crate::revision::EMERY;

    #[tokio::test]
    async fn commit_conflict() {
        let memory = Memory::default();

        // both runs observe the empty store, the winner swaps first
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
