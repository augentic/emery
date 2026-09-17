//! Renders engine results as command-line text.
//!
//! Each function writes the `--format text` representation of one command
//! result. Structured output is provided by the result types' Serde
//! implementations.

use std::fmt;

use emery_engine::show::{Artifact, ShowOutput};
use emery_engine::specify::{Diff, SpecifyOutput};

/// Writes a [`SpecifyOutput`] revision line with its [`Diff`] beneath it.
pub fn specify(output: &SpecifyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "committed revision {}", output.revision)?;
    if let Some(diff) = &output.diff {
        if diff.from == output.revision {
            writeln!(w, "  diff vs {}: none (byte-stable)", diff.from)?;
        } else {
            writeln!(w, "  diff vs {}:", diff.from)?;
            changes(diff, w)?;
        }
    }
    Ok(())
}

// Writes one line per changed preamble, requirement, and section, prefixed
// by the projection it appears in.
fn changes(diff: &Diff, w: &mut dyn fmt::Write) -> fmt::Result {
    let spec = format!("{}.md", Artifact::Spec.as_ref());
    if diff.spec.preamble {
        writeln!(w, "    {spec} ~ preamble")?;
    }
    for entry in &diff.spec.added {
        writeln!(w, "    {spec} + {} {}", entry.id, entry.subject)?;
    }
    for entry in &diff.spec.removed {
        writeln!(w, "    {spec} - {} {}", entry.id, entry.subject)?;
    }
    for changed in &diff.spec.changed {
        let entry = &changed.requirement;
        let fields = changed.fields.join(", ");
        writeln!(w, "    {spec} ~ {} {}: {fields}", entry.id, entry.subject)?;
    }

    let design = format!("{}.md", Artifact::Design.as_ref());
    if diff.design.preamble {
        writeln!(w, "    {design} ~ preamble")?;
    }
    for kind in &diff.design.added {
        writeln!(w, "    {design} + {}", kind.as_ref())?;
    }
    for kind in &diff.design.removed {
        writeln!(w, "    {design} - {}", kind.as_ref())?;
    }
    for kind in &diff.design.changed {
        writeln!(w, "    {design} ~ {}", kind.as_ref())?;
    }
    Ok(())
}

/// Writes a [`ShowOutput`] document body alone.
///
/// No status line or revision identifier is added, allowing the Markdown to be
/// piped directly to another command.
pub fn show(output: &ShowOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    w.write_str(&output.body)
}
