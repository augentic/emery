//! Hashes workspace source files for per-revision change detection.
//!
//! Captures the lowercase SHA-256 digest of every regular file in directories
//! cited by a workspace source's claims, alongside any base-snapshot or
//! standing-citation files that still exist.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Context;
use emery_adapter::beneath;
use omnia_sdk::Error;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::revision::{Cited, Spec};
use crate::specify::{Bound, Extract, shape};
use crate::{Entry, EntryKind, Files};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Snapshot(BTreeMap<String, BTreeMap<String, String>>);

impl Snapshot {
    pub const NAME: &str = "sources";

    // Hashes each workspace source's cited directories and tracked prior
    // files before temporary checkouts are removed, populating `extract.files`
    // and settling the revision's snapshot.
    pub async fn capture<F: Files>(
        files: &F, bound: &[Bound], extracts: &mut [Extract], base: Option<SnapshotBase>,
    ) -> Result<Self, Error> {
        let mut caches: BTreeMap<String, ListingCache> = BTreeMap::new();
        for (source, extract) in bound.iter().zip(extracts.iter_mut()) {
            let Some(workspace) = &source.workspace else { continue };
            let cache = caches
                .entry(workspace.at.clone())
                .or_insert_with(|| ListingCache::new(workspace.at.clone()));
            extract.files =
                Some(hash_source(files, workspace, extract, base.as_ref(), cache).await?);
        }
        Ok(base.map_or_else(|| Self::from_extracts(extracts), |base| base.into_snapshot(extracts)))
    }

    fn from_extracts(extracts: &[Extract]) -> Self {
        let sources = extracts
            .iter()
            .filter_map(|extract| {
                let files = extract.files.as_ref().filter(|files| !files.is_empty())?;
                Some((extract.source.clone(), files.clone()))
            })
            .collect();
        Self(sources)
    }

    #[must_use]
    pub fn file_delta(
        &self, source: &str, current: &BTreeMap<String, String>,
    ) -> Option<FileDelta> {
        self.0.get(source).map(|base| FileDelta::between(base, current))
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, Error> {
        Ok(serde_json::from_slice(bytes).context("`sources` is not a snapshot")?)
    }

    pub fn to_json(&self) -> Result<String, Error> {
        let mut text =
            serde_json::to_string_pretty(self).context("`sources` does not serialise")?;
        text.push('\n');
        Ok(text)
    }
}

// Changed and removed workspace files of one source against its base snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDelta {
    changed: BTreeSet<String>,
    gone: BTreeSet<String>,
}

impl FileDelta {
    #[must_use]
    fn between(base: &BTreeMap<String, String>, current: &BTreeMap<String, String>) -> Self {
        let changed = current
            .iter()
            .filter(|(file, digest)| base.get(*file) != Some(*digest))
            .map(|(file, _)| file.clone())
            .collect();
        let gone = base.keys().filter(|file| !current.contains_key(*file)).cloned().collect();
        Self { changed, gone }
    }

    #[must_use]
    pub fn changed_count(&self) -> usize {
        self.changed.len()
    }

    #[must_use]
    pub fn gone_count(&self) -> usize {
        self.gone.len()
    }

    #[must_use]
    pub fn is_changed(&self, path: &str) -> bool {
        self.changed.contains(&canonical_file(path))
    }

    #[must_use]
    pub fn is_changed_or_gone(&self, path: &str) -> bool {
        let file = canonical_file(path);
        self.changed.contains(&file) || self.gone.contains(&file)
    }

    #[must_use]
    pub fn touches_citation(&self, cited: &Cited) -> bool {
        cited.path.as_deref().is_some_and(|path| self.is_changed_or_gone(path))
    }
}

fn canonical_file(raw: &str) -> String {
    let file = shape::file(raw);
    beneath(file).unwrap_or_else(|_| file.to_owned())
}

// Owned base-revision context an amend carries forward while its files exist.
#[derive(Clone, Debug, Default)]
pub struct SnapshotBase {
    snapshot: Option<Snapshot>,
    cited: BTreeMap<String, BTreeSet<String>>,
}

impl SnapshotBase {
    #[must_use]
    pub fn new(snapshot: Option<&Snapshot>, spec: &Spec) -> Self {
        let mut cited: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for requirement in &spec.requirements {
            for source_ref in &requirement.sources {
                if let Some(raw) = source_ref.path.as_deref()
                    && let Ok(file) = beneath(shape::file(raw))
                {
                    cited.entry(source_ref.source.clone()).or_default().insert(file);
                }
            }
        }
        Self {
            snapshot: snapshot.cloned(),
            cited,
        }
    }

    fn tracked_files(&self, source: &str) -> BTreeSet<String> {
        let mut files = self.cited.get(source).cloned().unwrap_or_default();
        if let Some(recorded) = self.snapshot.as_ref().and_then(|snapshot| snapshot.0.get(source)) {
            for path in recorded.keys() {
                if let Ok(file) = beneath(path) {
                    files.insert(file);
                }
            }
        }
        files
    }

