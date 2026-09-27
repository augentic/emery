//! Renders engine results as `--format text` output.

use std::fmt;

use emery_engine::show::ShowOutput;
use emery_engine::specify::{Diff, SpecifyOutput};

/// Writes a [`SpecifyOutput`] revision line with a one-line diff summary.
pub fn specify(output: &SpecifyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "committed revision {}", output.revision)?;
    if let Some(diff) = &output.diff {
        if diff.from == output.revision {
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
    write!(
        w,
        "spec +{} -{} ~{}",
        diff.spec.added.len(),
        diff.spec.removed.len(),
        diff.spec.changed.len()
    )?;
    if diff.spec.preamble {
        write!(w, " preamble")?;
    }
    write!(
        w,
        ", design +{} -{} ~{}",
        diff.design.added.len(),
        diff.design.removed.len(),
        diff.design.changed.len()
    )?;
    if diff.design.preamble {
        write!(w, " preamble")?;
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
