//! Persists revisions: the staged candidate in the engine's storage, the current revision in the target repository.
//!
//! A candidate stands in the engine's own storage: `stage` writes it for
//! review, `staged` reads it back, and `clear` removes it. The current
//! revision lives in the repository the target names, under `spec/` at the
//! commit labelled `emery/<id>`: `current` reads it from the label the
//! `current-revision` pointer names, or from the head's tree, and `commit`
//! seals a revision there, labels it, and points the pointer at it. Content
//! is checked against its identifier when read, so a document rewritten
//! beneath the engine is refused rather than served.

use std::fmt;

use anyhow::Context;
use omnia_sdk::{BlobStore, Error, StateStore, Vcs, server_error};
use serde::{Deserialize, Serialize};

use crate::revision::{Design, Diff, Document as _, Plan, Revision, Spec};
use crate::vcs::{BuildRepo, REVISION, RevisionMessage, SPECIFICATION, WorkingCopy};
use crate::{EntryKind, Files, secrets};

/// The state-store key containing the current revision identifier.
///
/// A cache of the label the repository holds: the repository is the truth,
/// and a pointer the repository does not bear out is passed over.
pub const REVISION_KEY: &str = "current-revision";

/// The state-store key containing the staged candidate record.
pub const STAGED_KEY: &str = "staged";

/// The blob container containing the staged candidate's documents under `<id>/`.
pub const STAGED_CONTAINER: &str = "staged";

/// What kind of candidate is staged for `emery apply` to seal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StagedKind {
    /// A whole revision, drafted from the sources alone.
    Revision,
    /// A change against the current revision, prepared on a base.
    Amendment,
}

impl fmt::Display for StagedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Revision => "revision",
            Self::Amendment => "amendment",
        })
    }
}

// The staged record at `STAGED_KEY`.
#[derive(Serialize, Deserialize)]
struct Slot {
    id: String,
    kind: StagedKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    base: Option<StagedBase>,
}

// The revision an amendment was prepared against, at the commit it was read at.
#[derive(Serialize, Deserialize)]
struct StagedBase {
    revision: String,
    commit: String,
}

impl Slot {
    const fn revision(id: String) -> Self {
        Self {
            id,
            kind: StagedKind::Revision,
            base: None,
        }
    }

    // Fails closed on bytes that decode as no record.
    async fn read<S: StateStore>(store: &S) -> Result<Option<Self>, Error> {
        let Some(raw) =
            StateStore::get(store, STAGED_KEY).await.context("getting the staged record")?
        else {
            return Ok(None);
        };
        let slot = serde_json::from_slice(&raw).context("decoding the staged record")?;
        Ok(Some(slot))
    }
}

// The candidate `stage` left for `apply`, read back whole.
pub struct Staged {
    pub id: String,
    pub kind: StagedKind,
    pub revision: Revision,
}

// The revision a repository holds, and the commit it was read from. The
// documents may not read as a revision this engine knows, an outdated
// grammar: the id and the commit still stand, so a seal lands over them,
// while a verb that needs the revision raises the refusal.
pub struct Current {
    pub id: String,
    pub revision: Result<Revision, Error>,
    pub commit: String,
}

// Where a target repository stands for the revision: its head, the
// revision it holds, and the label the pointer names when that label does
// not stand over the head. Read once by `current`, and handed to `commit`
// so the seal lands over what was read.
pub struct Standing {
    pub head: Option<String>,
    pub current: Option<Current>,
    pub behind: Option<String>,
    // The pointer as read, for the compare-and-swap; absent when storage
    // could not be read too, so the swap fails closed against a present key.
    pointer: Option<Vec<u8>>,
}

impl Standing {
    // The current revision's diff against `incoming`, where the repository
    // holds one this engine reads.
    pub fn diff(&self, incoming: &Revision) -> Option<Diff> {
        let current = self.current.as_ref()?;
        let revision = current.revision.as_ref().ok()?;
        Some(Diff::between(&current.id, revision, incoming))
    }

