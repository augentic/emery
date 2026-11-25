//! Persists revisions: the staged candidate in the engine's storage, the current revision in the target repository.
//!
//! A candidate stands in the engine's own storage: `stage` writes a revision
//! for review and `stage_amendment` an amendment, `staged` and
//! `staged_amendment` read each back, and `clear` removes whichever stands.
//! The slot holds one at a time: a revision never stages over an amendment,
//! which awaits its own seal or another `amend`, and an amendment never
//! stages over a revision candidate, which awaits `apply`. The current
//! revision lives in the repository the target names, under `spec/` at the
//! commit labelled `emery/<id>`: `current` reads it from the label the
//! `current-revision` pointer names, or from the head's tree, and `commit`
//! seals a `Sealing` there — a candidate revision over the current one, a
//! fresh candidate starting the history over, or an amendment's three
//! documents over the base it was prepared against — labels it, and points
//! the pointer at it. Content is checked against its identifier when read,
//! so a document rewritten beneath the engine is refused rather than served.

use anyhow::Context;
use omnia_sdk::blobstore::{self, Container};
use omnia_sdk::keyvalue::types::{Precondition, Revision as StoreRevision};
use omnia_sdk::keyvalue::{self, Bucket};
use omnia_sdk::{Error, ErrorKind, Vcs, server_error};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::revision::{
    Amendment, AmendmentBase, Design, Diff, Document as _, Documents, Plan, Revision, Spec,
};
use crate::vcs::{BuildRepo, REVISION, RevisionMessage, SPECIFICATION, Summary, WorkingCopy};
use crate::{EntryKind, files, secrets};

/// What the default key-value bucket last held at a key, for a conditional write.
struct KeyEntry {
    value: Option<Vec<u8>>,
    revision: Option<StoreRevision>,
}

impl KeyEntry {
    const fn empty() -> Self {
        Self {
            value: None,
            revision: None,
        }
    }
}

async fn bucket() -> Result<Bucket, Error> {
    keyvalue::default().await
}

async fn open_staged() -> Result<Container, Error> {
    if blobstore::exists(STAGED_CONTAINER).await? {
        blobstore::open(STAGED_CONTAINER).await
    } else {
        blobstore::create(STAGED_CONTAINER).await
    }
}

async fn staged_key() -> KeyEntry {
    match bucket().await {
        Ok(bucket) => key_entry(&bucket, STAGED_KEY, "the staged record is unreadable").await,
        Err(error) => {
            tracing::warn!(%error, "the staged record is unreadable");
            KeyEntry::empty()
        }
    }
}

async fn revision_key() -> KeyEntry {
    match bucket().await {
        Ok(bucket) => {
            key_entry(&bucket, REVISION_KEY, "the current revision id is unreadable").await
        }
        Err(error) => {
            tracing::warn!(%error, "the current revision id is unreadable");
            KeyEntry::empty()
        }
    }
}

async fn key_entry(bucket: &Bucket, key: &str, unreadable: &'static str) -> KeyEntry {
    match bucket.get(key.to_owned()).await {
        Ok(Some(entry)) => KeyEntry {
            value: Some(entry.value),
            revision: Some(entry.revision),
        },
        Ok(None) => KeyEntry::empty(),
        Err(error) => {
            tracing::warn!(%error, "{unreadable}");
            KeyEntry::empty()
        }
    }
}

async fn cas_bytes(
    bucket: &Bucket, key: &str, entry: &KeyEntry, value: &[u8], what: &'static str,
) -> Result<(), Error> {
    match bucket.compare_and_swap(key, entry.revision, value, None).await.context(what)? {
        Ok(_) => Ok(()),
        Err(_conflict) => Err(server_error!("{what}: another writer changed the entry first")),
    }
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum StagedKind {
    /// A whole revision, drafted from the sources alone.
    Revision,
    /// A change against the current revision, prepared on a base.
    Amendment,
}

impl StagedKind {
    // The documents a candidate of this kind is stored as under its id.
    const fn documents(self) -> &'static [&'static str] {
        match self {
            Self::Revision => &Revision::DOCUMENTS,
            Self::Amendment => &[Amendment::NAME],
        }
    }

    const fn noun(self) -> &'static str {
        match self {
            Self::Revision => "candidate",
            Self::Amendment => "amendment",
        }
    }
}

