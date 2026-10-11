//! Scripts the `Files` capability and records every read the engine makes.
//!
//! Content is scripted per working copy and path; an unscripted path reads as
//! an empty file, so a scenario scripts only the paths it wants the gate to
//! see, and a path scripted `unreadable` fails its read.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use emery_engine::Files;
use omnia_sdk::{Error, server_error};

// `None` scripts a path the read fails at.
type Keyed = BTreeMap<(String, String), Option<Vec<u8>>>;

/// A scripted `Files` with a record of every read.
#[derive(Clone, Debug, Default)]
pub struct FilesScript {
    /// What each path beneath a working copy reads as, keyed by both.
    pub contents: Arc<Mutex<Keyed>>,
    /// Every read, in order: the working copy and the path.
    pub reads: Arc<Mutex<Vec<(String, String)>>>,
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

    /// Returns every read so far, in order.
    pub fn reads(&self) -> Vec<(String, String)> {
        self.reads.lock().expect("reads").clone()
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
}
