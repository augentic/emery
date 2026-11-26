//! Script mount helpers for adapter guests.

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use serde::de::DeserializeOwned;

use crate::script::{CALLS, MOUNT, STATE, ScriptCall};

/// Reads and parses `state.json` from the script mount.
///
/// # Panics
///
/// Panics when the state file is missing or invalid JSON.
#[must_use]
pub fn read_state<T: DeserializeOwned>() -> T {
    let path = mount(STATE);
    let bytes = fs::read(&path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|err| panic!("state JSON: {err}"))
}

/// Appends one dispatch record to `calls.jsonl`.
///
/// # Panics
///
/// Panics when the calls log cannot be opened or written.
pub fn append_call(call: &ScriptCall) {
    let path = mount(CALLS);
    let line = serde_json::to_string(call).expect("call serialises");
    let mut file =
        fs::OpenOptions::new().create(true).append(true).open(path).expect("open calls log");
    writeln!(file, "{line}").expect("append call");
}

/// Waits until every id in `expected` has touched the arrived directory, or times out.
///
/// # Panics
///
/// Panics when the arrived directory cannot be written or the barrier times out.
pub fn rendezvous_wait(key: &str, expected: &[String]) {
    if !expected.iter().any(|id| id == key) {
        return;
    }
    let arrived = mount("arrived");
    fs::create_dir_all(&arrived).expect("arrived dir");
    fs::write(arrived.join(sanitise(key)), []).expect("mark arrived");
    if expected.iter().all(|id| arrived.join(sanitise(id)).is_file()) {
        return;
    }
    for _ in 0..500_000 {
        if expected.iter().all(|id| arrived.join(sanitise(id)).is_file()) {
            return;
        }
        std::hint::spin_loop();
    }
    let missing: Vec<_> =
        expected.iter().filter(|id| !arrived.join(sanitise(id)).is_file()).cloned().collect();
    panic!(
        "`{key}` spun for parallel dispatches {missing:?} never arrived: the engine is running \
         them one at a time"
    );
}

fn mount(relative: &str) -> PathBuf {
    PathBuf::from(MOUNT).join(relative)
}

fn sanitise(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' { ch } else { '_' }
        })
        .collect()
}
