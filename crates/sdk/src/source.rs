//! The source adapter role
//!
//! [`SourceAdapter`] is what an adapter implements: the noun its source goes
//! by, the kind of source it reads, the reference documents it embeds, and
//! the `extract` operation that reads a source and returns evidence. The
//! trait carries what every adapter shares — the resolve-time metadata, the
//! extraction prompt, and the one model call — so an implementation states
//! only what is its own. The model answers claims; the SDK stamps the
//! adapter's source kind.
//!
//! The trait is native; the wasm export lives in the `export` child, built
//! for `wasm32` alone. Keeping them apart lets an adapter be exercised
//! natively against a scripted model, with the component wiring added only at
//! the guest boundary. The `brief` child is the role's prose: the one brief
//! an extraction puts to the model.

mod brief;
// The component export, re-exported at the crate root for the `source!`
// macro; no adapter names it.
#[cfg(target_arch = "wasm32")]
pub mod export;

use std::future::Future;

use emery_adapter::source::{
    AdapterMetadata, Claim, Evidence, SourceContent, SourceInput, SourceKind,
};
use emery_prose::registry::{self, Doc};
use omnia_guest::model::Question;
use omnia_guest::{Error, Model, server_error};
use schemars::JsonSchema;
use serde::Deserialize;

use self::brief::Brief;
pub use self::brief::Material;
use crate::references;

// The one extraction prompt every adapter embeds.
const PROMPT: &str = "prompts/extract.md";

/// Contract implemented by source adapters.
///
/// Generic over [`Model`] for native test doubles and the wasm host model;
/// deliberately not object-safe.
pub trait SourceAdapter {
    /// The noun the prompt calls this adapter's source (`documentation`,
    /// `TypeScript / JavaScript`).
    const SOURCE: &'static str;

    /// The kind of source this adapter reads: a fact about its input, never
    /// answered by the model.
    const KIND: SourceKind;

    /// Returns the adapter's embedded reference documents, the extraction
    /// prompt among them.
    fn docs() -> &'static [Doc];

    /// The document this adapter returns for `claims`.
    #[must_use]
    fn stamp(claims: Vec<Claim>) -> Evidence {
        Evidence {
            kind: Self::KIND,
            claims,
        }
    }

    /// Extracts the source's claim set.
    ///
    /// An implementation refuses unusable input with `BadRequest`; the engine
    /// reports any other class as an adapter failure.
    fn extract<P: Model>(
        model: &P, ctx: &Context<'_>,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send;

    /// Reports resolve-time metadata; by default the SDK's own version is the
    /// exact `emery` pin.
    #[must_use]
    fn metadata() -> AdapterMetadata {
        AdapterMetadata {
            emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }
    }

    /// The extraction prompt: the `prompts/extract.md` document among
    /// [`Self::docs`].
    ///
    /// # Errors
    ///
    /// `server_error` when the build did not embed it.
    fn prompt() -> Result<&'static str, Error> {
        registry::body(Self::docs(), PROMPT)
            .ok_or_else(|| server_error!("`{PROMPT}` is not embedded"))
    }

    /// Asks the model for the source's claims and returns the accepted
    /// document: the one model call an adapter makes.
    ///
    /// The prompt is [`Self::prompt`]; the brief names the source and carries
    /// `material`; the `list_docs` / `read_doc` tools answer from
    /// [`Self::docs`]; a bound workspace is lent. The schema steers a
    /// claims-only answer; the SDK stamps [`Self::KIND`]. The schema
    /// cannot express every rule a claim must satisfy, so each candidate the
    /// backend proposes is run through the contract's claim gate before it is
    /// accepted; a miss goes back to the model as findings and the backend
    /// asks again. The engine re-runs the same gate on receipt, but an
    /// adapter that checks in place rarely hands it evidence to reject.
    ///
    /// # Errors
    ///
    /// A request the host refuses, or the last gate findings once the
    /// backend's rounds are spent, is `BadRequest`; a tool or transport
    /// failure is `BadGateway`; a missing prompt is `ServerError`.
    fn evidence<P: Model>(
        model: &P, ctx: &Context<'_>, material: Material,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        async move {
            let system = Self::prompt()?;
            let brief = Brief {
                source: Self::SOURCE,
                ctx,
                material: &material,
            };

            let mut question =
                Question::<Answer>::new("evidence").system(system).tools(references::tools());
            if let Some(lend) = ctx.lend() {
                question = question.workspace(lend);
            }

            question
                .ask(
                    model,
                    brief.to_string(),
                    Some(references::answering(Self::docs())),
                    |answer| {
                        let findings = Self::stamp(answer.claims.clone()).findings();
                        if findings.is_empty() { Ok(()) } else { Err(findings) }
                    },
                )
                .await
                .map(|answer| Self::stamp(answer.claims))
                .map_err(Error::from)
        }
    }
}

/// The model's extraction answer: claims alone.
///
/// The SDK stamps [`SourceAdapter::KIND`]. A stray `kind` key is a schema
/// miss.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "Emery evidence answer")]
pub struct Answer {
    /// Extracted claims.
    pub claims: Vec<Claim>,
}

/// Call-scoped adapter environment: which adapter was addressed, and with
/// what input.
#[derive(Debug)]
pub struct Context<'a> {
    /// The adapter id the call was addressed to.
    pub adapter_id: &'a str,
    /// The source key and the workspace or inline value to extract from.
    pub input: &'a SourceInput,
}

impl Context<'_> {
    // The workspace lent to the model: a bound tree's root; nothing for an
    // inline value, which rides the brief instead.
    fn lend(&self) -> Option<&str> {
        match &self.input.content {
            SourceContent::Workspace(root) => Some(root),
            SourceContent::Value(_) => None,
        }
    }
}
