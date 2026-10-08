//! Defines the verification verdict and its validation rule.
//!
//! A [`Verdict`] says whether the integrated tree passed the adapter's
//! checks and names each that failed. [`Verdict::findings`] is the verdict
//! gate: `passed` must agree with `failures`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What a verification of the integrated tree found.
///
/// Unknown fields are rejected during deserialisation.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = "Emery verification verdict")]
pub struct Verdict {
    /// Whether every check passed.
    pub passed: bool,
    /// Each check that failed, named with the tail of its output.
    pub failures: Vec<String>,
}

impl Verdict {
    /// Returns every rule the verdict breaks, one line each.
    ///
    /// An empty result means the verdict passes the gate: `passed` is
    /// `true` with no failure listed, or `false` with at least one.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_adapter::target::Verdict;
    ///
    /// let verdict: Verdict =
    ///     serde_json::from_str(r#"{ "passed": true, "failures": ["cargo test: 1 failed"] }"#)?;
    ///
    /// // Passed, yet a failure is listed.
    /// assert_eq!(verdict.findings().len(), 1);
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    #[must_use]
    pub fn findings(&self) -> Vec<String> {
        match (self.passed, self.failures.is_empty()) {
            (true, false) => {
                let listed = match self.failures.len() {
                    1 => "1 failure is".to_owned(),
                    count => format!("{count} failures are"),
                };
                vec![format!(
                    "- `passed` is true, yet {listed} listed; a verdict with failures is not passed"
                )]
            }
            (false, true) => {
                vec![
                    "- `passed` is false, yet no failure is listed; name each check that failed"
                        .to_owned(),
                ]
            }
            _ => Vec::new(),
        }
    }
}
