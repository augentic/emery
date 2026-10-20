//! Persists content-addressed revisions: the staged candidate and the current revision.
//!
//! A revision stands in one of two slots: `stage` writes a candidate for
//! review, which `staged` reads back and `clear` removes; `commit` seals a
//! revision as the current one, atomically swapping the current identifier
//! and pruning the revision it displaced, which `current` reads. Content is
//! checked against its identifier when read, so corruption is detected
//! rather than served.

use std::fmt;

use anyhow::Context;
use omnia_sdk::{BlobStore, Error, StateStore, server_error};
use serde::Serialize;

use crate::revision::{Design, Diff, Document as _, Plan, Revision, Spec};

/// The state-store key containing the current revision identifier.
pub const REVISION_KEY: &str = "current-revision";

/// The blob container containing the current revision's documents under `<id>/`.
pub const CONTAINER: &str = "revisions";

/// The state-store key containing the staged candidate's identifier.
pub const STAGED_KEY: &str = "staged";

/// The blob container containing the staged candidate's documents under `<id>/`.
pub const STAGED_CONTAINER: &str = "staged";

// Where a revision stands: the key naming it, the container holding its
// documents, and how a refusal names it.
#[derive(Clone, Copy)]
struct Slot {
    key: &'static str,
    container: &'static str,
    noun: &'static str,
}

const CURRENT: Slot = Slot {
    key: REVISION_KEY,
    container: CONTAINER,
    noun: "revision",
};

const STAGED: Slot = Slot {
    key: STAGED_KEY,
    container: STAGED_CONTAINER,
    noun: "staged candidate",
};

/// What kind of candidate is staged for `emery apply` to seal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StagedKind {
    /// A whole revision, drafted from the sources alone.
    Revision,
}

impl fmt::Display for StagedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Revision => "revision",
        })
    }
}

// The candidate `stage` left for `apply`, read back whole.
pub struct Staged {
    pub id: String,
    pub kind: StagedKind,
    pub revision: Revision,
}

// What `commit` sealed: the new current id, the id it displaced, and the
// diff against it where the outgoing revision was readable.
pub struct Committed {
    pub id: String,
    pub parent: Option<String>,
    pub diff: Option<Diff>,
}

// Stages `revision` as the candidate for `emery apply`, replacing any staged
// before it: every document written under the candidate's id, the staged id
// swapped by compare-and-swap, and the candidate it displaced pruned.
pub async fn stage<S: StateStore + BlobStore>(
    store: &S, revision: &Revision,
) -> Result<String, Error> {
    Observation::of(store, STAGED).await.swap(store, revision).await
}

// The staged candidate's id alone, or `None` when nothing is staged.
pub async fn staged_id<S: StateStore>(store: &S) -> Result<Option<String>, Error> {
    STAGED.token(store).await
}

// The staged candidate whole, or `None` when nothing is staged. Fails as
// `current` does: closed on a candidate that no longer matches its id, and
// `spec-outdated` on one under another grammar.
pub async fn staged<S: StateStore + BlobStore>(store: &S) -> Result<Option<Staged>, Error> {
    let Some(id) = STAGED.token(store).await? else {
        return Ok(None);
    };
    let revision = STAGED.load(store, &id).await?;
    Ok(Some(Staged {
        id,
        kind: StagedKind::Revision,
        revision,
    }))
}

// Removes the staged candidate, documents and id; nothing staged is nothing to remove.
pub async fn clear<S: StateStore + BlobStore>(store: &S) -> Result<(), Error> {
    let Some(id) = STAGED.token(store).await? else {
        return Ok(());
    };
    StateStore::delete(store, STAGED.key).await.context("clearing the staged candidate")?;
    STAGED.prune(store, &id).await;
    Ok(())
}

