//! Validates root-relative paths within a source.

/// Returns a normalised root-relative path or an explanatory error.
///
/// Empty and `.` segments are removed. A leading `/`, any `..` segment, or a
/// path with no remaining segments is rejected. Error text is phrased to
/// follow the offending path, as in `` `x` escapes the source root ``.
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
