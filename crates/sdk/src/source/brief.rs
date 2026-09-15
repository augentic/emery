//! The evidence brief
//!
//! The one brief an adapter puts to the model: which source it is
//! extracting, the material it has been given and what that material is
//! lent, where the reference documents are, and the fixed closing ask. Its
//! `Display` is the user turn. An adapter chooses only the [`Material`]; the
//! SDK owns the envelope so every adapter's brief reads alike and the closing
//! ask cannot drift between adapters.

use std::fmt::{self, Display, Formatter};

use emery_adapter::source::SourceContent;
use omnia_guest::{Error, bad_request, server_error};

use super::Context;

/// What the model is given to extract from.
#[derive(Debug, Eq, PartialEq)]
pub enum Material {
    /// The bound input itself: a lent workspace, described as this adapter's
    /// source tree, or an inline value quoted into the turn.
    Bound,
    /// A note the adapter prepared for a source that needs its own handling.
    Prepared(String),
    /// Files beneath the input's root, named relative to it. The lend is the
    /// files' common ancestor — one directory, enforced by the grant — and
    /// only a scattered set lends the root itself, with the paths stated.
    /// `path` anchors come back relative to the lend and are re-rooted under
    /// the source root when the materials are joined. The paths are sorted
    /// and deduped; one that escapes the root is refused.
    Within(Vec<String>),
}

// What one material is lent: the directory the model receives, its path
// beneath the source root, and the files to mine relative to it.
#[derive(Debug)]
pub struct Lend {
    // The directory lent through the request's workspace grant; none for an
    // inline value.
    pub workspace: Option<String>,
    // The lend's path beneath the source root, empty when the root itself is
    // lent. Every `path` anchor the material answers is re-rooted under it.
    pub within: String,
    // For `Within`, the files to mine relative to the lend, sorted and
    // deduped; empty otherwise.
    pub files: Vec<String>,
}

impl Lend {
    // What `material` is lent under `ctx`. A `Within` path that escapes the
    // root, or a set naming no file, is `bad_request`; `Within` over an
    // inline value is the adapter's own defect, so `server_error`.
    pub fn of(material: &Material, ctx: &Context<'_>) -> Result<Self, Error> {
        let key = &ctx.input.key;
        let root = match (&ctx.input.content, material) {
            (SourceContent::Workspace(root), _) => root,
            (SourceContent::Value(_), Material::Within(_)) => {
                return Err(server_error!(
                    "`{key}`: a `Within` material needs a workspace input, not an inline value"
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

        let Material::Within(paths) = material else {
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
            return Err(bad_request!("`{key}`: a `Within` material names no file"));
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

// A `Within` path as its segments: `/`-separated, with empty and `.`
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

// The brief: the adapter's source noun, the call's context, the material
// and what it is lent; rendered as the user turn.
pub struct Brief<'a> {
    pub source: &'static str,
    pub ctx: &'a Context<'a>,
    pub material: &'a Material,
    pub lend: &'a Lend,
}

impl Display for Brief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let input = self.ctx.input;
        write!(
            f,
            "Extract the claim set of the {source} source bound to adapter `{id}` (source key \
             `{key}`).\n\n",
            source = self.source,
            id = self.ctx.adapter_id,
            key = input.key,
        )?;

        match (self.material, &input.content) {
            (Material::Prepared(note), _) => f.write_str(note)?,
            (Material::Within(_), _) => {
                writeln!(
                    f,
                    "`$SOURCE_DIR` is the read-only view at `{workspace}` — the part of the \
                     {source} source tree this call mines. Mine these files beneath it and \
                     nothing else:",
                    workspace = self.lend.workspace.as_deref().unwrap_or_default(),
                    source = self.source,
                )?;
                for file in &self.lend.files {
                    write!(f, "\n- `{file}`")?;
                }
                f.write_str(
                    "\n\nAnchor every `path` relative to `$SOURCE_DIR`. Nothing outside it is \
                     reachable; extract mines only this source.",
                )?;
            }
            (Material::Bound, SourceContent::Workspace(root)) => write!(
                f,
                "`$SOURCE_DIR` is the read-only view at `{root}` — the {source} source tree the \
                 prompt walks. Nothing outside it is reachable; extract mines only this source.",
                source = self.source,
            )?,
            (Material::Bound, SourceContent::Value(value)) => write!(
                f,
                "The bound material is this inline value; no `$SOURCE_DIR` is lent:\n\n{value}\n\n\
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
