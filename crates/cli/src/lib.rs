//! Implements Emery's command-line interface.
//!
//! [`run`] returns a buffered response, leaving process I/O and exit handling
//! to the caller. The crate installs no subscriber of its own; tracing follows
//! the `RUST_LOG` the runtime sets from its verbosity flags, which the grammar
//! declares (`-v`, `-q`) and never reads.

mod config;

use std::borrow::Cow;
use std::ffi::OsString;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::builder::{PossibleValue, PossibleValuesParser, TypedValueParser};
use clap::{Parser, Subcommand};
use emery_engine::amend::{AmendInput, amend};
use emery_engine::apply::{ApplyInput, apply};
use emery_engine::build::{BuildInput, build};
use emery_engine::show::{Artifact, ShowInput, show};
use emery_engine::specify::{SpecifyInput, specify};
use emery_engine::{Provider, is_retryable};
use omnia_sdk::Error;
use omnia_sdk::api::command::{
    Command, Failure, Format, Parsed, Response, Shell, Verbosity, completions, parse,
};
use omnia_sdk::api::{Client, Metadata};
use strum::VariantArray as _;

const NAME: &str = "emery";
const ABOUT: &str = "Deterministic primitives for spec-driven development";

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

    let client = Client::new(provider);
    let metadata = Metadata::from_env("EMERY");
    let invoked = Invoked::from(&app.verb);
    let format = app.format;
    let command = Command::new(&client, &metadata, format).hints(move |error| invoked.hint(error));

    match app.verb {
        Verb::Completions { shell } => completions::<App>(shell, NAME),
        Verb::Specify(arguments) => match SpecifyInput::try_from(arguments) {
            Ok(input) => command.call(specify, input).await,
            Err(error) => refuse(format, invoked, error),
        },
        Verb::Apply(arguments) => match ApplyInput::try_from(arguments) {
            Ok(input) => command.call(apply, input).await,
            Err(error) => refuse(format, invoked, error),
        },
        Verb::Amend(arguments) => match AmendInput::try_from(arguments) {
            Ok(input) => command.call(amend, input).await,
            Err(error) => refuse(format, invoked, error),
        },
        Verb::Build(arguments) => match BuildInput::try_from(arguments) {
            Ok(input) => command.call(build, input).await,
            Err(error) => refuse(format, invoked, error),
        },
        Verb::Show(arguments) => match ShowInput::try_from(arguments) {
            Ok(input) => command.call(show, input).await,
            Err(error) => refuse(format, invoked, error),
        },
    }
}

