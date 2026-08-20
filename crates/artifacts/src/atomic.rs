//! Crash-safe writers shared by every `.emery/*.yaml` writer: write
//! to a temp file in the same parent, `sync_all`, then `persist`
//! (atomic rename) so readers never observe a partial write.

use std::path::Path;

use omnia_guest::Error;
use serde::Serialize;

use crate::{io, yaml};

/// Serialise `value` as YAML (with a guaranteed trailing newline) and
/// atomically persist it at `path`. See module-level docs for the
/// atomicity envelope.
///
/// # Errors
///
/// Propagates serialization and filesystem failures.
pub fn yaml_write<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    bytes_write(path, serialise_yaml(value)?.as_bytes())
}

/// Serialise `value` as a YAML document with a guaranteed single
/// trailing newline, returning the string rather than writing it.
///
/// # Errors
///
/// Returns an error when YAML serialization fails.
pub fn serialise_yaml<T: Serialize>(value: &T) -> Result<String, Error> {
    let mut content = serde_saphyr::to_string(value).map_err(yaml)?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(content)
}

/// Atomically write `bytes` to `path`. Used for non-YAML writers (e.g.
/// the PID stamp in `.emery/plan.lock`) where the caller has already
/// produced the exact on-disk bytes.
///
/// # Errors
///
/// Propagates directory, temporary-file, write, sync, and persist failures.
pub fn bytes_write(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(io)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(io)?;
    std::io::Write::write_all(tmp.as_file_mut(), bytes).map_err(io)?;
    tmp.as_file_mut().sync_all().map_err(io)?;
    tmp.persist(path).map_err(|err| io(err.error))?;
    Ok(())
}

/// Atomically copy `src` to `path`, streaming so the payload need not
/// fit in memory. Same crash-safety envelope as [`bytes_write`].
///
/// # Errors
///
/// Propagates directory, temporary-file, read, write, sync, and persist failures.
pub fn copy_write(path: &Path, src: &Path) -> Result<(), Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(io)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(io)?;
    let mut from = std::fs::File::open(src).map_err(io)?;
    std::io::copy(&mut from, tmp.as_file_mut()).map_err(io)?;
    tmp.as_file_mut().sync_all().map_err(io)?;
    tmp.persist(path).map_err(|err| io(err.error))?;
    Ok(())
}