// Commits `revision` as the current revision: every document written, the
// current id swapped by compare-and-swap, and the revision it displaced
// pruned. A storage failure, including another writer swapping first, is
// `server_error`.
pub async fn commit<S: StateStore + BlobStore>(
    store: &S, revision: &Revision,
) -> Result<Committed, Error> {
    // observe once for the diff and the CAS
    let observed = Observation::of(store, CURRENT).await;
    let parent = observed.id().map(ToOwned::to_owned);
    let diff = observed.diff(revision);
    let id = observed.swap(store, revision).await?;

    Ok(Committed { id, parent, diff })
}

// The current revision and its id, or `None` before the first commit: fails
// closed on a revision that no longer matches its id, and `spec-outdated` on
// one under another grammar.
pub async fn current<S: StateStore + BlobStore>(
    store: &S,
) -> Result<Option<(String, Revision)>, Error> {
    let Some(id) = CURRENT.token(store).await? else {
        return Ok(None);
    };
    let revision = CURRENT.load(store, &id).await?;

    Ok(Some((id, revision)))
}

// The current revision's diff against `incoming`, where the current
// revision is readable: the advisory baseline for a candidate's diff, which
// an unreadable or outdated revision suppresses rather than refuses.
pub async fn diff<S: StateStore + BlobStore>(store: &S, incoming: &Revision) -> Option<Diff> {
    Observation::of(store, CURRENT).await.diff(incoming)
}

// The `spec-not-generated` refusal of a verb that reads the current
// revision, naming the staged candidate where one stands and is not sealed.
pub async fn not_generated<S: StateStore>(store: &S) -> Error {
    let description = match STAGED.token(store).await {
        Ok(Some(candidate)) => format!(
            "no specification revision has been committed; candidate `{candidate}` is staged and \
             not sealed"
        ),
        Ok(None) => "no specification revision has been committed".to_owned(),
        Err(error) => {
            tracing::warn!(%error, "the staged id is unreadable");
            "no specification revision has been committed".to_owned()
        }
    };
    Error::NotFound {
        code: "spec-not-generated".into(),
        description,
    }
}

// The `nothing-staged` refusal of a verb that reads the staged candidate.
pub fn nothing_staged() -> Error {
    Error::NotFound {
        code: "nothing-staged".into(),
        description: "no candidate is staged".into(),
    }
}

fn key(id: &str, name: &str) -> String {
    format!("{id}/{name}.json")
}

impl Slot {
    async fn token<S: StateStore>(self, store: &S) -> Result<Option<String>, Error> {
        let Some(raw) = StateStore::get(store, self.key)
            .await
            .with_context(|| format!("getting the {} id", self.noun))?
        else {
            return Ok(None);
        };
        let id =
            String::from_utf8(raw).with_context(|| format!("decoding the {} id", self.noun))?;
        Ok(Some(id))
    }

    async fn load<S: BlobStore>(self, store: &S, id: &str) -> Result<Revision, Error> {
        let spec = self.read(store, id, Spec::NAME).await?;
        let design = self.read(store, id, Design::NAME).await?;
        let plan = self.read(store, id, Plan::NAME).await?;
        Revision::read(id, &spec, &design, &plan)
    }

    async fn read<S: BlobStore>(self, store: &S, id: &str, name: &str) -> Result<Vec<u8>, Error> {
        BlobStore::get(store, self.container, &key(id, name))
            .await
            .with_context(|| format!("reading {} document", self.noun))?
            .ok_or_else(|| server_error!("{} `{id}` does not contain `{name}`", self.noun))
    }

    // A document that will not delete stays as an inert orphan, so the failure
    // is logged rather than raised.
    async fn prune<S: BlobStore>(self, store: &S, id: &str) {
        for name in [Spec::NAME, Design::NAME, Plan::NAME] {
            if let Err(error) = BlobStore::delete(store, self.container, &key(id, name)).await {
                tracing::warn!(%id, name, %error, "a displaced document could not be pruned");
            }
        }
    }
}

#[derive(Debug)]
struct Observation {
    slot: Slot,
    // Absent when storage could not be read too, so the CAS fails closed
    // against a present key.
    token: Option<Vec<u8>>,
    revision: Option<Revision>,
}