    // Untouched sources keep their base entries; an extracted workspace source
    // replaces its entry when non-empty or already tracked; an inline source
    // drops any stale workspace entry under that name.
    fn into_snapshot(self, extracts: &[Extract]) -> Snapshot {
        let mut sources = self.snapshot.map_or_else(BTreeMap::new, |snapshot| snapshot.0);
        for extract in extracts {
            match &extract.files {
                Some(files) if !files.is_empty() || sources.contains_key(&extract.source) => {
                    sources.insert(extract.source.clone(), files.clone());
                }
                Some(_) => {}
                None => {
                    sources.remove(&extract.source);
                }
            }
        }
        Snapshot(sources)
    }
}

// Where a workspace source starts beneath a working copy `Files` reads.
#[derive(Clone, Debug)]
pub struct Workspace {
    at: String,
    within: String,
}

impl Workspace {
    #[must_use]
    pub fn new(at: impl Into<String>, within: &Path) -> Self {
        let within = within.iter().map(|part| part.to_string_lossy()).collect::<Vec<_>>().join("/");
        Self {
            at: at.into(),
            within,
        }
    }

    #[must_use]
    pub fn lent_root(&self) -> String {
        if self.within == "." { self.at.clone() } else { format!("{}/{}", self.at, self.within) }
    }

    fn at(&self) -> &str {
        &self.at
    }

    fn resolve(&self, relative: &str) -> String {
        join_rel(&self.within, relative)
    }
}

async fn hash_source<F: Files>(
    files: &F, workspace: &Workspace, extract: &Extract, base: Option<&SnapshotBase>,
    cache: &mut ListingCache,
) -> Result<BTreeMap<String, String>, Error> {
    // directories holding any file a claim's `path` names
    let cited_dirs: BTreeSet<String> = extract
        .evidence
        .claims
        .iter()
        .filter_map(|claim| claim.path.as_deref())
        .filter_map(|raw| beneath(shape::file(raw)).ok())
        .map(|file| split_parent(&file).0.to_owned())
        .collect();
    let prior_files = base.map_or_else(BTreeSet::new, |base| base.tracked_files(&extract.source));

    if cited_dirs.is_empty() && prior_files.is_empty() {
        return Ok(BTreeMap::new());
    }

    // regular files directly inside cited directories
    let mut present = BTreeSet::new();
    for dir in &cited_dirs {
        let Some(entries) = cache.directory(files, &workspace.resolve(dir)).await? else {
            continue;
        };
        for entry in entries {
            if entry.kind == EntryKind::File
                && !entry.name.contains('/')
                && let Ok(clean) = beneath(&join_rel(dir, &entry.name))
            {
                present.insert(clean);
            }
        }
    }

    // prior files outside those directories that still exist as regular files
    for file in prior_files {
        if !present.contains(&file)
            && cache.has_regular_file(files, &workspace.resolve(&file)).await?
        {
            present.insert(file);
        }
    }

    // digest each present file
    let mut digests = BTreeMap::new();
    for file in present {
        let bytes = files.read(workspace.at(), &workspace.resolve(&file)).await?;
        digests.insert(file, sha256_hex(&bytes));
    }
    Ok(digests)
}

// Cached directory listings beneath one working copy, deciding presence by
// parent listing without following symbolic links.
struct ListingCache {
    at: String,
    directories: BTreeMap<String, Option<Vec<Entry>>>,
}

impl ListingCache {
    const fn new(at: String) -> Self {
        Self {
            at,
            directories: BTreeMap::new(),
        }
    }

    async fn directory<F: Files>(
        &mut self, files: &F, dir: &str,
    ) -> Result<Option<&[Entry]>, Error> {
        if !self.directories.contains_key(".") {
            let root = files.list(&self.at, ".").await?;
            self.directories.insert(".".to_owned(), Some(root));
        }
        let mut current = ".".to_owned();
        for segment in dir.split('/').filter(|segment| !segment.is_empty() && *segment != ".") {
            let next = join_rel(&current, segment);
            if !self.directories.contains_key(&next) {
                let is_directory =
                    self.directories.get(&current).and_then(Option::as_deref).is_some_and(
                        |entries| {
                            entries.iter().any(|entry| {
                                entry.kind == EntryKind::Directory && entry.name == segment
                            })
                        },
                    );
                let listed =
                    if is_directory { Some(files.list(&self.at, &next).await?) } else { None };
                self.directories.insert(next.clone(), listed);
            }
            if self.directories.get(&next).is_none_or(Option::is_none) {
                return Ok(None);
            }
            current = next;
        }
        Ok(self.directories.get(&current).and_then(Option::as_deref))
    }

    async fn has_regular_file<F: Files>(&mut self, files: &F, path: &str) -> Result<bool, Error> {
        let (dir, name) = split_parent(path);
        Ok(self.directory(files, dir).await?.is_some_and(|entries| {
            entries.iter().any(|entry| entry.kind == EntryKind::File && entry.name == name)
        }))
    }
}

fn split_parent(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or((".", path))
}

fn join_rel(dir: &str, name: &str) -> String {
    match (dir, name) {
        (".", path) => path.to_owned(),
        (dir, ".") => dir.to_owned(),
        (dir, path) => format!("{dir}/{path}"),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
