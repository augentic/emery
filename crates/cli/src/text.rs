//! Renders engine results as `--format text` output.

use std::fmt;

use emery_engine::build::{BuildOutput, BuiltSlice};
use emery_engine::show::ShowOutput;
use emery_engine::specify::{Diff, SpecifyOutput, Waves};

/// Writes a [`SpecifyOutput`] revision line, the plan's shape, one line per repository source, and a one-line diff summary.
pub fn specify(output: &SpecifyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "committed revision {}", output.revision)?;
    waves(&output.waves, w)?;
    for read in &output.repositories {
        writeln!(
            w,
            "  {} read from {} at {}: {}",
            read.source, read.repository, read.revision, read.commit
        )?;
    }
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

/// Writes a [`BuildOutput`] revision line, the plan's shape, the base, the slices resumed, each wave with the slices it merged, and the label.
///
/// A wave line names the head it was verified at, and says `repaired` when
/// its verification changed the tree. Each slice line beneath it counts the
/// requirements covered of those it holds, names any left uncovered, counts
/// the paths its build changed, names the merge commit that brought them in,
/// and the paths an earlier build of it conflicted at.
pub fn build(output: &BuildOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "built revision {}", output.revision)?;
    waves(&output.waves, w)?;
    writeln!(w, "  base {}", output.base)?;
    if !output.resumed.is_empty() {
        let ids: Vec<String> = output.resumed.iter().map(ToString::to_string).collect();
        writeln!(w, "  resumed: {}", ids.join(", "))?;
    }
    // a wave is verified when it integrated any slice, so the heads pair with
    // the waves the slices record, in order
    let mut numbered: Vec<usize> = output.slices.iter().map(|slice| slice.wave).collect();
    numbered.dedup();
    for (head, wave) in output.verified.iter().zip(numbered) {
        let merged: Vec<&BuiltSlice> =
            output.slices.iter().filter(|slice| slice.wave == wave).collect();
        write!(
            w,
            "  wave {wave}: {} verified at {}",
            counted(merged.len(), "slice"),
            head.get(..8).unwrap_or(head)
        )?;
        if output.repaired.contains(&wave) {
            w.write_str(", repaired")?;
        }
        writeln!(w)?;
        for slice in merged {
            let total = slice.covered.len() + slice.uncovered.len();
            write!(w, "    {} {}: covered {}/{total}", slice.id, slice.name, slice.covered.len())?;
            if !slice.uncovered.is_empty() {
                let ids: Vec<String> = slice.uncovered.iter().map(ToString::to_string).collect();
                write!(w, " (uncovered {})", ids.join(", "))?;
            }
            write!(w, ", written {}", counted(slice.written.len(), "file"))?;
            match &slice.commit {
                Some(commit) => write!(w, ", merged {commit}")?,
                None => write!(w, ", nothing to merge")?,
            }
            if !slice.conflicts.is_empty() {
                write!(w, ", conflicted ({})", slice.conflicts.join(", "))?;
            }
            writeln!(w)?;
        }
    }
    writeln!(w, "  labelled {} at {}", output.label, output.head)?;
    if let Some(remote) = &output.pushed {
        writeln!(w, "  pushed to {remote}")?;
    }
    Ok(())
}

// `  plan: 4 slices in 3 waves, widest 2`: how many slices the plan holds,
// how many waves they fall into, and the most ready at once.
fn waves(waves: &Waves, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(
        w,
        "  plan: {} in {}, widest {}",
        counted(waves.slices(), "slice"),
        counted(waves.len(), "wave"),
        waves.widest()
    )
}

fn counted(count: usize, noun: &str) -> String {
    if count == 1 { format!("1 {noun}") } else { format!("{count} {noun}s") }
}

/// Writes a [`ShowOutput`] document body alone.
///
/// No status line or revision identifier is added, allowing the Markdown to be
/// piped directly to another command.
pub fn show(output: &ShowOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    w.write_str(&output.body)
}