impl fmt::Debug for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.noun)
    }
}

impl Observation {
    // Bad state suppresses only the advisory diff; the CAS still refuses a
    // stale token. Each suppressed failure is logged, since the CAS that
    // follows can only report it as a conflict.
    async fn of<S: StateStore + BlobStore>(store: &S, slot: Slot) -> Self {
        let nothing = Self {
            slot,
            token: None,
            revision: None,
        };

        // the slot's id, or nothing when there is none or it cannot be read
        let token = match StateStore::get(store, slot.key).await {
            Ok(Some(token)) => token,
            Ok(None) => return nothing,
            Err(error) => {
                tracing::warn!(%error, noun = slot.noun, "the id is unreadable");
                return nothing;
            }
        };

        // the revision it names, or nothing when it cannot be read
        let Ok(id) = str::from_utf8(&token) else {
            return Self {
                slot,
                token: Some(token),
                revision: None,
            };
        };

        let revision = match slot.load(store, id).await {
            Ok(revision) => Some(revision),
            Err(error) => {
                tracing::warn!(%id, %error, noun = slot.noun, "the revision is unreadable");
                None
            }
        };

        Self {
            slot,
            token: Some(token),
            revision,
        }
    }

    // A lost swap leaves the written documents as an inert, unreferenced
    // orphan.
    async fn swap<S: StateStore + BlobStore>(
        self, store: &S, revision: &Revision,
    ) -> Result<String, Error> {
        let slot = self.slot;
        if !BlobStore::container_exists(store, slot.container).await? {
            BlobStore::create_container(store, slot.container).await?;
        }

        let id = revision.id()?;
        for (name, body) in [
            (Spec::NAME, revision.spec.to_json()?),
            (Design::NAME, revision.design.to_json()?),
            (Plan::NAME, revision.plan.to_json()?),
        ] {
            BlobStore::put(store, slot.container, &key(&id, name), body.as_bytes())
                .await
                .with_context(|| format!("writing {} document", slot.noun))?;
        }

        StateStore::cas(store, slot.key, self.token.as_deref(), id.as_bytes())
            .await
            .with_context(|| format!("swapping the {} id", slot.noun))?;

        // prune the outgoing revision
        if let Some(outgoing) = self.id().filter(|outgoing| *outgoing != id) {
            slot.prune(store, outgoing).await;
        }

        Ok(id)
    }

    fn id(&self) -> Option<&str> {
        self.token.as_deref().and_then(|token| str::from_utf8(token).ok())
    }

    fn diff(&self, incoming: &Revision) -> Option<Diff> {
        self.id()
            .zip(self.revision.as_ref())
            .map(|(from, outgoing)| Diff::between(from, outgoing, incoming))
    }
}

// Two runs racing one id cannot be arranged through the CLI, whose `commit`
// and `stage` each observe and swap as one.
#[cfg(test)]
mod tests {
    use omnia_test::guest::Memory;

    use super::*;
    use crate::revision::EMERY;

    #[tokio::test]
    async fn commit_conflict() {
        let memory = Memory::default();

        // both runs observe the empty store, the winner swaps first
        let stale = Observation::of(&memory, CURRENT).await;
        let observed = Observation::of(&memory, CURRENT).await;
        let winning = revision("winner");
        let winner = observed.swap(&memory, &winning).await.expect("commit");

        let err = stale
            .swap(&memory, &revision("loser"))
            .await
            .expect_err("a stale observation must never last-write-wins over the swapped id");
        assert_eq!(err.code(), "server_error", "typed failure");
        assert!(
            err.description().contains("swapping the revision id"),
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
                parent: None,
                preamble: vec![preamble.to_string()],
                requirements: vec![],
            },
            design: Design {
                emery: EMERY,
                parent: None,
                preamble: vec![],
                sections: vec![],
            },
            plan: Plan {
                emery: EMERY,
                parent: None,
                preamble: vec![],
                slices: vec![],
            },
        }
    }
}
