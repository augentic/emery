//! Resolves the paths a slice's merge left conflicted, with the model, and merges the slice again.
//!
//! [`UnresolvedMerge`] reads both sides of each conflicted path, puts them to
//! the model as a [`MergeBrief`] beside the plan entries of the slices that
//! wrote them, and seals and merges the resolutions; the brief holds every
//! answer to one whole file per path, free of conflict markers.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use emery_adapter::beneath;
use omnia_sdk::vcs::{Change, ChangeKind, Rule, Strategy};
use omnia_sdk::{Error, Model, Vcs, server_error};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{Built, Progress, Run, gated, kind_at};
use crate::brief::{Brief, Review};
use crate::revision::Slice;
use crate::vcs::WorkingCopy;
use crate::{EntryKind, Files, is_retryable};

// A slice's merge under the adapter's rules, left at the paths no rule
// resolved: what this slice and one merged before it in the wave both
// rewrote.
pub struct UnresolvedMerge<'a> {
    pub built: &'a Built<'a>,
    // what the slice's build changed, as its working copy holds it
    pub pending: &'a [Change],
    // the slice's sealed commit, and the message it and its merge carry
    pub commit: String,
    pub message: &'a str,
    pub paths: Vec<String>,
}

impl UnresolvedMerge<'_> {
    // Resolves every path with the model, seals the resolutions in the
    // slice's working copy over its commit, and merges the slice again with
    // each resolution taken whole: the merge commit, and the paths resolved.
    pub async fn resolve<P: Model + Vcs + Files>(
        self, run: &Run<'_>, provider: &P, progress: &Progress,
    ) -> Result<(String, Vec<String>), Error> {
        let slice = self.built.slice;
        let worktree = &self.built.worktree;
        tracing::info!(
            slice = %slice.id,
            conflicts = self.paths.join(", "),
            "the rules left conflicts; resolving them with the model"
        );

        // each path a text file on both sides, read before any turn is spent
        let mut conflicts = Vec::with_capacity(self.paths.len());
        for path in &self.paths {
            let integrated = self.side(provider, run.integration, path, Side::Integrated).await?;
            let slice = self.side(provider, worktree, path, Side::Slice).await?;
            conflicts.push(Conflict {
                path: path.clone(),
                integrated,
                slice,
            });
        }

        // the plan wave's other merged slices that wrote a conflicted path,
        // whose plan entries say what their side is for; staged slices are
        // merged already and sit outside `built`, and may sit after this one
        // in id order, so the wave comes from the plan rather than this run's
        // wave counter, which restarts on a resume
        let waves = run.plan.waves();
        let wave_slices = waves.iter().find(|wave| wave.contains(&slice.id)).unwrap_or(&[]);
        let merged: Vec<&Slice> = wave_slices
            .iter()
            .filter(|&&id| id != slice.id)
            .filter(|id| progress.merged.contains(id))
            .filter_map(|id| {
                let plan_slice = run.plan.slice(*id)?;
                let touches =
                    progress.built.iter().find(|built| built.id == *id).is_none_or(|built| {
                        built.written.iter().any(|path| self.paths.contains(path))
                    });
                touches.then_some(plan_slice)
            })
            .collect();
        let brief = MergeBrief {
            slice,
            merged,
            conflicts,
        };
        let resolutions = match brief.judge(provider).await {
            Ok(resolutions) => resolutions,
            Err(error @ Error::BadRequest { .. }) if !is_retryable(&error) => {
                return Err(self.unresolved(format_args!(
                    "the model's resolution was refused within the available rounds: {}",
                    findings_of(&error.description())
                )));
            }
            Err(error) => return Err(error),
        };

        // the resolutions written in the slice's copy, held to the seal gates,
        // and sealed over its commit; one that keeps the slice's own side
        // changes nothing, and the slice's commit stands
        for file in &resolutions {
            let path = beneath(&file.path)
                .map_err(|bad| server_error!("the resolution at `{}` {bad}", file.path))?;
            provider.write(worktree.path(), &path, file.content.as_bytes()).await?;
        }
        let pending = worktree.pending(provider).await?;
        gated(provider, worktree.path(), "the merge's resolution", &pending).await?;
        let mut resolved: Vec<String> = resolutions.into_iter().map(|file| file.path).collect();
        resolved.sort_unstable();
        let subject = format!("{} {} resolved at {}", slice.id, slice.name, resolved.join(", "));
        let commit = worktree.commit(provider, &subject).await?.unwrap_or(self.commit);

        // merged again, each resolution taken whole and the rules as before
        let policy: Vec<Rule> = resolved
            .iter()
            .map(|path| Rule {
                paths: format!("/{path}"),
                strategy: Strategy::Theirs,
            })
            .chain(run.policy.iter().cloned())
            .collect();
        let merged = run.integration.merge(provider, &commit, self.message, &policy).await?;
        match merged.commit {
            Some(commit) => Ok((commit, resolved)),
            None => Err(server_error!(
                "its merge still conflicts at {} once the model resolved {}",
                merged.conflicts.join(", "),
                resolved.join(", ")
            )),
        }
    }

    // One side of a conflicted path as text, or the refusal: a path the
    // engine never writes, one the slice removed, one `side` does not hold
    // as a regular file, or one that is not text is nothing the model can
    // resolve, so no turn is spent on it.
    async fn side<P: Files>(
        &self, provider: &P, copy: &WorkingCopy, path: &str, side: Side,
    ) -> Result<String, Error> {
        if let Err(bad) = beneath(path) {
            return Err(self.unresolved(format_args!("`{path}` {bad}")));
        }
        let removed = side == Side::Slice
            && self.pending.iter().any(|c| c.path == path && c.kind == ChangeKind::Deleted);
        if removed {
            return Err(self.unresolved(format_args!("the slice removed `{path}`")));
        }
        match kind_at(provider, copy.path(), path).await? {
            Some(EntryKind::File) => {}
            Some(EntryKind::Link) => {
                return Err(
                    self.unresolved(format_args!("a symbolic link stands at `{path}` in {side}"))
                );
            }
            Some(EntryKind::Directory) => {
                return Err(
                    self.unresolved(format_args!("a directory stands at `{path}` in {side}"))
                );
            }
            None => {
                return Err(self.unresolved(format_args!("{side} does not hold `{path}`")));
            }
        }
        String::from_utf8(provider.read(copy.path(), path).await?)
            .map_err(|_bytes| self.unresolved(format_args!("`{path}` is not text in {side}")))
    }

    fn unresolved(&self, why: impl Display) -> Error {
        Error::BadRequest {
            code: "merge-unresolved".into(),
            description: format!(
                "its merge conflicts at {}, and the conflict cannot be resolved: {why}",
                self.paths.join(", ")
            ),
        }
    }
}