    // The commit a seal lands over: the current revision's, else the head.
    pub fn over(&self) -> Option<&str> {
        self.current.as_ref().map(|current| current.commit.as_str()).or(self.head.as_deref())
    }
}

// What `commit` sealed: the revision's id, the id it displaced, the commit
// and label it stands at, and the diff against the displaced revision.
pub struct Sealed {
    pub id: String,
    pub parent: Option<String>,
    pub commit: String,
    pub label: String,
    pub diff: Option<Diff>,
}

// What the tree at a commit holds at `spec/`.
enum Occupancy {
    Absent,
    Held,
    Other(String),
}

impl Occupancy {
    const EXPECTED: [&str; 3] = ["design.json", "plan.json", "spec.json"];

    // What stands at `spec/` beneath `at`: nothing, exactly the three
    // documents as files, or something else, named.
    async fn of<F: Files>(files: &F, at: &str) -> Result<Self, Error> {
        let root = files.list(at, ".").await?;
        let Some(entry) = root.into_iter().find(|entry| entry.name == SPECIFICATION) else {
            return Ok(Self::Absent);
        };
        match entry.kind {
            EntryKind::Directory => {}
            EntryKind::File => return Ok(Self::Other("a file".to_owned())),
            EntryKind::Link => return Ok(Self::Other("a symbolic link".to_owned())),
        }
        let mut names: Vec<String> = files
            .list(at, SPECIFICATION)
            .await?
            .into_iter()
            .map(|entry| match entry.kind {
                EntryKind::File => entry.name,
                EntryKind::Directory => format!("{}/", entry.name),
                EntryKind::Link => format!("{} (a symbolic link)", entry.name),
            })
            .collect();
        names.sort_unstable();
        if names.iter().map(String::as_str).eq(Self::EXPECTED) {
            return Ok(Self::Held);
        }
        let listed = names.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ");
        Ok(Self::Other(if listed.is_empty() { "an empty directory".to_owned() } else { listed }))
    }
}

// Stages `revision` as the candidate for `emery apply`, replacing any staged
// before it: every document written under the candidate's id, the staged
// record swapped by compare-and-swap, and the candidate it displaced pruned.
pub async fn stage<S: StateStore + BlobStore>(
    store: &S, revision: &Revision,
) -> Result<String, Error> {
    let token = match StateStore::get(store, STAGED_KEY).await {
        Ok(token) => token,
        Err(error) => {
            tracing::warn!(%error, "the staged record is unreadable");
            None
        }
    };
    if !BlobStore::container_exists(store, STAGED_CONTAINER).await? {
        BlobStore::create_container(store, STAGED_CONTAINER).await?;
    }

    let documents = revision.documents()?;
    let id = Revision::identify(
        documents[0].1.as_bytes(),
        documents[1].1.as_bytes(),
        documents[2].1.as_bytes(),
    );
    for (name, body) in &documents {
        BlobStore::put(store, STAGED_CONTAINER, &key(&id, name), body.as_bytes())
            .await
            .context("writing staged candidate document")?;
    }
    let record =
        serde_json::to_vec(&Slot::revision(id.clone())).context("encoding the staged record")?;
    StateStore::cas(store, STAGED_KEY, token.as_deref(), &record)
        .await
        .context("swapping the staged record")?;

    // prune the candidate displaced; bytes that decode as no record name none
    let displaced = token
        .as_deref()
        .and_then(|raw| serde_json::from_slice::<Slot>(raw).ok())
        .map(|slot| slot.id)
        .filter(|outgoing| *outgoing != id);
    if let Some(outgoing) = displaced {
        prune(store, &outgoing).await;
    }
    Ok(id)
}

// The staged candidate's id alone, or `None` when nothing is staged.
pub async fn staged_id<S: StateStore>(store: &S) -> Result<Option<String>, Error> {
    Ok(Slot::read(store).await?.map(|slot| slot.id))
}

