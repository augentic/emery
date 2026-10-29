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
use emery_engine::amend::{AmendInput, amend};
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
const AMEND_DESC: &str = "Stage an amendment against the current revision.\n\n\
    Name one or more adapters, use `--description <adapter>=<text>` for inline input, or \
    use `--config <path>` for the `[[source]]` tables of a change file, read and layered \
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
const SHOW_DESC: &str = "Print an artifact from the current revision.\n\n\
    The revision is read from the repository the `[target]` table of `--config [<path>]` \
    (default: `emery.toml`) names, where `emery apply` sealed it. `--staged` prints it \
    from the candidate `emery specify` staged and `emery apply` has not sealed, and \
    needs no target. `amendment` prints the staged amendment, which stands staged alone \
    until it is sealed, and needs neither. Text output contains only the artifact body. \
    `--format json` also includes the revision id and the typed document.";
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
        Verb::Apply(arguments) => {
            command.call(apply, || ApplyInput::try_from(arguments), text::apply).await
        }
        Verb::Amend(arguments) => {
            command.call(amend, || AmendInput::try_from(arguments), text::amend).await
        }
        Verb::Build(arguments) => {
            command.call(build, || BuildInput::try_from(arguments), text::build).await
        }
        Verb::Show(arguments) => {
            command.call(show, || ShowInput::try_from(arguments), text::show).await
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
        let SpecifyArgs {
            adapters,
            descriptions,
            config,
            apply,
            fresh,
        } = args;
        let mut input: Self = config::SourceCarriers {
            adapters: &adapters,
            descriptions: &descriptions,
            config: config.as_deref(),
        }
        .try_into()?;
        input.apply = apply;
        input.fresh = fresh;
        Ok(input)
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
        let ApplyArgs { repository, config } = args;
        let target = config::target(config.as_deref(), repository.as_deref())?
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
        let AmendArgs {
            adapters,
            descriptions,
            config,
            apply,
        } = args;
        let SpecifyInput { sources, target, .. } = config::SourceCarriers {
            adapters: &adapters,
            descriptions: &descriptions,
            config: config.as_deref(),
        }
        .try_into()?;

        // the target is the change file's, else the project's
        let target = match (target, &config) {
            (Some(target), _) => Some(target),
            (None, Some(_)) => config::target(None, None)?,
            (None, None) => None,
        };
        Ok(Self {
            sources,
            target,
            apply,
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
    /// Reviewable artifact of the current revision, or the staged amendment.
    #[arg(value_parser = artifacts())]
    artifact: Artifact,
    /// Read the staged candidate in place of the current revision; the amendment is staged or nothing.
    #[arg(long)]
    staged: bool,
    /// Operator-owned config naming the repository the revision lives in; the omitted value selects emery.toml.
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = config::CONFIG_FILE)]
    config: Option<PathBuf>,
}

impl TryFrom<ShowArgs> for ShowInput {
    type Error = Error;

    fn try_from(args: ShowArgs) -> Result<Self, Error> {
        let ShowArgs {
            artifact,
            staged,
            config,
        } = args;
        let target = if staged || matches!(artifact, Artifact::Amendment) {
            None
        } else {
            config::target(config.as_deref(), None)?
        };
        Ok(Self {
            artifact,
            staged,
            target,
        })
    }
}

