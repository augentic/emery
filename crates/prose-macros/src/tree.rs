//! The walk over a Markdown tree and the link check over what it finds.
//!
//! [`table`] lists every `.md` file beneath a root, following symlinked
//! directories, and refuses a relative link with no target or a symlink
//! cycle, so a prose defect fails the build rather than surfacing when a
//! model asks for the document.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

// One document: its tree-relative path and the canonical file to embed.
#[derive(Debug)]
pub struct Entry {
    pub path: String,
    pub file: PathBuf,
}

// Returns every document beneath `root`, sorted by path, with links checked.
pub fn table(root: &Path) -> Result<Vec<Entry>> {
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let mut entries = walk(root, "", &[])?;
    if entries.is_empty() {
        bail!("no markdown documents found under {}", root.display());
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    for entry in &entries {
        check_links(&entry.file)?;
    }
    Ok(entries)
}

// Recurses into `dir`, collecting Markdown files. Symlinks are followed, so
// the canonical ancestors are the cycle guard.
fn walk(dir: &Path, path: &str, ancestors: &[PathBuf]) -> Result<Vec<Entry>> {
    let canonical = fs::canonicalize(dir)?;
    if ancestors.contains(&canonical) {
        bail!("symlink cycle: {} re-enters {}", dir.display(), canonical.display());
    }
    let mut lineage = ancestors.to_vec();
    lineage.push(canonical);

    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if path.is_empty() { name } else { format!("{path}/{name}") };

        let metadata = fs::metadata(&entry_path)?;
        if metadata.is_dir() {
            found.extend(walk(&entry_path, &path, &lineage)?);
        } else if metadata.is_file() && entry_path.extension().is_some_and(|ext| ext == "md") {
            found.push(Entry {
                path,
                file: fs::canonicalize(&entry_path)?,
            });
        }
    }
    Ok(found)
}

// Fails on any relative link in `file` whose target does not exist. Fenced
// code is skipped so a `](` inside a snippet is not a link.
fn check_links(file: &Path) -> Result<()> {
    let body = fs::read_to_string(file)?;
    let dir = file.parent().expect("a file has a parent");
    let mut in_fence = false;

    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        for target in link_targets(line) {
            let path = target.split('#').next().unwrap_or_default();
            if path.is_empty() || target.contains("://") || target.starts_with("mailto:") {
                continue;
            }

            let path = dir.join(path);
            if !path.exists() {
                bail!(
                    "{}: broken link `{target}` ({} does not exist)",
                    file.display(),
                    path.display()
                );
            }
        }
    }

    Ok(())
}

fn link_targets(mut body: &str) -> impl Iterator<Item = &str> {
    std::iter::from_fn(move || {
        let open = body.find("](")?;
        body = &body[open + 2..];
        let close = body.find(')')?;
        let target = body[..close].trim();
        body = &body[close + 1..];
        Some(target)
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;

    // Keep (entry-point-unreachable): a directory symlink is followed and a
    // `](` inside fenced code is not a link; the live engine tree has neither,
    // so this is the only coverage.
    #[test]
    fn embeds() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let shared = tmp.path().join("shared");
        write(&shared, "rule.md", "# Rule\n");
        let tree = tmp.path().join("prompts");
        write(&tree, "b.md", "# B\n");
        write(&tree, "a.md", "see [b](b.md)\n\n```swift\nprocessEffects([UInt8](effects))\n```\n");
        let _ = symlink(&shared, tree.join("runtime"));

        let entries = table(&tree).expect("table");
        let paths: Vec<&str> = entries.iter().map(|entry| entry.path.as_str()).collect();
        assert_eq!(paths, ["a.md", "b.md", "runtime/rule.md"]);
        assert!(entries.iter().all(|entry| entry.file.is_absolute()));
    }

    // Keep (entry-point-unreachable): a missing tree, an empty one, a dangling
    // link, and a symlink cycle each fail the build; no live corpus can
    // arrange any of them.
    #[test]
    fn refuses() {
        let tmp = tempfile::tempdir().expect("tempdir");

        let err = table(&tmp.path().join("absent")).expect_err("absent");
        assert!(err.to_string().contains("not a directory"), "{err}");

        let empty = tmp.path().join("empty");
        fs::create_dir_all(&empty).expect("mkdir");
        let err = table(&empty).expect_err("empty");
        assert!(err.to_string().contains("no markdown documents"), "{err}");

        let dangling = tmp.path().join("dangling");
        write(&dangling, "a.md", "see [missing](nope.md)\n");
        let err = table(&dangling).expect_err("dangling");
        assert!(err.to_string().contains("nope.md"), "{err}");

        let cycle = tmp.path().join("cycle");
        write(&cycle, "intro.md", "# Intro\n");
        let _ = symlink(Path::new("."), cycle.join("loop"));
        let err = table(&cycle).expect_err("cycle");
        assert!(err.to_string().contains("symlink cycle"), "{err}");
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, body).expect("write");
    }
}