// The findings a refused question's description carries, without the
// candidate they were found against, which here spells whole files; a
// description in another shape is kept whole.
fn findings_of(description: &str) -> &str {
    description.split_once("## Findings\n\n").map_or(description, |(_, findings)| {
        findings.split("\n\nProduce a ").next().unwrap_or(findings)
    })
}

// The two sides of a conflicted path.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Integrated,
    Slice,
}

impl Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Integrated => "the integrated tree",
            Self::Slice => "the slice",
        })
    }
}

// The lines git leaves at a conflict it did not resolve, each opening a line.
// The `=======` between them stands alone in a setext heading too, so the
// two that carry a label are the markers, and a block always has them.
const MARKERS: [&str; 2] = ["<<<<<<< ", ">>>>>>> "];

// The brief that resolves the paths a slice's merge left conflicted.
pub struct MergeBrief<'a> {
    // The slice being merged.
    pub slice: &'a Slice,
    // The slices merged before it in the wave that wrote a conflicted path.
    pub merged: Vec<&'a Slice>,
    // Each conflicted path with both of its sides.
    pub conflicts: Vec<Conflict>,
}

// One path a slice's merge left conflicted, as each side holds it.
pub struct Conflict {
    // The path, relative to the tree root.
    pub path: String,
    // The file as the integrated tree holds it.
    pub integrated: String,
    // The file as the slice wrote it.
    pub slice: String,
}

