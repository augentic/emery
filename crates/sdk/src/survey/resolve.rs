//! Names what an import leads to, and spells the path a specifier reaches.
//!
//! An adapter's resolver follows each import the way its language's loader
//! would and settles the outcome as a [`Target`]: a module of the tree, a
//! package outside it, a data file, or nothing the tree answers. A survey
//! reads the targets to close over a surface's modules, to list the
//! packages a seam calls through, and to say what it could not follow.
//! [`normalize`] spells the root-relative path a relative specifier reaches.

/// What one import leads to, as the adapter's resolver settled it.
///
/// An import the tree does not answer is [`Target::Unresolved`], never
/// dropped, so a survey can say what it could not follow and widen what it
/// lays. Each accessor answers for its own variant alone.
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