fn refuse(format: Format, invoked: Invoked, error: Error) -> Response {
    let mut failure = Failure::from(error);
    if failure.hint().is_none()
        && let Some(hint) = invoked.hint(failure.error())
    {
        failure = failure.with_hint(hint);
    }
    Response::failure(format.encode(&failure).bytes, failure.exit_code())
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

const COMPLETIONS_DESC: &str = "Generate shell completions.\n\n\
    Pipe into your shell's completion directory. Example: \
    `emery completions zsh > ~/.zsh/_emery`";

const SPECIFY_DESC: &str = "Stage spec.md, design.md, and plan.md from source adapters.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, \
    or use `--config [<path>]` (default: `emery.toml`). With no sources, Emery looks \
    for `emery.toml` in the project root. Command-line sources layer over the config: \
    one named as a `[[source]]` is — by its adapter's package name — takes that entry's \
    place whole; any other is appended.\n\n\
    An adapter is an exact package reference, `namespace:name@version`. Emery reads it \
    from its store, `~/.emery/adapters`, where the file `namespace_name@version.wasm` is \
    that release on this machine, and fetches a release the store lacks through the \
    `emery` namespace's registry, `augentic.io`, keeping it there. Another namespace is \
    fetched by `wkg get <reference> -o ~/.emery/adapters/`. A `[[source]]` naming a \
    `repository` and `revision` is read from a clone kept under `.emery/vcs/`, checked out \
    at the revision for the run. Each run loads its adapters, reconciles their claims, and \
    stages the drafted revision as the one candidate, replacing any staged before it, for \
    `emery show <artifact> --staged` to review and `emery apply` to seal; `--apply` seals \
    it in the same run. The `[target]` table of the config, named or discovered, says \
    which repository the specification lives in: the diff reported is against the \
    revision it holds, which a seal displaces, and `--apply` needs one. A revision that \
    amends an earlier one, as `emery apply` seals an amendment, is a lineage a specify \
    would start over, so the run is refused `lineage-exists` before any adapter loads \
    unless `--fresh` says so: the candidate is then staged fresh and sealed with no parent \
    over the repository's head, every earlier label left standing.";

const APPLY_DESC: &str = "Seal the staged candidate or amendment as the current revision.\n\n\
    The candidate `emery specify` staged, or the amendment `emery amend` staged, is sealed \
    in the repository the `[target]` table of `--config [<path>]` (default: `emery.toml`) \
    names — a URL, cloned under `.emery/vcs/repos/<name>`; `.`, the project checkout; or, \
    with none, a repository created once at `.emery/vcs/repos/<name>` and kept — as one \
    commit over the head, holding `spec/spec.json`, `spec/design.json`, and \
    `spec/plan.json`, labelled `emery/<revision>`; `--repository` takes the place of the \
    table's. An amendment seals its three documents over the commit its base was read at, \
    the commit naming the base as the parent and carrying what changed, and only while \
    that base is still the current revision there; otherwise the verb fails with \
    `amendment-stale` and the amendment stays staged for `emery amend` to prepare again. \
    Over a revision sealed from an amendment, a candidate seals only when `emery specify \
    --fresh` staged it, with no parent and over the head, every earlier label left \
    standing; otherwise the verb fails with `lineage-exists` and the candidate stays staged. \
    The checkout is never touched: merge or rebase the label to carry the specification on \
    the branch. `emery show` and `emery build` read the sealed revision from then on, and \
    the staging slot is cleared. With nothing staged the verb fails with `nothing-staged`.";

const AMEND_DESC: &str = "Stage an amendment against the current revision.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, or \
    use `--config [<path>]` for the `[[source]]` tables of a change file, read and layered \
    as `emery specify` reads them. The current revision is read from the repository the \
    `[target]` table of that file names, or of the project-root `emery.toml` when the file \
    names none, and its requirements stand as the baseline the change is matched against: \
    a claim restating or revising one keeps the requirement's id, a claim no requirement \
    covers is added under a new id, and a requirement no claim touches stands unchanged. \
    No source may be named `baseline`. A change that leaves every requirement as it \
    stands stages nothing. Otherwise the amendment is staged for `emery show amendment` to \
    review and `emery apply` to seal, replacing any amendment staged before it; a staged \
    revision candidate is never replaced, so seal it with `emery apply` first. `--apply` \
    seals the staged amendment in the same run; without it nothing is sealed.";

const BUILD_DESC: &str = "Build the current plan through a target adapter.\n\n\
    Name the adapter, or use `--config [<path>]` (default: `emery.toml`) to read its \
    `[target]` table. With no adapter, Emery looks for `emery.toml` in the project root. \
    A command-line adapter layers over the config, taking the place of its `adapter` \
    under its `name` and dropping its `digest`; `--repository` takes the place of its \
    `repository`.\n\n\
    The build lands as commits in one repository, never in the checkout, the one \
    `--repository` or the `[target] repository` names, which also holds the revision it \
    builds, sealed by `emery apply` under `emery/<revision>`: a URL, cloned under \
    `.emery/vcs/repos/<name>` and built from its default branch; `.`, the project \
    checkout, from its head, which must hold no pending change outside `.emery/`; or, \
    with none named, the repository `emery apply` created at `.emery/vcs/repos/<name>`. \
    `<name>` is the `[target] name`, the adapter's package name when the config gives \
    none. A build in which no slice is merged yet first has the adapter initialise the \
    tree — scaffold the files of its layout the tree lacks, with no model turn — and \
    seals them as the `Initialise` commit every slice builds over. A label \
    `emery/<revision>` the repository already holds, descending from the head, is \
    resumed: the slices it records are not built again, and neither are the slices \
    `.emery/vcs/integration` holds merged over it from a run that ended before their wave \
    was verified, which are verified instead. The rest go \
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
    The revision is read from the repository the `[target]` table of `--config [<path>]` \
    (default: `emery.toml`) names, where `emery apply` sealed it. `schedule` cuts the \
    sealed plan into build slices under `--budget`, the `[target] budget`, or the target \
    adapter's default, with no model turn. `--staged` prints `spec`, `design`, or `plan` \
    from the candidate `emery specify` staged and `emery apply` has not sealed, and \
    needs no target. `amendment` prints the staged amendment, which stands staged alone \
    until it is sealed, and needs neither. Text output contains only the artifact body. \
    `--format json` also includes the revision id and the typed document.";

#[derive(Debug, Subcommand)]
enum Verb {
    /// Stage spec.md, design.md, and plan.md from the named sources as a candidate
    #[command(long_about = SPECIFY_DESC)]
    Specify(SpecifyArgs),
    /// Seal the staged candidate or amendment as the current revision
    #[command(long_about = APPLY_DESC)]
    Apply(ApplyArgs),
    /// Stage an amendment against the current revision from the named sources
    #[command(long_about = AMEND_DESC)]
    Amend(AmendArgs),
    /// Build every slice of the current plan through a target adapter
    #[command(long_about = BUILD_DESC)]
    Build(BuildArgs),
    /// Print a reviewable artifact of the current revision, the staged candidate, or the staged amendment to stdout
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
    /// Start the history over: stage the candidate as a fresh revision, sealed with no parent over the repository's head where the current revision amends an earlier one.
    #[arg(long)]
    fresh: bool,
}

impl TryFrom<SpecifyArgs> for SpecifyInput {
    type Error = Error;

    fn try_from(args: SpecifyArgs) -> Result<Self, Error> {
        let (sources, target) = config::SourceCarriers {
            adapters: &args.adapters,
            descriptions: &args.descriptions,
            config: args.config.as_deref(),
        }
        .resolve()?;
        Ok(Self {
            sources,
            apply: args.apply,
            fresh: args.fresh,
            target,
        })
    }
}

#[derive(Debug, clap::Args)]
struct ApplyArgs {
    /// The repository the revision is sealed in: a URL, cloned under .emery/vcs/repos/<name>, or `.` for the project checkout; over `--config`, it replaces the `[target] repository`.
    #[arg(long, value_name = "URL|.")]
    repository: Option<String>,
    /// Operator-owned config; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = config::CONFIG_FILE)]
    config: Option<PathBuf>,
}

impl TryFrom<ApplyArgs> for ApplyInput {
    type Error = Error;

    fn try_from(args: ApplyArgs) -> Result<Self, Error> {
        let target = config::target(args.config.as_deref(), args.repository.as_deref())?
            .ok_or_else(|| config::missing_target("`emery apply`"))?;
        Ok(Self { target })
    }
}

#[derive(Debug, clap::Args)]
struct AmendArgs {
    /// Workspace-backed change source adapters, each an exact package reference
    /// `namespace:name@version`. Each source is named for its adapter's
    /// package name; over `--config`, it replaces the `[[source]]` of that name.
    adapters: Vec<String>,
    /// Bind an inline change source as `<adapter>=<text>`; repeatable.
    #[arg(long = "description", short = 'd')]
    descriptions: Vec<String>,
    /// The change file whose `[[source]]` tables state the change; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = config::CONFIG_FILE)]
    config: Option<PathBuf>,
    /// Seal the staged amendment as the current revision in the same run, as `emery apply` would.
    #[arg(long)]
    apply: bool,
}

impl TryFrom<AmendArgs> for AmendInput {
    type Error = Error;

    fn try_from(args: AmendArgs) -> Result<Self, Error> {
        let (sources, target) = config::SourceCarriers {
            adapters: &args.adapters,
            descriptions: &args.descriptions,
            config: args.config.as_deref(),
        }
        .resolve_amend()?;
        Ok(Self {
            sources,
            target,
            apply: args.apply,
        })
    }
}

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
    /// The maximum number of requirements per build slice; over `--config`, it replaces the `[target] budget`.
    #[arg(long, short = 'b', env = "EMERY_BUDGET", value_name = "N")]
    budget: Option<NonZeroUsize>,
}

