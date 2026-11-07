//! Renders verb outputs as `--format text` stdout.

use std::collections::BTreeSet;
use std::fmt;

use omnia_sdk::api::command::Render;

use crate::amend::{AmendOutput, SliceChange, SliceClass};
use crate::apply::ApplyOutput;
use crate::build::{BuildOutput, BuiltSlice, Candidate, StagedKind};
use crate::show::ShowOutput;
use crate::specify::{Diff, RepositorySource, SectionKind, SpecifyOutput, Waves};

fn specify(output: &SpecifyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    write!(w, "staged candidate {}", output.revision)?;
    if output.fresh {
        w.write_str(" (fresh)")?;
    }
    writeln!(w)?;
    waves(&output.waves, w)?;
    repositories(&output.repositories, w)?;
    diff(output.diff.as_ref(), w)?;
    match &output.applied {
        Some(applied) => sealed(applied, "  ", w),
        None => writeln!(w, "  nothing sealed; `emery apply` seals it"),
    }
}

fn apply(output: &ApplyOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    sealed(output, "", w)?;
    if output.repository != "." {
        writeln!(w, "  repository {}", output.repository)?;
    }
    diff(output.diff.as_ref(), w)
}

fn amend(output: &AmendOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    match &output.amendment {
        Some(id) => writeln!(w, "staged amendment {id}")?,
        None => writeln!(w, "nothing staged: no change against the current revision")?,
    }
    writeln!(w, "  base {} at {}", output.base.revision, short_commit(&output.base.commit))?;
    let tally = &output.tally;
    write!(w, "  {} added, {} changed, {} retired", tally.added, tally.changed, tally.retired)?;
    if tally.by_scope > 0 {
        write!(w, " ({} by scope)", tally.by_scope)?;
    }
    writeln!(w, ", {} conflict, {} unchanged", tally.conflict, tally.unchanged)?;
    if !output.conflicts.is_empty() {
        writeln!(w, "  conflicts: {}", comma_separated(&output.conflicts))?;
    }
    if !output.stems.is_empty() {
        writeln!(w, "  stems: {}", comma_separated(&output.stems))?;
    }
    if !output.sections.is_empty() {
        let sections = &output.sections;
        let kinds: BTreeSet<SectionKind> = [&sections.added, &sections.changed, &sections.removed]
            .into_iter()
            .flatten()
            .copied()
            .collect();
        let names = sections
            .preamble
            .then_some("preamble")
            .into_iter()
            .chain(kinds.iter().map(AsRef::as_ref));
        writeln!(w, "  design: {}", comma_separated(names))?;
    }
    if !output.slices.is_empty() {
        writeln!(w, "  slices: {}", comma_separated(output.slices.iter().map(slice_change)))?;
    }
    repositories(&output.repositories, w)?;
    if let Some(applied) = &output.applied {
        sealed(applied, "  ", w)?;
    } else if output.staged {
        writeln!(w, "  nothing sealed; `emery show amendment` reviews it")?;
    }
    Ok(())
}

fn slice_change(slice: &SliceChange) -> impl fmt::Display {
    // A changed slice where no slicing turn ran keeps its base brief.
    let tag = match slice.class {
        SliceClass::Added => " (new)",
        SliceClass::Changed if slice.keeps_brief() => " (brief kept)",
        SliceClass::Changed => "",
        SliceClass::Removed => " (removed)",
    };
    fmt::from_fn(move |f| write!(f, "{}{tag}", slice.name))
}

fn repositories(reads: &[RepositorySource], w: &mut dyn fmt::Write) -> fmt::Result {
    for read in reads {
        writeln!(
            w,
            "  {} read from {} at {}: {}",
            read.source, read.repository, read.revision, read.commit
        )?;
    }
    Ok(())
}

// `sealed revision <id> as emery/<id> at <commit8>`, led by `indent`, and
// `amends <parent>` beneath it, indented once more, for a sealed amendment.
fn sealed(output: &ApplyOutput, indent: &str, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(
        w,
        "{indent}sealed revision {} as {} at {}",
        output.revision,
        output.label,
        short_commit(&output.commit)
    )?;
    if let (StagedKind::Amendment, Some(parent)) = (output.kind, &output.parent) {
        writeln!(w, "{indent}  amends {parent}")?;
    }
    Ok(())
}

