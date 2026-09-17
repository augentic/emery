//! The check that holds an embedded table to the tree it was listed from.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::Doc;

/// Returns every way the table `docs` and the Markdown tree at `root` disagree; empty when they agree.
///
/// The tree is walked as it stands on disk, symlinked directories followed,
/// and every `.md` beneath it must be listed once in `docs` under its
/// tree-relative path. Every relative link in a listed body — outside fenced
/// code, its fragment dropped — must name a document in the table, so what a
/// prompt tells the model to read is always there to be read. Each finding
/// names the document and what is wrong with it.
///
/// This is the test that keeps a [`prose!`](crate::prose) list in step with
/// its tree: run it natively over the tree beside the crate, from a suite
/// that can name the table.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use emery_prose::Doc;
///
/// static DOCS: &[Doc] =
///     emery_prose::prose!("../tests/fixtures", ["prompts/extract.md", "references/ids.md"]);
///
/// let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
/// let findings = emery_prose::check(DOCS, &tree);
/// assert!(findings.is_empty(), "{}", findings.join("\n"));
/// ```
#[must_use]
pub fn check(docs: &[Doc], root: &Path) -> Vec<String> {
    let on_disk = match walk(root, "", &[]) {
        Ok(paths) => paths,
        Err(finding) => return vec![finding],
    };

    let mut findings = Vec::new();
    let mut listed = BTreeSet::new();
    for doc in docs {
        if !listed.insert(doc.path.to_owned()) {
            findings.push(format!("`{}` is listed twice", doc.path));
        }
    }
    for path in on_disk.difference(&listed) {
        findings.push(format!("`{path}` is in the tree but not in the table"));
    }
    for path in listed.difference(&on_disk) {
        findings.push(format!("`{path}` is in the table but not in the tree"));
    }

    for doc in docs {
        for target in links(doc.body) {
            match resolve(doc.path, target) {
                Some(linked) if listed.contains(&linked) => {}
                Some(linked) => findings.push(format!(
                    "`{}` links `{target}`, and the table holds no `{linked}`",
                    doc.path
                )),
                None => {
                    findings
                        .push(format!("`{}` links `{target}`, which leaves the tree", doc.path));
                }
            }
        }
    }
    findings
}

// Every `.md` beneath `dir` by tree-relative path. Symlinks are followed, so
// the canonical ancestors are the cycle guard.
fn walk(dir: &Path, prefix: &str, ancestors: &[PathBuf]) -> Result<BTreeSet<String>, String> {
    let unreadable = |err: io::Error| format!("`{}` cannot be read: {err}", dir.display());
    let canonical = fs::canonicalize(dir).map_err(unreadable)?;
    if ancestors.contains(&canonical) {
        return Err(format!(
            "`{}` re-enters `{}`: a symlink cycle",
            dir.display(),
            canonical.display()
        ));
    }
    let lineage = [ancestors, &[canonical]].concat();

    let mut found = BTreeSet::new();
    for entry in fs::read_dir(dir).map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let file = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };

        // `metadata` follows a symlink, so a linked directory is walked.
        let metadata = fs::metadata(&file).map_err(unreadable)?;
        if metadata.is_dir() {
            found.extend(walk(&file, &path, &lineage)?);
        } else if metadata.is_file() && file.extension().is_some_and(|ext| ext == "md") {
            found.insert(path);
        }
    }
    Ok(found)
}

// The relative link targets in `body`, outside fenced code and without their
// fragments; an absolute URL or a mailto is not a document.
fn links(body: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut fenced = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }

        let mut rest = line;
        while let Some(open) = rest.find("](") {
            rest = &rest[open + 2..];
            let Some(close) = rest.find(')') else { break };
            let target = rest[..close].trim().split('#').next().unwrap_or_default();
            rest = &rest[close + 1..];
            if !target.is_empty() && !target.contains("://") && !target.starts_with("mailto:") {
                targets.push(target);
            }
        }
    }
    targets
}

// Where `target`, linked from the document at `from`, lands in the tree;
// `None` when it climbs out of it.
fn resolve(from: &str, target: &str) -> Option<String> {
    let mut segments: Vec<&str> =
        from.rsplit_once('/').map(|(dir, _)| dir.split('/').collect()).unwrap_or_default();
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            name => segments.push(name),
        }
    }
    Some(segments.join("/"))
}