// The staged record at `STAGED_KEY`.
#[derive(Serialize, Deserialize)]
struct Slot {
    id: String,
    kind: StagedKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    base: Option<AmendmentBase>,
    // A revision candidate staged to start the history over: sealed with no
    // parent over the head where the current revision amends an earlier one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    fresh: bool,
}

impl Slot {
    const fn revision(id: String, fresh: bool) -> Self {
        Self {
            id,
            kind: StagedKind::Revision,
            base: None,
            fresh,
        }
    }

    const fn amendment(id: String, base: AmendmentBase) -> Self {
        Self {
            id,
            kind: StagedKind::Amendment,
            base: Some(base),
            fresh: false,
        }
    }

    // Fails closed on bytes that decode as no record.
    async fn read() -> Result<Option<Self>, Error> {
        let bucket = bucket().await?;
        let Some(raw) = bucket.read(STAGED_KEY).await.context("getting the staged record")? else {
            return Ok(None);
        };
        Self::decode(&raw).map(Some)
    }

    fn decode(raw: &[u8]) -> Result<Self, Error> {
        Ok(serde_json::from_slice(raw).context("decoding the staged record")?)
    }

    fn from_token(token: Option<&[u8]>) -> Option<Self> {
        token.and_then(|raw| Self::decode(raw).ok())
    }

    fn no_amendment(token: Option<&[u8]>) -> Result<(), Error> {
        match Self::from_token(token) {
            Some(slot) if slot.kind == StagedKind::Amendment => Err(amendment_pending(&slot.id)),
            Some(_) | None => Ok(()),
        }
    }

    fn no_candidate(token: Option<&[u8]>) -> Result<(), Error> {
        match Self::from_token(token) {
            Some(slot) if slot.kind == StagedKind::Revision => Err(candidate_pending(&slot.id)),
            Some(_) | None => Ok(()),
        }
    }

    // Swaps this record into the slot over `entry`, the key as it was read,
    // and prunes the candidate it displaced, bytes that decode as no record
    // naming none.
    async fn swap(self, entry: &KeyEntry) -> Result<(), Error> {
        let bucket = bucket().await?;
        let when = entry.revision.map_or(Precondition::Absent, Precondition::Matches);
        match bucket
            .set_json_if(STAGED_KEY, &self, None, when)
            .await
            .context("swapping the staged record")?
        {
            Ok(_) => {}
            Err(_conflict) => {
                return Err(server_error!(
                    "swapping the staged record: another writer changed the entry first"
                ));
            }
        }
        if let Some(slot) =
            Self::from_token(entry.value.as_deref()).filter(|slot| slot.id != self.id)
        {
            prune(&slot.id, slot.kind).await;
        }
        Ok(())
    }

    async fn item(self) -> Result<StagedItem, Error> {
        match self.kind {
            StagedKind::Revision => {
                Staged::read(self.id, self.fresh).await.map(StagedItem::Revision)
            }
            StagedKind::Amendment => {
                StagedAmendment::read(self.id).await.map(StagedItem::Amendment)
            }
        }
    }
}

// A document that will not delete stays as an inert orphan, so the failure
// is logged rather than raised.
async fn prune(id: &str, kind: StagedKind) {
    let Ok(container) = open_staged().await else { return };
    for name in kind.documents() {
        if let Err(error) = container.remove(&key(id, name)).await {
            tracing::warn!(%id, name, %error, "a displaced candidate document could not be pruned");
        }
    }
}

fn key(id: &str, name: &str) -> String {
    format!("{id}/{name}.json")
}

// The revision candidate `stage` left for `apply`, read back whole, and
// whether it was staged to start the history over.
pub struct Staged {
    pub id: String,
    pub revision: Revision,
    pub fresh: bool,
}

impl Staged {
    async fn read(id: String, fresh: bool) -> Result<Self, Error> {
        let spec = document(&id, Spec::NAME).await?;
        let design = document(&id, Design::NAME).await?;
        let plan = document(&id, Plan::NAME).await?;
        let revision = Revision::read(&id, &spec, &design, &plan)?;
        Ok(Self { id, revision, fresh })
    }
}

