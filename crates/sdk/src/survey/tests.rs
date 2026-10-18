//! Extracts stated behaviours from workspace test files and Gherkin features.
//!
//! [`Test`] and [`Statement`] record test titles alongside the modules a test
//! imports so [`stated`] can include them in seam briefs.

/// One test file of the tree: what it imports, and what it states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Test {
    /// The test's root-relative path.
    pub path: String,
    /// The modules of the tree it imports; for a feature file, the ones the
    /// step modules beside it import.
    pub imports: Vec<String>,
    /// The behaviours it states, in file order.
    pub statements: Vec<Statement>,
}

/// One behaviour a test states, at the line that states it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Statement {
    /// A case's title under its suites' titles, or a scenario's under its
    /// feature's, outermost first.
    pub text: String,
    /// The 1-based line that states it.
    pub line: u32,
}

// The Gherkin keywords that open a feature and a scenario.
const FEATURE: &str = "Feature:";
const SCENARIOS: &[&str] = &["Scenario Outline:", "Scenario Template:", "Scenario:", "Example:"];

/// Returns each scenario a Gherkin feature file states, under its feature's title.
///
/// `Scenario:`, `Scenario Outline:`, `Scenario Template:`, and `Example:`
/// each open one. Its text is `Feature › Scenario`, or the scenario's title
/// alone before any `Feature:` line.
///
/// # Examples
///
/// ```
/// use emery_sdk::survey::tests::scenarios;
///
/// let stated = scenarios("Feature: Orders\n  Scenario: An order is created\n    Given a body\n");
/// assert_eq!(stated.len(), 1);
/// assert_eq!(stated[0].text, "Orders › An order is created");
/// assert_eq!(stated[0].line, 2);
/// ```
#[must_use]
pub fn scenarios(text: &str) -> Vec<Statement> {
    let mut feature: Option<&str> = None;
    let mut statements = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if let Some(title) = line.strip_prefix(FEATURE) {
            feature = Some(title.trim());
            continue;
        }
        let Some(title) = SCENARIOS.iter().find_map(|keyword| line.strip_prefix(keyword)) else {
            continue;
        };
        let title = title.trim();
        statements.push(Statement {
            text: feature
                .map_or_else(|| title.to_owned(), |feature| format!("{feature} › {title}")),
            line: u32::try_from(index + 1).unwrap_or(u32::MAX),
        });
    }
    statements
}

/// Returns the brief section listing the behaviours `tests` state, or `None` for no statement.
///
/// Each statement is listed at its line, as `` - `path#L<n>` — text ``.
/// `described` names what a listed line is in the adapter's language — `a
/// case's title under its suites'`, `a test's docstring or name under its
/// class's` — in the section's opening sentence.
#[must_use]
pub fn stated<'t>(tests: impl IntoIterator<Item = &'t Test>, described: &str) -> Option<String> {
    let lines: Vec<String> = tests
        .into_iter()
        .flat_map(|test| {
            test.statements.iter().map(move |statement| {
                format!("- `{}#L{}` — {}", test.path, statement.line, statement.text)
            })
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    Some(format!(
        "Behaviours the tree's own tests state, each at its line — {described}, a scenario's \
         under its feature's. One the laid code confirms is a `requirement` anchored at the code \
         that exhibits it, named for that code, the test's sentence leading its statement with \
         the code's identifiers standing nowhere in it; one the code does not hold is not \
         invented. A test is read for what it states and the values it asserts, and is no \
         `requirement`'s or `criterion`'s anchor:\n\n{}",
        lines.join("\n")
    ))
}
