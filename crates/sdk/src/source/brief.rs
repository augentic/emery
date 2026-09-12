//! The evidence brief
//!
//! The one brief an adapter puts to the model: which source it is
//! extracting, the material it has been given, where the reference documents
//! are, and the fixed closing ask. Its `Display` is the user turn. An adapter
//! chooses only the [`Material`]; the SDK owns the envelope so every
//! adapter's brief reads alike and the closing ask cannot drift between
//! adapters.

use std::fmt::{self, Display, Formatter};

use emery_adapter::source::SourceContent;

use super::Context;

/// What the model is given to extract from.
#[derive(Debug)]
pub enum Material {
    /// The bound input itself: a lent workspace, described as this adapter's
    /// source tree, or an inline value quoted into the turn.
    Bound,
    /// A note the adapter prepared for a source that needs its own handling.
    Prepared(String),
}

// The brief: the adapter's source noun, the call's context, and the
// material; rendered as the user turn.
pub struct Brief<'a> {
    pub source: &'static str,
    pub ctx: &'a Context<'a>,
    pub material: &'a Material,
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
             Answer with one JSON object matching the gated Evidence schema. The caller persists \
             the document; do not write it yourself.",
        )
    }
}