// The staged candidate whole, or `None` when nothing is staged: fails closed
// on a candidate that no longer matches its id, and `spec-outdated` on one
// under another grammar.
pub async fn staged<S: StateStore + BlobStore>(store: &S) -> Result<Option<Staged>, Error> {
    let Some(Slot { id, kind, .. }) = Slot::read(store).await? else {
        return Ok(None);
    };
    let mut bodies = Vec::with_capacity(Revision::DOCUMENTS.len());
    for name in Revision::DOCUMENTS {
        let body = BlobStore::get(store, STAGED_CONTAINER, &key(&id, name))
            .await
            .context("reading staged candidate document")?
            .ok_or_else(|| server_error!("staged candidate `{id}` does not contain `{name}`"))?;
        bodies.push(body);
    }
    let revision = Revision::read(&id, &bodies[0], &bodies[1], &bodies[2])?;
    Ok(Some(Staged { id, kind, revision }))
}

// Removes the staged candidate, documents and id; nothing staged is nothing to remove.
pub async fn clear<S: StateStore + BlobStore>(store: &S) -> Result<(), Error> {
    let Some(id) = staged_id(store).await? else {
        return Ok(());
    };
    StateStore::delete(store, STAGED_KEY).await.context("clearing the staged candidate")?;
    prune(store, &id).await;
    Ok(())
}

// Where `repo` stands for the revision, the repository brought to where it
// can be read: the revision at the label the pointer names, where that
// label stands over the head; else the one the head's tree holds; else
// none. Fails `spec-mismatch` on a tree whose `spec/` is not the three
// documents or does not match the id its label names, and `spec-outdated`
// on one under another grammar.
pub async fn current<P: Vcs + Files + StateStore>(
    provider: &P, repo: &BuildRepo,
) -> Result<Standing, Error> {
    let pointer = match StateStore::get(provider, REVISION_KEY).await {
        Ok(pointer) => pointer,
        Err(error) => {
            tracing::warn!(%error, "the current revision id is unreadable");
            None
        }
    };
    let Some(head) = repo.head(provider).await? else {
        return Ok(Standing {
            head: None,
            current: None,
            behind: None,
            pointer,
        });
    };

    // the label the pointer names, where it stands over the head
    let mut behind = None;
    let named = pointer.as_deref().and_then(|pointer| str::from_utf8(pointer).ok());
    if let Some(id) = named {
        let label = label(id);
        match repo.labelled(provider, &label).await? {
            Some(commit) if repo.descends(provider, &head, &commit).await? => {
                let revision = read_at(provider, repo, &commit, Some(id)).await?;
                return Ok(Standing {
                    head: Some(head),
                    current: revision.map(|(id, revision)| Current { id, revision, commit }),
                    behind: None,
                    pointer,
                });
            }
            Some(commit) => {
                tracing::warn!(label, head, commit, "the label does not stand over the head");
                behind = Some(label);
            }
            None => tracing::warn!(label, "the pointer names a label the repository lacks"),
        }
    }

    // the head's tree
    let current = read_at(provider, repo, &head, None).await?.map(|(id, revision)| Current {
        id,
        revision,
        commit: head.clone(),
    });
    Ok(Standing {
        head: Some(head),
        current,
        behind,
        pointer,
    })
}

