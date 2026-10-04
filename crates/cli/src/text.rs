//! Renders engine results as `--format text` output.

use std::fmt;

use emery_engine::build::BuildOutput;
use emery_engine::show::ShowOutput;
use emery_engine::specify::{Diff, SpecifyOutput};

/// Writes a [`SpecifyOutput`] revision line with a one-line diff summary.
pub fn specify(output: &SpecifyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "committed revision {}", output.revision)?;
    if let Some(diff) = &output.diff {
        if diff.is_empty() {
            writeln!(w, "  diff vs {}: none (byte-stable)", diff.from)?;
        } else {
            write!(w, "  diff vs {}: ", diff.from)?;
            summary(diff, w)?;
            writeln!(w)?;
        }
    }
    Ok(())
}

fn summary(diff: &Diff, w: &mut dyn fmt::Write) -> fmt::Result {
    let (spec, design, plan) = (&diff.spec, &diff.design, &diff.plan);
    let documents = [
        ("spec", spec.added.len(), spec.removed.len(), spec.changed.len(), spec.preamble),
        ("design", design.added.len(), design.removed.len(), design.changed.len(), design.preamble),
        ("plan", plan.added.len(), plan.removed.len(), plan.changed.len(), plan.preamble),
    ];
    for (position, (name, added, removed, changed, preamble)) in documents.into_iter().enumerate() {
        if position > 0 {
            w.write_str(", ")?;
        }
        write!(w, "{name} +{added} -{removed} ~{changed}")?;
        if preamble {
            w.write_str(" preamble")?;
        }
    }
    Ok(())
}

/// Writes a [`BuildOutput`] revision line, then one line per slice built.
///
/// Each slice line counts the requirements covered of those it holds, names
/// any left uncovered, and counts the files written.
pub fn build(output: &BuildOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "built revision {}", output.revision)?;
    for slice in &output.slices {
        let total = slice.covered.len() + slice.uncovered.len();
        write!(w, "  {} {}: covered {}/{total}", slice.id, slice.name, slice.covered.len())?;
        if !slice.uncovered.is_empty() {
            let ids: Vec<String> = slice.uncovered.iter().map(ToString::to_string).collect();
            write!(w, " (uncovered {})", ids.join(", "))?;
        }
        let files = if slice.written.len() == 1 { "file" } else { "files" };
        writeln!(w, ", written {} {files}", slice.written.len())?;
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
