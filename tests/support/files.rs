//! Scripted `Files` capability recording working-copy reads, listings, and writes.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use emery_engine::{Entry, EntryKind, Files};
use omnia_sdk::{Error, server_error};

// `None` scripts a path the read fails at.
type Keyed = BTreeMap<(String, String), Option<Vec<u8>>>;
type Listings = BTreeMap<(String, String), Vec<Entry>>;
type Write = (String, String, Vec<u8>);

/// A scripted `Files` with a record of every read, listing, and write.
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
    /// Every write, in order: the working copy, the path, and the bytes.
    pub writes: Arc<Mutex<Vec<Write>>>,
}

impl FilesScript {
    /// Scripts what `path` beneath `at` reads as.
    pub fn holds(&self, at: &str, path: &str, content: &str) {
        self.holds_bytes(at, path, content.as_bytes());
    }

    /// Scripts what `path` beneath `at` reads as, byte for byte.
    pub fn holds_bytes(&self, at: &str, path: &str, content: &[u8]) {
        self.script(at, path, Some(content.to_vec()));
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

    /// Returns every write so far, in order.
    pub fn writes(&self) -> Vec<Write> {
        self.writes.lock().expect("writes").clone()
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

    fn write(
        &self, at: &str, path: &str, bytes: &[u8],
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.writes.lock().expect("writes").push((at.to_owned(), path.to_owned(), bytes.to_vec()));
        self.script(at, path, Some(bytes.to_vec()));
        async move { Ok(()) }
    }
}
