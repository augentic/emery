//! Renders verb outputs as `--format text` stdout.

use std::collections::BTreeSet;
use std::fmt;

use omnia_sdk::command::Render;

use crate::amend::{AmendOutput, PartChange, PartClass};
use crate::apply::ApplyOutput;
use crate::build::{BuildOutput, BuiltSlice, Candidate, ScheduledSlice, StagedKind};
use crate::show::ShowOutput;
use crate::specify::{Diff, RepositorySource, SectionKind, SpecifyOutput, Waves};

impl Render for SpecifyOutput {
    fn render(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        write!(w, "staged candidate {}", self.revision)?;
        if self.fresh {
            w.write_str(" (fresh)")?;
        }
        writeln!(w)?;
        waves(&self.waves, w)?;
        repositories(&self.repositories, w)?;
        diff(self.diff.as_ref(), w)?;
        match &self.applied {
            Some(applied) => sealed(applied, "  ", w),
            None => writeln!(w, "  nothing sealed; `emery apply` seals it"),
        }
    }
}

impl Render for ApplyOutput {
    fn render(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        sealed(self, "", w)?;
        if self.repository != "." {
            writeln!(w, "  repository {}", self.repository)?;
        }
        diff(self.diff.as_ref(), w)
    }
}

impl Render for AmendOutput {
    fn render(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        match &self.amendment {
            Some(id) => writeln!(w, "staged amendment {id}")?,
            None => writeln!(w, "nothing staged: no change against the current revision")?,
        }
        writeln!(w, "  base {} at {}", self.base.revision, short_commit(&self.base.commit))?;
        let tally = &self.tally;
        write!(w, "  {} added, {} changed, {} retired", tally.added, tally.changed, tally.retired)?;
        if tally.by_scope > 0 {
            write!(w, " ({} by scope)", tally.by_scope)?;
        }
        writeln!(w, ", {} conflict, {} unchanged", tally.conflict, tally.unchanged)?;
        if !self.conflicts.is_empty() {
            writeln!(w, "  conflicts: {}", comma_separated(&self.conflicts))?;
        }
        if !self.stems.is_empty() {
            writeln!(w, "  stems: {}", comma_separated(&self.stems))?;
        }
        if !self.sections.is_empty() {
            let sections = &self.sections;
            let kinds: BTreeSet<SectionKind> =
                [&sections.added, &sections.changed, &sections.removed]
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
        if !self.parts.is_empty() {
            writeln!(w, "  parts: {}", comma_separated(self.parts.iter().map(part_change)))?;
        }
        repositories(&self.repositories, w)?;
        if let Some(applied) = &self.applied {
            sealed(applied, "  ", w)?;
        } else if self.staged {
            writeln!(w, "  nothing sealed; `emery show amendment` reviews it")?;
        }
        Ok(())
    }
}

impl Render for BuildOutput {
    fn render(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        writeln!(w, "built revision {}", self.revision)?;
        build_plan(self, w)?;
        let mut over = self.schedule.iter().filter(|slice| slice.over_budget).peekable();
        if over.peek().is_some() {
            writeln!(w, "  over budget: {}", comma_separated(over.map(over_slice)))?;
        }
        match &self.candidate {
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
        writeln!(w, "  base {}", self.base)?;
        if self.repository != "." {
            writeln!(w, "  repository {}", self.repository)?;
        }
        if let Some(initialised) = &self.initialised {
            writeln!(
                w,
                "  initialised {}, {}",
                short_commit(&initialised.commit),
                counted(initialised.paths.len(), "file")
            )?;
        }
        if !self.resumed.is_empty() {
            writeln!(w, "  resumed: {}", comma_separated(&self.resumed))?;
        }
        for verified in &self.verified {
            let wave = verified.wave;
            write!(
                w,
                "  wave {wave}: {} verified at {}",
                counted(verified.slices.len(), "slice"),
                short_commit(&verified.head)
            )?;
            // `repaired` when verification modified files outside `ours` rules;
            // `sealed` when it left only regenerated `ours` paths (lockfiles).
            if self.repaired.contains(&wave) {
                w.write_str(", repaired")?;
            } else if self.sealed.iter().any(|seal| seal.wave == wave) {
                w.write_str(", sealed")?;
            }
            writeln!(w)?;
            for id in &verified.slices {
                match self.slices.iter().find(|slice| slice.id == *id && slice.wave == wave) {
                    Some(slice) => built(slice, w)?,
                    None => writeln!(w, "    {id}: staged by an earlier run, not built again")?,
                }
            }
        }
        writeln!(w, "  labelled {} at {}", self.label, self.head)
    }
}

impl Render for ShowOutput {
    fn render(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        // write the document body alone so the Markdown can be piped directly
        w.write_str(&self.body)
    }
}

fn part_change(part: &PartChange) -> impl fmt::Display {
    // A changed part where no slicing turn ran keeps its base brief.
    let tag = match part.class {
        PartClass::Added => " (new)",
        PartClass::Changed if part.keeps_brief() => " (brief kept)",
        PartClass::Changed => "",
        PartClass::Removed => " (removed)",
    };
    fmt::from_fn(move |f| write!(f, "{}{tag}", part.name))
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
    name: &str, added: usize, removed: usize, changed: usize, retired: usize, preamble: bool,
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

fn build_plan(output: &BuildOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    let parts = &output.parts;
    let waves = &output.waves;
    writeln!(
        w,
        "  plan: {} in {}; budget {} ({}): {} in {}, widest {}",
        counted(parts.parts(), "part"),
        counted(parts.len(), "wave"),
        output.budget,
        output.budget_source.as_ref(),
        counted(waves.slices(), "slice"),
        counted(waves.len(), "wave"),
        waves.widest()
    )
}

fn over_slice(slice: &ScheduledSlice) -> impl fmt::Display {
    fmt::from_fn(move |f| {
        write!(
            f,
            "{} {} ({})",
            slice.id,
            slice.name,
            counted(slice.requirements.len(), "requirement")
        )
    })
}

fn built(slice: &BuiltSlice, w: &mut dyn fmt::Write) -> fmt::Result {
    let total = slice.covered.len() + slice.uncovered.len();
    write!(w, "    {} {}", slice.id, slice.name)?;
    if slice.count > 1 {
        write!(w, " ({}/{})", slice.index, slice.count)?;
    }
    write!(w, ": covered {}/{total}", slice.covered.len())?;
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

// `  plan: 4 parts in 3 waves, widest 2`: how many parts the plan holds,
// how many waves they fall into, and the most ready at once.
fn waves(waves: &Waves, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(
        w,
        "  plan: {} in {}, widest {}",
        counted(waves.parts(), "part"),
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
