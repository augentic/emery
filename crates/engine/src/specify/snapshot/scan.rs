//! Extracts deterministic line hashes and structural line spans from UTF-8 source text.

mod braces;
mod lexer;
mod python;

use std::path::Path;

use sha2::{Digest as _, Sha256};

use self::braces::BraceScanner;
use self::lexer::Dialect;
use self::python::PythonScanner;
use super::LineSpan;

#[derive(Debug, Default)]
pub struct Scanned {
    pub lines: Vec<u64>,
    pub routines: Vec<LineSpan>,
    pub imports: Vec<usize>,
    pub values: Vec<usize>,
}

impl Scanned {
    fn with_lines(lines: Vec<u64>) -> Self {
        Self {
            lines,
            ..Self::default()
        }
    }

    fn finalise(&mut self) {
        self.routines.sort_unstable();
        self.routines.dedup();
        self.imports.sort_unstable();
        self.imports.dedup();
        self.values.sort_unstable();
        self.values.dedup();
    }
}

#[must_use]
pub fn scan(path: &str, text: &str) -> Scanned {
    let lines: Vec<u64> = text.lines().map(hash_line).collect();
    if lines.is_empty() {
        return Scanned::default();
    }

    match FileKind::of(path) {
        FileKind::ProseOrData => Scanned::with_lines(lines),
        FileKind::Python => PythonScanner::scan(text, lines),
        FileKind::Rust => BraceScanner::scan(text, lines, Dialect::Rust),
        FileKind::JsTs => BraceScanner::scan(text, lines, Dialect::JsTs),
    }
}

fn hash_line(line: &str) -> u64 {
    let digest = Sha256::digest(line.as_bytes());
    u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileKind {
    ProseOrData,
    Python,
    Rust,
    JsTs,
}

impl FileKind {
    fn of(path: &str) -> Self {
        let Some(ext) = Path::new(path).extension().and_then(|ext| ext.to_str()) else {
            return Self::JsTs;
        };
        match ext.to_ascii_lowercase().as_str() {
            "py" | "pyi" => Self::Python,
            "rs" => Self::Rust,
            "md" | "markdown" | "txt" | "rst" | "adoc" | "json" | "toml" | "yaml" | "yml"
            | "sql" | "csv" | "xml" | "html" | "htm" | "css" | "svg" | "lock" | "feature" => {
                Self::ProseOrData
            }
            _ => Self::JsTs,
        }
    }
}