// Seals `revision` in `repo` as the current revision, over what `standing`
// read: the three documents written under `spec/` in a working copy cut at
// the current revision's commit, else at the head, else, for a repository
// the engine creates, in the repository created now as its root commit; what
// they changed held to the secret gate and sealed under a `RevisionMessage`;
// the commit labelled `emery/<id>`; the pointer swapped by compare-and-swap;
// the staged candidate cleared; the copy removed. A revision the repository
// already holds seals nothing and stands labelled as it was.
pub async fn commit<P: Vcs + Files + StateStore + BlobStore>(
    provider: &P, repo: &BuildRepo, revision: &Revision, standing: Standing,
) -> Result<Sealed, Error> {
    let documents = revision.documents()?;
    let id = Revision::identify(
        documents[0].1.as_bytes(),
        documents[1].1.as_bytes(),
        documents[2].1.as_bytes(),
    );
    let diff = standing.diff(revision);
    let message = RevisionMessage {
        revision: id,
        parent: standing.current.as_ref().map(|current| current.id.clone()),
        base: standing.over().map(str::to_owned),
    };

    // the tree the seal lands over
    let (copy, cut) = match message.base.as_deref() {
        Some(at) => (WorkingCopy::cut(provider, &repo.path(), REVISION, at).await?, true),
        None if repo.is_created() => {
            repo.init(provider).await?;
            (WorkingCopy::checkout(&repo.path()), false)
        }
        None => {
            return Err(match repo {
                BuildRepo::Project => Error::BadRequest {
                    code: "base-not-sealed".into(),
                    description: "the project repository has no commit to hold the specification"
                        .into(),
                },
                BuildRepo::Target { .. } => {
                    server_error!("{} answered no commit to seal over", repo.path())
                }
            });
        }
    };

    let sealed = seal_copy(provider, repo, &documents, &message, &copy).await;
    if cut {
        copy.remove(provider).await?;
    }
    let commit = sealed?;

    // the pointer, then the candidate the seal consumed
    StateStore::cas(
        provider,
        REVISION_KEY,
        standing.pointer.as_deref(),
        message.revision.as_bytes(),
    )
    .await
    .context("swapping the revision id")?;
    clear(provider).await?;
    Ok(Sealed {
        label: label(&message.revision),
        id: message.revision,
        parent: message.parent,
        commit,
        diff,
    })
}

// The `spec-not-generated` refusal of a verb that reads the current
// revision, naming the label the pointer names where it fell behind the
// head, and the staged candidate where one stands and is not sealed.
pub async fn not_generated<S: StateStore>(store: &S, standing: &Standing) -> Error {
    let mut clauses = vec!["no specification revision is sealed in the repository".to_owned()];
    if standing.head.is_none() {
        clauses[0].push_str(", which has no commit");
    }
    if let Some(label) = &standing.behind {
        clauses.push(format!("label `{label}` does not stand over its head"));
    }
    match staged_id(store).await {
        Ok(Some(candidate)) => {
            clauses.push(format!("candidate `{candidate}` is staged and not sealed"));
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "the staged record is unreadable"),
    }
    Error::NotFound {
        code: "spec-not-generated".into(),
        description: clauses.join("; "),
    }
}

// The `build-target-required` refusal of `verb`, which reads or seals the
// current revision and was given no target naming the repository it lives in.
pub fn target_required(verb: &str) -> Error {
    Error::BadRequest {
        code: "build-target-required".into(),
        description: format!(
            "the specification has no repository to live in: {verb} needs a `[target]` table \
             naming one"
        ),
    }
}

// The `nothing-staged` refusal of a verb that reads the staged candidate.
pub fn nothing_staged() -> Error {
    Error::NotFound {
        code: "nothing-staged".into(),
        description: "no candidate is staged".into(),
    }
}

// The label a revision stands under in its repository.
pub fn label(id: &str) -> String {
    format!("emery/{id}")
}