impl TryFrom<BuildArgs> for BuildInput {
    type Error = Error;

    fn try_from(args: BuildArgs) -> Result<Self, Error> {
        config::TargetCarriers {
            adapter: args.adapter.as_deref(),
            repository: args.repository.as_deref(),
            config: args.config.as_deref(),
            jobs: args.jobs,
            budget: args.budget,
        }
        .resolve()
    }
}

#[derive(Debug, clap::Args)]
struct ShowArgs {
    /// Reviewable artifact of the current revision, or the staged amendment.
    #[arg(value_parser = artifacts())]
    artifact: Artifact,
    /// Read the staged candidate in place of the current revision; the amendment is staged or nothing.
    #[arg(long)]
    staged: bool,
    /// Operator-owned config naming the repository the revision lives in; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = config::CONFIG_FILE)]
    config: Option<PathBuf>,
    /// The maximum number of requirements per build slice for `schedule`; over `--config`, it replaces the `[target] budget`.
    #[arg(long, short = 'b', env = "EMERY_BUDGET", value_name = "N")]
    budget: Option<NonZeroUsize>,
}

impl TryFrom<ShowArgs> for ShowInput {
    type Error = Error;

    fn try_from(args: ShowArgs) -> Result<Self, Error> {
        config::show(args.artifact, args.staged, args.config.as_deref(), args.budget)
    }
}

