//! Places an edit's `find` in its file: by marker key, as written, or with its layout read past.

use std::fmt;
use std::ops::Range;

use crate::target::Marker;

// How one edit's `find` was placed in its file: by the key of a marker the
// file is guarded with, matching once as written, or matching once with its
// layout whitespace read past.
#[derive(Clone, Copy)]
pub enum Step {
    Key,
    Exact,
    Layout,
}

impl Step {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Exact => "exact",
            Self::Layout => "layout",
        }
    }
}

pub struct Placed {
    pub file: String,
    pub step: Step,
}

// Logged as `"path"=step`, so a reader of the turn's events counts the
// edits that reached a marker by its key, matched as written, or needed
// the layout read past.
impl fmt::Debug for Placed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}={}", self.file, self.step.as_str())
    }
}

// Why an edit's `find` placed nothing: no place, several, or a marker named
// by its key whose block the file no longer holds.
pub enum Miss {
    Nothing,
    Several(usize),
    Filled(String),
}

// Where `find` stands in `content`, and the step that placed it: the block
// of the marker whose key `find` is, else the one place `find` matches as
// written, else the one place it matches with its layout whitespace read
// past — so a key names a block without its lines, and a block quoted in
// the brief matches where the formatter has re-laid it.
pub fn place(content: &str, find: &str, markers: &[Marker]) -> Result<(Range<usize>, Step), Miss> {
    if let Some(marker) = markers.iter().find(|marker| marker.key == find.trim()) {
        return match span(content, &marker.block) {
            Ok((range, _)) => Ok((range, Step::Key)),
            Err(Miss::Nothing) => Err(Miss::Filled(marker.key.clone())),
            Err(miss) => Err(miss),
        };
    }
    span(content, find)
}

// The one place `text` stands in `content`, as written or modulo layout.
fn span(content: &str, text: &str) -> Result<(Range<usize>, Step), Miss> {
    let mut exact = content.match_indices(text).map(|(start, _)| start..start + text.len());
    match (exact.next(), exact.count()) {
        (Some(range), 0) => Ok((range, Step::Exact)),
        (Some(_), more) => Err(Miss::Several(more + 1)),
        (None, _) => Canonical::of(content).find(content, text).map(|range| (range, Step::Layout)),
    }
}

// A text with its layout whitespace read past: outside string literals and
// line comments, each whitespace run dropped, or kept as one space where it
// parts two identifier characters, and every kept byte mapped to where it
// stands in the text. Literals and comments are kept as written, since
// their whitespace is theirs.
struct Canonical {
    text: Vec<u8>,
    origin: Vec<usize>,
}

impl Canonical {
    fn of(content: &str) -> Self {
        let bytes = content.as_bytes();
        let mut canonical = Self {
            text: Vec::with_capacity(bytes.len()),
            origin: Vec::with_capacity(bytes.len()),
        };
        let mut pending = None;
        let mut verbatim = Verbatim::None;
        let mut at = 0;
        while at < bytes.len() {
            let byte = bytes[at];
            match verbatim {
                Verbatim::Literal => {
                    canonical.keep(byte, at);
                    if byte == b'\\' && at + 1 < bytes.len() {
                        canonical.keep(bytes[at + 1], at + 1);
                        at += 1;
                    } else if byte == b'"' {
                        verbatim = Verbatim::None;
                    }
                }
                Verbatim::Comment => {
                    canonical.keep(byte, at);
                    if byte == b'\n' {
                        verbatim = Verbatim::None;
                    }
                }
                Verbatim::None if byte.is_ascii_whitespace() => {
                    pending.get_or_insert(at);
                }
                Verbatim::None => {
                    if let Some(run) = pending.take()
                        && is_identifier(byte)
                        && canonical.text.last().is_some_and(|&last| is_identifier(last))
                    {
                        canonical.keep(b' ', run);
                    }
                    canonical.keep(byte, at);
                    // HACK: a quote in a char literal, `'"'`, would open a
                    // literal that never closes, so it is kept whole here
                    if byte == b'\'' && bytes[at + 1..].starts_with(b"\"'") {
                        canonical.keep(bytes[at + 1], at + 1);
                        canonical.keep(bytes[at + 2], at + 2);
                        at += 2;
                    } else if byte == b'"' {
                        verbatim = Verbatim::Literal;
                    } else if byte == b'/' && bytes.get(at + 1) == Some(&b'/') {
                        verbatim = Verbatim::Comment;
                    }
                }
            }
            at += 1;
        }
        canonical
    }

    fn keep(&mut self, byte: u8, at: usize) {
        self.text.push(byte);
        self.origin.push(at);
    }