// Writes `documents` into `copy`, seals them under `message`, and labels
// the commit; answers the commit `copy` was cut at when nothing changed.
async fn seal_copy<P: Vcs + Files>(
    provider: &P, repo: &BuildRepo, documents: &[(&str, String); 3], message: &RevisionMessage,
    copy: &WorkingCopy,
) -> Result<String, Error> {
    let at = copy.path();
    let over = message.base.as_deref();
    if let Occupancy::Other(what) = Occupancy::of(provider, at).await? {
        return Err(mismatch(format!(
            "`{SPECIFICATION}/` at {} holds {what}, so the directory is the operator's and the \
             specification cannot live there",
            over.map_or_else(|| "the root".to_owned(), |over| format!("commit `{over}`"))
        )));
    }
    for (name, body) in documents {
        provider.write(at, &path(name), body.as_bytes()).await?;
    }

    // what changed, held to the secret gate and sealed
    let pending = copy.pending(provider).await?;
    let leaks = secrets::findings(provider, at, &pending).await?;
    if !leaks.is_empty() {
        return Err(secrets::refusal("the seal", &leaks));
    }
    let commit = match (copy.commit(provider, &message.to_string()).await?, over) {
        (Some(commit), _) => commit,
        (None, Some(over)) => {
            tracing::info!(
                revision = %message.revision,
                commit = over,
                "the repository already holds the revision"
            );
            over.to_owned()
        }
        (None, None) => {
            return Err(server_error!("sealing the specification made no root commit"));
        }
    };
    repo.label(provider, &label(&message.revision), &commit).await?;
    Ok(commit)
}

// The revision the tree at `commit` holds under `spec/`, with its id; `None`
// when the tree holds no `spec/`. Held to `expect`, the id the label names,
// where there is one.
async fn read_at<P: Vcs + Files>(
    provider: &P, repo: &BuildRepo, commit: &str, expect: Option<&str>,
) -> Result<Option<(String, Result<Revision, Error>)>, Error> {
    let copy = WorkingCopy::cut(provider, &repo.path(), REVISION, commit).await?;
    let read = read_tree(provider, copy.path(), commit, expect).await;
    copy.remove(provider).await?;
    read
}

async fn read_tree<F: Files>(
    files: &F, at: &str, commit: &str, expect: Option<&str>,
) -> Result<Option<(String, Result<Revision, Error>)>, Error> {
    match Occupancy::of(files, at).await? {
        Occupancy::Absent => {
            return expect.map_or(Ok(None), |id| {
                Err(mismatch(format!(
                    "commit `{commit}`, labelled `{}`, holds no `{SPECIFICATION}/`",
                    label(id)
                )))
            });
        }
        Occupancy::Held => {}
        Occupancy::Other(what) => {
            return Err(mismatch(format!(
                "`{SPECIFICATION}/` at commit `{commit}` holds {what}, where the three documents \
                 of a revision should stand"
            )));
        }
    }
    let spec = files.read(at, &path(Spec::NAME)).await?;
    let design = files.read(at, &path(Design::NAME)).await?;
    let plan = files.read(at, &path(Plan::NAME)).await?;
    let id = Revision::identify(&spec, &design, &plan);
    if let Some(expected) = expect.filter(|expected| *expected != id) {
        return Err(mismatch(format!(
            "`{SPECIFICATION}/` at commit `{commit}`, labelled `{}`, does not match revision \
             `{expected}`; the documents there are `emery show` output, never sources",
            label(expected)
        )));
    }
    let revision = Revision::read(&id, &spec, &design, &plan);
    Ok(Some((id, revision)))
}

fn mismatch(description: String) -> Error {
    Error::BadRequest {
        code: "spec-mismatch".into(),
        description,
    }
}

// The path a document lives at in the repository.
fn path(name: &str) -> String {
    format!("{SPECIFICATION}/{name}.json")
}

fn key(id: &str, name: &str) -> String {
    format!("{id}/{name}.json")
}

// A document that will not delete stays as an inert orphan, so the failure
// is logged rather than raised.
async fn prune<S: BlobStore>(store: &S, id: &str) {
    for name in Revision::DOCUMENTS {
        if let Err(error) = BlobStore::delete(store, STAGED_CONTAINER, &key(id, name)).await {
            tracing::warn!(%id, name, %error, "a displaced candidate document could not be pruned");
        }
    }
}
