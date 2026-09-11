//! Text output
//!
//! The human-readable rendering of each command result. JSON output falls
//! out of the result types' `Serialize` derives; text output needs a hand
//! written shape per result, and those shapes live here as the render fns
//! the command projector encodes `--format text` through.
//!
//! Keeping text rendering apart from the engine's result types lets the
//! terminal presentation follow the Developer Guide's output conventions
//! without those conventions leaking into the engine.

use std::fmt;

use emery_engine::show::{Artifact, ShowOutput};
use emery_engine::specify::{Diff, SpecifyOutput};

/// Writes the `specify` result: the committed-revision line and its indented
/// detail.
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

// Writes one line per changed requirement and section, prefixed by the
// projection it appears in.
fn changes(diff: &Diff, w: &mut dyn fmt::Write) -> fmt::Result {
    let spec = Artifact::Spec.projection();
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

    let design = Artifact::Design.projection();
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

/// Writes the document body alone — a deliberate exception to the
/// result-line convention so `emery show spec` pipes cleanly; the revision id
/// rides the JSON envelope.
pub fn show(output: &ShowOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    w.write_str(&output.body)
}
