//! Renders engine results as `--format text` output.

use std::fmt;

use emery_engine::amend::AmendOutput;
use emery_engine::apply::ApplyOutput;
use emery_engine::build::{BuildOutput, BuiltSlice};
use emery_engine::show::ShowOutput;
use emery_engine::specify::{Diff, SpecifyOutput, Waves};

// Writes a `SpecifyOutput` candidate line, the plan's shape, one line per repository source, a one-line diff summary, and what the run sealed.
//
// The first line names the staged candidate. The last says `nothing
// sealed` and names `emery apply` as the way to seal it, or, under
// `--apply`, is the `apply` line for the revision the run sealed.
pub fn specify(output: &SpecifyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "staged candidate {}", output.revision)?;
    waves(&output.waves, w)?;
    for read in &output.repositories {
        writeln!(
            w,
            "  {} read from {} at {}: {}",
            read.source, read.repository, read.revision, read.commit
        )?;
    }
    diff(output.diff.as_ref(), w)?;
    match &output.applied {
        Some(applied) => {
            write!(w, "  ")?;
            sealed(applied, w)
        }
        None => writeln!(w, "  nothing sealed; `emery apply` seals it"),
    }
}

// Writes an `ApplyOutput` revision line, the repository it was sealed in, and a one-line diff summary.
//
// The first line names the revision, its label, and the commit it stands
// at. The repository follows when it is not the project's own checkout.
pub fn apply(output: &ApplyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    sealed(output, w)?;
    if output.repository != "." {
        writeln!(w, "  repository {}", output.repository)?;
    }
    diff(output.diff.as_ref(), w)
}

// Writes an `AmendOutput` amendment line, the base, the counts by class, the conflicts, the stems touched, and one line per repository source.
//
// The first line names the staged amendment, or says no change was found
// against the base and nothing staged. The counts line is the one
// `amendment.md` opens with. The conflicts and stems lines appear when
// there are any, and the last line names `emery show amendment` as the way
// to review what was staged.
pub fn amend(output: &AmendOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    match &output.amendment {
        Some(id) => writeln!(w, "staged amendment {id}")?,
        None => writeln!(w, "nothing staged: no change against the current revision")?,
    }
    writeln!(
        w,
        "  base {} at {}",
        output.base.revision,
        output.base.commit.get(..8).unwrap_or(&output.base.commit)
    )?;
    let tally = &output.tally;
    writeln!(
        w,
        "  {} added, {} changed, {} retired, {} conflict, {} unchanged",
        tally.added, tally.changed, tally.retired, tally.conflict, tally.unchanged
    )?;
    if !output.conflicts.is_empty() {
        writeln!(w, "  conflicts: {}", output.conflicts.join(", "))?;
    }
    if !output.stems.is_empty() {
        writeln!(w, "  stems: {}", output.stems.join(", "))?;
    }
    for read in &output.repositories {
        writeln!(
            w,
            "  {} read from {} at {}: {}",
            read.source, read.repository, read.revision, read.commit
        )?;
    }
    if output.amendment.is_some() {
        writeln!(w, "  nothing sealed; `emery show amendment` reviews it")?;
    }
    Ok(())
}

fn sealed(output: &ApplyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(
        w,
        "sealed revision {} as {} at {}",
        output.revision,
        output.label,
        output.commit.get(..8).unwrap_or(&output.commit)
    )
}

fn diff(diff: Option<&Diff>, w: &mut dyn fmt::Write) -> fmt::Result {
    let Some(diff) = diff else {
        return Ok(());
    };
    if diff.is_empty() {
        writeln!(w, "  diff vs {}: none (byte-stable)", diff.from)
    } else {
        write!(w, "  diff vs {}: ", diff.from)?;
        summary(diff, w)?;
        writeln!(w)
    }
}

// `spec +1 -0 ~1 retired 1 preamble, design +0 -0 ~1, plan +1 -0 ~1`: the
// counts per document, `retired` after the specification's when any
// requirement was withdrawn, `preamble` when a document's changed.
fn summary(diff: &Diff, w: &mut dyn fmt::Write) -> fmt::Result {
    let (spec, design, plan) = (&diff.spec, &diff.design, &diff.plan);
    let documents = [
        (
            "spec",
            spec.added.len(),
            spec.removed.len(),
            spec.changed.len(),
            spec.retired.len(),
            spec.preamble,
        ),
        (
            "design",
            design.added.len(),
            design.removed.len(),
            design.changed.len(),
            0,
            design.preamble,
        ),
        ("plan", plan.added.len(), plan.removed.len(), plan.changed.len(), 0, plan.preamble),
    ];
    for (position, (name, added, removed, changed, retired, preamble)) in
        documents.into_iter().enumerate()
    {
        if position > 0 {
            w.write_str(", ")?;
        }
        write!(w, "{name} +{added} -{removed} ~{changed}")?;
        if retired > 0 {
            write!(w, " retired {retired}")?;
        }
        if preamble {
            w.write_str(" preamble")?;
        }
    }
    Ok(())
}

// Writes a `BuildOutput` revision line, the plan's shape, the base, the slices resumed, each wave with the slices it merged, and the label.
//
// The base line is followed by the repository the label stands in when it
// is not the project's own checkout, and by the `Initialise` commit the
// run sealed the adapter's layout as, with its file count, when it sealed
// one. A wave line names the head it was
// verified at, and says `repaired` when its verification repaired the
// tree, `sealed` when what it left was a regenerated path under the
// adapter's `ours` rules alone, a lockfile. Each slice line beneath it
// counts the requirements covered of those it holds, names any left
// uncovered, counts the paths its build changed, names the merge commit
// that brought them in, and the paths the model resolved its merge at; a
// slice an earlier run staged in the integration working copy says so
// instead, since this run verified it and built nothing. A candidate
// `emery specify` staged and nothing sealed is named after the plan line,
// since the build read the current revision and not it.
pub fn build(output: &BuildOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "built revision {}", output.revision)?;
    waves(&output.waves, w)?;
    if let Some(candidate) = &output.candidate {
        writeln!(w, "  candidate {candidate} is staged and not sealed; `emery apply` seals it")?;
    }
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

// Writes a `ShowOutput` document body alone.
//
// No status line or revision identifier is added, allowing the Markdown to be
// piped directly to another command.
pub fn show(output: &ShowOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    w.write_str(&output.body)
}
