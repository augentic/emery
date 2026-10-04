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
        // `build` alone is `async func` in the WIT, so no `async:` list is needed
        generate_all,
        pub_export_macro: true,
        // the shared `types` are the source generation's, so one Rust type
        // stands for each record on both axes
        with: {
            "emery:adapter/types@0.1.0": crate::source::bindings::wit,
        },
    });
}

use crate::source::bindings::wit;
use crate::target::{Report, Slice, TargetMetadata};

impl From<TargetMetadata> for wit::TargetMetadata {
    fn from(metadata: TargetMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
        }
    }
}

impl From<wit::TargetMetadata> for TargetMetadata {
    fn from(metadata: wit::TargetMetadata) -> Self {
        Self {
            emery_version: metadata.emery_version,
        }
    }
}

impl From<Slice> for wit::Slice {
    fn from(slice: Slice) -> Self {
        Self {
            id: slice.id,
            name: slice.name,
            requirements: slice.requirements,
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
            requirements: slice.requirements,
            spec: slice.spec,
            design: slice.design,
            plan: slice.plan,
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

    use super::generated::emery::adapter::target as imported;
    use crate::source::bindings::wit;
    use crate::target::{Report, Slice, TargetMetadata};

    /// Returns the metadata the adapter registered as `id` declares.
    #[must_use]
    pub fn metadata(id: &str) -> TargetMetadata {
        imported::metadata(id).into()
    }

    /// Builds `slice` into the tree at `workspace` using the adapter registered as `id`.
    ///
    /// A failure names the adapter; the caller, which dispatches slice by
    /// slice, names the slice.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::BadRequest`] when the adapter rejects its input.
    /// - Returns [`Error::BadGateway`] when the adapter fails internally.
    pub async fn build(id: &str, slice: &Slice, workspace: &str) -> Result<Report, Error> {
        let report = imported::build(id.to_string(), slice.clone().into(), workspace.to_string())
            .await
            .map_err(|err| match err {
                wit::Error::InvalidRequest(detail) => bad_request!("adapter `{id}`: {detail}"),
                wit::Error::Internal(detail) => bad_gateway!("adapter `{id}`: {detail}"),
            })?;
        Ok(report.into())
    }
}
