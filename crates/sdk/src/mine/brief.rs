//! The turn an adapter puts to the model, and what each seam is lent.
//!
//! An adapter chooses only the [`Seam`]. The SDK owns the rest of the turn
//! — which source is being extracted, what the model may read, where the
//! reference documents are, and the fixed closing ask — so every adapter's
//! turn reads alike and the closing ask cannot drift.

use std::fmt::{self, Display, Formatter};

use emery_adapter::source::{SourceContent, SourceInput};
use omnia_sdk::{Error, bad_request, server_error};

use super::{Context, Seam};
use crate::path;

// What one seam is lent: the root the model receives, and the files to mine
// relative to it.
#[derive(Debug)]
pub struct Lend {
    // The source root, lent through the request's workspace grant; none for
    // an inline value.
    pub workspace: Option<String>,
    // For `Files`, the files to mine relative to the root, sorted and
    // deduped; empty otherwise.
    pub files: Vec<String>,
}

impl Lend {
    // What `seam` is lent of `input`. A `Files` path that escapes the
    // root, or a set naming no file, is `bad_request`; `Files` over an
    // inline value is the adapter's own defect, so `server_error`.
    pub fn of(seam: &Seam, input: &SourceInput) -> Result<Self, Error> {
        let key = &input.key;
        let root = match (&input.content, seam) {
            (SourceContent::Workspace(root), _) => root,
            (SourceContent::Value(_), Seam::Files(_)) => {
                return Err(server_error!(
                    "`{key}`: a `Files` seam needs a workspace input, not an inline value"
                ));
            }
            (SourceContent::Value(_), _) => {
                return Ok(Self {
                    workspace: None,
                    files: Vec::new(),
                });
            }
        };

        let Seam::Files(paths) = seam else {
            return Ok(Self {
                workspace: Some(root.clone()),
                files: Vec::new(),
            });
        };

        let mut files = Vec::with_capacity(paths.len());
        for named in paths {
            let file = path::beneath(named)
                .map_err(|reason| bad_request!("`{key}`: `{named}` {reason}"))?;
            files.push(file);
        }
        files.sort();
        files.dedup();
        if files.is_empty() {
            return Err(bad_request!("`{key}`: a `Files` seam names no file"));
        }

        Ok(Self {
            workspace: Some(root.clone()),
            files,
        })
    }
}

// The brief: the call's context, the seam and what it is lent; rendered
// as the user turn.
pub struct Brief<'a, P> {
    pub ctx: &'a Context<'a, P>,
    pub seam: &'a Seam,
    pub lend: &'a Lend,
}

impl<P> Display for Brief<'_, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let input = self.ctx.input;
        write!(
            f,
            "Extract the claim set of the source bound to adapter `{id}` (source key `{key}`).\n\n",
            id = self.ctx.adapter_id,
            key = input.key,
        )?;

        match (self.seam, &input.content) {
            (Seam::Note(note), _) => f.write_str(note)?,
            (Seam::Files(_), _) => {
                writeln!(
                    f,
                    "`$SOURCE_DIR` is the read-only view at `{workspace}` — the source tree. Mine \
                     these files beneath it and nothing else:",
                    workspace = self.lend.workspace.as_deref().unwrap_or_default(),
                )?;
                for file in &self.lend.files {
                    write!(f, "\n- `{file}`")?;
                }
                f.write_str(
                    "\n\nAnchor every `path` relative to `$SOURCE_DIR`. Nothing outside it is \
                     reachable; extract mines only this source.",
                )?;
            }
            (Seam::Whole, SourceContent::Workspace(root)) => write!(
                f,
                "`$SOURCE_DIR` is the read-only view at `{root}` — the source tree the prompt \
                 walks. Nothing outside it is reachable; extract mines only this source."
            )?,
            (Seam::Whole, SourceContent::Value(value)) => write!(
                f,
                "The bound seam is this inline value; no `$SOURCE_DIR` is lent:\n\n{value}\n\n\
                 Nothing else is reachable; extract mines only this source."
            )?,
        }

        f.write_str(
            "\n\nThe prompt's references are available through this call's `read_doc` tool \
             (`list_docs` enumerates them); load referenced bodies on demand.\n\n\
             Answer with one JSON object matching the gated claims schema. The caller persists \
             the document; do not write it yourself.",
        )
    }
}
