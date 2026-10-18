//! Extracts subcommand names from `emery --help` output.

use emery_adapter::is_kebab;

/// Returns the sorted verb names from the `Commands:` section of `help`.
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
