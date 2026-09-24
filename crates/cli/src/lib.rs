//! Implements Emery's command-line interface.
//!
//! The interface provides the `specify`, `show`, and `completions` commands.
//! It translates command-line sources into engine inputs, renders text output,
//! and supplies recovery hints for known failures.
//!
//! [`run`] returns a buffered response, leaving process I/O and exit handling
//! to the caller. [`plan`] reads the same invocation ahead of the run for the
//! runtime that hosts it: the adapters to declare and the verbosity flags to
//! act on. The crate installs no subscriber of its own; tracing follows the
//! `RUST_LOG` the runtime sets from those flags, which the grammar declares
//! (`-v`, `-q`) and the run never reads.

mod sources;
mod text;

use std::borrow::Cow;
use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};
use clap::{Parser, Subcommand};
use emery_engine::show::{Artifact, ShowInput, show};
use emery_engine::specify::{SpecifyInput, specify};
use emery_engine::{AdapterRef, Provider, Registries};
use omnia_sdk::Error;
use omnia_sdk::api::command::{Command, Parsed, Response, Shell, Verbosity, completions, parse};
use omnia_sdk::api::{Client, Format, Metadata};
use omnia_sdk::plugins::Digest;
use strum::VariantArray as _;

const ABOUT: &str = "Deterministic primitives for spec-driven development";
const SPECIFY_DESC: &str = "Generate spec.md and design.md from source adapters.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, \
    or use `--config [<path>]` (default: `emery.toml`). With no sources, Emery looks \
    for `emery.toml` in the project root. Config and command-line sources cannot be \
    combined; the project-root file's `[registries]` table routes a package adapter \
    named on the command line all the same.\n\n\
    Adapter paths are project-relative. A bare adapter name is a guest the deployment \
    declares; the shipped binary declares none. Each run reloads adapters, reconciles \
    their claims, and atomically commits a new revision.";
const SHOW_DESC: &str = "Print an artifact from the current revision.\n\n\
    Text output contains only the artifact body. `--format json` also includes the \
    revision id and the typed document.";
const COMPLETIONS_DESC: &str = "Generate shell completions.\n\n\
    Pipe into your shell's completion directory. Example: \
    `emery completions zsh > ~/.zsh/_emery`";
const NAME: &str = "emery";

/// Executes the command described by `argv` using a [`Provider`].
///
/// The returned [`Response`] contains the exit status and buffered standard
/// output and error. Help, version, and usage responses are produced without
/// invoking an engine operation.
pub async fn run<P, I, T>(provider: P, argv: I) -> Response
where
    P: Provider,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let app = match parse::<App>(argv) {
        Parsed::App(app) => app,
        Parsed::Display(text) => return Response::success(text),
        Parsed::Usage(error) => return Response::usage(&error),
    };

    let client = Client::new(NAME, provider);
    let metadata = Metadata::from_env("EMERY");
    let command = Command::new(&client, &metadata, app.format).hints(|error| hint(&error.code()));

    match app.verb {
        Verb::Completions { shell } => completions::<App>(shell, NAME),
        Verb::Specify(arguments) => {
            command.call(specify, || arguments.decode(), text::specify).await
        }
        Verb::Show(ShowArgs { artifact }) => {
            command.call(show, || Ok(ShowInput { artifact }), text::show).await
        }
    }
}

/// What a runtime reads from an invocation before the run it hosts.
///
/// The deployment's guest list is the loader's allow-list, so a runtime
/// declares the adapters a `specify` invocation names before the engine asks
/// for them, and it sets the run's tracing level from the verbosity flags the
/// grammar declares.
#[derive(Debug, Default)]
pub struct Plan {
    /// Every adapter the run names, once each by the guest it loads as
    /// ([`AdapterRef::guest`]), with the `[[source]] digest` pinning it, in
    /// declaration order.
    pub adapters: Vec<(AdapterRef, Option<Digest>)>,
    /// The registry serving each package namespace the run may fetch from.
    pub registries: Registries,
    /// How many times `-v` / `--verbose` is given.
    pub verbose: u8,
    /// How many times `-q` / `--quiet` is given.
    pub quiet: u8,
}

