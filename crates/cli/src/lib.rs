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
use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};
use clap::{Parser, Subcommand};
use emery_engine::apply::{ApplyInput, apply};
use emery_engine::build::{BuildInput, build};
use emery_engine::show::{Artifact, ShowInput, show};
use emery_engine::specify::{SpecifyInput, specify};
use emery_engine::{Provider, is_retryable};
use omnia_sdk::Error;
use omnia_sdk::api::command::{Command, Parsed, Response, Shell, Verbosity, completions, parse};
use omnia_sdk::api::{Client, Format, Metadata};
use strum::VariantArray as _;

const ABOUT: &str = "Deterministic primitives for spec-driven development";
const SPECIFY_DESC: &str = "Stage spec.md, design.md, and plan.md from source adapters.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, \
    or use `--config [<path>]` (default: `emery.toml`). With no sources, Emery looks \
    for `emery.toml` in the project root. Command-line sources layer over the config: \
    one named as a `[[source]]` is — by its adapter's package name — takes that entry's \
    place, keeping its `rank`; any other is appended.\n\n\
    An adapter is an exact package reference, `namespace:name@version`. Emery reads it \
    from its store, `~/.emery/adapters`, where the file `namespace_name@version.wasm` is \
    that release on this machine, and fetches a release the store lacks through the \
    `emery` namespace's registry, `augentic.io`, keeping it there. Another namespace is \
    fetched by `wkg get <reference> -o ~/.emery/adapters/`. A `[[source]]` naming a \
    `repository` and `revision` is read from a clone kept under `.emery/vcs/`, checked out \
    at the revision for the run. Each run loads its adapters, reconciles their claims, and \
    stages the drafted revision as the one candidate, replacing any staged before it, for \
    `emery show <artifact> --staged` to review and `emery apply` to seal; `--apply` seals \
    it in the same run. The diff reported is against the current revision, which a seal \
    displaces.";
const APPLY_DESC: &str = "Seal the staged candidate as the current revision.\n\n\
    The candidate `emery specify` staged becomes the revision `emery show` and `emery \
    build` read, the one it displaces is pruned, and the staging slot is cleared. With \
    nothing staged the verb fails with `nothing-staged`.";
const BUILD_DESC: &str = "Build the current plan through a target adapter.\n\n\
    Name the adapter, or use `--config [<path>]` (default: `emery.toml`) to read its \
    `[target]` table. With no adapter, Emery looks for `emery.toml` in the project root. \
    A command-line adapter layers over the config, taking the place of its `adapter` \
    under its `name` and dropping its `digest`; `--repository` takes the place of its \
    `repository`.\n\n\
    The build lands as commits in one repository, never in the checkout, the one \
    `--repository` or the `[target] repository` names: a URL, cloned under \
    `.emery/vcs/repos/<name>` and built from its default branch; `.`, the project \
    checkout, from its head, which must hold no pending change outside `.emery/`; or, \
    with none named, a repository the build creates once at `.emery/vcs/repos/<name>` \
    and keeps, its root commit the adapter's layout. `<name>` is the `[target] name`, \
    the adapter's package name when the config gives none. A build no label resumes first has the adapter initialise the tree — scaffold \
    the files of its layout the tree lacks, with no model turn — and seals them as the \
    `Initialise` commit every slice builds over. A label `emery/<revision>` the repository \
    already holds, descending from that commit, is resumed: the slices it records are not built \
    again, and neither are the slices `.emery/vcs/integration` holds merged over it from a \
    run that ended before their wave was verified, which are verified instead. The rest go \
    in waves: every slice whose dependencies are merged is built at once, up to `--jobs` \
    concurrently, each in its own working copy under `.emery/vcs/worktrees/<id>` cut at \
    the wave's head; what each changed is sealed as one commit and merged in id order into \
    `.emery/vcs/integration` under the adapter's merge rules, a path the rules leave \
    conflicted resolved with the model and merged in its place. The adapter then verifies \
    the integrated tree, repairing what its checks find, and the wave is labelled; the \
    label stands in the repository for the operator to read or push. A build that fails \
    upstream or runs out of the model's time is put once more in a fresh copy, a \
    verification once more over the same one. A conflict the model cannot resolve ends the \
    run (`merge-unresolved`), as a wave the adapter does not verify does (`verify-failed`), \
    the label where the last verified wave left it and the integration working copy left \
    for the next run to resume from. The adapter is an exact package reference, \
    `namespace:name@version`, read from the store `~/.emery/adapters` and fetched through \
    the `emery` namespace's registry when the store lacks it, as for `specify`.";