async fn document(id: &str, name: &str) -> Result<Vec<u8>, Error> {
    let container = open_staged().await?;
    let object = key(id, name);
    let Some(bytes) = container.read(&object).await.context("reading staged candidate document")?
    else {
        return Err(server_error!("staged candidate `{id}` does not contain `{name}`"));
    };
    Ok(bytes.to_vec())
}

// The amendment `stage_amendment` left for review, read back whole.
pub struct StagedAmendment {
    pub id: String,
    pub amendment: Amendment,
}

impl StagedAmendment {
    async fn read(id: String) -> Result<Self, Error> {
        let body = document(&id, Amendment::NAME).await?;
        let amendment = Amendment::read(&id, &body)?;
        Ok(Self { id, amendment })
    }
}

// Whatever stands in the slot, read back whole.
pub enum StagedItem {
    Revision(Staged),
    Amendment(StagedAmendment),
}

// What a seal writes: the three documents as a revision, the revision it
// records as its parent, the commit it lands over, and, for an amendment,
// what it changed against that parent.
pub struct Sealing {
    revision: Revision,
    parent: Option<String>,
    over: Option<String>,
    summary: Option<Summary>,
}

impl Sealing {
    // A candidate revision over what `standing` read: the current revision
    // its parent and the commit holding it, else the head, the one it lands
    // over. `lineage-exists` where the current revision amends an earlier
    // one, since a candidate drafted from the sources alone would start
    // that history over; `fresh` is the seal that does so deliberately.
    pub fn candidate(revision: Revision, standing: &Standing) -> Result<Self, Error> {
        standing.no_lineage()?;
        Ok(Self {
            revision,
            parent: standing.current.as_ref().map(|current| current.id.clone()),
            over: standing.over().map(str::to_owned),
            summary: None,
        })
    }

    // A candidate revision starting the history over: no parent, cut over
    // the repository's head whatever revision it holds, so every earlier
    // label stands and the new one descends from the head alone.
    pub fn fresh(revision: Revision, standing: &Standing) -> Self {
        Self {
            revision,
            parent: None,
            over: standing.head.clone(),
            summary: None,
        }
    }

    // An amendment's three documents over the base it was prepared
    // against, that base its parent and the commit it was read at the one it
    // lands over, the counts by class and the parts changed its summary.
    // `amendment-stale` unless `standing` reads that base as the current
    // revision, at that commit, so a revision sealed since is never
    // displaced by a change prepared against the one before it.
    pub fn amendment(amendment: Amendment, standing: &Standing) -> Result<Self, Error> {
        if !standing.is_at(&amendment.base) {
            return Err(stale(&amendment.base, standing));
        }
        let summary = Summary::of(&amendment);
        Ok(Self {
            revision: Revision {
                spec: amendment.spec,
                design: amendment.design,
                plan: amendment.plan,
            },
            parent: Some(amendment.base.revision),
            over: Some(amendment.base.commit),
            summary: Some(summary),
        })
    }
}

// The `amendment-stale` refusal of a seal of an amendment prepared against
// `base` where `standing` reads another revision, or none, as current.
fn stale(base: &AmendmentBase, standing: &Standing) -> Error {
    let now = standing.current.as_ref().map_or_else(
        || "holds no revision".to_owned(),
        |current| format!("holds revision `{}` at commit `{}`", current.id, current.commit),
    );
    Error::bad_request(
        "amendment-stale",
        format!(
            "the amendment was prepared against revision `{}` at commit `{}`, and the repository \
             now {now}",
            base.revision, base.commit
        ),
    )
}

// The revision a repository holds, and the commit it was read from. The
// documents may not read as a revision this engine knows, an outdated
// grammar: the id and the commit still stand, so a seal lands over them,
// while a verb that needs the revision raises the refusal. The parent is
// read off the specification's own top-level key, so a lineage is seen
// under an outdated grammar too.
pub struct Current {
    pub id: String,
    pub revision: Result<Revision, Error>,
    pub commit: String,
    pub parent: Option<String>,
}

impl Current {
    // The revision the tree at `commit` holds under `spec/`, with its id;
    // `None` when the tree holds no `spec/`. Held to `expect`, the id the
    // label names, where there is one.
    async fn read_at<P: Vcs>(
        vcs: &P, repo: &BuildRepo, commit: &str, expect: Option<&str>,
    ) -> Result<Option<Self>, Error> {
        let copy = WorkingCopy::cut(vcs, &repo.path(), REVISION, commit).await?;
        let current = Self::read_tree(copy.path(), commit, expect).await;
        copy.remove(vcs).await?;
        current
    }

