//! The source adapter role
//!
//! [`SourceAdapter`] is what an adapter implements: the kind of source it
//! reads, the reference documents it embeds, and the materials its input
//! splits into. The trait carries what every adapter
//! shares — the resolve-time metadata, the extraction prompt, the model call
//! per material, and the `extract` operation that surveys the input, mines
//! every material with at most four calls pending, and joins the
//! partials into one document — so an implementation states only what is
//! its own. The model answers claims alone; the kind of source rides the
//! adapter's metadata, read by the engine before any extract.
//!
//! The trait is native; the wasm export lives in the `export` child, built
//! for `wasm32` alone. Keeping them apart lets an adapter be exercised
//! natively against a scripted model, with the component wiring added only at
//! the guest boundary. The `brief` child is the role's prose: the one brief
//! an extraction puts to the model, and what each material is lent. The
//! `survey` child is what a tree adapter's survey is built from: the walk
//! that lists its files, the mechanical cut by directory, and the one model
//! call that cuts by what the files serve.

mod brief;
// The component export, re-exported at the crate root for the `source!`
// macro; no adapter names it.
#[cfg(target_arch = "wasm32")]
pub mod export;
pub mod survey;

use std::future::Future;

use emery_adapter::source::{AdapterMetadata, Backing, Claim, Evidence, SourceInput, SourceKind};
use emery_prose::registry::{self, Doc};
use futures::stream::{self, StreamExt as _};
use omnia_guest::model::Question;
use omnia_guest::{Error, Model, bad_gateway, bad_request, not_found, server_error};

pub use self::brief::Material;
use self::brief::{Brief, Lend};
use crate::references;

// Completions one adapter holds pending at once.
const CONCURRENT: usize = 4;

/// Contract implemented by source adapters.
///
/// Generic over [`Model`] for native test doubles and the wasm host model;
/// deliberately not object-safe.
pub trait SourceAdapter {
    /// The kind of source this adapter reads, reported through
    /// [`Self::metadata`] so the engine ranks its evidence before any
    /// extract.
    const KIND: SourceKind;

    /// Returns the adapter's embedded reference documents, including the
    /// extraction prompt.
    fn docs() -> &'static [Doc];

    /// The materials to mine, one model call each; by default the bound
    /// input, whole.
    ///
    /// An implementation lists or reads its input and refuses an unusable
    /// one with `BadRequest` here, before any model call is spent. It cuts
    /// its input mechanically, or asks the model once through
    /// [`survey::by_model`] — never more: the survey chooses how a source
    /// splits, it does not mine. A survey of one is a single
    /// [`Self::evidence`] call; a survey of several runs them together and
    /// joins the answers. A tree adapter lists its files with
    /// [`survey::files`], cuts them with [`survey::by_directory`] or
    /// [`survey::by_model`], then names each cut as the [`Material`] its
    /// source kind calls for.
    ///
    /// A survey that asks the model is an `async fn`; one that does not is
    /// ready at once, and says so by returning
    /// [`std::future::ready`] over the materials it computed.
    ///
    /// # Errors
    ///
    /// `BadRequest` when the input cannot be mined, or the model's survey
    /// could not be brought within its rounds; `ServerError` for a survey
    /// prompt the build did not embed.
    fn survey<P: Model>(
        _model: &P, _ctx: &Context<'_>,
    ) -> impl Future<Output = Result<Vec<Material>, Error>> + Send {
        async { Ok(vec![Material::Bound]) }
    }

