//! The [`SourceAdapter`] trait and its provided extraction.
//!
//! An implementation states the kind of source it reads, the documents it
//! embeds, and how its input cuts into [materials](crate#vocabulary). The
//! trait provides the rest — the metadata, the prompt, the model call per
//! material, and [`SourceAdapter::extract`], which surveys the input, mines
//! every material, and joins the answers into one document.
//!
//! The trait is native. The wasm export is the `export` child, built for
//! `wasm32` alone, so an adapter is tested natively against a scripted model
//! and the component wiring is added only at the guest boundary.

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

/// What a source adapter implements.
///
/// An implementation states the kind of source it reads ([`Self::KIND`]), the
/// documents it embeds ([`Self::docs`]), and, for a source worth splitting,
/// how it cuts into [materials](crate#vocabulary) ([`Self::survey`]). The
/// provided methods do the rest: [`Self::extract`] surveys the input, asks the
/// model about each material, and joins the answers into one [`Evidence`]
/// document.
///
/// The trait is generic over [`Model`], so an adapter is tested natively
/// against a scripted model and runs in the guest against the host's. It is
/// not object-safe.
///
/// # Examples
///
/// See the [crate-level example](crate#examples) for a complete adapter.
pub trait SourceAdapter {
    /// The kind of source this adapter reads.
    ///
    /// Reported in [`Self::metadata`], so the engine ranks the adapter's
    /// evidence before any extract; the model never answers it.
    const KIND: SourceKind;

    /// Returns the adapter's embedded documents, including the extraction prompt.
    fn docs() -> &'static [Doc];

    /// Returns the materials to mine, one model call each.
    ///
    /// The default is the whole input as one [`Material::Bound`]. An adapter
    /// whose source is worth splitting overrides this to list or read its
    /// input and cut it:
    ///
    /// - mechanically, with [`survey::files`] and [`survey::by_directory`];
    /// - or by asking the model once, with [`survey::by_model`], how the files
    ///   group by what they serve.
    ///
    /// The survey only decides the cut; it never mines. Input the adapter
    /// cannot use is refused here, before any model call is spent. A survey
    /// that does not ask the model returns [`std::future::ready`] over its
    /// materials rather than being an `async fn`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] when the input cannot be mined or the
    /// model's grouping could not be brought within its rounds, and
    /// [`Error::ServerError`] for a survey prompt the build did not embed.
    fn survey<P: Model>(
        _model: &P, _ctx: &Context<'_>,
    ) -> impl Future<Output = Result<Vec<Material>, Error>> + Send {
        async { Ok(vec![Material::Bound]) }
    }

    /// Extracts the source's claims as one [`Evidence`] document.
    ///
    /// Provided; an adapter overrides [`Self::survey`] instead. The survey's
    /// materials are each put to the model by [`Self::evidence`], several at
    /// a time, and the answers are joined in material order. Each material's
    /// `path` anchors and path backings are re-rooted under the directory it
    /// was lent, so the document cites one path space however the source was
    /// cut.
    ///
    /// Every material is waited for. When more than one fails, the error names
    /// them all and takes the class of the first.
    ///
    /// # Errors
    ///
    /// - [`Error::BadRequest`] when the survey refuses the input or yields no
    ///   material, a [`Material::Within`] path escapes the root, or the
    ///   model's answer still fails the claim gate once its rounds are spent.
    /// - [`Error::ServerError`] for a [`Material::Within`] over an inline
    ///   value, or a prompt the build did not embed.
    /// - [`Error::BadGateway`] for a tool or transport failure.
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

    /// Returns the metadata the engine reads before any extract.
    ///
    /// The default requires the SDK's own version of Emery and reports
    /// [`Self::KIND`] as the kind of source.
    #[must_use]
    fn metadata() -> AdapterMetadata {
        AdapterMetadata {
            emery_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            kind: Self::KIND,
        }
    }

    /// Returns the extraction prompt: `prompts/extract.md` among [`Self::docs`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::ServerError`] when the build did not embed it.
    fn prompt() -> Result<&'static str, Error> {
        registry::body(Self::docs(), "prompts/extract.md")
            .ok_or_else(|| server_error!("`prompts/extract.md` is not embedded"))
    }

    /// Asks the model about one material and returns the accepted claims.
    ///
    /// The system prompt is [`Self::prompt`]. The turn names the adapter and
    /// the source key, describes `material`, and lends the model the directory
    /// the material may read; the `list_docs` and `read_doc` tools answer from
    /// [`Self::docs`]. The answer is checked against the claim gate
    /// ([`Evidence::findings`]), and findings go back to the model for another
    /// round until it answers clean or the host's rounds are spent. The engine
    /// runs the same gate again on receipt.
    ///
    /// # Errors
    ///
    /// - [`Error::BadRequest`] when the host refuses the request, the rounds
    ///   are spent with findings outstanding, or a [`Material::Within`] path
    ///   escapes the root.
    /// - [`Error::ServerError`] for a prompt the build did not embed, or a
    ///   [`Material::Within`] over an inline value.
    /// - [`Error::BadGateway`] for a tool or transport failure.
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

/// What one call knows: which adapter was addressed, and with what input.
#[derive(Debug)]
pub struct Context<'a> {
    /// The id the call addressed the adapter by.
    pub adapter_id: &'a str,
    /// The source key and the workspace or inline value to read.
    pub input: &'a SourceInput,
}

// A lone material's failure is the source's as it stands; several are reported
// together, under the first one's class.
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

// The first failed material decides the class; the report names them all.
fn reclass(class: &Error, description: &str) -> Error {
    match class {
        Error::BadRequest { .. } => bad_request!("{description}"),
        Error::NotFound { .. } => not_found!("{description}"),
        Error::ServerError { .. } => server_error!("{description}"),
        Error::BadGateway { .. } => bad_gateway!("{description}"),
    }
}

// Re-rooting each material's anchors under what it was lent gives the source
// one path space however it was cut.
fn join(lends: &[Lend], partials: Vec<Evidence>) -> Vec<Claim> {
    lends
        .iter()
        .zip(partials)
        .flat_map(|(lend, partial)| {
            partial.claims.into_iter().map(move |claim| reroot(&lend.within, claim))
        })
        .collect()
}

// A material lent the root itself has nothing to re-root. An anchor's `#L`
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
