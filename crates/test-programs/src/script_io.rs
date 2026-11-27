//! Script mount helpers for adapter guests.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use wasip3::clocks::monotonic_clock;

use crate::script::{
    ARRIVED, CALLS_DIR, FAILURES_DIR, MOUNT, STATE, ScriptCall, VERIFIES_DIR, sanitise_id,
};

const POLL_NS: u64 = 2_000_000;
const TIMEOUT_NS: u64 = 15_000_000_000;

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

/// Records one adapter dispatch under its own numbered JSON file in `calls/`.
///
/// # Panics
///
/// Panics when the dispatch record cannot be written.
pub fn append_call(call: &ScriptCall) {
    let index = claim_slot(CALLS_DIR);
    let dir = mount(CALLS_DIR);
    let staged = dir.join(format!("{index:06}.tmp"));
    let path = dir.join(format!("{index:06}.json"));
    let bytes = serde_json::to_vec(call).expect("call serialises");
    fs::write(&staged, bytes).expect("write call log");
    fs::rename(&staged, path).expect("place call log");
}

/// Claims the zero-based dispatch index for `slice_id` within the current run.
///
/// # Panics
///
/// Panics when the failure-slot directory cannot be updated.
#[must_use]
pub fn claim_build_attempt(slice_id: &str) -> usize {
    claim_slot(&format!("{FAILURES_DIR}/{}", sanitise_id(slice_id)))
}

/// Claims the zero-based verify index within the current run.
///
/// # Panics
///
/// Panics when the verify-slot directory cannot be updated.
#[must_use]
pub fn claim_verify_attempt() -> usize {
    claim_slot(VERIFIES_DIR)
}

/// Waits until every id in `expected` has touched the arrived directory, or times out.
///
/// # Panics
///
/// Panics when the arrived directory cannot be written or the barrier times out.
pub async fn rendezvous_wait(key: &str, expected: &BTreeSet<String>) {
    if !expected.contains(key) {
        return;
    }
    let arrived = mount(ARRIVED);
    let _ = fs::create_dir_all(&arrived);
    fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(arrived.join(sanitise_id(key)))
        .expect("mark arrived");
    let all_arrived = || expected.iter().all(|id| arrived.join(sanitise_id(id)).is_file());
    if all_arrived() {
        return;
    }
    let deadline = monotonic_clock::now().saturating_add(TIMEOUT_NS);
    while monotonic_clock::now() < deadline {
        monotonic_clock::wait_for(POLL_NS).await;
        if all_arrived() {
            return;
        }
    }
    let missing: Vec<_> =
        expected.iter().filter(|id| !arrived.join(sanitise_id(id)).is_file()).cloned().collect();
    panic!(
        "`{key}` waited for parallel dispatches {missing:?} that never arrived: the engine is \
         running them one at a time"
    );
}

/// Suspends the current guest task forever on a WASI waitable until cancelled.
pub async fn hold_forever() -> ! {
    loop {
        monotonic_clock::wait_for(TIMEOUT_NS).await;
    }
}

fn claim_slot(relative_dir: &str) -> usize {
    let dir = mount(relative_dir);
    let _ = fs::create_dir_all(&dir);
    for index in 0usize.. {
        let slot = dir.join(format!("{index:06}.slot"));
        match fs::OpenOptions::new().write(true).create_new(true).open(&slot) {
            Ok(_) => return index,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
            Err(err) => panic!("claiming slot {}: {err}", slot.display()),
        }
    }
    unreachable!("slot sequence is unbounded")
}

fn mount(relative: &str) -> PathBuf {
    PathBuf::from(MOUNT).join(relative)
}
