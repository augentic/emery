//! Scripts the `Files` capability and records every read the engine makes.
//!
//! Content is scripted per working copy and path; an unscripted path reads as
//! absent, so a scenario scripts only the files it wants the gate to see.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

use emery_engine::Files;
use omnia_sdk::Error;

type Keyed = BTreeMap<(String, String), Vec<u8>>;

/// A scripted `Files` with a record of every read.
#[derive(Clone, Debug, Default)]
pub struct FilesScript {
    /// File content keyed by working copy and path beneath it.
    pub contents: Arc<Mutex<Keyed>>,
    /// Every read, in order: the working copy and the path.
    pub reads: Arc<Mutex<Vec<(String, String)>>>,
}

impl FilesScript {
    /// Scripts what `path` beneath `at` reads as.
    pub fn holds(&self, at: &str, path: &str, content: &str) {
        self.contents
            .lock()
            .expect("contents")
            .insert((at.to_owned(), path.to_owned()), content.as_bytes().to_vec());
    }

    /// Returns every read so far, in order.
    pub fn reads(&self) -> Vec<(String, String)> {
        self.reads.lock().expect("reads").clone()
    }
}

impl Files for FilesScript {
    fn read(
        &self, at: &str, path: &str,
    ) -> impl Future<Output = Result<Option<Vec<u8>>, Error>> + Send {
        self.reads.lock().expect("reads").push((at.to_owned(), path.to_owned()));
        let answer =
            self.contents.lock().expect("contents").get(&(at.to_owned(), path.to_owned())).cloned();
        async move { Ok(answer) }
    }
}
