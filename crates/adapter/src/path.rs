//! Holds a path to the root it is relative to.
//!
//! A claim's anchor and a build's written file are both `/`-separated paths
//! beneath a root the engine lends: the source root for a claim, the
//! workspace root for a report. [`beneath`] is the one rule both gates hold
//! such a path to, and [`BadPath`] names what it breaks.

use std::fmt::{self, Display, Formatter};

/// The directories no path may name.
///
/// `.emery/` is the engine's own root, where the committed revision lives,
/// and `.git/` is the checkout's; both wherever they occur under a root.
pub const SKIP_DIRS: &[&str] = &[".emery", ".git"];

/// The files of the engine's own that no path may name.
///
/// They are the Markdown projections of the current revision, wherever they
/// occur under a root.
pub const SKIP_FILES: &[&str] = &["spec.md", "design.md", "plan.md"];

/// Returns `path` as a `/`-separated path beneath a root, or the rule it breaks.
///
/// The result drops empty and `.` segments, so two spellings of one file
/// compare equal. A path passes when it is relative, climbs no higher than
/// the root, names a file beneath it, and names nothing reserved: no segment
/// of [`SKIP_DIRS`] and no final segment of [`SKIP_FILES`].
///
/// # Errors
///
/// Returns [`BadPath`] describing the first rule the path breaks.
///
/// # Examples
///
/// ```
/// use emery_adapter::{BadPath, beneath};
///
/// assert_eq!(beneath("./src//orders.rs"), Ok("src/orders.rs".to_owned()));
/// assert_eq!(beneath("../secret"), Err(BadPath::Escapes));
/// assert_eq!(beneath("./"), Err(BadPath::NoFile));
/// assert_eq!(beneath(".emery/storage/x"), Err(BadPath::SkipDir(".emery".to_owned())));
/// assert_eq!(beneath("docs/spec.md"), Err(BadPath::SkipFile("spec.md".to_owned())));
/// ```
pub fn beneath(path: &str) -> Result<String, BadPath> {
    if path.is_empty() || path.starts_with('/') || path.split('/').any(|segment| segment == "..") {
        return Err(BadPath::Escapes);
    }

    let segments: Vec<&str> =
        path.split('/').filter(|segment| !segment.is_empty() && *segment != ".").collect();
    let Some(file) = segments.last() else {
        return Err(BadPath::NoFile);
    };
    if let Some(dir) = segments.iter().find(|segment| SKIP_DIRS.contains(segment)) {
        return Err(BadPath::SkipDir((*dir).to_owned()));
    }
    if SKIP_FILES.contains(file) {
        return Err(BadPath::SkipFile((*file).to_owned()));
    }

    Ok(segments.join("/"))
}

/// The rule a root-relative path breaks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BadPath {
    /// The path is empty, absolute, or climbs above the root.
    Escapes,
    /// The path names the root itself, not a file beneath it.
    NoFile,
    /// The path is under a directory of [`SKIP_DIRS`].
    SkipDir(String),
    /// The path names a file of [`SKIP_FILES`].
    SkipFile(String),
}

impl Display for BadPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Escapes => f.write_str("escapes the root"),
            Self::NoFile => f.write_str("names no file"),
            Self::SkipDir(dir) => write!(f, "is under the reserved `{dir}/`"),
            Self::SkipFile(file) => write!(f, "names the engine's own `{file}`"),
        }
    }
}

impl std::error::Error for BadPath {}