/// Reads what the runtime hosting `argv` declares and selects before the run.
///
/// The adapters come from the carriers the run itself decodes — positional
/// adapters, `--description`, `--config`, or the project-root `emery.toml` —
/// read the same way, from the same working directory. An invocation that is
/// not a `specify`, or whose sources do not decode, names no adapter: the run
/// reports the usage error or refusal, and the runtime pre-empts none of it.
/// Two references naming one guest are one entry, pinned by the first digest
/// among them; the run refuses the pair itself.
#[must_use]
pub fn plan<I, T>(argv: I) -> Plan
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let Parsed::App(app) = parse::<App>(argv) else {
        return Plan::default();
    };
    let mut plan = Plan {
        verbose: app.verbosity.verbose,
        quiet: app.verbosity.quiet,
        ..Plan::default()
    };
    let Verb::Specify(arguments) = app.verb else {
        return plan;
    };
    let Ok(SpecifyInput { sources, registries }) = arguments.decode() else {
        return plan;
    };

    for source in sources {
        let guest = source.adapter.guest();
        match plan.adapters.iter_mut().find(|(adapter, _)| adapter.guest() == guest) {
            Some((_, pin)) => {
                if pin.is_none() {
                    *pin = source.digest;
                }
            }
            None => plan.adapters.push((source.adapter, source.digest)),
        }
    }
    plan.registries = registries;

    plan
}

// `bin_name` pins usage text to `emery`: Omnia forwards the engine guest's
// own id as argv[0], and clap only reads argv[0] when `bin_name` is unset.
#[derive(Debug, Parser)]
#[command(
    name = NAME,
    bin_name = NAME,
    version = env!("CARGO_PKG_VERSION"),
    about = ABOUT,
    disable_help_subcommand = true,
    subcommand_required = true,
    arg_required_else_help = true
)]
struct App {
    #[command(subcommand)]
    verb: Verb,
    /// Select the output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text", global = true)]
    format: Format,
    // The runtime acts on these, through `plan`, before the guest runs;
    // declaring them lists them in help and completions and refuses `-v`
    // beside `-q`.
    #[command(flatten)]
    verbosity: Verbosity,
}

#[derive(Debug, Subcommand)]
enum Verb {
    /// Generate spec.md and design.md from the named sources
    #[command(long_about = SPECIFY_DESC)]
    Specify(SpecifyArgs),
    /// Print a reviewable artifact of the current revision to stdout
    #[command(long_about = SHOW_DESC)]
    Show(ShowArgs),
    /// Print a shell-completion script for `<shell>` to stdout
    #[command(long_about = COMPLETIONS_DESC)]
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Debug, clap::Args)]
struct SpecifyArgs {
    /// Workspace-backed source adapters: a project-relative `.wasm` path, a
    /// package reference, or a bare name the deployment declares.
    adapters: Vec<String>,
    /// Bind an inline source as `<adapter>=<text>`; repeatable.
    #[arg(long = "description", short = 'd')]
    descriptions: Vec<String>,
    /// Operator-owned config; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = sources::CONFIG_FILE)]
    config: Option<PathBuf>,
}

impl SpecifyArgs {
    fn decode(self) -> Result<SpecifyInput, Error> {
        let Self {
            adapters,
            descriptions,
            config,
        } = self;
        let sources::Decoded { sources, registries } =
            sources::decode(&adapters, &descriptions, config.as_deref())?;
        Ok(SpecifyInput { sources, registries })
    }
}

#[derive(Debug, clap::Args)]
struct ShowArgs {
    /// Reviewable artifact of the current revision.
    #[arg(value_parser = artifacts())]
    artifact: Artifact,
}

fn artifacts() -> impl TypedValueParser<Value = Artifact> {
    PossibleValuesParser::new(Artifact::VARIANTS.iter().map(|artifact| {
        let help = match artifact {
            Artifact::Spec => "The behavioural specification artifact.",
            Artifact::Design => "The rebuild design artifact.",
        };
        PossibleValue::new(artifact.as_ref()).help(help)
    }))
    .try_map(|value: String| value.parse::<Artifact>())
}

// Flag and verb vocabulary stays at the CLI boundary.
fn hint(code: &str) -> Option<Cow<'static, str>> {
    let hint = match code {
        "unsupported-version" => {
            "update emery: `brew upgrade emery`, or `cargo install --git https://github.com/augentic/emery --locked`"
        }
        "specify-source-required" => {
            "pass one or more adapters to `emery specify`, or add an `emery.toml` at the project root"
        }
        "spec-not-generated" => {
            "run `emery specify <adapter>...` to commit a revision, then re-run show"
        }
        "spec-outdated" => {
            "the revision predates this emery's grammar: re-run `emery specify <adapter>...` to regenerate it"
        }
        "refused" => {
            "the loader refused the component; the message above names why: a missing export, an invalid artifact, a pre-compiled artifact where raw wasm is required, a mismatched digest, or a bare name this deployment does not declare"
        }
        "unavailable" => {
            "the registry could not supply the package: check the network and that the exact version is published under its namespace"
        }
        _ => return None,
    };
    Some(Cow::Borrowed(hint))
}
