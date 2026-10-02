//! Cuts a surveyed tree into the seams `extract` mines.
//!
//! A tree within the inline budget, or of one module, is one seam over every
//! module, held to every stem its surfaces carry. A larger tree is one seam
//! per stem, over the modules the surfaces under it reach, the entry first;
//! where one of those imports what the resolver cannot follow or loads a
//! module by a computed name, the modules of the directory the import
//! leads into follow. A tree the
//! survey names no surface in is cut mechanically: one seam under the
//! package's or the root directory's name within the budget, one per
//! top-level directory past it. A seam's data files sit among its modules
//! and the tree's own tests follow those, each listed in the brief of every
//! seam whose modules it imports.

use super::code::{Recogniser, Surface, Tree};
use super::tests::{Test, stated};
use super::{skeleton, unique};
use crate::{Seam, kebab};

// A data file within this many bytes is laid directly after the first module
// naming it, a larger one after every module. Laid after a long module such
// as a generated table, a small file could fall past where the laid run
// ends, and a `criterion` could not cite its value.
const DATA_BESIDE_BYTES: u64 = 8 * 1024;

const NO_SURFACE: &str = "No surface was found in this source: its survey named no route, command, \
                          job, consumer, or exported API — no bootstrap the manifest names or a \
                          conventional entry holds, no handler registered with a package, \
                          no function, method, or class under a package's decorator, \
                          and no function or class exported at an entry module for a caller. \
                          Read it as a library is read — for what its exports do for a caller — \
                          and claim what the code exhibits.";

pub(super) struct Lead {
    seam: Seam,
    // The modules laid past the surfaces' closures, in the directories their
    // unfollowed imports lead into. Only the cut by stem fills it.
    widened: Vec<String>,
}

impl Lead {
    fn new(text: String, files: Vec<String>, stems: Vec<String>) -> Self {
        Self {
            seam: Seam {
                text,
                files,
                stems,
                ..Seam::default()
            },
            widened: Vec::new(),
        }
    }

    // Every module, the surfaces' closures first.
    pub(super) fn whole<R: Recogniser>(tree: &Tree<R>, surfaces: &[Surface]) -> Self {
        let files = unique(
            surfaces
                .iter()
                .flat_map(|surface| surface.closure.iter().cloned())
                .chain(tree.modules.keys().cloned()),
        );
        let stems = unique(surfaces.iter().map(|surface| surface.stem.clone()));
        let text = format!(
            "The surfaces of this source, found by reading its code — where control enters it \
             from outside the process:\n\n{}\n\nEvery `requirement` and `criterion` belongs to \
             one of these surfaces: lead its id with that surface's id, and claim a behaviour \
             under the surface whose caller observes it, once.",
            listed(surfaces)
        );
        Self::new(text, files, stems)
    }

    pub(super) fn by_stem<R: Recogniser>(tree: &Tree<R>, surfaces: &[Surface]) -> Vec<Self> {
        unique(surfaces.iter().map(|surface| surface.stem.as_str()))
            .into_iter()
            .map(|stem| {
                let under: Vec<&Surface> = surfaces.iter().filter(|s| s.stem == stem).collect();
                let mut files =
                    unique(under.iter().flat_map(|surface| surface.closure.iter().cloned()));
                let widened = tree.widening(&files);
                files.extend(widened.iter().cloned());

                let (count, reach, whose) = match under.len() {
                    1 => ("surface".to_owned(), "it reaches", "its"),
                    n => (format!("{n} surfaces"), "they reach", "their"),
                };
                let text = format!(
                    "This call mines the {count} under the stem `{stem}` alone:\n\n{}\n\nThe \
                     files below are what {reach} from {whose} entry, the entry first. What the \
                     tree does for another surface is that surface's call to claim, even in a \
                     module the two share.",
                    listed(under.iter().copied()),
                );
                Self {
                    widened,
                    ..Self::new(text, files, vec![stem.to_owned()])
                }
            })
            .collect()
    }

    pub(super) fn unsurfaced<R: Recogniser>(tree: &Tree<R>) -> Self {
        let files: Vec<String> = tree.modules.keys().cloned().collect();
        let stem = fallback_stem(tree);
        let text = format!(
            "{NO_SURFACE} The modules below are the whole source, mined under the one stem \
             `{stem}`: lead every `requirement` and `criterion` id with it, and name each \
             behaviour for the export that exhibits it."
        );
        Self::new(text, files, vec![stem])
    }

    pub(super) fn by_directory<R: Recogniser>(tree: &Tree<R>) -> Vec<Self> {
        // cut beneath `src/` and beneath a lone top-level package
        let packages: Vec<&str> = tree
            .modules
            .keys()
            .filter_map(|path| barrel(tree.dialect.barrels, path))
            .map(|package| package.strip_prefix("src/").unwrap_or(package))
            .filter(|package| !package.contains('/'))
            .collect();
        let package = match packages.as_slice() {
            [one] => Some(format!("{one}/")),
            _ => None,
        };

        // group the modules by top-level directory, under its name
        let mut groups: Vec<(String, String, Vec<String>)> = Vec::new();
        let mut loose: Vec<String> = Vec::new();
        for path in tree.modules.keys() {
            let rest = path.strip_prefix("src/").unwrap_or(path);
            let rest =
                package.as_deref().and_then(|package| rest.strip_prefix(package)).unwrap_or(rest);
            let Some((name, _)) = rest.split_once('/') else {
                loose.push(path.clone());
                continue;
            };
            let dir = &path[..path.len() - rest.len() + name.len()];
            let stem = kebab(name).unwrap_or_else(|| fallback_stem(tree));
            match groups.iter_mut().find(|(s, ..)| *s == stem) {
                Some((_, _, files)) => files.push(path.clone()),
                None => groups.push((stem, format!("`{dir}/`"), vec![path.clone()])),
            }
        }

        // the root's own modules join the first group
        if groups.is_empty() {
            groups.push((fallback_stem(tree), "the root".to_owned(), Vec::new()));
        }
        if !loose.is_empty() {
            let (_, dir, files) = &mut groups[0];
            loose.append(files);
            *files = loose;
            dir.push_str(", with the root's own modules");
        }

        groups
            .into_iter()
            .map(|(stem, dir, files)| {
                let text = format!(
                    "{NO_SURFACE} It is past the budget of one call and cut by directory: this \
                     call mines the {} under {dir} alone, under the stem `{stem}` — lead every \
                     `requirement` and `criterion` id with it. What another directory's modules \
                     do is another call's to claim, even where these import them.",
                    if files.len() == 1 {
                        "module".to_owned()
                    } else {
                        format!("{} modules", files.len())
                    },
                );
                Self::new(text, files, vec![stem])
            })
            .collect()
    }

