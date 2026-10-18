//! Import resolution targets and relative path normalisation.
//!
//! An adapter's resolver maps each import specifier to a [`Target`] and uses
//! [`normalize`] to resolve relative paths beneath the workspace root.

/// What one import leads to, as the adapter's resolver settled it.
///
/// An import the tree does not answer is [`Target::Unresolved`], never
/// dropped, so a survey can say what it could not follow and widen what it
/// mines. One the resolver followed to a file the survey set aside is
/// [`Target::Skipped`]: known, so it widens nothing, and no module's, so it
/// reaches nothing. Each accessor answers for its own variant alone.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Target {
    /// A module of the tree, by root-relative path.
    Module(String),
    /// A package outside the tree, by the name the resolver reads: the
    /// specifier as written, or its top-level segment.
    Package(String),
    /// A data file of the tree, by root-relative path.
    Data(String),
    /// A file of the tree the survey's keep policy set aside, by
    /// root-relative path: a test helper, a migration.
    Skipped(String),
    /// A relative or aliased specifier no module or data file of the tree
    /// answers, as written.
    Unresolved(String),
}

impl Target {
    /// Returns the module's root-relative path, for a [`Target::Module`].
    #[must_use]
    pub fn module(&self) -> Option<&str> {
        match self {
            Self::Module(path) => Some(path),
            _ => None,
        }
    }

    /// Returns the package's name, for a [`Target::Package`].
    #[must_use]
    pub fn package(&self) -> Option<&str> {
        match self {
            Self::Package(name) => Some(name),
            _ => None,
        }
    }

    /// Returns the data file's root-relative path, for a [`Target::Data`].
    #[must_use]
    pub fn data(&self) -> Option<&str> {
        match self {
            Self::Data(path) => Some(path),
            _ => None,
        }
    }

    /// Returns the file's root-relative path, for a [`Target::Skipped`].
    #[must_use]
    pub fn skipped(&self) -> Option<&str> {
        match self {
            Self::Skipped(path) => Some(path),
            _ => None,
        }
    }

    /// Returns the specifier as written, for a [`Target::Unresolved`].
    #[must_use]
    pub fn unresolved(&self) -> Option<&str> {
        match self {
            Self::Unresolved(specifier) => Some(specifier),
            _ => None,
        }
    }
}

/// Returns the root-relative path `path` reaches from the directory `dir`.
///
/// Both are `/`-separated and root-relative, `dir` empty for the root
/// itself. Empty segments and `.` are dropped and `..` climbs one segment;
/// `None` where it climbs above the root.
///
/// # Examples
///
/// ```
/// use emery_sdk::survey::resolve::normalize;
///
/// assert_eq!(normalize("src/routes", "./orders"), Some("src/routes/orders".to_owned()));
/// assert_eq!(normalize("src/routes", "../lib/db"), Some("src/lib/db".to_owned()));
/// assert_eq!(normalize("", "../secret"), None);
/// ```
#[must_use]
pub fn normalize(dir: &str, path: &str) -> Option<String> {
    let mut segments: Vec<&str> = Vec::new();
    for segment in dir.split('/').chain(path.split('/')) {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    Some(segments.join("/"))
}
