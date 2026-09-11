//! The revision store
//!
//! Where committed revisions live. A revision is the specification and design
//! one `specify` run produced, committed as canonical JSON under the id of its
//! content; the store commits a new revision, reads the current one, and
//! reports how it differs from the one it replaced.
//!
//! A revision is identified by the digest of its content, never a sequence
//! number, so the same revision always has the same id and a document that no
//! longer matches its id is recognised as corruption. Only the current
//! revision is kept, which keeps the store small and its meaning simple.

use anyhow::Context;
use omnia_guest::{BlobStore, Error, StateStore, server_error};
use serde::Serialize;
use strum::VariantArray as _;

use crate::artifact::{Design, Document, ReqId, Requirement, Revision, SectionKind, Spec, digest};

/// Keyvalue key holding the current revision id.
pub const CURRENT: &str = "current-revision";

/// Blobstore container holding every revision's documents under `<id>/`.
pub const CONTAINER: &str = "revisions";

/// Commits `revision` to `store` — diff against the readable outgoing
/// revision, write, swap the current id, prune — returning the content id
/// and the advisory re-mine diff.
///
/// # Errors
///
/// Fails if another run swapped the id first or storage refuses the write.
pub async fn commit<S: StateStore + BlobStore>(
    store: &S, revision: &Revision,
) -> Result<(String, Option<Diff>), Error> {
    // One observation feeds both the advisory diff and the CAS.
    let observed = observe(store).await;
    let diff = observed.outgoing.as_ref().map(|outgoing| Diff::between(outgoing, revision));
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

    let id = revision.id();
    for (document, body) in revision.files() {
        BlobStore::put(store, CONTAINER, &key(&id, document), body.as_bytes())
            .await
            .context("writing revision document")?;
    }

    StateStore::cas(store, CURRENT, observed.token.as_deref(), id.as_bytes())
        .await
        .context("swapping current revision")?;

    // The swap landed; prune the outgoing revision.
    if let Some(outgoing) = observed.outgoing_id().filter(|outgoing| *outgoing != id) {
        for document in Document::VARIANTS {
            let _ = BlobStore::delete(store, CONTAINER, &key(outgoing, *document)).await;
        }
    }

    Ok(id)
}

// The blob name a revision's document is stored under.
fn key(id: &str, document: Document) -> String {
    format!("{id}/{}", document.file())
}

/// Returns the current revision in `store`, or `None` before the first commit.
///
/// # Errors
///
/// Fails closed for a dangling, incomplete, unreadable, or tampered revision.
pub async fn current<S: StateStore + BlobStore>(store: &S) -> Result<Option<Revision>, Error> {
    let Some(raw) = StateStore::get(store, CURRENT).await.context("getting current revision id")?
    else {
        return Ok(None);
    };

    let id = String::from_utf8(raw).context("decoding current revision id")?;
    let revision = load(store, &id).await?;

    Ok(Some(revision))
}

// Observes the CAS token and outgoing revision without failing; bad state
// suppresses only the advisory diff, never the CAS, which still refuses a
// stale token.
async fn observe<S: StateStore + BlobStore>(store: &S) -> Observation {
    let mut observed = Observation {
        token: StateStore::get(store, CURRENT).await.ok().flatten(),
        outgoing: None,
    };
    observed.outgoing = match observed.outgoing_id() {
        Some(id) => load(store, id).await.ok(),
        None => None,
    };
    observed
}

// Loads revision `id` and checks that its bytes still hash to that id: the
// store is content-addressed, so documents that no longer match the id they
// sit under are corruption, not a revision.
async fn load<S: BlobStore>(store: &S, id: &str) -> Result<Revision, Error> {
    let spec = read(store, id, Document::Spec).await?;
    let design = read(store, id, Document::Design).await?;

    let files =
        [(Document::Spec.file(), spec.as_slice()), (Document::Design.file(), design.as_slice())];
    if digest(files.into_iter()) != id {
        return Err(server_error!("revision `{id}` does not match its content"));
    }

    // The bytes are the ones committed: a revision under another grammar is
    // outdated, and one this grammar cannot read was not written by this
    // engine.
    let spec = serde_json::from_slice(&spec)
        .with_context(|| format!("revision `{id}`: `{}` is not JSON", Document::Spec.file()))?;
    let design = serde_json::from_slice(&design)
        .with_context(|| format!("revision `{id}`: `{}` is not JSON", Document::Design.file()))?;

    Revision::read(spec, design)
}

