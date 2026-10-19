//! Implements the WebAssembly interface shared by target adapters and the engine.
//!
//! Adapters implement [`export::Guest`], while the engine invokes them
//! through [`import`]. Contract records and errors are converted at this
//! boundary.

mod generated {
    #![allow(
        missing_docs,
        unsafe_code,
        clippy::pedantic,
        clippy::nursery,
        reason = "wit-bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    wit_bindgen::generate!({
        world: "target-adapter",
        path: "../../wit",
        // `run` alone is `async func` in the WIT, so no `async:` list is needed
        generate_all,
        pub_export_macro: true,
        // the shared `types` are the source generation's, so one Rust type
        // stands for each of them on both axes
        with: {
            "emery:adapter/types@0.1.0": crate::source::bindings::types,
        },
        // the world imports and exports one interface, so the export's
        // records alias the import's and one Rust type stands for each
        merge_structurally_equal_types: true,
    });
}

use self::generated::emery::adapter::target as wit;
use crate::{Route, Trigger};
use crate::target::{
    DesignType, MergeRule, MergeStrategy, Outcome, Report, Requirement, Scenario, Slice,
    TargetMetadata, Task, Verdict,
};

impl From<TargetMetadata> for wit::TargetMetadata {
    fn from(metadata: TargetMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
            merge_rules: metadata.merge_rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<wit::TargetMetadata> for TargetMetadata {
    fn from(metadata: wit::TargetMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
            merge_rules: metadata.merge_rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<MergeRule> for wit::MergeRule {
    fn from(rule: MergeRule) -> Self {
        Self {
            paths: rule.paths.into_owned(),
            strategy: rule.strategy.into(),
        }
    }
}

impl From<wit::MergeRule> for MergeRule {
    fn from(rule: wit::MergeRule) -> Self {
        Self {
            paths: rule.paths.into(),
            strategy: rule.strategy.into(),
        }
    }
}

impl From<MergeStrategy> for wit::MergeStrategy {
    fn from(strategy: MergeStrategy) -> Self {
        match strategy {
            MergeStrategy::Union => Self::Union,
            MergeStrategy::Ours => Self::Ours,
            MergeStrategy::Theirs => Self::Theirs,
        }
    }
}

impl From<wit::MergeStrategy> for MergeStrategy {
    fn from(strategy: wit::MergeStrategy) -> Self {
        match strategy {
            wit::MergeStrategy::Union => Self::Union,
            wit::MergeStrategy::Ours => Self::Ours,
            wit::MergeStrategy::Theirs => Self::Theirs,
        }
    }
}

impl From<Slice> for wit::Slice {
    fn from(slice: Slice) -> Self {
        Self {
            id: slice.id,
            name: slice.name,
            base: slice.base,
            requirements: slice.requirements.into_iter().map(Into::into).collect(),
            types: slice.types.into_iter().map(Into::into).collect(),
            brief: slice.brief,
            spec: slice.spec,
            design: slice.design,
            plan: slice.plan,
        }
    }
}

impl From<wit::Slice> for Slice {
    fn from(slice: wit::Slice) -> Self {
        Self {
            id: slice.id,
            name: slice.name,
            base: slice.base,
            requirements: slice.requirements.into_iter().map(Into::into).collect(),
            types: slice.types.into_iter().map(Into::into).collect(),
            brief: slice.brief,
            spec: slice.spec,
            design: slice.design,
            plan: slice.plan,
        }
    }
}

impl From<Requirement> for wit::Requirement {
    fn from(requirement: Requirement) -> Self {
        Self {
            id: requirement.id,
            subject: requirement.subject,
            statement: requirement.statement,
            scenarios: requirement.scenarios.into_iter().map(Into::into).collect(),
            trigger: requirement.trigger.map(Into::into),
        }
    }
}

impl From<wit::Requirement> for Requirement {
    fn from(requirement: wit::Requirement) -> Self {
        Self {
            id: requirement.id,
            subject: requirement.subject,
            statement: requirement.statement,
            scenarios: requirement.scenarios.into_iter().map(Into::into).collect(),
            trigger: requirement.trigger.map(Into::into),
        }
    }
}

impl From<Trigger> for wit::Trigger {
    fn from(trigger: Trigger) -> Self {
        match trigger {
            Trigger::Http(route) => Self::Http(route.into()),
            Trigger::Message(name) => Self::Message(name),
        }
    }
}

impl From<wit::Trigger> for Trigger {
    fn from(trigger: wit::Trigger) -> Self {
        match trigger {
            wit::Trigger::Http(route) => Self::Http(route.into()),
            wit::Trigger::Message(name) => Self::Message(name),
        }
    }
}

impl From<Route> for wit::Route {
    fn from(route: Route) -> Self {
        Self {
            method: route.method,
            path: route.path,
        }
    }
}

impl From<wit::Route> for Route {
    fn from(route: wit::Route) -> Self {
        Self {
            method: route.method,
            path: route.path,
        }
    }
}

impl From<Scenario> for wit::Scenario {
    fn from(scenario: Scenario) -> Self {
        Self {
            name: scenario.name,
            given: scenario.given,
            when: scenario.when,
            then: scenario.then,
            and: scenario.and,
        }
    }
}

impl From<wit::Scenario> for Scenario {
    fn from(scenario: wit::Scenario) -> Self {
        Self {
            name: scenario.name,
            given: scenario.given,
            when: scenario.when,
            then: scenario.then,
            and: scenario.and,
        }
    }
}

impl From<DesignType> for wit::DesignType {
    fn from(design_type: DesignType) -> Self {
        Self {
            key: design_type.key,
            signature: design_type.signature,
        }
    }
}

impl From<wit::DesignType> for DesignType {
    fn from(design_type: wit::DesignType) -> Self {
        Self {
            key: design_type.key,
            signature: design_type.signature,
        }
    }
}

impl From<Verdict> for wit::Verdict {
    fn from(verdict: Verdict) -> Self {
        Self {
            passed: verdict.passed,
            failures: verdict.failures,
        }
    }
}

impl From<wit::Verdict> for Verdict {
    fn from(verdict: wit::Verdict) -> Self {
        Self {
            passed: verdict.passed,
            failures: verdict.failures,
        }
    }
}

impl From<Report> for wit::Report {
    fn from(report: Report) -> Self {
        Self {
            covered: report.covered,
            written: report.written,
        }
    }
}

impl From<wit::Report> for Report {
    fn from(report: wit::Report) -> Self {
        Self {
            covered: report.covered,
            written: report.written,
        }
    }
}

impl From<Task> for wit::Task {
    fn from(task: Task) -> Self {
        match task {
            Task::Init => Self::Init,
            Task::Build(slice) => Self::Build(slice.into()),
            Task::Verify => Self::Verify,
        }
    }
}

impl From<wit::Task> for Task {
    fn from(task: wit::Task) -> Self {
        match task {
            wit::Task::Init => Self::Init,
            wit::Task::Build(slice) => Self::Build(slice.into()),
            wit::Task::Verify => Self::Verify,
        }
    }
}

impl From<Outcome> for wit::Outcome {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Initialised => Self::Initialised,
            Outcome::Built(report) => Self::Built(report.into()),
            Outcome::Verified(verdict) => Self::Verified(verdict.into()),
        }
    }
}

impl From<wit::Outcome> for Outcome {
    fn from(outcome: wit::Outcome) -> Self {
        match outcome {
            wit::Outcome::Initialised => Self::Initialised,
            wit::Outcome::Built(report) => Self::Built(report.into()),
            wit::Outcome::Verified(verdict) => Self::Verified(verdict.into()),
        }
    }
}

/// The WebAssembly guest interface implemented by a target adapter.
///
/// This module exposes the generated `Guest` trait, its records, and the
/// `export!` macro.
pub mod export {
    // The root glob carries the bindgen support items the `export!` macro
    // expands against; the second names the world's records and `Guest`.
    pub use super::generated::exports::emery::adapter::target::*;
    pub use super::generated::*;
}

/// The WebAssembly client used to invoke a loaded target adapter.
pub mod import {
    use omnia_sdk::{Error, bad_gateway, bad_request};

    use super::wit as imported;
    use crate::source::bindings::types;
    use crate::target::{Outcome, TargetMetadata, Task};

    /// Returns the metadata the adapter registered as `id` declares.
    #[must_use]
    pub fn metadata(id: &str) -> TargetMetadata {
        imported::metadata(id).into()
    }

    /// Runs `task` in the tree at `workspace` using the adapter registered as `id`.
    ///
    /// A failure names the adapter; the caller, which dispatches task by
    /// task, names the slice or the wave.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] when the adapter fails internally.
    pub async fn run(id: &str, workspace: &str, task: &Task) -> Result<Outcome, Error> {
        let outcome = imported::run(id.to_string(), workspace.to_string(), task.clone().into())
            .await
            .map_err(|err| match err {
                types::Error::InvalidRequest(detail) => bad_request!("adapter `{id}`: {detail}"),
                types::Error::Internal(detail) => bad_gateway!("adapter `{id}`: {detail}"),
            })?;
        Ok(outcome.into())
    }
}
