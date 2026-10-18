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
/// The base line is followed by the repository the label stands in when it
/// is not the project's own checkout, and by the `Initialise` commit the
/// run sealed the adapter's layout as, with its file count, when it sealed
/// one. A wave line names the head it was
/// verified at, and says `repaired` when its verification repaired the
/// tree, `sealed` when what it left was a regenerated path under the
/// adapter's `ours` rules alone, a lockfile. Each slice line beneath it
/// counts the requirements covered of those it holds, names any left
/// uncovered, counts the paths its build changed, names the merge commit
/// that brought them in, and the paths the model resolved its merge at; a
/// slice an earlier run staged in the integration working copy says so
/// instead, since this run verified it and built nothing.
pub fn build(output: &BuildOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "built revision {}", output.revision)?;
    waves(&output.waves, w)?;
    writeln!(w, "  base {}", output.base)?;
    if output.repository != "." {
        writeln!(w, "  repository {}", output.repository)?;
    }
    if let Some(initialised) = &output.initialised {
        writeln!(
            w,
            "  initialised {}, {}",
            initialised.commit.get(..8).unwrap_or(&initialised.commit),
            counted(initialised.paths.len(), "file")
        )?;
    }
    if !output.resumed.is_empty() {
        let ids: Vec<String> = output.resumed.iter().map(ToString::to_string).collect();
        writeln!(w, "  resumed: {}", ids.join(", "))?;
    }
    for verified in &output.verified {
        let wave = verified.wave;
        write!(
            w,
            "  wave {wave}: {} verified at {}",
            counted(verified.slices.len(), "slice"),
            verified.head.get(..8).unwrap_or(&verified.head)
        )?;
        if output.repaired.contains(&wave) {
            w.write_str(", repaired")?;
        } else if output.sealed.iter().any(|seal| seal.wave == wave) {
            w.write_str(", sealed")?;
        }
        writeln!(w)?;
        for id in &verified.slices {
            match output.slices.iter().find(|slice| slice.id == *id && slice.wave == wave) {
                Some(slice) => built(slice, w)?,
                None => writeln!(w, "    {id}: staged by an earlier run, not built again")?,
            }
        }
    }
    writeln!(w, "  labelled {} at {}", output.label, output.head)
}

fn built(slice: &BuiltSlice, w: &mut dyn fmt::Write) -> fmt::Result {
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
    if !slice.resolved.is_empty() {
        write!(w, ", resolved ({})", slice.resolved.join(", "))?;
    }
    writeln!(w)
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
