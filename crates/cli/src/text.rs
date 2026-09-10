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

use emery_engine::show::{Document, ShowOutput};
use emery_engine::specify::{Changes, SpecifyOutput};

/// Writes the `specify` result: the committed-revision line and its indented
/// detail.
pub fn specify(output: &SpecifyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "committed revision {}", output.revision)?;
    if let Some(diff) = &output.diff {
        if diff.is_empty() {
            writeln!(w, "  diff vs {}: none (byte-stable)", diff.from)?;
        } else {
            writeln!(w, "  diff vs {}: {}", diff.from, diff.artifacts.join(", "))?;
            changes(w, Document::Spec, &diff.spec)?;
            changes(w, Document::Design, &diff.design)?;
        }
    }
    Ok(())
}

// Writes one line per changed section, prefixed by its document.
fn changes(w: &mut dyn fmt::Write, document: Document, changes: &Changes) -> fmt::Result {
    let document = document.file();
    for heading in &changes.added {
        writeln!(w, "    {document} + {heading}")?;
    }
    for heading in &changes.removed {
        writeln!(w, "    {document} - {heading}")?;
    }
    for heading in &changes.changed {
        writeln!(w, "    {document} ~ {heading}")?;
    }
    Ok(())
}

/// Writes the document body alone — a deliberate exception to the
/// result-line convention so `emery show spec` pipes cleanly; the revision id
/// rides the JSON envelope.
pub fn show(output: &ShowOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    w.write_str(&output.body)
}
