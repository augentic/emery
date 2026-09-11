//! The carried master
//!
//! Finds the master a project keeps beside its code: `.emery/spec.json` and
//! `.emery/design.json`, each the `emery show <document> --format json`
//! envelope of one revision. Like the project-root `emery.toml`, the pair is
//! discovered on every `specify` with no flag, so a project that carries its
//! master continues it and one that does not starts from the store.
//!
//! The pair travels together: one file without the other, or a file that is
//! not a `show` envelope, is refused as `master-invalid` rather than half-read,
//! so a run never continues a master it did not fully see.

use std::path::{Path, PathBuf};

use emery_engine::show::Document;
use emery_engine::specify::Carried;
use omnia_guest::{Error, server_error};
use serde_json::Value;

/// The project directory carrying the master envelopes.
pub const DIR: &str = ".emery";

/// Discovers the carried master: both envelopes' `document` values, or
/// `None` when the project carries neither.
///
/// # Errors
///
/// `master-invalid` when only one envelope is present or a file is not a
/// `show --format json` envelope; `server_error` when a file cannot be read.
pub fn discover() -> Result<Option<Carried>, Error> {
    let spec = envelope(Document::Spec)?;
    let design = envelope(Document::Design)?;

    match (spec, design) {
        (None, None) => Ok(None),
        (Some(spec), Some(design)) => Ok(Some(Carried { spec, design })),
        (spec, _) => {
            let (present, missing) = if spec.is_some() {
                (Document::Spec, Document::Design)
            } else {
                (Document::Design, Document::Spec)
            };
            Err(invalid(format!(
                "`{}` is present but `{}` is not; the pair travels together",
                path(present).display(),
                path(missing).display()
            )))
        }
    }
}

// Reads one envelope's `document`, or `None` when the file is absent.
fn envelope(document: Document) -> Result<Option<Value>, Error> {
    let path = path(document);
    let shown = path.display();
    if !path.try_exists().map_err(|e| server_error!("reading {shown}: {e}"))? {
        return Ok(None);
    }

    let raw = std::fs::read(&path).map_err(|e| server_error!("reading {shown}: {e}"))?;
    let envelope: Envelope = serde_json::from_slice(&raw).map_err(|err| {
        invalid(format!("{shown} is not an `emery show --format json` envelope: {err}"))
    })?;

    Ok(Some(envelope.document))
}

// The one envelope field the master rides in; the rest is the run's.
#[derive(serde::Deserialize)]
struct Envelope {
    document: Value,
}

fn path(document: Document) -> PathBuf {
    Path::new(DIR).join(document.file())
}

fn invalid(description: String) -> Error {
    Error::BadRequest {
        code: "master-invalid".into(),
        description,
    }
}