    // A tree with no surface has no anchors, so its exports are read for what
    // they do. Neither a data file nor a test is an anchor. A test importing
    // no module of the tree follows every seam.
    pub(super) fn finish<R: Recogniser>(self, tree: &Tree<R>, surfaces: &[Surface]) -> Seam {
        let files = &self.seam.files;
        let under: Vec<&Surface> =
            surfaces.iter().filter(|surface| self.seam.stems.contains(&surface.stem)).collect();
        let anchors = if surfaces.is_empty() {
            Vec::new()
        } else {
            skeleton::anchors(tree, files, under.iter().copied())
        };
        let attached: Vec<&Test> = tree
            .tests
            .iter()
            .filter(|test| {
                test.imports.is_empty() || test.imports.iter().any(|m| files.contains(m))
            })
            .collect();
        let text = self.brief(tree, &attached);
        let Seam { files, stems, .. } = self.seam;
        let mut files = with_data(tree, files);
        files.extend(attached.iter().map(|test| test.path.clone()));
        Seam {
            text,
            files,
            stems,
            anchors,
        }
    }

    fn brief<R: Recogniser>(&self, tree: &Tree<R>, tests: &[&Test]) -> String {
        let dialect = tree.dialect;
        let files = &self.seam.files;
        let modules = || files.iter().filter_map(|path| tree.modules.get(path)).map(|m| &**m);
        let mut sections = vec![self.seam.text.clone()];
        sections.extend(skeleton::boundaries(dialect, modules()));
        sections.extend(skeleton::packages(modules()));
        sections.extend(skeleton::calls(tree, files));
        sections.extend(skeleton::decisions(modules()));
        sections.extend(skeleton::data(modules()));
        sections.extend(stated(tests.iter().copied(), dialect.described));
        sections.extend(skeleton::unfollowed(modules(), &self.widened));
        let unparsed: Vec<String> =
            modules().filter(|m| !m.parsed).map(|m| format!("`{}`", m.path)).collect();
        if !unparsed.is_empty() {
            sections.push(format!(
                "The parser could not read {} whole; what {} declares is not in the lists above \
                 and is read from the text alone.",
                unparsed.join(", "),
                if unparsed.len() == 1 { "it" } else { "each" }
            ));
        }
        sections.join("\n\n")
    }
}

// The directory a barrel module makes importable as one package.
fn barrel<'p>(barrels: &[&str], path: &'p str) -> Option<&'p str> {
    let (dir, file) = path.rsplit_once('/')?;
    let stem = file.split_once('.').map_or(file, |(stem, _)| stem);
    barrels.contains(&stem).then_some(dir)
}

// The package's name, else the root directory's, else `module`.
fn fallback_stem<R: Recogniser>(tree: &Tree<R>) -> String {
    let named = tree
        .manifest
        .name
        .as_deref()
        .and_then(|name| kebab(name.rsplit('/').next().unwrap_or(name)));
    named
        .or_else(|| tree.root.file_name().and_then(|name| name.to_str()).and_then(kebab))
        .unwrap_or_else(|| "module".to_owned())
}

// Each data file a module names, once: a small one directly after the first
// module naming it, a large one after every module.
fn with_data<R: Recogniser>(tree: &Tree<R>, modules: Vec<String>) -> Vec<String> {
    let mut files: Vec<String> = Vec::with_capacity(modules.len());
    let mut large: Vec<String> = Vec::new();
    for path in modules {
        let named = tree.modules.get(&path).map(|module| module.data()).unwrap_or_default();
        files.push(path);
        for data in named {
            if files.iter().chain(&large).any(|file| file == data) {
                continue;
            }
            let size = std::fs::metadata(tree.root.join(data)).map_or(u64::MAX, |m| m.len());
            if size <= DATA_BESIDE_BYTES {
                files.push(data.to_owned());
            } else {
                large.push(data.to_owned());
            }
        }
    }
    files.extend(large);
    files
}

fn listed<'s>(surfaces: impl IntoIterator<Item = &'s Surface>) -> String {
    surfaces
        .into_iter()
        .map(|surface| {
            let ids: Vec<String> = surface.ids.iter().map(|id| format!("`{id}`")).collect();
            let reached: Vec<String> = surface
                .closure
                .iter()
                .filter(|path| **path != surface.entry)
                .map(|path| format!("`{path}`"))
                .collect();
            format!(
                "- Surface `{}` — entry `{}` — stem `{}`: {}; {} {}; {}.",
                surface.name,
                surface.entry,
                surface.stem,
                surface.detail.join("; "),
                if ids.len() == 1 { "id" } else { "ids" },
                ids.join(", "),
                if reached.is_empty() {
                    "reaches nothing beyond its entry".to_owned()
                } else {
                    format!("reaches {}", reached.join(", "))
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
