//! Holds a path to the root it is relative to.
//!
//! A claim's anchor and a build's written file are both `/`-separated paths
//! beneath a root the engine lends: the source root for a claim, the
//! workspace root for a report. [`beneath`] is the one rule both gates hold
//! such a path to, and [`BadPath`] names what it breaks. [`standing`] reads
//! what a tree holds at such a path without following a link anywhere on
//! it, and [`write_whole`] writes a file there the same way, since the
//! lexical rule cannot see a link the tree holds.

use std::fmt::{self, Display, Formatter};
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write as _};
use std::path::Path;

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

/// The root-level directory the committed specification lives in.
///
/// Reserved at the root alone, so a project's `src/spec/` is any other
/// directory.
pub const SPECIFICATION: &str = "spec";

/// Returns `path` as a `/`-separated path beneath a root, or the rule it breaks.
///
/// The result drops empty and `.` segments, so two spellings of one file
/// compare equal. A path passes when it is relative, climbs no higher than
/// the root, names a file beneath it, and names nothing reserved: no segment
/// of [`SKIP_DIRS`], no first segment of [`SPECIFICATION`], and no final
/// segment of [`SKIP_FILES`].
///
/// A `..` segment is refused wherever it stands, even where it would resolve
/// beneath the root, since a path a model or an adapter names is held to the
/// spelling the tree holds. The engine's `preopen_path` is the operator's
/// rule, over a path typed at the command line, and steps back over `..`.
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
/// assert_eq!(beneath("spec/spec.json"), Err(BadPath::Specification));
/// assert_eq!(beneath("src/spec/cases.rs"), Ok("src/spec/cases.rs".to_owned()));
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
    if segments[0] == SPECIFICATION {
        return Err(BadPath::Specification);
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
    /// The path leads with the root-level [`SPECIFICATION`] directory.
    Specification,
}

impl Display for BadPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Escapes => f.write_str("escapes the root"),
            Self::NoFile => f.write_str("names no file"),
            Self::SkipDir(dir) => write!(f, "is under the reserved `{dir}/`"),
            Self::SkipFile(file) => write!(f, "names the engine's own `{file}`"),
            Self::Specification => {
                write!(f, "is under `{SPECIFICATION}/`, where the specification lives")
            }
        }
    }
}

impl std::error::Error for BadPath {}

/// What a tree holds at a root-relative path, read without following a link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// Nothing stands at the path, or at some directory on the way to it.
    Absent,
    /// A regular file stands at the path.
    File,
}

/// Returns what the tree beneath `root` holds at `file`, following no link.
///
/// `file` is a root-relative path, as [`beneath`] answers one. Every
/// directory on the way is read by its own metadata, so a symbolic link
/// anywhere on the path is refused rather than followed to wherever the tree
/// reaches, and so is a file standing where a directory is needed.
///
/// # Errors
///
/// Returns [`NoFollow`] naming what stands in the way, or the read the tree
/// refused.
///
/// # Examples
///
/// ```
/// use emery_adapter::{Standing, standing};
///
/// let root = tempfile::tempdir()?;
/// std::fs::create_dir(root.path().join("src"))?;
/// std::fs::write(root.path().join("src/lib.rs"), "")?;
///
/// assert_eq!(standing(root.path(), "src/lib.rs")?, Standing::File);
/// assert_eq!(standing(root.path(), "src/main.rs")?, Standing::Absent);
/// assert_eq!(standing(root.path(), "docs/index.md")?, Standing::Absent);
/// assert!(standing(root.path(), "src").is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn standing(root: &Path, file: &str) -> Result<Standing, NoFollow> {
    let (dirs, _) = file.rsplit_once('/').unwrap_or(("", file));
    let mut prefix = String::with_capacity(file.len());
    for segment in dirs.split('/').filter(|segment| !segment.is_empty()) {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(segment);
        let Some(kind) = kind_at(root, &prefix)? else {
            return Ok(Standing::Absent);
        };
        if kind.is_symlink() {
            return Err(NoFollow::Link(prefix));
        }
        if !kind.is_dir() {
            return Err(NoFollow::NotDirectory(prefix));
        }
    }

    let Some(kind) = kind_at(root, file)? else {
        return Ok(Standing::Absent);
    };
    if kind.is_symlink() {
        Err(NoFollow::Symlink)
    } else if kind.is_dir() {
        Err(NoFollow::Directory)
    } else if kind.is_file() {
        Ok(Standing::File)
    } else {
        Err(NoFollow::NotFile)
    }
}

