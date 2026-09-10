//! Adapter types
//!
//! The data types an adapter works with: the contract DTOs re-exported from
//! `emery-source`, and the [`Context`] describing the environment one
//! `extract` call runs in — which adapter was addressed, which reference
//! documents are available, and whether the model may read the workspace.
//!
//! Gathering these in one module gives adapter code a single import path
//! regardless of which crate defines each type.

use emery_prose::registry::Doc;
pub use emery_source::types::{
    AdapterMetadata, Authority, Backing, Claim, ClaimKind, Evidence, SourceContent, SourceInput,
};

/// Call-scoped adapter environment.
#[derive(Clone, Debug)]
pub struct Context<'a> {
    /// The adapter id the call was addressed to.
    pub adapter_id: &'a str,
    /// Embedded reference documents served by the judgment's tool closure.
    pub docs: &'static [Doc],
    /// Workspace lend, absent for inline values.
    pub lend: Option<String>,
}
