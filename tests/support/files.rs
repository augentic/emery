//! Scripts the `Files` capability and records every read and listing the engine makes.
//!
//! Content is scripted per working copy and path; an unscripted path reads as
//! an empty file, so a scenario scripts only the paths it wants the gate to
//! see, and a path scripted `unreadable` fails its read. A listing is
//! scripted per working copy and directory; an unscripted directory lists
//! nothing.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use emery_engine::{Entry, EntryKind, Files};
use omnia_sdk::{Error, server_error};

// `None` scripts a path the read fails at.
type Keyed = BTreeMap<(String, String), Option<Vec<u8>>>;
type Listings = BTreeMap<(String, String), Vec<Entry>>;

/// A scripted `Files` with a record of every read and listing.
#[derive(Clone, Debug, Default)]
pub struct FilesScript {
    /// What each path beneath a working copy reads as, keyed by both.
    pub contents: Arc<Mutex<Keyed>>,
    /// What each directory beneath a working copy lists as, keyed by both.
    pub listings: Arc<Mutex<Listings>>,
    /// Every read, in order: the working copy and the path.
    pub reads: Arc<Mutex<Vec<(String, String)>>>,
    /// Every listing, in order: the working copy and the directory.
    pub listed: Arc<Mutex<Vec<(String, String)>>>,
}

impl FilesScript {
    /// Scripts what `path` beneath `at` reads as.
    pub fn holds(&self, at: &str, path: &str, content: &str) {
        self.script(at, path, Some(content.as_bytes().to_vec()));
    }

    /// Scripts `path` beneath `at` as one whose read fails.
    pub fn unreadable(&self, at: &str, path: &str) {
        self.script(at, path, None);
    }

    /// Scripts what the directory `path` beneath `at` lists as: each entry's name and kind.
    pub fn lists(&self, at: &str, path: &str, entries: &[(&str, EntryKind)]) {
        let entries = entries
            .iter()
            .map(|(name, kind)| Entry {
                name: (*name).to_owned(),
                kind: *kind,
            })
            .collect();
        self.listings.lock().expect("listings").insert((at.to_owned(), path.to_owned()), entries);
    }

    /// Returns every read so far, in order.
    pub fn reads(&self) -> Vec<(String, String)> {
        self.reads.lock().expect("reads").clone()
    }

    /// Returns every listing so far, in order.
    pub fn listed(&self) -> Vec<(String, String)> {
        self.listed.lock().expect("listed").clone()
    }

    fn script(&self, at: &str, path: &str, content: Option<Vec<u8>>) {
        self.contents.lock().expect("contents").insert((at.to_owned(), path.to_owned()), content);
    }
}

impl Files for FilesScript {
    fn read(&self, at: &str, path: &str) -> impl Future<Output = Result<Vec<u8>, Error>> + Send {
        self.reads.lock().expect("reads").push((at.to_owned(), path.to_owned()));
        let scripted =
            self.contents.lock().expect("contents").get(&(at.to_owned(), path.to_owned())).cloned();
        let answer = match scripted {
            Some(Some(bytes)) => Ok(bytes),
            Some(None) => Err(server_error!("reading `{path}` under `{at}`: scripted unreadable")),
            None => Ok(Vec::new()),
        };
        async move { answer }
    }

    fn list(&self, at: &str, path: &str) -> impl Future<Output = Result<Vec<Entry>, Error>> + Send {
        self.listed.lock().expect("listed").push((at.to_owned(), path.to_owned()));
        let entries = self
            .listings
            .lock()
            .expect("listings")
            .get(&(at.to_owned(), path.to_owned()))
            .cloned()
            .unwrap_or_default();
        async move { Ok(entries) }
    }
}