// The kind of entry at a root-relative path, the link itself where one
// stands there, or none where the tree holds nothing.
fn kind_at(root: &Path, path: &str) -> Result<Option<fs::FileType>, NoFollow> {
    match root.join(path).symlink_metadata() {
        Ok(meta) => Ok(Some(meta.file_type())),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(NoFollow::Io {
            path: path.to_owned(),
            source,
        }),
    }
}

/// Makes the directories of the root-relative path `dirs` beneath `root`.
///
/// Each is made one at a time with `create_dir`, never `create_dir_all`,
/// which follows a link standing where a directory is to go. A directory
/// already there is kept.
///
/// # Errors
///
/// Returns the error the filesystem answered, or one naming the segment
/// where something other than a directory stands.
pub fn make_dirs(root: &Path, dirs: &str) -> io::Result<()> {
    let mut full = root.to_path_buf();
    for segment in dirs.split('/').filter(|segment| !segment.is_empty()) {
        full.push(segment);
        match fs::create_dir(&full) {
            Ok(()) => {}
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                if !full.symlink_metadata()?.file_type().is_dir() {
                    return Err(io::Error::other(format!(
                        "`{}` is not a directory",
                        full.strip_prefix(root).unwrap_or(&full).display()
                    )));
                }
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

/// Writes `bytes` whole at the root-relative `file` beneath `root`.
///
/// `standing` is what [`standing`] read there. A file found absent is
/// created, with the directories above it made through [`make_dirs`], and
/// never followed, so a link raced into its place fails the open; one found
/// held is truncated in place, keeping its mode. `std` has no no-follow open
/// on every target, so that open alone trusts the walk.
///
/// # Errors
///
/// Returns the error the filesystem answered.
pub fn write_whole(root: &Path, file: &str, standing: Standing, bytes: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true);
    match standing {
        Standing::Absent => {
            if let Some((dirs, _)) = file.rsplit_once('/') {
                make_dirs(root, dirs)?;
            }
            options.create_new(true)
        }
        Standing::File => options.truncate(true),
    };
    options.open(root.join(file))?.write_all(bytes)
}

/// Why a root-relative path cannot be read or written without following a link.
///
/// Each variant displays as the clause that follows the path, as
/// [`BadPath`]'s do: `` `link/hook` leads through the symlink `link` ``.
#[derive(Debug)]
pub enum NoFollow {
    /// A symbolic link stands at the named directory on the way.
    Link(String),
    /// Something other than a directory stands at the named directory on the way.
    NotDirectory(String),
    /// A symbolic link stands at the path.
    Symlink,
    /// A directory stands at the path.
    Directory,
    /// Something that is neither a file, a directory, nor a link stands at the path.
    NotFile,
    /// The tree could not be read at the named path.
    Io {
        /// The root-relative path the read was refused at.
        path: String,
        /// The error the filesystem answered.
        source: io::Error,
    },
}

impl Display for NoFollow {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Link(prefix) => write!(f, "leads through the symlink `{prefix}`"),
            Self::NotDirectory(prefix) => {
                write!(f, "leads through `{prefix}`, which is not a directory")
            }
            Self::Symlink => f.write_str("is a symlink"),
            Self::Directory => f.write_str("names a directory"),
            Self::NotFile => f.write_str("is not a regular file"),
            Self::Io { path, source } => write!(f, "cannot be read at `{path}`: {source}"),
        }
    }
}

impl std::error::Error for NoFollow {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
