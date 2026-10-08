//! Implements Emery's command-line interface.
//!
//! [`run`] returns a buffered response, leaving process I/O and exit handling
//! to the caller. The crate installs no subscriber of its own; tracing follows
//! the `RUST_LOG` the runtime sets from its verbosity flags, which the grammar
//! declares (`-v`, `-q`) and never reads.

mod config;
mod text;

use std::borrow::Cow;
use std::convert::TryFrom;
use std::ffi::OsString;
use std::path::PathBuf;

use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};
use clap::{Parser, Subcommand};
use emery_engine::Provider;
use emery_engine::build::{BuildInput, build};
use emery_engine::show::{Artifact, ShowInput, show};
use emery_engine::specify::{SpecifyInput, specify};
use omnia_sdk::Error;
use omnia_sdk::api::command::{Command, Parsed, Response, Shell, Verbosity, completions, parse};
use omnia_sdk::api::{Client, Format, Metadata};
use strum::VariantArray as _;

const ABOUT: &str = "Deterministic primitives for spec-driven development";
const SPECIFY_DESC: &str = "Generate spec.md, design.md, and plan.md from source adapters.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, \
    or use `--config [<path>]` (default: `emery.toml`). With no sources, Emery looks \
    for `emery.toml` in the project root. Config and command-line sources cannot be \
    combined.\n\n\
    An adapter is an exact package reference, `namespace:name@version`. Emery reads it \
    from its store, `~/.emery/adapters`, where the file `namespace_name@version.wasm` is \
    that release on this machine, and fetches a release the store lacks through the \
    `emery` namespace's registry, `augentic.io`, keeping it there. Another namespace is \
    fetched by `wkg get <reference> -o ~/.emery/adapters/`. A `[[source]]` naming a \
    `repository` and `revision` is read from a clone kept under `.emery/vcs/`, checked out \
    at the revision for the run. Each run loads its adapters, reconciles their claims, and \
    atomically commits a new revision.";
const BUILD_DESC: &str = "Build the current plan through a target adapter.\n\n\
    Name the adapter, or use `--config [<path>]` (default: `emery.toml`) to read its \
    `[target]` table. With no adapter, Emery looks for `emery.toml` in the project root. \
    Config and a command-line adapter cannot be combined.\n\n\
    The build starts from a sealed commit — the project checkout's head, which must hold \
    no pending change outside `.emery/`, or the `branch` of the `[target] repository` the \
    config names, cloned under `.emery/vcs/` — in a working copy of its own, never the \
    checkout. Every slice of the plan is built in turn, each after the slices it depends \
    on, and sealed as one commit; the integrated head is labelled `emery/<revision>` and \
    pushed when `[target] remote` names where. The first slice that fails ends the run; \
    the slices built before it stay committed in `.emery/vcs/integration`. The adapter is \
    an exact package reference, `namespace:name@version`, read from the store \
    `~/.emery/adapters` and fetched through the `emery` namespace's registry when the \
    store lacks it, as for `specify`.";
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
            command.call(specify, || SpecifyInput::try_from(arguments), text::specify).await
        }
        Verb::Build(arguments) => {
            command.call(build, || BuildInput::try_from(arguments), text::build).await
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
    // Declared so help and completions list them and `-v` beside `-q` is
    // refused; the runtime reads them from argv before the guest runs.
    #[command(flatten)]
    #[expect(dead_code, reason = "the runtime acts on the flags; the grammar only declares them")]
    verbosity: Verbosity,
}

#[derive(Debug, Subcommand)]
enum Verb {
    /// Generate spec.md, design.md, and plan.md from the named sources
    #[command(long_about = SPECIFY_DESC)]
    Specify(SpecifyArgs),
    /// Build every slice of the current plan through a target adapter
    #[command(long_about = BUILD_DESC)]
    Build(BuildArgs),
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
    /// Workspace-backed source adapters, each an exact package reference
    /// `namespace:name@version`. Each source is named for its adapter's
    /// package name.
    adapters: Vec<String>,
    /// Bind an inline source as `<adapter>=<text>`; repeatable.
    #[arg(long = "description", short = 'd')]
    descriptions: Vec<String>,
    /// Operator-owned config; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = config::CONFIG_FILE)]
    config: Option<PathBuf>,
}

impl TryFrom<SpecifyArgs> for SpecifyInput {
    type Error = Error;

    fn try_from(args: SpecifyArgs) -> Result<Self, Error> {
        let SpecifyArgs {
            adapters,
            descriptions,
            config,
        } = args;
        config::SourceCarriers {
            adapters: &adapters,
            descriptions: &descriptions,
            config: config.as_deref(),
        }
        .try_into()
    }
}

#[derive(Debug, clap::Args)]
struct BuildArgs {
    /// The target adapter, an exact package reference `namespace:name@version`.
    adapter: Option<String>,
    /// Operator-owned config; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = config::CONFIG_FILE)]
    config: Option<PathBuf>,
}

impl TryFrom<BuildArgs> for BuildInput {
    type Error = Error;

    fn try_from(args: BuildArgs) -> Result<Self, Error> {
        let BuildArgs { adapter, config } = args;
        config::TargetCarriers {
            adapter: adapter.as_deref(),
            config: config.as_deref(),
        }
        .try_into()
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
            Artifact::Plan => "The build plan artifact.",
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
        "build-target-required" => {
            "pass a target adapter to `emery build`, or add a `[target]` table to `emery.toml` at the project root"
        }
        "spec-not-generated" => {
            "run `emery specify <adapter>...` to commit a revision, then re-run show or build"
        }
        "spec-outdated" => {
            "the revision predates this emery's grammar: re-run `emery specify <adapter>...` to regenerate it"
        }
        "repository-required" => {
            "a build lands as a commit: run `emery build` from a repository checkout, or name one with `[target] repository` and `branch` in `emery.toml`"
        }
        "base-not-sealed" => {
            "commit or stash the pending changes so the build starts from a sealed commit; `.emery/` is never counted, so add it to `.gitignore`"
        }
        "revision-not-found" => {
            "check the `repository`, `revision`, `branch`, or `remote` in `emery.toml` against what the repository holds"
        }
        "adapter-reference" => "an adapter is an exact package reference, `namespace:name@version`",
        "refused" => {
            "the loader refused the component; the message above names why: an invalid artifact, a pre-compiled artifact where raw wasm is required, a mismatched digest, or a release the binary routes nowhere, which is fetched with `wkg get <reference> -o ~/.emery/adapters/`"
        }
        "unavailable" => {
            "the registry could not supply the release: check the network and that the exact version is published under its namespace; a copy at `~/.emery/adapters/<namespace>_<name>@<version>.wasm` stands in for it"
        }
        _ => return None,
    };
    Some(Cow::Borrowed(hint))
}
