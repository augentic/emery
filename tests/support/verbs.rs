//! Live verbs
//!
//! Reads the verb names out of `emery --help`, so a suite that needs to know
//! which commands exist learns it from the shipped surface rather than from a
//! list that would have to be kept in step by hand.

use emery_source::is_kebab;

/// Extracts the sorted verb names from the `Commands:` section of
/// `emery --help`.
pub fn verbs(help: &str) -> Vec<&str> {
    let mut names: Vec<&str> = help
        .lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            is_kebab(name).then_some(name)
        })
        .collect();
    names.sort_unstable();
    names
}
