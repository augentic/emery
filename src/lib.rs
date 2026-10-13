//! Implements the WebAssembly engine guest the shipped runtime embeds.
//!
//! The guest passes process arguments to the command interface and supplies
//! host-provided model, storage, version-control, source, target, and plugin
//! capabilities, so every external effect stays subject to the runtime's
//! grants. The one capability it answers itself, the engine's `Files`, reads
//! what a working copy holds at a path through the `.` preopen: a file's
//! bytes, or the target a symbolic link names, and what a directory holds.

#![cfg(target_arch = "wasm32")]

use std::future::Future;
use std::path::Path;
use std::{fs, io};

use emery_engine::{Entry, EntryKind, Files};
use omnia_sdk::api::command::{self, Response};
use omnia_sdk::{BlobStore, Error, Model, Plugins, StateStore, Vcs, server_error};
use wasip3::cli::environment;
use wasip3::exports::cli::run::Guest;

// Empty impls retain the WASI defaults selected by the runtime's grants.
struct Provider;

impl Model for Provider {}
impl StateStore for Provider {}
impl BlobStore for Provider {}
impl Plugins for Provider {}
impl Vcs for Provider {}
impl emery_adapter::source::Source for Provider {}
impl emery_adapter::target::Target for Provider {}

impl Files for Provider {
    fn read(&self, at: &str, path: &str) -> impl Future<Output = Result<Vec<u8>, Error>> + Send {
        let outcome = carried(&Path::new(at).join(path))
            .map_err(|error| server_error!("reading `{path}` under `{at}`: {error}"));
        async move { outcome }
    }

    fn list(&self, at: &str, path: &str) -> impl Future<Output = Result<Vec<Entry>, Error>> + Send {
        let outcome = listed(&Path::new(at).join(path))
            .map_err(|error| server_error!("listing `{path}` under `{at}`: {error}"));
        async move { outcome }
    }
}

// What a commit stores at each entry of the directory at `full`, a link
// told from what it leads to by the entry's own metadata.
fn listed(full: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(full)? {
        let entry = entry?;
        let kind = fs::symlink_metadata(entry.path())?.file_type();
        let kind = if kind.is_symlink() {
            EntryKind::Link
        } else if kind.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::File
        };
        entries.push(Entry {
            name: entry.file_name().to_string_lossy().into_owned(),
            kind,
        });
    }
    Ok(entries)
}

// What a commit stores at `full`: a symbolic link's target, never followed,
// or a regular file's bytes. Anything else is refused rather than read as
// empty. `std` on `wasm32` has no no-follow open, so the file read after the
// walk trusts it, as the SDK's writer does.
fn carried(full: &Path) -> io::Result<Vec<u8>> {
    let kind = fs::symlink_metadata(full)?.file_type();
    if kind.is_symlink() {
        Ok(fs::read_link(full)?.into_os_string().into_encoded_bytes())
    } else if kind.is_file() {
        fs::read(full)
    } else {
        Err(io::Error::other("neither a file nor a symbolic link stands there"))
    }
}

struct CliGuest;

wasip3::cli::command::export!(CliGuest);

impl Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        command::execute_wasi(dispatch()).await;
        Ok(())
    }
}

async fn dispatch() -> Response {
    emery_cli::run(Provider, environment::get_arguments()).await
}