fn artifacts() -> impl TypedValueParser<Value = Artifact> {
    PossibleValuesParser::new(Artifact::VARIANTS.iter().map(|artifact| {
        let help = match artifact {
            Artifact::Spec => "The behavioural specification artifact.",
            Artifact::Design => "The rebuild design artifact.",
            Artifact::Plan => "The build plan artifact.",
            Artifact::Schedule => "The budgeted build schedule cut from the plan.",
            Artifact::Amendment => "The staged amendment, awaiting its seal.",
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
    Amend,
    Build,
    Show,
    Completions,
}

impl Invoked {
    fn hint(self, error: &Error) -> Option<Cow<'static, str>> {
        let code = error.code();
        let hint = match code.as_str() {
            "bad_gateway" | "bad_request" if self == Self::Build && is_retryable(error) => {
                "re-run `emery build` to resume from the last verified wave"
            }
            "unsupported-version" => {
                "update emery: `brew upgrade emery`, or `cargo install --git https://github.com/augentic/emery --locked`"
            }
            "specify-source-required" if self == Self::Amend => {
                "pass one or more adapters to `emery amend`, or `--config <path>`"
            }
            "specify-source-required" => {
                "pass one or more adapters to `emery specify`, or add `emery.toml`"
            }
            "build-target-required" => self.target_hint(),
            "spec-not-generated" => self.not_generated_hint(&error.description()),
            "spec-mismatch" if error.description().contains("edited or merged by hand") => {
                "keep one side's `spec/` whole (`git checkout <branch> -- spec/`) and re-run `emery amend`"
            }
            "spec-mismatch" => {
                "restore `spec/` (`git checkout emery/<revision> -- spec/`) or move the directory aside"
            }
            "nothing-staged" if error.description().contains("amendment") => {
                "run `emery amend <adapter>...` to stage an amendment first"
            }
            "nothing-staged" => "run `emery specify <adapter>...` to stage a candidate first",
            "amendment-stale" => "re-run `emery amend <adapter>...` and then `emery apply`",
            "lineage-exists" if self == Self::Specify => {
                "run `emery amend <adapter>...`, or `emery specify --fresh <adapter>...` to start over"
            }
            "lineage-exists" => {
                "run `emery specify --fresh <adapter>...` then `emery apply`, or use `emery amend <adapter>...`"
            }
            "amendment-pending" if self == Self::Specify => {
                "seal the amendment with `emery apply`, or inspect it with `emery show amendment`"
            }
            "amendment-pending" if self == Self::Amend => {
                "seal the candidate with `emery apply`, or inspect it with `emery show spec --staged`"
            }
            "amendment-pending" => "inspect the staged amendment with `emery show amendment`",
            "retire-unmatched" => {
                "compare the claim's id against `emery show spec` and remove or reword the withdrawal"
            }
            "spec-outdated" => "re-run `emery specify <adapter>...` to regenerate the revision",
            "repository-required" => {
                "run `git init` and commit, set `[target] repository` to a URL, or drop the key"
            }
            "base-not-sealed" => {
                "commit or stash pending changes, and add `.emery/` to `.gitignore`"
            }
            "revision-not-found" => {
                "check `repository` and `revision` in `emery.toml`, or remove `.emery/vcs/repos/<name>` if the URL changed"
            }
            "verify-failed" => {
                "inspect `.emery/vcs/integration` and re-run `emery build`, or run `git bisect` on `emery/<revision>`"
            }
            "merge-unresolved" => {
                "re-run `emery build` to retry the slice, or add a merge rule in the target adapter"
            }
            "secret-detected" => {
                "remove the credential from `.emery/vcs/worktrees/<slice>` or the sources and re-run"
            }
            "spec-rewritten" => {
                "remove changes to `spec/` in `.emery/vcs/worktrees/<slice>` and re-run `emery build`"
            }
            "adapter-reference" => {
                "an adapter is an exact package reference, `namespace:name@version`"
            }
            "refused" => {
                "rebuild or update the adapter in `~/.emery/adapters/`, or fetch it with `wkg get <reference> -o ~/.emery/adapters/`"
            }
            "unavailable" => {
                "check the network and version, or place `<namespace>_<name>@<version>.wasm` in `~/.emery/adapters/`"
            }
            _ => return None,
        };
        Some(Cow::Borrowed(hint))
    }

    // The description says where the revision stands: a label behind its head,
    // a staged candidate or amendment, or nothing sealed; `amend` is told the
    // way back to a sealed revision ends in re-running it.
    fn not_generated_hint(self, description: &str) -> &'static str {
        match self {
            _ if description.contains("does not stand over its head") => {
                "merge or rebase `emery/<revision>` onto `HEAD`, or run `emery specify --apply`"
            }
            Self::Amend if description.contains("candidate `") => {
                "run `emery apply` to seal the staged candidate, then `emery amend`"
            }
            _ if description.contains("candidate `") => {
                "run `emery apply` to seal the staged candidate, or `emery show <artifact> --staged`"
            }
            _ if description.contains("amendment `") => {
                "inspect the staged amendment with `emery show amendment`"
            }
            Self::Amend => "run `emery specify --apply <adapter>...` first, then `emery amend`",
            _ => "run `emery specify --apply <adapter>...` to seal a revision first",
        }
    }

    // A run naming its sources on the command line reads no file, so `specify`
    // and `amend` are told to pass `--config` rather than to add a table they
    // may already hold.
    const fn target_hint(self) -> &'static str {
        match self {
            Self::Build => {
                "pass `<adapter>` to `emery build`, or add a `[target]` table to `emery.toml`"
            }
            Self::Specify | Self::Amend => {
                "pass `--config` to read `[target]` from `emery.toml`, or add a `[target]` table"
            }
            Self::Apply | Self::Show | Self::Completions => {
                "add a `[target]` table to `emery.toml`, or pass `--config <path>`"
            }
        }
    }
}

impl From<&Verb> for Invoked {
    fn from(verb: &Verb) -> Self {
        match verb {
            Verb::Specify(_) => Self::Specify,
            Verb::Apply(_) => Self::Apply,
            Verb::Amend(_) => Self::Amend,
            Verb::Build(_) => Self::Build,
            Verb::Show(_) => Self::Show,
            Verb::Completions { .. } => Self::Completions,
        }
    }
}
