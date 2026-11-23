//! Extracts leaf routine spans, imports, and top-level values from Python source lines.

use super::Scanned;
use crate::specify::snapshot::LineSpan;

pub struct PythonScanner {
    stack: Vec<Block>,
    in_triple: Option<u8>,
    bracket_depth: usize,
    continued: bool,
    pending_decorator: Option<Decorator>,
    last_code_line: usize,
    out: Scanned,
}

struct Block {
    indent: usize,
    start_line: usize,
    is_routine: bool,
    has_nested_routine: bool,
}

impl Block {
    const fn leaf_span(&self, end_line: usize) -> Option<LineSpan> {
        if self.is_routine && !self.has_nested_routine && end_line >= self.start_line {
            Some(LineSpan::new(self.start_line, end_line))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
struct Decorator {
    indent: usize,
    line: usize,
}

struct LineInfo {
    has_code: bool,
    backslash: bool,
    top_level_eq: bool,
}

impl PythonScanner {
    #[must_use]
    pub fn scan(text: &str, lines: Vec<u64>) -> Scanned {
        let raw_lines: Vec<&str> = text.lines().collect();
        let mut scanner = Self {
            stack: Vec::new(),
            in_triple: None,
            bracket_depth: 0,
            continued: false,
            pending_decorator: None,
            last_code_line: 0,
            out: Scanned::with_lines(lines),
        };
        scanner.run(&raw_lines);
        scanner.out.finalise();
        scanner.out
    }

    fn run(&mut self, raw_lines: &[&str]) {
        let mut idx = 0usize;
        while idx < raw_lines.len() {
            let line_no = idx + 1;
            let line = raw_lines[idx];

            // continuation inside a multiline string, open delimiter, or trailing `\`
            if self.in_triple.is_some() || self.bracket_depth > 0 || self.continued {
                let info = self.scan_line(line);
                self.continued = info.backslash;
                if info.has_code {
                    self.last_code_line = line_no;
                }
                idx += 1;
                continue;
            }

            let (indent, trimmed) = indent_and_trimmed(line);
            if trimmed.is_empty() || trimmed.starts_with('#') {
                idx += 1;
                continue;
            }

            // close open blocks at or deeper than this logical statement's indentation
            self.close_blocks(indent);
            if self.pending_decorator.is_some_and(|dec| dec.indent != indent) {
                self.pending_decorator = None;
            }

            // advance through the end of this logical statement
            let stmt_start_line = line_no;
            let mut stmt_has_top_eq = false;
            let mut stmt_end_idx = idx;
            loop {
                let info = self.scan_line(raw_lines[stmt_end_idx]);
                stmt_has_top_eq |= info.top_level_eq;
                self.continued = info.backslash;
                self.last_code_line = stmt_end_idx + 1;
                if (self.in_triple.is_none() && self.bracket_depth == 0 && !self.continued)
                    || stmt_end_idx + 1 >= raw_lines.len()
                {
                    break;
                }
                stmt_end_idx += 1;
            }
            let stmt_end_line = stmt_end_idx + 1;

            self.classify_statement(
                indent,
                trimmed,
                stmt_start_line,
                stmt_end_line,
                stmt_has_top_eq,
            );
            idx = stmt_end_idx + 1;
        }

        self.close_blocks(0);
    }

    fn classify_statement(
        &mut self, indent: usize, trimmed: &str, start_line: usize, end_line: usize,
        has_top_eq: bool,
    ) {
        if trimmed.starts_with('@') {
            self.pending_decorator.get_or_insert(Decorator {
                indent,
                line: start_line,
            });
        } else if starts_with_keyword(trimmed, "import") || starts_with_keyword(trimmed, "from") {
            self.pending_decorator = None;
            if indent == 0 {
                self.out.imports.extend(start_line..=end_line);
            }
        } else if starts_with_keyword(trimmed, "def") || starts_with_async_def(trimmed) {
            let start = self.pending_decorator.take().map_or(start_line, |dec| dec.line);
            for ancestor in &mut self.stack {
                if ancestor.is_routine {
                    ancestor.has_nested_routine = true;
                }
            }
            self.stack.push(Block {
                indent,
                start_line: start,
                is_routine: true,
                has_nested_routine: false,
            });
        } else if starts_with_keyword(trimmed, "class") {
            let start = self.pending_decorator.take().map_or(start_line, |dec| dec.line);
            self.stack.push(Block {
                indent,
                start_line: start,
                is_routine: false,
                has_nested_routine: false,
            });
        } else {
            self.pending_decorator = None;
            if indent == 0 && has_top_eq && !starts_with_keyword(trimmed, "type") {
                self.out.values.extend(start_line..=end_line);
            }
        }
    }

    fn close_blocks(&mut self, min_keep_indent: usize) {
        while self.stack.last().is_some_and(|top| top.indent >= min_keep_indent) {
            let Some(block) = self.stack.pop() else { break };
            if let Some(span) = block.leaf_span(self.last_code_line) {
                self.out.routines.push(span);
            }
        }
    }

    fn scan_line(&mut self, line: &str) -> LineInfo {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut p = 0usize;
        let mut has_code = self.in_triple.is_some();
        let mut top_level_eq = false;
        let mut last_non_ws = None;

        while p < len {
            if let Some(quote) = self.in_triple {
                if bytes.get(p..p + 3) == Some(&[quote, quote, quote]) {
                    self.in_triple = None;
                    last_non_ws = Some(quote);
                    p += 3;
                } else if bytes[p] == b'\\' {
                    p = (p + 2).min(len);
                } else {
                    p += 1;
                }
                continue;
            }

            let b = bytes[p];
            if matches!(b, b' ' | b'\t' | b'\r') {
                p += 1;
                continue;
            }
            if b == b'#' {
                break;
            }
            has_code = true;

            if (b == b'"' || b == b'\'') && bytes.get(p..p + 3) == Some(&[b, b, b]) {
                self.in_triple = Some(b);
                last_non_ws = Some(b);
                p += 3;
                continue;
            }
            if b == b'"' || b == b'\'' {
                p += 1;
                while p < len {
                    if bytes[p] == b'\\' {
                        p = (p + 2).min(len);
                    } else if bytes[p] == b {
                        p += 1;
                        break;
                    } else {
                        p += 1;
                    }
                }
                last_non_ws = Some(b);
                continue;
            }
            if matches!(b, b'(' | b'[' | b'{') {
                self.bracket_depth += 1;
                last_non_ws = Some(b);
                p += 1;
                continue;
            }
            if matches!(b, b')' | b']' | b'}') {
                self.bracket_depth = self.bracket_depth.saturating_sub(1);
                last_non_ws = Some(b);
                p += 1;
                continue;
            }
            if b == b'='
                && self.bracket_depth == 0
                && !matches!(bytes.get(p + 1), Some(b'='))
                && !matches!(
                    p.checked_sub(1).and_then(|prev| bytes.get(prev)),
                    Some(b'=' | b'!' | b'<' | b'>' | b':')
                )
            {
                top_level_eq = true;
            }
            last_non_ws = Some(b);
            p += 1;
        }

        LineInfo {
            has_code,
            backslash: self.in_triple.is_none()
                && self.bracket_depth == 0
                && last_non_ws == Some(b'\\'),
            top_level_eq,
        }
    }
}

fn indent_and_trimmed(line: &str) -> (usize, &str) {
    let mut indent = 0usize;
    let mut byte_offset = 0usize;
    for b in line.bytes() {
        match b {
            b' ' => {
                indent += 1;
                byte_offset += 1;
            }
            b'\t' => {
                indent += 4;
                byte_offset += 1;
            }
            _ => break,
        }
    }
    (indent, &line[byte_offset..])
}

fn starts_with_keyword(trimmed: &str, keyword: &str) -> bool {
    trimmed
        .strip_prefix(keyword)
        .is_some_and(|rest| rest.bytes().next().is_some_and(|b| b == b' ' || b == b'\t'))
}

fn starts_with_async_def(trimmed: &str) -> bool {
    trimmed
        .strip_prefix("async")
        .filter(|rest| rest.bytes().next().is_some_and(|b| b == b' ' || b == b'\t'))
        .is_some_and(|rest| starts_with_keyword(rest.trim_start(), "def"))
}