    /// Extracts the source's claim set: the survey's materials, mined with
    /// at most four model calls pending and joined in material
    /// order into one document.
    ///
    /// Provided: an adapter states its materials through [`Self::survey`]
    /// and leaves the fan-out to the SDK. The survey runs first, alone, and
    /// no material is mined until it has returned. Each material's claims
    /// are appended in material order, its `path` anchors and path backings
    /// re-rooted under what it was lent, so a source cites one path space
    /// however it was split. Every material is waited for, then the failures
    /// are reported together, each with its material index, under the first
    /// one's class. A survey of one material is a single `evidence` call and
    /// its outcome, unchanged.
    ///
    /// # Errors
    ///
    /// `BadRequest` for a survey that refuses its input or yields nothing, a
    /// `Within` path that escapes the root, or a model call that ends on
    /// rejected findings; `ServerError` for a `Within` material over an
    /// inline value or a missing prompt; `BadGateway` for a tool or
    /// transport failure.
    fn extract<P: Model>(
        model: &P, ctx: &Context<'_>,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        async move {
            let key = &ctx.input.key;
            let materials = Self::survey(model, ctx).await?;
            if materials.is_empty() {
                return Err(bad_request!("`{key}`: the survey found nothing to mine"));
            }

            let lends = materials
                .iter()
                .map(|material| Lend::of(material, ctx))
                .collect::<Result<Vec<_>, _>>()?;
            let outcomes: Vec<_> = stream::iter(materials)
                .map(|material| Self::evidence(model, ctx, material))
                .buffered(CONCURRENT)
                .collect()
                .await;
            let partials = collect(key, outcomes)?;

            Ok(Evidence {
                claims: join(&lends, partials),
            })
        }
    }

    /// Reports resolve-time metadata: by default the SDK's own version as
    /// the exact `emery` pin, and [`Self::KIND`] as the kind of source.
    #[must_use]
    fn metadata() -> AdapterMetadata {
        AdapterMetadata {
            emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            kind: Self::KIND,
        }
    }

    /// The extraction prompt: the `prompts/extract.md` document among
    /// [`Self::docs`].
    ///
    /// # Errors
    ///
    /// `server_error` when the build did not embed it.
    fn prompt() -> Result<&'static str, Error> {
        registry::body(Self::docs(), "prompts/extract.md")
            .ok_or_else(|| server_error!("`prompts/extract.md` is not embedded"))
    }

    /// Asks the model for one material's claims and returns the accepted
    /// document: the one model call per material.
    ///
    /// The prompt is [`Self::prompt`]; the brief names the adapter and source
    /// key and carries `material`; the `list_docs` / `read_doc` tools answer from
    /// [`Self::docs`]; what the material is lent — a bound root, or a
    /// `Within` set's common ancestor — rides the workspace grant. The schema
    /// is the contract's [`Evidence`], which steers a claims-only answer: the
    /// kind of source is the adapter's metadata, never the model's to state.
    /// The schema cannot express every rule a claim must satisfy, so each
    /// candidate the backend proposes is run through the contract's claim
    /// gate before it is accepted; a miss goes back to the model as findings
    /// and the backend asks again. The engine re-runs the same gate on
    /// receipt, but an adapter that checks in place rarely hands it evidence
    /// to reject.
    ///
    /// # Errors
    ///
    /// A request the host refuses, the last gate findings once the backend's
    /// rounds are spent, or a `Within` path that escapes the root is
    /// `BadRequest`; a tool or transport failure is `BadGateway`; a missing
    /// prompt, or `Within` over an inline value, is `ServerError`.
    fn evidence<P: Model>(
        model: &P, ctx: &Context<'_>, material: Material,
    ) -> impl Future<Output = Result<Evidence, Error>> + Send {
        async move {
            let system = Self::prompt()?;
            let lend = Lend::of(&material, ctx)?;
            let brief = Brief {
                ctx,
                material: &material,
                lend: &lend,
            };

            let mut question =
                Question::<Evidence>::new("evidence").system(system).tools(references::tools());
            if let Some(workspace) = &lend.workspace {
                question = question.workspace(workspace);
            }

            question
                .ask(
                    model,
                    brief.to_string(),
                    Some(references::answering(Self::docs())),
                    |answer| {
                        let findings = answer.findings();
                        if findings.is_empty() { Ok(()) } else { Err(findings) }
                    },
                )
                .await
                .map_err(Error::from)
        }
    }
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

// Every material's evidence in material order, or one error naming each
// failed material under the first failure's class. A lone material's
// failure is the source's as it stands: a survey of one is a single
// evidence call.
fn collect(key: &str, outcomes: Vec<Result<Evidence, Error>>) -> Result<Vec<Evidence>, Error> {
    let count = outcomes.len();
    let mut partials = Vec::with_capacity(count);
    let mut failures = Vec::new();
    for (index, outcome) in outcomes.into_iter().enumerate() {
        match outcome {
            Ok(evidence) => partials.push(evidence),
            Err(error) if count == 1 => return Err(error),
            Err(error) => failures.push((index, error)),
        }
    }

    let Some((_, first)) = failures.first() else {
        return Ok(partials);
    };
    let report: Vec<String> = failures
        .iter()
        .map(|(index, error)| format!("- material {index}: {}", error.description()))
        .collect();

    Err(reclass(
        first,
        &format!("`{key}`: {} of {count} materials failed:\n{}", failures.len(), report.join("\n")),
    ))
}

// `description` under `class`'s variant: the first failed material decides
// how the source's failure is classified; the report names them all.
fn reclass(class: &Error, description: &str) -> Error {
    match class {
        Error::BadRequest { .. } => bad_request!("{description}"),
        Error::NotFound { .. } => not_found!("{description}"),
        Error::ServerError { .. } => server_error!("{description}"),
        Error::BadGateway { .. } => bad_gateway!("{description}"),
    }
}

// The partials' claims in material order, each material's anchors re-rooted
// under what it was lent, so a source cites one path space however it was
// split.
fn join(lends: &[Lend], partials: Vec<Evidence>) -> Vec<Claim> {
    lends
        .iter()
        .zip(partials)
        .flat_map(|(lend, partial)| {
            partial.claims.into_iter().map(move |claim| reroot(&lend.within, claim))
        })
        .collect()
}

// `claim` with its `path` anchor and path backing beneath `within`; a
// material lent the root itself has nothing to re-root. An anchor's `#L`
// suffix follows the path, so a prefix leaves it intact.
fn reroot(within: &str, mut claim: Claim) -> Claim {
    if within.is_empty() {
        return claim;
    }

    claim.path = claim.path.map(|path| format!("{within}/{path}"));
    claim.backing = claim.backing.map(|backing| match backing {
        Backing::Path(path) => Backing::Path(format!("{within}/{path}")),
        payload @ Backing::Payload(_) => payload,
    });

    claim
}
