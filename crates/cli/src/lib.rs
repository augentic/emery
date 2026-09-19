//! Implements Emery's command-line interface.
//!
//! The interface provides the `specify`, `show`, and `completions` commands.
//! It translates command-line sources into engine inputs, renders text output,
//! and supplies recovery hints for known failures.
//!
//! [`run`] returns a buffered response, leaving process I/O and exit handling
//! to the caller. The global `--verbose` and `--quiet` flags select the
//! tracing filter [`run`] reports through its callback before the verb runs;
//! the crate installs no subscriber of its own.

mod sources;
mod text;

use std::borrow::Cow;
use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};
use clap::error::ErrorKind;
use clap::{ArgAction, CommandFactory as _, Parser, Subcommand};
use emery_engine::Provider;
use emery_engine::show::{Artifact, ShowInput, show};
use emery_engine::specify::{SpecifyInput, specify};
use omnia_sdk::Error;
use omnia_sdk::api::command::{Command, Parsed, Response, Shell, completions, parse};
use omnia_sdk::api::{Client, Format, Metadata};
use strum::VariantArray as _;

const ABOUT: &str = "Deterministic primitives for spec-driven development";
const SPECIFY_DESC: &str = "Generate spec.md and design.md from source adapters.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, \
    or use `--config [<path>]` (default: `emery.toml`). With no sources, Emery looks \
    for `emery.toml` in the project root. Config and command-line sources cannot be \
    combined.\n\n\
    Adapter paths are project-relative. Each run reloads adapters, reconciles their \
    claims, and atomically commits a new revision.";
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
///
/// The global `--verbose` and `--quiet` flags select a [`Verbosity`] reported
/// through `verbosity` once the grammar parses and before the verb runs.
/// Combining them is a usage error, and the callback is never reached.
pub async fn run<P, I, T, F>(provider: P, argv: I, set_filter: F) -> Response
where
    P: Provider,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
    F: FnOnce(Verbosity),
{
    let app = match parse::<App>(argv) {
        Parsed::App(app) => app,
        Parsed::Display(text) => return Response::success(text),
        Parsed::Usage(error) => return Response::usage(&error),
    };

    match app.verbosity() {
        Ok(level) => set_filter(level),
        Err(error) => return Response::usage(&error),
    }

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
    // Select the output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text", global = true)]
    format: Format,
    // Show engine debug tracing on stderr; repeat for trace detail.
    #[arg(short, long, action = ArgAction::Count, global = true)]
    verbose: u8,
    // Silence tracing.
    #[arg(short, long, global = true)]
    quiet: bool,
}

impl App {
    // Folds the counted `-v` and `-q` into one level, refusing both together
    // as a usage error. Clap validates a `conflicts_with` per command level,
    // so `-v` on the root and `-q` on the verb would never meet.
    fn verbosity(&self) -> Result<Verbosity, clap::Error> {
        match (self.verbose, self.quiet) {
            (1.., true) => Err(Self::command().error(
                ErrorKind::ArgumentConflict,
                "the argument '--verbose' cannot be used with '--quiet'",
            )),
            (0, true) => Ok(Verbosity::Quiet),
            (0, false) => Ok(Verbosity::Info),
            (1, false) => Ok(Verbosity::Debug),
            (2.., false) => Ok(Verbosity::Trace),
        }
    }
}

/// The tracing level the global `--verbose` and `--quiet` flags select.
///
/// A bare run is [`Self::Info`], `-q` is [`Self::Quiet`], `-v` is
/// [`Self::Debug`], and `-vv` or more is [`Self::Trace`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Verbosity {
    /// No tracing, selected by `-q`.
    Quiet,
    /// INFO progress on a bare invocation.
    Info,
    /// INFO progress plus engine DEBUG, selected by `-v`.
    Debug,
    /// DEBUG everywhere plus engine TRACE, selected by `-vv`.
    Trace,
}

impl Verbosity {
    /// Returns the tracing directives this level selects, refined by the ambient `RUST_LOG`.
    ///
    /// - [`Self::Quiet`] selects `off`
    /// - [`Self::Info`] selects `info`
    /// - [`Self::Debug`] selects `info` plus `emery_cli`, `emery_engine`, and `omnia_sdk` at `debug`
    /// - [`Self::Trace`] selects `debug` plus `emery_cli`, `emery_engine`, and `omnia_sdk` at `trace`
    ///
    /// Every level but [`Self::Quiet`] appends the directives of a set `RUST_LOG`
    /// after its own, so for a target both name the environment's wins.
    #[must_use]
    pub fn into_filter(self) -> Cow<'static, str> {
        let preset = match self {
            Self::Quiet => return Cow::Borrowed("off"),
            Self::Info => "info",
            Self::Debug => "info,emery_cli=debug,emery_engine=debug,omnia_sdk=debug",
            Self::Trace => "debug,emery_cli=trace,emery_engine=trace,omnia_sdk=trace",
        };
        match std::env::var("RUST_LOG") {
            Ok(rust_log) if !rust_log.is_empty() => Cow::Owned(format!("{preset},{rust_log}")),
            _ => Cow::Borrowed(preset),
        }
    }
}

use std::fmt::{self, Display, Formatter};

impl Display for Verbosity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.into_filter())
    }
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
            "the loader refused the component; the message above names why (export or location)"
        }
        "unavailable" => {
            "the registry could not supply the package: check the network and that the exact version is published under its namespace"
        }
        _ => return None,
    };
    Some(Cow::Borrowed(hint))
}