fn diff(diff: Option<&Diff>, w: &mut dyn fmt::Write) -> fmt::Result {
    let Some(diff) = diff else {
        return Ok(());
    };
    if diff.is_empty() {
        return writeln!(w, "  diff vs {}: none (byte-stable)", diff.from);
    }
    let (spec, design, plan) = (&diff.spec, &diff.design, &diff.plan);
    let documents = [
        document_diff(
            "spec",
            spec.added.len(),
            spec.removed.len(),
            spec.changed.len(),
            spec.retired.len(),
            spec.preamble,
        ),
        document_diff(
            "design",
            design.added.len(),
            design.removed.len(),
            design.changed.len(),
            0,
            design.preamble,
        ),
        document_diff(
            "plan",
            plan.added.len(),
            plan.removed.len(),
            plan.changed.len(),
            0,
            plan.preamble,
        ),
    ];
    writeln!(w, "  diff vs {}: {}", diff.from, comma_separated(&documents))
}

// `spec +1 -0 ~1 retired 1 preamble`: the counts for one document, `retired`
// when any requirement was withdrawn, `preamble` when its preamble changed.
fn document_diff(
    name: &'static str, added: usize, removed: usize, changed: usize, retired: usize,
    preamble: bool,
) -> impl fmt::Display {
    fmt::from_fn(move |f| {
        write!(f, "{name} +{added} -{removed} ~{changed}")?;
        if retired > 0 {
            write!(f, " retired {retired}")?;
        }
        if preamble {
            f.write_str(" preamble")?;
        }
        Ok(())
    })
}

fn build(output: &BuildOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "built revision {}", output.revision)?;
    waves(&output.waves, w)?;
    match &output.candidate {
        Some(Candidate {
            id,
            kind: StagedKind::Revision,
        }) => writeln!(w, "  candidate {id} is staged and not sealed; `emery apply` seals it")?,
        Some(Candidate {
            id,
            kind: StagedKind::Amendment,
        }) => writeln!(w, "  amendment {id} is staged; `emery show amendment` reviews it")?,
        None => {}
    }
    writeln!(w, "  base {}", output.base)?;
    if output.repository != "." {
        writeln!(w, "  repository {}", output.repository)?;
    }
    if let Some(initialised) = &output.initialised {
        writeln!(
            w,
            "  initialised {}, {}",
            short_commit(&initialised.commit),
            counted(initialised.paths.len(), "file")
        )?;
    }
    if !output.resumed.is_empty() {
        writeln!(w, "  resumed: {}", comma_separated(&output.resumed))?;
    }
    for verified in &output.verified {
        let wave = verified.wave;
        write!(
            w,
            "  wave {wave}: {} verified at {}",
            counted(verified.slices.len(), "slice"),
            short_commit(&verified.head)
        )?;
        // `repaired` when verification modified files outside `ours` rules;
        // `sealed` when it left only regenerated `ours` paths (lockfiles).
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
        write!(w, " (uncovered {})", comma_separated(&slice.uncovered))?;
    }
    write!(w, ", written {}", counted(slice.written.len(), "file"))?;
    match &slice.commit {
        Some(commit) => write!(w, ", merged {commit}")?,
        None => write!(w, ", nothing to merge")?,
    }
    if !slice.resolved.is_empty() {
        write!(w, ", resolved ({})", comma_separated(&slice.resolved))?;
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

fn short_commit(commit: &str) -> &str {
    commit.get(..8).unwrap_or(commit)
}

fn counted(count: usize, noun: &str) -> impl fmt::Display {
    let plural = if count == 1 { "" } else { "s" };
    fmt::from_fn(move |f| write!(f, "{count} {noun}{plural}"))
}

fn comma_separated<I>(items: I) -> impl fmt::Display
where
    I: IntoIterator + Clone,
    I::Item: fmt::Display,
{
    fmt::from_fn(move |f| {
        for (position, item) in items.clone().into_iter().enumerate() {
            if position > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{item}")?;
        }
        Ok(())
    })
}

// Writes the document body alone so the Markdown can be piped directly.
fn show(output: &ShowOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    w.write_str(&output.body)
}

impl Render for SpecifyOutput {
    fn render(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        specify(self, out)
    }
}

impl Render for ApplyOutput {
    fn render(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        apply(self, out)
    }
}

impl Render for AmendOutput {
    fn render(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        amend(self, out)
    }
}

impl Render for BuildOutput {
    fn render(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        build(self, out)
    }
}

impl Render for ShowOutput {
    fn render(&self, out: &mut dyn fmt::Write) -> fmt::Result {
        show(self, out)
    }
}