/// One conflicted path as the model resolved it.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Resolution {
    /// The conflicted path.
    pub path: String,
    /// The whole file, both sides combined.
    pub content: String,
}

/// The model's answer: one resolution per conflicted path.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeAnswer {
    files: Vec<Resolution>,
}

impl Brief for MergeBrief<'_> {
    type Answer = MergeAnswer;
    type Output = Vec<Resolution>;

    const NAME: &'static str = "merge";
    const PROSE: &'static [&'static str] = &["merge.md"];

    fn label(&self) -> Cow<'static, str> {
        Cow::Owned(format!("merge-{}", self.slice.id))
    }

    fn tighten(&self, schema: &mut Value) -> Result<(), Error> {
        let files = Self::at(schema, "/properties/files")?;
        files.insert("minItems".to_owned(), json!(self.conflicts.len()));
        files.insert("maxItems".to_owned(), json!(self.conflicts.len()));
        let paths: Vec<&str> = self.conflicts.iter().map(|c| c.path.as_str()).collect();
        Self::at(schema, "/$defs/Resolution/properties/path")?
            .insert("enum".to_owned(), json!(paths));
        Ok(())
    }

    fn verify(&self, answer: &MergeAnswer, review: &mut Review) {
        let expected: BTreeMap<&str, &Conflict> =
            self.conflicts.iter().map(|conflict| (conflict.path.as_str(), conflict)).collect();
        let mut seen = BTreeMap::new();
        for file in &answer.files {
            let Some(conflict) = expected.get(file.path.as_str()) else {
                review.note(format_args!("`{}` is not one of the conflicted paths", file.path));
                continue;
            };
            if seen.insert(file.path.as_str(), ()).is_some() {
                review.note(format_args!("`{}` is answered more than once", file.path));
                continue;
            }
            if file.content.lines().any(|line| MARKERS.iter().any(|m| line.starts_with(m))) {
                review.note(format_args!("`{}` still carries conflict markers", file.path));
            }
            let held = !conflict.integrated.trim().is_empty() || !conflict.slice.trim().is_empty();
            if held && file.content.trim().is_empty() {
                review.note(format_args!("`{}` is empty, so both sides are lost", file.path));
            }
        }
        for path in expected.keys().filter(|path| !seen.contains_key(*path)) {
            review.note(format_args!("`{path}` is not answered"));
        }
    }

    fn into_output(self, answer: MergeAnswer) -> Result<Self::Output, Error> {
        Ok(answer.files)
    }
}

impl Display for MergeBrief<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "# Slice being merged\n\n{}\n", self.slice)?;

        if !self.merged.is_empty() {
            f.write_str("# Merged before it in the wave\n")?;
            for slice in &self.merged {
                writeln!(f, "\n{slice}\n")?;
            }
        }

        f.write_str("# Conflicts\n")?;
        for conflict in &self.conflicts {
            writeln!(f, "\n## `{}`\n", conflict.path)?;
            fenced(f, "Integrated tree", &conflict.integrated)?;
            fenced(f, "Slice", &conflict.slice)?;
        }
        Ok(())
    }
}

// A file under its heading, fenced longer than any run of backticks it holds.
fn fenced(f: &mut Formatter<'_>, heading: &str, content: &str) -> fmt::Result {
    let longest = content.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    let body = content.strip_suffix('\n').unwrap_or(content);
    writeln!(f, "### {heading}\n\n{fence}\n{body}\n{fence}\n")
}
