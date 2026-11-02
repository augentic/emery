//! Validates an embedded document table against its source files.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::Doc;

/// Returns inconsistencies between `docs`, its source tree, its prompts, and its imports.
///
/// An empty result means all of the following are true:
///
/// - Every Markdown file beneath `root` appears exactly once in `docs`.
/// - Every relative link in an embedded document resolves to another entry
///   or to an import.
/// - Every path in `prompts` identifies an embedded document.
/// - Every other document is reachable by following links from a prompt.
/// - No document shares its path with an import.
///
/// `imports` are documents another table embeds beside `docs`, so a link may
/// name one; none is required beneath `root` or reached from a prompt. A
/// document at an import's path would answer lookups meant for the import,
/// so it is a finding.
///
/// Symlinked directories are followed. An unreadable path or a symlink cycle
/// is reported as a finding. A link inside a code fence or an inline code
/// span is ignored. A URL fragment does not affect the document path. Each
/// finding names the path and the rule it breaks.
///
/// Use this function in a native test beside a [`prose!`](crate::prose)
/// invocation.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use emery_prose::Doc;
///
/// static PROSE: &[Doc] =
///     emery_prose::prose!["../tests/prose/extract.md", "../tests/prose/references/ids.md"];
///
/// let tree = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/prose");
/// let findings = emery_prose::check(PROSE, &tree, &["extract.md"], &[]);
/// assert!(findings.is_empty(), "{}", findings.join("\n"));
/// ```
#[must_use]
pub fn check(
    docs: &[Doc], root: impl AsRef<Path>, prompts: &[&str], imports: &[Doc],
) -> Vec<String> {
    check_tree(docs, root.as_ref(), prompts, imports)
}

fn check_tree(docs: &[Doc], root: &Path, prompts: &[&str], imports: &[Doc]) -> Vec<String> {
    let files = match markdown_files(root) {
        Ok(paths) => paths,
        Err(finding) => return vec![finding],
    };
    let on_disk: BTreeSet<&str> = files.iter().map(String::as_str).collect();

    // hold the table to the tree
    let mut findings = Vec::new();
    let mut listed = BTreeSet::new();
    for doc in docs {
        if !listed.insert(doc.path) {
            findings.push(format!("`{}` is listed twice", doc.path));
        }
    }
    for path in on_disk.difference(&listed) {
        findings.push(format!("`{path}` is in the tree but not in the table"));
    }
    for path in listed.difference(&on_disk) {
        findings.push(format!("`{path}` is in the table but not in the tree"));
    }

    // hold the table apart from the imports
    let imported: BTreeSet<&str> = imports.iter().map(|doc| doc.path).collect();
    for path in listed.intersection(&imported) {
        findings.push(format!("`{path}` shadows an import"));
    }

    // hold every link to the table or the imports
    for doc in docs {
        for target in relative_links(doc.body) {
            match resolve_link(doc.path, target) {
                Some(linked)
                    if listed.contains(linked.as_str()) || imported.contains(linked.as_str()) => {}
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

    // hold every document to a prompt
    let mut seen_prompts = BTreeSet::new();
    for &prompt in prompts {
        if seen_prompts.insert(prompt) && !listed.contains(prompt) {
            findings.push(format!("`{prompt}` is a prompt the table does not hold"));
        }
    }
    let mut reached = reachable(docs, prompts);
    for doc in docs {
        if reached.insert(doc.path) {
            findings.push(format!("`{}` is reached from no prompt", doc.path));
        }
    }
    findings
}

fn reachable(docs: &[Doc], prompts: &[&str]) -> BTreeSet<&'static str> {
    let mut reached = BTreeSet::new();
    let mut frontier: Vec<&Doc> =
        prompts.iter().filter_map(|prompt| crate::find(docs, prompt)).collect();
    while let Some(doc) = frontier.pop() {
        if !reached.insert(doc.path) {
            continue;
        }
        for target in relative_links(doc.body) {
            let linked = resolve_link(doc.path, target).and_then(|path| crate::find(docs, &path));
            frontier.extend(linked);
        }
    }
    reached
}

fn markdown_files(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = BTreeSet::new();
    let mut ancestors = Vec::new();
    walk_dir(root, "", &mut ancestors, &mut files)?;
    Ok(files)
}

// Symlinks are followed, so the canonical ancestors are the cycle guard.
fn walk_dir(
    dir: &Path, prefix: &str, ancestors: &mut Vec<PathBuf>, files: &mut BTreeSet<String>,
) -> Result<(), String> {
    let canonical = fs::canonicalize(dir).map_err(|err| unreadable(dir, &err))?;
    if ancestors.contains(&canonical) {
        return Err(format!(
            "`{}` re-enters `{}`: a symlink cycle",
            dir.display(),
            canonical.display()
        ));
    }
    ancestors.push(canonical);

    for entry in fs::read_dir(dir).map_err(|err| unreadable(dir, &err))? {
        let entry = entry.map_err(|err| unreadable(dir, &err))?;
        let file = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let path = if prefix.is_empty() { name.into_owned() } else { format!("{prefix}/{name}") };

        let metadata = fs::metadata(&file).map_err(|err| unreadable(&file, &err))?;
        if metadata.is_dir() {
            walk_dir(&file, &path, ancestors, files)?;
        } else if metadata.is_file() && file.extension().is_some_and(|ext| ext == "md") {
            files.insert(path);
        }
    }

    ancestors.pop();
    Ok(())
}

fn unreadable(path: &Path, err: &io::Error) -> String {
    format!("`{}` cannot be read: {err}", path.display())
}

fn relative_links(body: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut fence: Option<&str> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
            continue;
        }
        if let Some(marker) = ["```", "~~~"].into_iter().find(|marker| trimmed.starts_with(marker))
        {
            fence = Some(marker);
            continue;
        }

        let mut rest = line;
        while !rest.is_empty() {
            let next_code = rest.find('`');
            let next_link = rest.find("](");
            match (next_code, next_link) {
                (Some(tick), Some(open)) if tick < open => {
                    rest = skip_code_span(&rest[tick..]);
                }
                (_, Some(open)) => {
                    rest = &rest[open + 2..];
                    let Some(close) = rest.find(')') else { break };
                    let target = rest[..close].trim();
                    let (target, _anchor) = target.split_once('#').unwrap_or((target, ""));
                    rest = &rest[close + 1..];
                    if !target.is_empty()
                        && !target.contains("://")
                        && !target.starts_with("mailto:")
                    {
                        targets.push(target);
                    }
                }
                _ => break,
            }
        }
    }
    targets
}

fn skip_code_span(from_tick: &str) -> &str {
    let ticks_len = from_tick.bytes().take_while(|&b| b == b'`').count();
    let (ticks, after_open) = from_tick.split_at(ticks_len);
    after_open.find(ticks).map_or(after_open, |close| &after_open[close + ticks_len..])
}

fn resolve_link(from: &str, target: &str) -> Option<String> {
    if target.starts_with('/') {
        return None;
    }

    // the directory of `from`, as segments
    let mut segments: Vec<&str> = match from.rsplit_once('/') {
        Some((dir, _file)) => dir.split('/').collect(),
        None => Vec::new(),
    };
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            name => segments.push(name),
        }
    }
    (!segments.is_empty()).then(|| segments.join("/"))
}
