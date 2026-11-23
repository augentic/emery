//! Hashes workspace source files and line structure for per-revision change detection.
//!
//! Captures the lowercase SHA-256 digest, per-line 64-bit hashes, and structural
//! line spans of every regular file in directories cited by a workspace source's
//! claims, alongside any base-snapshot or standing-citation files that still exist.

mod diff;
mod scan;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;
use std::path::Path;

use anyhow::Context;
use emery_adapter::beneath;
use emery_adapter::source::Anchor;
use omnia_sdk::Error;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest as _, Sha256};

use self::diff::SpanDelta;
use crate::revision::{Cited, Spec};
use crate::specify::{Bound, Extract, shape};
use crate::{Entry, EntryKind, Files};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "[usize; 2]", into = "[usize; 2]")]
pub struct LineSpan {
    pub start: usize,
    pub end: usize,
}

impl LineSpan {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub const fn contains(self, line: usize) -> bool {
        self.start <= line && line <= self.end
    }

    #[must_use]
    pub const fn lines(self) -> RangeInclusive<usize> {
        self.start..=self.end
    }
}

impl From<[usize; 2]> for LineSpan {
    fn from([start, end]: [usize; 2]) -> Self {
        Self { start, end }
    }
}

impl From<LineSpan> for [usize; 2] {
    fn from(span: LineSpan) -> Self {
        [span.start, span.end]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileSnapshot {
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routines: Vec<LineSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<usize>,
}

impl FileSnapshot {
    #[must_use]
    pub fn from_bytes(path: &str, bytes: &[u8]) -> Self {
        let sha256 = sha256_hex(bytes);
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Self::digest_only(sha256);
        };
        let scanned = scan::scan(path, text);
        Self {
            sha256,
            lines: scanned.lines,
            routines: scanned.routines,
            imports: scanned.imports,
            values: scanned.values,
        }
    }

    #[must_use]
    const fn digest_only(sha256: String) -> Self {
        Self {
            sha256,
            lines: Vec::new(),
            routines: Vec::new(),
            imports: Vec::new(),
            values: Vec::new(),
        }
    }

    #[must_use]
    pub const fn has_lines(&self) -> bool {
        !self.lines.is_empty()
    }

    #[must_use]
    pub fn is_import_line(&self, line: usize) -> bool {
        self.imports.binary_search(&line).is_ok()
    }

    #[must_use]
    pub fn is_value_line(&self, line: usize) -> bool {
        self.values.binary_search(&line).is_ok()
    }

    fn normalise(&mut self) {
        self.routines.sort_unstable();
        self.routines.dedup();
        self.imports.sort_unstable();
        self.imports.dedup();
        self.values.sort_unstable();
        self.values.dedup();
    }
}

impl<'de> Deserialize<'de> for FileSnapshot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Legacy(String),
            Structured {
                sha256: String,
                #[serde(default)]
                lines: Vec<u64>,
                #[serde(default)]
                routines: Vec<LineSpan>,
                #[serde(default)]
                imports: Vec<usize>,
                #[serde(default)]
                values: Vec<usize>,
            },
        }

        match Repr::deserialize(deserializer)? {
            Repr::Legacy(sha256) => Ok(Self::digest_only(sha256)),
            Repr::Structured {
                sha256,
                lines,
                routines,
                imports,
                values,
            } => {
                let mut snapshot = Self {
                    sha256,
                    lines,
                    routines,
                    imports,
                    values,
                };
                snapshot.normalise();
                Ok(snapshot)
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Snapshot(BTreeMap<String, BTreeMap<String, FileSnapshot>>);

impl Snapshot {
    pub const NAME: &str = "sources";

    // Snapshots each workspace source's cited directories and tracked prior
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
                Some(snapshot_source(files, workspace, extract, base.as_ref(), cache).await?);
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
        &self, source: &str, current: &BTreeMap<String, FileSnapshot>,
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

// Changed spans and removed workspace files of one source against its base snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileDelta {
    changed: BTreeMap<String, SpanDelta>,
    gone: BTreeSet<String>,
}

impl FileDelta {
    #[must_use]
    fn between(
        base: &BTreeMap<String, FileSnapshot>, current: &BTreeMap<String, FileSnapshot>,
    ) -> Self {
        let mut changed = BTreeMap::new();
        for (file, cur) in current {
            match base.get(file) {
                None => {
                    changed.insert(file.clone(), SpanDelta::Whole);
                }
                Some(prior) if prior.sha256 != cur.sha256 => {
                    changed.insert(file.clone(), SpanDelta::between(prior, cur));
                }
                Some(_) => {}
            }
        }
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
        let parsed = ParsedAnchor::parse(path);
        self.changed.get(&parsed.file).is_some_and(|delta| delta.overlaps_current(parsed.span))
    }

    #[must_use]
    pub fn touches_citation(&self, cited: &Cited) -> bool {
        let Some(raw) = cited.path.as_deref() else {
            return false;
        };
        let parsed = ParsedAnchor::parse(raw);
        self.gone.contains(&parsed.file)
            || self.changed.get(&parsed.file).is_some_and(|delta| delta.overlaps_base(parsed.span))
    }
}

struct ParsedAnchor {
    file: String,
    span: Option<LineSpan>,
}

impl ParsedAnchor {
    fn parse(raw: &str) -> Self {
        let span =
            Anchor::parse(raw).ok().and_then(|anchor| anchor.lines).and_then(|(start, end)| {
                Some(LineSpan::new(usize::try_from(start).ok()?, usize::try_from(end).ok()?))
            });
        let file = shape::file(raw);
        let canonical = beneath(file).unwrap_or_else(|_| file.to_owned());
        Self {
            file: canonical,
            span,
        }
    }
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

async fn snapshot_source<F: Files>(
    files: &F, workspace: &Workspace, extract: &Extract, base: Option<&SnapshotBase>,
    cache: &mut ListingCache,
) -> Result<BTreeMap<String, FileSnapshot>, Error> {
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

    // snapshot each present file
    let mut snapshots = BTreeMap::new();
    for file in present {
        let bytes = files.read(workspace.at(), &workspace.resolve(&file)).await?;
        let entry = FileSnapshot::from_bytes(&file, &bytes);
        snapshots.insert(file, entry);
    }
    Ok(snapshots)
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
