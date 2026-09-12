//! The `emery` command line
//!
//! The operator-facing surface of Emery: the `specify`, `show`, and
//! `completions` verbs, their help text, the rules that turn a parsed
//! command into an engine operation, and the text shape of each result.
//!
//! The engine knows nothing about arguments, text, or exit codes. Keeping
//! that vocabulary here means the same operations can be driven by another
//! transport, and the command grammar can change without touching the
//! engine. The projection itself — decode → `Client::call` → encode, the
//! failure envelope, and the exit map — is omnia's command façade
//! (`omnia_guest::api::command`), so this crate owns only what is Emery's.

mod sources;
mod text;

use std::borrow::Cow;
use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};
use clap::{Parser, Subcommand};
use emery_engine::Provider;
use emery_engine::show::{Artifact, ShowInput, show};
use emery_engine::specify::{SpecifyInput, specify};
use omnia_guest::Error;
use omnia_guest::api::command::{Command, Parsed, Response, Shell, completions, parse};
use omnia_guest::api::{Client, Format, Metadata};
use strum::VariantArray as _;

const ABOUT: &str = "Deterministic primitives for spec-driven development";
const SPECIFY_DESC: &str = "Generate spec.md and design.md from source adapters.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, \
    or use `--config [<path>]` (default: `emery.toml`). With no sources, Emery looks \
    for `emery.toml` in the project root. Config and command-line sources cannot be \
    combined.\n\n\
    Adapter paths are project-relative. Each run reloads adapters, verifies optional \
    digest pins, reconciles their claims, and atomically commits a new revision.";
const SHOW_DESC: &str = "Print an artifact from the current revision.\n\n\
    Text output contains only the artifact body. `--format json` also includes the \
    revision id and the typed document.";
const COMPLETIONS_DESC: &str = "Generate shell completions.\n\n\
    Pipe into your shell's completion directory. Example: \
    `emery completions zsh > ~/.zsh/_emery`";

// The program name: the clap surface, the completions target, and the
// `Client` owner are one spelling.
const NAME: &str = "emery";

// The environment prefix carrying invocation metadata
// (`EMERY_REQUEST_ID`, `EMERY_CORRELATION_ID`, `EMERY_CAUSATION_ID`).
const ENV_PREFIX: &str = "EMERY";

/// Parses and executes one argument vector over `provider`, buffering both
/// output channels.
///
/// Clap's own outcomes — help and version on stdout at exit 0, a usage
/// error on stderr at `USAGE_EXIT` — are complete responses before any
/// verb runs. Each verb decodes into its engine input, runs its handler fn
/// through the client, and is projected by the façade: the success body
/// rides stdout in the selected format, the failure envelope rides stderr
/// with the exit status from the one exit map. `completions` never
/// reaches a handler.
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
    let metadata = Metadata::from_env(ENV_PREFIX);
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

// The `specify` grammar; field docs are its `--help` text. Decoding
// builds the engine input by exhaustive struct literal, so an engine
// field the grammar does not carry fails to compile here.
#[derive(Debug, clap::Args)]
struct SpecifyArgs {
    /// Workspace-backed source adapters or local component paths.
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
        let sources = sources::decode(&adapters, &descriptions, config.as_deref())?;
        Ok(SpecifyInput { sources })
    }
}

// The `show` grammar; field docs are its `--help` text.
#[derive(Debug, clap::Args)]
struct ShowArgs {
    /// Reviewable artifact of the current revision.
    #[arg(value_parser = artifacts())]
    artifact: Artifact,
}

// The engine's closed artifact vocabulary as clap values, each with its help
// line; the exhaustive match makes a new variant a façade compile error.
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

// Looks up the remedy hint the failure envelope carries for an `error`
// discriminant; flag and verb vocabulary lives here, never in engine
// descriptions.
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
            "the loader refused the component; the message above names why (digest, export, or location)"
        }
        "unavailable" => {
            "the registry could not supply the package: check the network, the exact version, and any `registry` override"
        }
        _ => return None,
    };
    Some(Cow::Borrowed(hint))
}