    // The one place `text` stands in the canonical content, as the bytes of
    // `content` it spans: the indentation before it taken where `text` opens
    // with whitespace, and the whitespace after it taken as `text`'s own
    // trailing whitespace has it, line end for line end and space for space,
    // so a block copied with the indentation of the line after it still
    // leaves that line as the file holds it.
    fn find(&self, content: &str, text: &str) -> Result<Range<usize>, Miss> {
        let sought = Self::of(text).text;
        if sought.is_empty() {
            return Err(Miss::Nothing);
        }
        let hits = occurrences(&self.text, &sought);
        let &[hit] = hits.as_slice() else {
            return Err(if hits.is_empty() { Miss::Nothing } else { Miss::Several(hits.len()) });
        };
        let bytes = content.as_bytes();
        let mut start = self.origin[hit];
        let mut end = self.origin[hit + sought.len() - 1] + 1;

        // the indentation before the opening line
        if text.starts_with(|c: char| c.is_whitespace()) {
            while start > 0 && matches!(bytes[start - 1], b' ' | b'\t') {
                start -= 1;
            }
        }

        // the whitespace after, as `text` has it
        for byte in text[text.trim_end().len()..].bytes() {
            if byte == b'\n' {
                let mut probe = end;
                while probe < bytes.len() && matches!(bytes[probe], b' ' | b'\t' | b'\r') {
                    probe += 1;
                }
                if probe < bytes.len() && bytes[probe] == b'\n' {
                    end = probe + 1;
                } else {
                    break;
                }
            } else if end < bytes.len() && matches!(bytes[end], b' ' | b'\t') {
                end += 1;
            }
        }
        Ok(start..end)
    }
}

// What the canonical reading keeps as written: a string literal to its
// closing quote, a line comment to its line end, or nothing.
#[derive(Clone, Copy)]
enum Verbatim {
    None,
    Literal,
    Comment,
}

const fn is_identifier(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

// Where `sought` stands in `text`, each occurrence once and none
// overlapping, as `str::matches` counts them.
fn occurrences(text: &[u8], sought: &[u8]) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut from = 0;
    while let Some(offset) = text[from..].windows(sought.len()).position(|window| window == sought)
    {
        hits.push(from + offset);
        from += offset + sought.len();
    }
    hits
}

// What the file holds nearest a `find` that matched nothing: the line its
// first line is found on, the occurrence the most lines of `find` follow
// where it is found more than once, and the line where the file parts from
// `find` after them; the opener of a call the formatter broke after it,
// `todo!(`, and the line after that, where no line holds the first line
// whole; or the file's opening line and length when no line holds it, so
// the refusal carries the text to copy.
pub fn nearest(content: &str, find: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let Some(opening) = lines.first() else {
        return "the file is empty".to_owned();
    };
    let sought: Vec<&str> = find.lines().skip_while(|line| line.trim().is_empty()).collect();
    let first = sought.first().map(|line| line.trim());
    let found = first.and_then(|first| {
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(first))
            .map(|(index, _)| (index, matched(&lines[index + 1..], &sought[1..])))
            .min_by_key(|&(index, followed)| (std::cmp::Reverse(followed), index))
    });
    let Some((index, followed)) = found else {
        if let Some((index, opener)) = first.and_then(|first| broken(&lines, first)) {
            let next = lines
                .get(index + 1)
                .map_or_else(String::new, |next| format!(", and line {} is `{next}`", index + 2));
            return format!(
                "no line holds its first line whole; its opener `{opener}` is line {} of \
                 {total}, the call broken over lines after it{next}",
                index + 1
            );
        }
        return if total == 1 {
            format!("no line holds its first line; the file's one line is `{opening}`")
        } else {
            format!(
                "no line holds its first line; the file has {total} lines and opens with \
                 `{opening}`"
            )
        };
    };
    let (line, number) = (lines[index], index + 1);
    let parted = index + 1 + followed;
    let following = match followed {
        0 => String::new(),
        1 => format!(", line {} matches", number + 1),
        _ => format!(", lines {} to {} match", number + 1, number + followed),
    };
    match (followed, lines.get(parted)) {
        (0, None) => format!("its first line is on line {number} of {total}, `{line}`, the last"),
        (_, None) => format!(
            "its first line is on line {number} of {total}, `{line}`{following}, and the file \
             ends there"
        ),
        (_, Some(next)) => format!(
            "its first line is on line {number} of {total}, `{line}`{following}, and line {} is \
             `{next}`",
            parted + 1
        ),
    }
}

// How many lines of `sought` the `lines` open with, each equal to its own
// but for the whitespace around it.
fn matched(lines: &[&str], sought: &[&str]) -> usize {
    lines.iter().zip(sought).take_while(|(line, want)| line.trim() == want.trim()).count()
}

// The line a call `first` opens, `todo!("…")`, stands on as its opener
// alone, `todo!(`, where the formatter broke the call after it: the one
// whose next line shares the most of what follows the opener in `first`,
// so the right marker is quoted where several are broken.
fn broken<'a>(lines: &[&'a str], first: &str) -> Option<(usize, &'a str)> {
    let opener = &first[..=first.find('(')?];
    let rest = first[opener.len()..].trim();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim() == opener)
        .map(|(index, line)| {
            let next = lines.get(index + 1).map_or("", |next| next.trim());
            let shared = next.bytes().zip(rest.bytes()).take_while(|(a, b)| a == b).count();
            (index, line.trim(), shared)
        })
        .max_by_key(|&(index, _, shared)| (shared, std::cmp::Reverse(index)))
        .map(|(index, line, _)| (index, line))
}
