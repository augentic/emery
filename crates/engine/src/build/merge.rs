//! Model-assisted conflict resolution for slice merges.
//!
//! [`MergeBrief`] presents both sides of each conflicted file and the
//! relevant slice plans to the model, validating that every conflicted path is
//! answered once without conflict markers.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use omnia_sdk::Error;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::brief::{Brief, Review};
use crate::revision::Slice;

// The lines git leaves at a conflict it did not resolve, each opening a line.
// The `=======` between them stands alone in a setext heading too, so the
// two that carry a label are the markers, and a block always has them.
const MARKERS: [&str; 2] = ["<<<<<<< ", ">>>>>>> "];

/// The brief that resolves the paths a slice's merge left conflicted.
pub struct MergeBrief<'a> {
    /// The slice being merged.
    pub slice: &'a Slice,
    /// The slices merged before it in the wave that wrote a conflicted path.
    pub merged: Vec<&'a Slice>,
    /// Each conflicted path with both of its sides.
    pub conflicts: Vec<Conflict>,
}

/// One path a slice's merge left conflicted, as each side holds it.
pub struct Conflict {
    /// The path, relative to the tree root.
    pub path: String,
    /// The file as the integrated tree holds it.
    pub integrated: String,
    /// The file as the slice wrote it.
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