    // Held to `expect` where a label named the commit; at a head no label
    // names, three documents that read as no revision of any grammar, or
    // that read as no one revision's (`Revision::findings`), were edited or
    // merged by hand, since a seal writes them whole, and are
    // `spec-mismatch` rather than a revision to seal over. Documents that
    // read as one revision are that revision whatever label the repository
    // holds for them, since a clone reads no remote's label and an operator
    // may delete a merged one.
    async fn read_tree(
        at: &str, commit: &str, expect: Option<&str>,
    ) -> Result<Option<Self>, Error> {
        match Occupancy::of(at).await? {
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
                    "`{SPECIFICATION}/` at commit `{commit}` holds {what}, where the three \
                     documents of a revision should stand"
                )));
            }
        }
        let spec = files::read(at, &path(Spec::NAME)).await?;
        let design = files::read(at, &path(Design::NAME)).await?;
        let plan = files::read(at, &path(Plan::NAME)).await?;
        let id = Revision::identify(&spec, &design, &plan);
        if let Some(expected) = expect.filter(|expected| *expected != id) {
            return Err(mismatch(format!(
                "`{SPECIFICATION}/` at commit `{commit}`, labelled `{}`, does not match revision \
                 `{expected}`; the documents there are `emery show` output, never sources",
                label(expected)
            )));
        }
        let revision = match Revision::read(&id, &spec, &design, &plan) {
            Err(error) if expect.is_none() && error.kind() != ErrorKind::BadRequest => {
                return Err(hand_merged(commit, "no revision", error.description()));
            }
            Ok(revision) if expect.is_none() => {
                let findings = revision.findings();
                if !findings.is_empty() {
                    return Err(hand_merged(commit, "no one revision's", &findings.join("; ")));
                }
                Ok(revision)
            }
            revision => revision,
        };
        let parent = revision
            .as_ref()
            .map_or_else(|_| spec_parent(&spec), |revision| revision.spec.parent.clone());
        Ok(Some(Self {
            id,
            revision,
            commit: commit.to_owned(),
            parent,
        }))
    }
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
    async fn of(at: &str) -> Result<Self, Error> {
        let root = files::list(at, ".").await?;
        let Some(entry) = root.into_iter().find(|entry| entry.name == SPECIFICATION) else {
            return Ok(Self::Absent);
        };
        match entry.kind {
            EntryKind::Directory => {}
            EntryKind::File => return Ok(Self::Other("a file".to_owned())),
            EntryKind::Link => return Ok(Self::Other("a symbolic link".to_owned())),
        }
        let mut names: Vec<String> = files::list(at, SPECIFICATION)
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

// The refusal of a `spec/` at an unlabelled head whose documents were edited
// or merged by hand; the CLI's hint matches on the `edited or merged by hand`
// clause.
fn hand_merged(commit: &str, what: &str, why: &str) -> Error {
    mismatch(format!(
        "`{SPECIFICATION}/` at commit `{commit}` holds documents that read as {what} ({why}), so \
         they were edited or merged by hand; a seal writes them whole"
    ))
}

// The top-level `parent` key of `spec`, for a specification under another
// grammar that `Revision::read` did not deserialise.
fn spec_parent(spec: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(spec).ok()?.get("parent")?.as_str().map(str::to_owned)
}

fn mismatch(description: String) -> Error {
    Error::bad_request("spec-mismatch", description)
}

// The path a document lives at in the repository.
fn path(name: &str) -> String {
    format!("{SPECIFICATION}/{name}.json")
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
    pointer_revision: Option<StoreRevision>,
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

    // `lineage-exists` where the current revision amends an earlier one, so
    // a verb that would seal a revision drafted from the sources alone over
    // it refuses rather than starting the history over unasked.
    pub fn no_lineage(&self) -> Result<(), Error> {
        match &self.current {
            Some(Current {
                id,
                parent: Some(parent),
                ..
            }) => Err(lineage_exists(id, parent)),
            Some(_) | None => Ok(()),
        }
    }

    // Whether the current revision stands at `base`, the revision and
    // commit an amendment was prepared against.
    fn is_at(&self, base: &AmendmentBase) -> bool {
        matches!(
            &self.current,
            Some(Current { id, commit, .. }) if *id == base.revision && *commit == base.commit
        )
    }
}

// The `lineage-exists` refusal of a verb that would seal a revision drafted
// from the sources alone over revision `id`, which amends `parent`.
fn lineage_exists(id: &str, parent: &str) -> Error {
    Error::bad_request(
        "lineage-exists",
        format!(
            "the current revision `{id}` amends `{parent}`, so the repository holds a lineage \
             that a revision drafted from the sources alone would start over"
        ),
    )
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

// Stages `revision` as the candidate for `emery apply`, replacing any
// revision staged before it: every document written under the candidate's
// id, the staged record swapped by compare-and-swap with whether the
// candidate is `fresh`, and the candidate it displaced pruned. A staged
// amendment is never displaced: it is `amendment-pending`, read before any
// document is written.
pub async fn stage(revision: &Revision, fresh: bool) -> Result<String, Error> {
    let entry = staged_key().await;
    Slot::no_amendment(entry.value.as_deref())?;

    let (id, documents) = revision.documents()?;
    let bodies = documents.each_ref().map(|(name, body)| (*name, body.as_str()));
    put(&id, &bodies).await?;
    Slot::revision(id.clone(), fresh).swap(&entry).await?;
    Ok(id)
}

// Stages `amendment` for review, replacing any amendment staged before it:
// the document written under the amendment's id, the staged record swapped
// by compare-and-swap with the base it was prepared against, and the
// amendment it displaced pruned. A staged revision candidate is never
// displaced: it is `amendment-pending`, read before any document is
// written.
pub async fn stage_amendment(amendment: &Amendment) -> Result<String, Error> {
    let entry = staged_key().await;
    Slot::no_candidate(entry.value.as_deref())?;

    let body = amendment.to_json()?;
    let id = Amendment::identify(body.as_bytes());
    put(&id, &[(Amendment::NAME, body.as_str())]).await?;
    Slot::amendment(id.clone(), amendment.base.clone()).swap(&entry).await?;
    Ok(id)
}

// Writes `documents` under `id` in the staged container, creating it on the first stage.
async fn put(id: &str, documents: &[(&str, &str)]) -> Result<(), Error> {
    let container = open_staged().await?;
    for (name, body) in documents {
        container
            .write(&key(id, name), body.as_bytes())
            .await
            .context("writing staged candidate document")?;
    }
    Ok(())
}

// The staged item's id and kind, whatever stands, or `None` when nothing is staged.
pub async fn staged_record() -> Result<Option<(String, StagedKind)>, Error> {
    Ok(Slot::read().await?.map(|slot| (slot.id, slot.kind)))
}

// Whatever stands in the slot, whole, or `None` when nothing is staged:
// fails closed on an item that no longer matches its id, and
// `spec-outdated` on one under another grammar.
pub async fn staged_item() -> Result<Option<StagedItem>, Error> {
    match Slot::read().await? {
        Some(slot) => slot.item().await.map(Some),
        None => Ok(None),
    }
}

// The staged revision candidate whole, or `None` when nothing is staged:
// `amendment-pending` when an amendment stands in the slot instead, read
// no further, and what `staged_item` fails with otherwise.
pub async fn staged() -> Result<Option<Staged>, Error> {
    match Slot::read().await? {
        Some(Slot {
            id,
            kind: StagedKind::Revision,
            fresh,
            ..
        }) => Staged::read(id, fresh).await.map(Some),
        Some(Slot {
            id,
            kind: StagedKind::Amendment,
            ..
        }) => Err(amendment_pending(&id)),
        None => Ok(None),
    }
}

// The staged amendment whole, or `None` when nothing is staged or a
// revision candidate stands in the slot, read no further, and what
// `staged_item` fails with otherwise.
pub async fn staged_amendment() -> Result<Option<StagedAmendment>, Error> {
    match Slot::read().await? {
        Some(Slot {
            id,
            kind: StagedKind::Amendment,
            ..
        }) => StagedAmendment::read(id).await.map(Some),
        Some(Slot {
            kind: StagedKind::Revision,
            ..
        })
        | None => Ok(None),
    }
}

// Refuses `amendment-pending` when an amendment is staged, so a verb that
// stages a revision refuses before it spends a turn on one. Reads the slot
// as `stage` does: a record that does not decode is corruption the stage
// swaps over, never a refusal.
pub async fn no_amendment() -> Result<(), Error> {
    let entry = staged_key().await;
    Slot::no_amendment(entry.value.as_deref())
}

// Refuses `amendment-pending` when a revision candidate is staged, so a verb
// that stages an amendment refuses before it spends a turn on one; the slot
// is read as `no_amendment` reads it.
pub async fn no_candidate() -> Result<(), Error> {
    let entry = staged_key().await;
    Slot::no_candidate(entry.value.as_deref())
}

// Removes the staged candidate, documents and record, whatever its kind;
// nothing staged is nothing to remove.
pub async fn clear() -> Result<(), Error> {
    let Some(Slot { id, kind, .. }) = Slot::read().await? else {
        return Ok(());
    };
    let bucket = bucket().await?;
    bucket.remove(STAGED_KEY).await.context("clearing the staged candidate")?;
    prune(&id, kind).await;
    Ok(())
}

// Where `repo` stands for the revision, the repository brought to where it
// can be read: the revision at the label the pointer names, where that
// label stands over the head; else the one the head's tree holds, whatever
// the pointer names, so two checkouts whose pointers name two labels a
// merge left behind the head read the one revision the head holds; else
// none. Fails `spec-mismatch` on a tree whose `spec/` is not the three
// documents, does not match the id its label names, or at an unlabelled
// head holds documents that read as no revision or as no one revision's,
// and `spec-outdated` on one under another grammar.
pub async fn current<P: Vcs>(vcs: &P, repo: &BuildRepo) -> Result<Standing, Error> {
    let KeyEntry {
        value: pointer,
        revision: pointer_revision,
    } = revision_key().await;
    let Some(head) = repo.head(vcs).await? else {
        return Ok(Standing {
            head: None,
            current: None,
            behind: None,
            pointer,
            pointer_revision,
        });
    };

    // the label the pointer names, where it stands over the head
    let mut behind = None;
    let named = pointer.as_deref().and_then(|pointer| str::from_utf8(pointer).ok());
    if let Some(id) = named {
        let label = label(id);
        match repo.labelled(vcs, &label).await? {
            Some(commit) if repo.descends(vcs, &head, &commit).await? => {
                let current = Current::read_at(vcs, repo, &commit, Some(id)).await?;
                return Ok(Standing {
                    head: Some(head),
                    current,
                    behind: None,
                    pointer,
                    pointer_revision,
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
    let current = Current::read_at(vcs, repo, &head, None).await?;
    Ok(Standing {
        head: Some(head),
        current,
        behind,
        pointer,
        pointer_revision,
    })
}

// Seals `sealing` in `repo` as the current revision, over what `standing`
// read: the three documents written under `spec/` in a working copy cut at
// the commit the sealing lands over, else, for a repository the engine
// creates, in the repository created now as its root commit; what they
// changed held to the secret gate and sealed under a `RevisionMessage`
// carrying the sealing's parent and summary; the commit labelled
// `emery/<id>`; the pointer swapped by compare-and-swap; the staged item
// cleared; the copy removed. A revision the repository already holds seals
// nothing and stands labelled as it was.
pub async fn commit<P: Vcs>(
    vcs: &P, repo: &BuildRepo, sealing: Sealing, standing: Standing,
) -> Result<Sealed, Error> {
    let (id, documents) = sealing.revision.documents()?;
    let diff = standing.diff(&sealing.revision);
    let message = RevisionMessage {
        revision: id,
        parent: sealing.parent,
        base: sealing.over,
        summary: sealing.summary,
    };

    // the tree the seal lands over
    let (copy, cut) = match message.base.as_deref() {
        Some(at) => (WorkingCopy::cut(vcs, &repo.path(), REVISION, at).await?, true),
        None if repo.is_created() => {
            repo.init(vcs).await?;
            (WorkingCopy::checkout(&repo.path()), false)
        }
        None => {
            return Err(match repo {
                BuildRepo::Project => Error::bad_request(
                    "base-not-sealed",
                    "the project repository has no commit to hold the specification",
                ),
                BuildRepo::Target { .. } => {
                    server_error!("{} answered no commit to seal over", repo.path())
                }
            });
        }
    };

    let sealed = seal_copy(vcs, repo, &documents, &message, &copy).await;
    if cut {
        copy.remove(vcs).await?;
    }
    let commit = sealed?;

    // the pointer, then the candidate the seal consumed
    let bucket = bucket().await?;
    let pointer = KeyEntry {
        value: standing.pointer,
        revision: standing.pointer_revision,
    };
    cas_bytes(
        &bucket,
        REVISION_KEY,
        &pointer,
        message.revision.as_bytes(),
        "swapping the revision id",
    )
    .await?;
    clear().await?;
    Ok(Sealed {
        label: label(&message.revision),
        id: message.revision,
        parent: message.parent,
        commit,
        diff,
    })
}

// Writes `documents` into `copy`, seals them under `message`, and labels
// the commit; answers the commit `copy` was cut at when nothing changed.
async fn seal_copy<P: Vcs>(
    vcs: &P, repo: &BuildRepo, documents: &Documents, message: &RevisionMessage, copy: &WorkingCopy,
) -> Result<String, Error> {
    let at = copy.path();
    let over = message.base.as_deref();
    if let Occupancy::Other(what) = Occupancy::of(at).await? {
        return Err(mismatch(format!(
            "`{SPECIFICATION}/` at {} holds {what}, so the directory is the operator's and the \
             specification cannot live there",
            over.map_or_else(|| "the root".to_owned(), |over| format!("commit `{over}`"))
        )));
    }
    for (name, body) in documents {
        files::write(at, &path(name), body.as_bytes()).await?;
    }

    // what changed, held to the secret gate and sealed
    let pending = copy.pending(vcs).await?;
    let leaks = secrets::findings(at, &pending).await?;
    if !leaks.is_empty() {
        return Err(secrets::refusal("the seal", &leaks));
    }
    let commit = match (copy.commit(vcs, &message.to_string()).await?, over) {
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
    repo.label(vcs, &label(&message.revision), &commit).await?;
    Ok(commit)
}

// The `spec-not-generated` refusal of a verb that reads the current
// revision, naming the label the pointer names where it fell behind the
// head, and the staged candidate where one stands and is not sealed.
pub async fn not_generated(standing: &Standing) -> Error {
    let mut clauses = vec!["no specification revision is sealed in the repository".to_owned()];
    if standing.head.is_none() {
        clauses[0].push_str(", which has no commit");
    }
    if let Some(label) = &standing.behind {
        clauses.push(format!("label `{label}` does not stand over its head"));
    }
    match Slot::read().await {
        Ok(Some(Slot { id, kind, .. })) => {
            clauses.push(format!("{} `{id}` is staged and not sealed", kind.noun()));
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "the staged record is unreadable"),
    }
    Error::not_found("spec-not-generated", clauses.join("; "))
}

// The `build-target-required` refusal of `verb`, which reads or seals the
// current revision and was given no target naming the repository it lives in.
pub fn target_required(verb: &str) -> Error {
    Error::bad_request(
        "build-target-required",
        format!(
            "the specification has no repository to live in: {verb} needs a `[target]` table \
             naming one"
        ),
    )
}

// The `nothing-staged` refusal of a verb that reads the staged candidate.
pub fn nothing_staged() -> Error {
    Error::not_found("nothing-staged", "no candidate is staged")
}

// The `nothing-staged` refusal of a verb that reads the staged amendment,
// which a staged revision candidate does not satisfy.
pub fn no_amendment_staged() -> Error {
    Error::not_found("nothing-staged", "no amendment is staged")
}

// The `amendment-pending` refusal of a verb that reads or stages a revision
// candidate while an amendment stands in the slot.
pub fn amendment_pending(id: &str) -> Error {
    Error::bad_request(
        "amendment-pending",
        format!("amendment `{id}` is staged in place of a revision candidate"),
    )
}

// The `amendment-pending` refusal of a verb that stages an amendment while a
// revision candidate stands in the slot, awaiting its seal.
pub fn candidate_pending(id: &str) -> Error {
    Error::bad_request(
        "amendment-pending",
        format!("candidate `{id}` is staged and not sealed, where the amendment would go"),
    )
}

// The label a revision stands under in its repository.
pub fn label(id: &str) -> String {
    format!("emery/{id}")
}