// Reads one document of revision `id`; a document absent under a named
// revision is corruption.
async fn read<S: BlobStore>(store: &S, id: &str, document: Document) -> Result<Vec<u8>, Error> {
    BlobStore::get(store, CONTAINER, &key(id, document))
        .await
        .context("reading revision document")?
        .ok_or_else(|| server_error!("revision `{id}` does not contain `{}`", document.file()))
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
    // Reads the outgoing revision's id from the token; a non-UTF-8 token
    // names no blobs.
    fn outgoing_id(&self) -> Option<&str> {
        self.token.as_deref().and_then(|raw| str::from_utf8(raw).ok())
    }
}

/// An ephemeral re-mine diff against the outgoing revision.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Diff {
    /// The outgoing revision id this run superseded.
    pub from: String,
    /// The requirements that changed.
    pub spec: SpecDiff,
    /// The sections that changed.
    pub design: DesignDiff,
}

impl Diff {
    // Diffs `incoming` against `outgoing` by typed equality: requirements by
    // id, sections by kind, never by position.
    fn between(outgoing: &Revision, incoming: &Revision) -> Self {
        Self {
            from: outgoing.id(),
            spec: SpecDiff::between(&outgoing.spec, &incoming.spec),
            design: DesignDiff::between(&outgoing.design, &incoming.design),
        }
    }
}

/// The requirements that differ between two revisions.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecDiff {
    /// Requirements present only in the incoming revision.
    pub added: Vec<Entry>,
    /// Requirements present only in the outgoing revision.
    pub removed: Vec<Entry>,
    /// Requirements present in both whose content changed.
    pub changed: Vec<Changed>,
}

impl SpecDiff {
    // Matches requirements by id — the position each run numbers in source
    // order — so a requirement whose place moved reads as a change.
    fn between(outgoing: &Spec, incoming: &Spec) -> Self {
        let mut diff = Self::default();
        for requirement in &incoming.requirements {
            match outgoing.requirement(requirement.id) {
                None => diff.added.push(Entry::from(requirement)),
                Some(before) => {
                    let fields = before.differences(requirement);
                    if !fields.is_empty() {
                        diff.changed.push(Changed {
                            requirement: Entry::from(requirement),
                            fields,
                        });
                    }
                }
            }
        }
        diff.removed.extend(
            outgoing
                .requirements
                .iter()
                .filter(|requirement| incoming.requirement(requirement.id).is_none())
                .map(Entry::from),
        );
        diff
    }
}

/// One requirement named by a diff.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Entry {
    /// The requirement id.
    pub id: ReqId,
    /// The requirement subject.
    pub subject: String,
}

impl From<&Requirement> for Entry {
    fn from(requirement: &Requirement) -> Self {
        Self {
            id: requirement.id,
            subject: requirement.subject.clone(),
        }
    }
}

/// One requirement whose content changed, and the fields that differ.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Changed {
    /// The requirement, named as the diff names every other.
    #[serde(flatten)]
    pub requirement: Entry,
    /// The differing fields, in declaration order.
    pub fields: Vec<&'static str>,
}

/// The design sections that differ between two revisions, by kind.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DesignDiff {
    /// Sections present only in the incoming revision.
    pub added: Vec<SectionKind>,
    /// Sections present only in the outgoing revision.
    pub removed: Vec<SectionKind>,
    /// Sections present in both whose blocks changed.
    pub changed: Vec<SectionKind>,
}

impl DesignDiff {
    fn between(outgoing: &Design, incoming: &Design) -> Self {
        let mut diff = Self::default();
        for section in &incoming.sections {
            match outgoing.section(section.kind) {
                None => diff.added.push(section.kind),
                Some(before) if before.blocks != section.blocks => diff.changed.push(section.kind),
                Some(_) => {}
            }
        }
        diff.removed.extend(
            outgoing
                .sections
                .iter()
                .filter(|section| incoming.section(section.kind).is_none())
                .map(|section| section.kind),
        );
        diff
    }
}

// Keep (entry-point-unreachable): two runs racing one current id cannot
// be arranged through the CLI, whose `commit` observes and swaps as one;
// everything else the store does is owned by the root scenarios.
#[cfg(test)]
mod tests {
    use omnia_test::guest::Memory;

    use super::*;
    use crate::artifact::EMERY;

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
        let committed = current(&memory).await.expect("current").expect("committed");
        assert_eq!(committed.id(), winner, "the current id still names the winner");
        let spec = memory.object(CONTAINER, &key(&winner, Document::Spec)).expect("winning spec");
        let (_, canonical) = winning.files().swap_remove(0);
        assert_eq!(spec, canonical.as_bytes(), "the winning revision is intact");
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
