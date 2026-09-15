//! The turn an adapter puts to the model, and what each seam is lent.
//!
//! An adapter chooses only the [`Seam`]. The SDK owns the rest of the turn
//! — which source is being extracted, what the model may read, where the
//! reference documents are, and the fixed closing ask — so every adapter's
//! turn reads alike and the closing ask cannot drift.

use std::fmt::{self, Display, Formatter};

use emery_adapter::source::SourceContent;
use omnia_guest::{Error, bad_request, server_error};

use super::{Context, Seam};

// What one seam is lent: the directory the model receives, its path
// beneath the source root, and the files to mine relative to it.
#[derive(Debug)]
pub struct Lend {
    // The directory lent through the request's workspace grant; none for an
    // inline value.
    pub workspace: Option<String>,
    // The lend's path beneath the source root, empty when the root itself is
    // lent. Every `path` anchor the seam answers is re-rooted under it.
    pub within: String,
    // For `Files`, the files to mine relative to the lend, sorted and
    // deduped; empty otherwise.
    pub files: Vec<String>,
}

impl Lend {
    // What `seam` is lent under `ctx`. A `Files` path that escapes the
    // root, or a set naming no file, is `bad_request`; `Files` over an
    // inline value is the adapter's own defect, so `server_error`.
    pub fn of(seam: &Seam, ctx: &Context<'_>) -> Result<Self, Error> {
        let key = &ctx.input.key;
        let root = match (&ctx.input.content, seam) {
            (SourceContent::Workspace(root), _) => root,
            (SourceContent::Value(_), Seam::Files(_)) => {
                return Err(server_error!(
                    "`{key}`: a `Files` seam needs a workspace input, not an inline value"
                ));
            }
            (SourceContent::Value(_), _) => {
                return Ok(Self {
                    workspace: None,
                    within: String::new(),
                    files: Vec::new(),
                });
            }
        };

        let Seam::Files(paths) = seam else {
            return Ok(Self {
                workspace: Some(root.clone()),
                within: String::new(),
                files: Vec::new(),
            });
        };

        let mut files = Vec::with_capacity(paths.len());
        for path in paths {
            files.push(segments(key, path)?);
        }
        files.sort();
        files.dedup();
        let Some((first, rest)) = files.split_first() else {
            return Err(bad_request!("`{key}`: a `Files` seam names no file"));
        };

        // The longest prefix every file's directory shares is the lend.
        let mut ancestor = parent(first);
        for file in rest {
            let shared = ancestor.iter().zip(parent(file)).take_while(|(a, b)| a == b).count();
            ancestor = &ancestor[..shared];
        }

        let within = ancestor.join("/");
        let workspace = if within.is_empty() { root.clone() } else { format!("{root}/{within}") };
        let files = files.iter().map(|file| file[ancestor.len()..].join("/")).collect();

        Ok(Self {
            workspace: Some(workspace),
            within,
            files,
        })
    }
}

// A `Files` path as its segments: `/`-separated, with empty and `.`
// segments dropped. A leading `/` or a `..` is an escape from the root.
fn segments<'a>(key: &str, path: &'a str) -> Result<Vec<&'a str>, Error> {
    if path.starts_with('/') || path.split('/').any(|segment| segment == "..") {
        return Err(bad_request!("`{key}`: `{path}` escapes the source root"));
    }

    let segments: Vec<&str> =
        path.split('/').filter(|segment| !segment.is_empty() && *segment != ".").collect();
    if segments.is_empty() {
        return Err(bad_request!("`{key}`: `{path}` names no file"));
    }

    Ok(segments)
}

// A file's directory: its segments but the last.
const fn parent<'a, 'b>(file: &'a [&'b str]) -> &'a [&'b str] {
    match file.split_last() {
        Some((_, parent)) => parent,
        None => &[],
    }
}

// The brief: the call's context, the seam and what it is lent; rendered
// as the user turn.
pub struct Brief<'a> {
    pub ctx: &'a Context<'a>,
    pub seam: &'a Seam,
    pub lend: &'a Lend,
}

impl Display for Brief<'_> {
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
                    "`$SOURCE_DIR` is the read-only view at `{workspace}` — the part of the \
                     source tree this call mines. Mine these files beneath it and nothing else:",
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
