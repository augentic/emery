//! Normalises a path named beneath the source root.

/// Returns `path` as a `/`-separated path beneath the root, or why it is not one.
///
/// Empty and `.` segments are dropped. A leading `/` or a `..` segment
/// escapes the root; a path with no segment left names no file. The reason
/// reads after the path it describes: `` `x` escapes the source root ``.
pub fn beneath(path: &str) -> Result<String, &'static str> {
    if path.starts_with('/') || path.split('/').any(|segment| segment == "..") {
        return Err("escapes the source root");
    }

    let segments: Vec<&str> =
        path.split('/').filter(|segment| !segment.is_empty() && *segment != ".").collect();
    if segments.is_empty() {
        return Err("names no file");
    }

    Ok(segments.join("/"))
}
