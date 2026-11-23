//! Line-level LCS diff and routine-span expansion between two [`FileSnapshot`]s.

use std::collections::BTreeSet;

use super::{FileSnapshot, LineSpan};

const MAX_DP_CELLS: usize = 4_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpanDelta {
    Whole,
    Lines { base: ActiveLines, current: ActiveLines },
}

impl SpanDelta {
    #[must_use]
    pub fn between(base_file: &FileSnapshot, current_file: &FileSnapshot) -> Self {
        if !base_file.has_lines() || !current_file.has_lines() {
            return Self::Whole;
        }
        let Some(matches) = lcs_matches(&base_file.lines, &current_file.lines) else {
            return Self::Whole;
        };

        let base_len = base_file.lines.len();
        let current_len = current_file.lines.len();
        let mut base = ActiveLines::new(base_len);
        let mut current = ActiveLines::new(current_len);
        let mut whole_current = false;

        let mut prev_b = 0usize;
        let mut prev_c = 0usize;
        for (next_b, next_c) in matches.into_iter().chain([(base_len + 1, current_len + 1)]) {
            let base_hunk: Vec<usize> =
                ((prev_b + 1)..next_b).filter(|&line| !base_file.is_import_line(line)).collect();
            let curr_hunk: Vec<usize> =
                ((prev_c + 1)..next_c).filter(|&line| !current_file.is_import_line(line)).collect();
            prev_b = next_b;
            prev_c = next_c;

            match (base_hunk.is_empty(), curr_hunk.is_empty()) {
                (true, true) => {}
                (false, true) => {
                    base.extend_exact(base_hunk);
                }
                (true, false) => {
                    current.record_current_hunk(&curr_hunk, current_file, &mut whole_current);
                }
                (false, false) => {
                    for line in base_hunk {
                        base.extend_with_routines(line, &base_file.routines);
                    }
                    current.record_current_hunk(&curr_hunk, current_file, &mut whole_current);
                }
            }
        }

        if whole_current {
            current = ActiveLines::all(current_len);
        }

        Self::Lines { base, current }
    }

    #[must_use]
    pub fn overlaps_current(&self, span: Option<LineSpan>) -> bool {
        match self {
            Self::Whole => true,
            Self::Lines { current, .. } => current.overlaps(span),
        }
    }

    #[must_use]
    pub fn overlaps_base(&self, span: Option<LineSpan>) -> bool {
        match self {
            Self::Whole => true,
            Self::Lines { base, .. } => base.overlaps(span),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLines {
    line_count: usize,
    lines: BTreeSet<usize>,
}

impl ActiveLines {
    const fn new(line_count: usize) -> Self {
        Self {
            line_count,
            lines: BTreeSet::new(),
        }
    }

    fn all(line_count: usize) -> Self {
        Self {
            line_count,
            lines: (1..=line_count).collect(),
        }
    }

    fn extend_exact(&mut self, hunk: impl IntoIterator<Item = usize>) {
        self.lines.extend(hunk);
    }

    fn extend_with_routines(&mut self, line: usize, routines: &[LineSpan]) {
        self.lines.insert(line);
        for &span in routines {
            if span.contains(line) {
                self.lines.extend(span.lines());
            }
        }
    }

    fn record_current_hunk(
        &mut self, hunk: &[usize], current_file: &FileSnapshot, whole_current: &mut bool,
    ) {
        if hunk.iter().any(|&line| current_file.is_value_line(line)) {
            *whole_current = true;
            return;
        }
        if *whole_current {
            return;
        }
        for &line in hunk {
            self.extend_with_routines(line, &current_file.routines);
        }
    }

    fn overlaps(&self, span: Option<LineSpan>) -> bool {
        let Some(span) = span else {
            return true;
        };
        span.end > self.line_count || self.lines.range(span.lines()).next().is_some()
    }
}

// Computes forward-biased 1-based LCS line matches, or `None` when the diff matrix exceeds cap.
fn lcs_matches(base: &[u64], current: &[u64]) -> Option<Vec<(usize, usize)>> {
    let prefix = base.iter().zip(current).take_while(|(left, right)| left == right).count();

    let mut matches: Vec<(usize, usize)> = (1..=prefix).map(|line| (line, line)).collect();
    let b_rem = &base[prefix..];
    let c_rem = &current[prefix..];
    let m = b_rem.len();
    let n = c_rem.len();
    if m == 0 || n == 0 {
        return Some(matches);
    }
    if m.saturating_mul(n) > MAX_DP_CELLS {
        return None;
    }

    let width = n + 1;
    let mut dp = vec![0u32; (m + 1) * width];
    for i in (0..m).rev() {
        let row = i * width;
        let next_row = (i + 1) * width;
        for j in (0..n).rev() {
            dp[row + j] = if b_rem[i] == c_rem[j] {
                dp[next_row + j + 1] + 1
            } else {
                dp[next_row + j].max(dp[row + j + 1])
            };
        }
    }

    let (mut i, mut j) = (0usize, 0usize);
    while i < m && j < n {
        if b_rem[i] == c_rem[j] {
            matches.push((prefix + i + 1, prefix + j + 1));
            i += 1;
            j += 1;
        } else if dp[(i + 1) * width + j] >= dp[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }

    Some(matches)
}