const SHOW_DESC: &str = "Print an artifact from the current revision.\n\n\
    `--staged` prints it from the candidate `emery specify` staged and `emery apply` has \
    not sealed. Text output contains only the artifact body. `--format json` also \
    includes the revision id and the typed document.";
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
    let invoked = Invoked::from(&app.verb);
    let command =
        Command::new(&client, &metadata, app.format).hints(move |error| hint(error, invoked));

    match app.verb {
        Verb::Completions { shell } => completions::<App>(shell, NAME),
        Verb::Specify(arguments) => {
            command.call(specify, || SpecifyInput::try_from(arguments), text::specify).await
        }
        Verb::Apply(ApplyArgs {}) => {
            command.call(apply, || Ok(ApplyInput::default()), text::apply).await
        }
        Verb::Build(arguments) => {
            command.call(build, || BuildInput::try_from(arguments), text::build).await
        }
        Verb::Show(ShowArgs { artifact, staged }) => {
            command.call(show, || Ok(ShowInput { artifact, staged }), text::show).await
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
    /// Stage spec.md, design.md, and plan.md from the named sources as a candidate
    #[command(long_about = SPECIFY_DESC)]
    Specify(SpecifyArgs),
    /// Seal the staged candidate as the current revision
    #[command(long_about = APPLY_DESC)]
    Apply(ApplyArgs),
    /// Build every slice of the current plan through a target adapter
    #[command(long_about = BUILD_DESC)]
    Build(BuildArgs),
    /// Print a reviewable artifact of the current revision, or the staged candidate, to stdout
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
    /// package name; over `--config`, it replaces the `[[source]]` of that name.
    adapters: Vec<String>,
    /// Bind an inline source as `<adapter>=<text>`; repeatable.
    #[arg(long = "description", short = 'd')]
    descriptions: Vec<String>,
    /// Operator-owned config; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = config::CONFIG_FILE)]
    config: Option<PathBuf>,
    /// Seal the staged candidate as the current revision in the same run, as `emery apply` would.
    #[arg(long)]
    apply: bool,
}

impl TryFrom<SpecifyArgs> for SpecifyInput {
    type Error = Error;

    fn try_from(args: SpecifyArgs) -> Result<Self, Error> {
        let SpecifyArgs {
            adapters,
            descriptions,
            config,
            apply,
        } = args;
        let mut input: Self = config::SourceCarriers {
            adapters: &adapters,
            descriptions: &descriptions,
            config: config.as_deref(),
        }
        .try_into()?;
        input.apply = apply;
        Ok(input)
    }
}

// No argument yet: the candidate is the one `specify` staged.
#[derive(Debug, clap::Args)]
struct ApplyArgs {}

#[derive(Debug, clap::Args)]
struct BuildArgs {
    /// The target adapter, an exact package reference `namespace:name@version`; over `--config`, it replaces the `[target] adapter`.
    adapter: Option<String>,
    /// The repository the build lands in: a URL, cloned under .emery/vcs/repos/<name>, or `.` for the project checkout; over `--config`, it replaces the `[target] repository`.
    #[arg(long, value_name = "URL|.")]
    repository: Option<String>,
    /// Operator-owned config; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = config::CONFIG_FILE)]
    config: Option<PathBuf>,
    /// How many slices of a wave to build at once; unset builds every slice of a wave at once, within the model backend's own agent cap.
    #[arg(long, short = 'j', env = "EMERY_JOBS", value_name = "N")]
    jobs: Option<NonZeroUsize>,
}

impl TryFrom<BuildArgs> for BuildInput {
    type Error = Error;

    fn try_from(args: BuildArgs) -> Result<Self, Error> {
        let BuildArgs {
            adapter,
            repository,
            config,
            jobs,
        } = args;
        let mut input: Self = config::TargetCarriers {
            adapter: adapter.as_deref(),
            repository: repository.as_deref(),
            config: config.as_deref(),
        }
        .try_into()?;
        input.jobs = jobs;
        Ok(input)
    }
}

#[derive(Debug, clap::Args)]
struct ShowArgs {
    /// Reviewable artifact of the current revision.
    #[arg(value_parser = artifacts())]
    artifact: Artifact,
    /// Read the staged candidate in place of the current revision.
    #[arg(long)]
    staged: bool,
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

// Flag and verb vocabulary stays at the CLI boundary. `Invoked::Build` tells a
// build's upstream failure, or a turn of its the model's time budget ended,
// which a re-run resumes, from a specify's, which a re-run starts over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Invoked {
    Specify,
    Apply,
    Build,
    Show,
    Completions,
}

impl From<&Verb> for Invoked {
    fn from(verb: &Verb) -> Self {
        match verb {
            Verb::Specify(_) => Self::Specify,
            Verb::Apply(_) => Self::Apply,
            Verb::Build(_) => Self::Build,
            Verb::Show(_) => Self::Show,
            Verb::Completions { .. } => Self::Completions,
        }
    }
}

fn hint(error: &Error, invoked: Invoked) -> Option<Cow<'static, str>> {
    let code = error.code();
    let hint = match code.as_str() {
        "bad_gateway" if invoked == Invoked::Build => {
            "the model backend failed twice upstream, or the repository's remote refused; re-run `emery build` once it answers: the label `emery/<revision>` stands at the last verified wave, and `.emery/vcs/integration` holds the slices merged since, so neither is built again"
        }
        "bad_request" if invoked == Invoked::Build && is_retryable(error) => {
            "the model ran out of its time budget on both tries of one turn; re-run `emery build` to put it again: the label `emery/<revision>` stands at the last verified wave, and `.emery/vcs/integration` holds the slices merged since, so neither is built again"
        }
        "unsupported-version" => {
            "update emery: `brew upgrade emery`, or `cargo install --git https://github.com/augentic/emery --locked`"
        }
        "specify-source-required" => {
            "pass one or more adapters to `emery specify`, or add an `emery.toml` at the project root"
        }
        "build-target-required" => {
            "pass a target adapter to `emery build`, or add a `[target]` table to `emery.toml` at the project root"
        }
        "spec-not-generated" if error.description().contains("is staged") => {
            "run `emery apply` to seal the staged candidate as the current revision, then re-run show or build; `emery show <artifact> --staged` reviews it first"
        }
        "spec-not-generated" => {
            "run `emery specify <adapter>...` to stage a candidate and `emery apply` to seal it (or `emery specify --apply` for both), then re-run show or build"
        }
        "nothing-staged" => {
            "run `emery specify <adapter>...` to stage a candidate, then re-run `emery apply` or `emery show <artifact> --staged`"
        }
        "spec-outdated" => {
            "the revision predates this emery's grammar: re-run `emery specify <adapter>...` to regenerate it"
        }
        "repository-required" => {
            "`[target] repository = \".\"` builds into the project checkout, which must be a repository: initialise and seal it, name a repository URL instead, or drop the key so the build creates a repository of its own under `.emery/vcs/repos/<name>`"
        }
        "base-not-sealed" => {
            "commit or stash the pending changes so the build starts from a sealed commit; `.emery/` is never counted, so add it to `.gitignore`"
        }
        "revision-not-found" => {
            "check the `repository` or `revision` in `emery.toml` against what the repository holds; a target's clone under `.emery/vcs/repos/<name>` keeps the origin it was first cloned or created with, so remove it, or name the target otherwise, to point the name at another repository"
        }
        "verify-failed" => {
            "inspect `.emery/vcs/integration`, which holds the wave's slices merged; re-run `emery build` to verify them again with nothing built again, or `git worktree remove --force` the copy from the repository the label stands in to rebuild the wave from the last verified head; `git bisect` over `emery/<revision>` attributes a failure to a slice"
        }
        "merge-unresolved" => {
            "two slices rewrite the same files and the model could not combine them: inspect the named paths in `.emery/vcs/worktrees/<slice>` and `.emery/vcs/integration`; re-run `emery build` to build the slice again over the merged head, which holds the slices merged before it; declare a merge rule in the adapter for a list file or a generated one; or re-run `emery specify` for a plan that keeps the two in one slice"
        }
        "secret-detected" => {
            "inspect the named path in `.emery/vcs/worktrees/<slice>` or `.emery/vcs/integration`, remove the credential from where the build read it (the sources, the environment, a file the model reached), and re-run `emery build`; nothing of the change set was sealed and the label stands at the last verified wave"
        }
        "adapter-reference" => "an adapter is an exact package reference, `namespace:name@version`",
        "refused" => {
            "the loader refused the component; the message above names why: an invalid artifact, a pre-compiled artifact where raw wasm is required, a mismatched digest, an export whose signature does not match this emery's contract (rebuild or update the copy in `~/.emery/adapters/`), or a release the binary routes nowhere, which is fetched with `wkg get <reference> -o ~/.emery/adapters/`"
        }
        "unavailable" => {
            "the registry could not supply the release: check the network and that the exact version is published under its namespace; a copy at `~/.emery/adapters/<namespace>_<name>@<version>.wasm` stands in for it"
        }
        _ => return None,
    };
    Some(Cow::Borrowed(hint))
}