fn artifacts() -> impl TypedValueParser<Value = Artifact> {
    PossibleValuesParser::new(Artifact::VARIANTS.iter().map(|artifact| {
        let help = match artifact {
            Artifact::Spec => "The behavioural specification artifact.",
            Artifact::Design => "The rebuild design artifact.",
            Artifact::Plan => "The build plan artifact.",
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
        "specify-source-required" if invoked == Invoked::Amend => {
            "pass one or more adapters to `emery amend`, or name a change file with `--config <path>`"
        }
        "specify-source-required" => {
            "pass one or more adapters to `emery specify`, or add an `emery.toml` at the project root"
        }
        "build-target-required" => target_hint(invoked),
        "spec-not-generated" => not_generated_hint(&error.description(), invoked),
        "spec-mismatch" if error.description().contains("edited or merged by hand") => {
            "`spec/` at the head holds documents no seal wrote, as a merge of two branches that each sealed a revision leaves them: keep one side's three files whole (`git checkout <branch> -- spec/`), commit, and run the other side's amendment again over it; `emery show` prints the specification, never edit the files"
        }
        "spec-mismatch" => {
            "`spec/` in the target repository is the engine's: restore the labelled commit's documents (`git checkout emery/<revision> -- spec/`), or move the directory aside, and re-run; `emery show` prints the specification, never edit the files"
        }
        "nothing-staged" if error.description().contains("amendment") => {
            "run `emery amend --config <path>` to stage an amendment against the current revision, then re-run `emery show amendment`"
        }
        "nothing-staged" => {
            "run `emery specify <adapter>...` to stage a candidate, then re-run `emery apply` or `emery show <artifact> --staged`"
        }
        "amendment-stale" => {
            "the current revision moved since the amendment was prepared, so it no longer applies: re-run `emery amend <adapter>...` against the current revision to prepare it again, then `emery apply`; nothing was sealed and the stale amendment stays staged until then"
        }
        "lineage-exists" if invoked == Invoked::Specify => {
            "the current revision amends an earlier one, so a specify would start the history over: stage a change against it with `emery amend <adapter>...`, or start over deliberately with `emery specify --fresh <adapter>...`, which seals a parentless revision over the head and leaves every earlier label standing; nothing was loaded"
        }
        "lineage-exists" => {
            "the staged candidate was drafted from the sources alone and the current revision amends an earlier one: re-run `emery specify --fresh <adapter>...` to stage it as a fresh start, then `emery apply`, or stage a change against the current revision with `emery amend <adapter>...` instead; nothing was sealed and the candidate stays staged"
        }
        "amendment-pending" if invoked == Invoked::Specify => {
            "an amendment is staged where the candidate would go: review it with `emery show amendment`, or re-run `emery amend` to prepare it again; a specify never displaces it"
        }
        "amendment-pending" if invoked == Invoked::Amend => {
            "a revision candidate is staged where the amendment would go: seal it with `emery apply`, or review it with `emery show <artifact> --staged`; an amend never displaces it"
        }
        "amendment-pending" => "review the staged amendment with `emery show amendment`",
        "retire-unmatched" => {
            "the change withdraws a behaviour the current revision does not state: compare the named retire claim's id against the requirements `emery show spec` prints, and reword the source to withdraw one of them, or drop the withdrawal; nothing was staged"
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
        "spec-rewritten" => {
            "`spec/` in the target repository holds the sealed revision and no build writes there: inspect the named paths in `.emery/vcs/worktrees/<slice>` or `.emery/vcs/integration`, stop what wrote them (the adapter's layout, or a command the model ran through its shell), and re-run `emery build`; nothing of the change set was sealed and the label stands at the last verified wave"
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

// The description says where the revision stands: a label behind its head, a
// staged candidate or amendment, or nothing sealed; `amend` is told the way
// back to a sealed revision ends in re-running it.
fn not_generated_hint(description: &str, invoked: Invoked) -> &'static str {
    match invoked {
        _ if description.contains("does not stand over its head") => {
            "the repository's head moved on without the label: merge or rebase `emery/<revision>` onto the head, or re-run `emery specify --apply` to seal a fresh revision over it"
        }
        Invoked::Amend if description.contains("candidate `") => {
            "an amendment is prepared against a sealed revision: run `emery apply` to seal the staged candidate, then re-run `emery amend`"
        }
        _ if description.contains("candidate `") => {
            "run `emery apply` to seal the staged candidate as the current revision, then re-run show or build; `emery show <artifact> --staged` reviews it first"
        }
        _ if description.contains("amendment `") => {
            "review the staged amendment with `emery show amendment`"
        }
        Invoked::Amend => {
            "an amendment is prepared against a sealed revision: run `emery specify <adapter>... --apply` to seal one, then re-run `emery amend`"
        }
        _ => {
            "run `emery specify <adapter>...` to stage a candidate and `emery apply` to seal it (or `emery specify --apply` for both), then re-run show or build"
        }
    }
}

// A run naming its sources on the command line reads no file, so `specify` and
// `amend` are told to pass `--config` rather than to add a table they may hold.
const fn target_hint(invoked: Invoked) -> &'static str {
    match invoked {
        Invoked::Build => {
            "pass a target adapter to `emery build`, or add a `[target]` table to `emery.toml` at the project root"
        }
        Invoked::Specify | Invoked::Amend => {
            "a run naming its sources on the command line reads no file: pass `--config` to read the `[target]` table of `emery.toml` at the project root, or `--config <path>` for another file; its `repository` is where the specification is sealed, a URL, `.` for the project checkout, or none for a repository kept under `.emery/vcs/repos/<name>`"
        }
        Invoked::Apply | Invoked::Show | Invoked::Completions => {
            "add a `[target]` table to `emery.toml` at the project root, or name one with `--config <path>`: its `repository` is where the specification is sealed, a URL, `.` for the project checkout, or none for a repository kept under `.emery/vcs/repos/<name>`"
        }
    }
}
