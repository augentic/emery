//! Defines the build report and its validation rules.
//!
//! A [`Report`] names what a build covered and wrote. [`Report::findings`] is
//! the report gate: it holds `covered` to the slice's requirement ids and
//! every `written` path beneath the workspace root.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::beneath;
use crate::target::Slice;

/// What a build reports beyond the tree it wrote.
///
/// Unknown fields are rejected during deserialisation.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = "Emery build report")]
pub struct Report {
    /// The requirement ids the build implemented, each one of the slice's.
    pub covered: Vec<String>,
    /// The files written or changed, as `/`-separated paths relative to the
    /// workspace root.
    pub written: Vec<String>,
}

impl Report {
    /// Returns every rule the report breaks against `slice`, one line each.
    ///
    /// An empty result means the report passes the gate: every `covered` id
    /// is one of the slice's requirements, named once, and every `written`
    /// path is one [`beneath`] the workspace root, naming its file once.
    ///
    /// # Examples
    ///
    /// ```
    /// use emery_adapter::target::{Report, Slice};
    ///
    /// let slice = Slice {
    ///     id: "SLICE-001".into(),
    ///     name: "orders".into(),
    ///     base: "1a2b3c4d".into(),
    ///     requirements: vec!["REQ-001".into()],
    ///     spec: String::new(),
    ///     design: String::new(),
    ///     plan: String::new(),
    /// };
    /// let report: Report = serde_json::from_str(
    ///     r#"{ "covered": ["REQ-001", "REQ-002"], "written": ["src/orders.rs", "../x"] }"#,
    /// )?;
    ///
    /// // A covered id the slice does not hold, and a path above the root.
    /// assert_eq!(report.findings(&slice).len(), 2);
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    #[must_use]
    pub fn findings(&self, slice: &Slice) -> Vec<String> {
        let mut findings = Vec::new();

        // covered ids: the slice's, each once
        let mut seen = BTreeSet::new();
        for id in &self.covered {
            if !slice.requirements.contains(id) {
                findings.push(format!(
                    "- covered `{id}` is not a requirement of slice `{}`; its requirements are {}",
                    slice.id,
                    slice.requirements.join(", ")
                ));
            } else if !seen.insert(id) {
                findings.push(format!("- covered `{id}` is listed twice"));
            }
        }

        // written paths: beneath the root, each file once however spelled
        let mut seen = BTreeSet::new();
        for path in &self.written {
            match beneath(path) {
                Ok(file) => {
                    if !seen.insert(file) {
                        findings.push(format!("- written `{path}` is listed twice"));
                    }
                }
                Err(bad) => findings.push(format!("- written `{path}` {bad}")),
            }
        }

        findings
    }

    /// Returns whether the report covers the requirement `id`.
    #[must_use]
    pub fn covers(&self, id: &str) -> bool {
        self.covered.iter().any(|covered| covered == id)
    }

    /// Returns the requirements of `slice` the report does not cover, in the slice's order.
    #[must_use]
    pub fn uncovered<'a>(&self, slice: &'a Slice) -> Vec<&'a str> {
        slice.requirements.iter().filter(|id| !self.covers(id)).map(String::as_str).collect()
    }
}
