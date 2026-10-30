# `emery-sdk`

`emery-sdk` is the Rust guest SDK for building WebAssembly (Wasm) source and target adapters for [Emery](https://github.com/augentic/emery). It provides the adapter traits, model turn runners, workspace traversal helpers, and WebAssembly Interface Types (WIT) component exports so an adapter crate needs no `wit-bindgen` setup or `build.rs`.

## Contents

- [Source adapters (`SourceAdapter`)](#source-adapters-sourceadapter)
  - [The survey pipeline](#the-survey-pipeline)
- [Target adapters (`target::TargetAdapter`)](#target-adapters-targettargetadapter)
  - [Tasks and outcomes](#tasks-and-outcomes)
  - [Scaffolding and guarded markers (`Scaffold`)](#scaffolding-and-guarded-markers-scaffold)
  - [Merge rules and worker grants (`Metadata` and `Grant`)](#merge-rules-and-worker-grants-metadata-and-grant)
- [Prompt corpus verification (`prose!` and `check`)](#prompt-corpus-verification-prose-and-check)
- [Links](#links)

---

## Source adapters (`SourceAdapter`)

A source adapter reads one kind of source—an operator brief, a documentation tree, or a codebase—and returns validated `Evidence` claims for Emery to synthesize into a specification.

Implement `SourceAdapter` on a unit struct, embed your prompt files with `prose!`, and export the WebAssembly component with `export_source!`:

```rust
mod survey;

use emery_sdk::{Context, Doc, Error, Evidence, Metadata, Model, SourceAdapter, SourceKind};

const KIND: SourceKind = SourceKind::Documentation;

pub struct Adapter;

impl SourceAdapter for Adapter {
    fn metadata() -> Metadata {
        Metadata { kind: KIND }
    }

    async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
        let seams = survey::survey(ctx.input)?;
        emery_sdk::extract(ctx, PROSE, &seams).await
    }
}

pub static PROSE: &[Doc] = emery_sdk::prose!["../prose/extract.md"];

#[cfg(target_arch = "wasm32")]
emery_sdk::export_source!(Adapter);
```

### The survey pipeline

Before calling `emery_sdk::extract`, an adapter runs a **survey** to divide its input into one or more extraction **seams** (`Seam`). Each `Seam` scopes one gated model turn under `extract.md`:

- **`text`:** Brief instructions or structural facts leading the turn.
- **`files`:** Workspace files to inline (up to `INLINE_BYTES`) or list, and the set of files where claim `path` anchors may land.
- **`stems`:** Allowed leading segments for `requirement`, `criterion`, and `retire` claim IDs.
- **`anchors`:** Allowed file and line spans (`<path>#L<start>-L<end>`) for `requirement` claims.
- **`lexicon`:** Technology names (`Lexicon`) that `requirement`, `criterion`, and `retire` claims are barred from naming directly.

Adapters choose between three survey patterns depending on their source format:

1. **Whole-input or file-list seams:** Small inputs or single-file sources return `Seam::whole()` or divide files discovered via `workspace::list(root, keep)` into `Seam::files(..)`.
2. **Model-assisted surface survey (`survey::surfaces`):** For multi-file documentation trees without a syntax parser, the adapter outlines the files and calls `survey::surfaces(ctx, docs, facts, check)` under `survey.md`. The model identifies each surface (`name`, `anchor`, `stem`) and `unreached` files, and the SDK validates every anchor and stem against the workspace before the adapter cuts the seams.
3. **Parsed code pipeline (`survey::seams`):** For programming languages, the adapter parses files into `survey::code::Module` records using `Parsed::read(root, dialect, listing, parse)` and settles a `survey::code::Tree` with a language-specific `Recogniser` and `Dialect`. Calling `survey::seams(ctx, docs, &tree)` runs the `survey.md` turn, derives routes, message triggers, call closures, and test scenarios from the syntax tree, cuts seams by stem, and extracts verbatim `type` and `contract` claims that `Survey::join` merges into the final `Evidence`.

Pure string and path helpers (`kebab`, `words`, `unique`, `push_unique`, and `survey::{Lines, resolve, route, tests}`) are provided so deterministic derivation stays in Rust code rather than model prompts.

---

## Target adapters (`target::TargetAdapter`)

A target adapter initializes a project's layout, builds individual plan slices into code, and verifies the integrated workspace after each build wave.

Implement `target::TargetAdapter` on a unit struct, embed `build.md` and `verify.md` in `PROSE`, and export the component with `export_target!`:

```rust
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write as _};
use std::path::Path;

use emery_sdk::target::{Grant, MergeRule, Metadata, Outcome, TargetAdapter, Task};
use emery_sdk::{Doc, Error, Model, server_error};

const RULES: &[MergeRule] = &[MergeRule::ours("Cargo.lock")];

const CARGO: Grant = Grant {
    egress: &["index.crates.io", "static.crates.io"],
    read: &["~/.rustup", "~/.cargo/bin"],
    env: &[("RUSTUP_HOME", "~/.rustup")],
};

pub struct Adapter;

impl TargetAdapter for Adapter {
    fn metadata() -> Metadata {
        Metadata {
            rules: RULES,
            grants: &[CARGO],
        }
    }

    async fn run<P: Model>(task: Task<'_, P>) -> Result<Outcome, Error> {
        match task {
            Task::Init(ctx) => {
                let manifest = Path::new(ctx.workspace).join("Cargo.toml");
                match OpenOptions::new().write(true).create_new(true).open(manifest) {
                    Ok(mut file) => file
                        .write_all(b"[workspace]\nmembers = [\"crates/*\"]\n")
                        .map_err(|err| server_error!("scaffolding `Cargo.toml`: {err}"))?,
                    Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
                    Err(err) => return Err(server_error!("scaffolding `Cargo.toml`: {err}")),
                }
                Ok(Outcome::Initialised)
            }
            Task::Build(ctx) => emery_sdk::target::build(&ctx, PROSE, |_| Ok(Vec::new()))
                .await
                .map(Outcome::Built),
            Task::Verify(ctx) => emery_sdk::target::verify(&ctx, PROSE, |_| Ok(Vec::new()))
                .await
                .map(Outcome::Verified),
        }
    }
}

pub static PROSE: &[Doc] = emery_sdk::prose!["../prose/build.md", "../prose/verify.md"];

#[cfg(target_arch = "wasm32")]
emery_sdk::export_target!(Adapter);
```

### Tasks and outcomes

`TargetAdapter::run` receives a `target::Task` and must return the matching `Outcome` variant:

- **`Task::Init(InitContext)` → `Outcome::Initialised`:** Runs once per repository before the first slice builds. The adapter scaffolds missing seed files deterministically in Rust (using `create_new` so existing files are preserved) with no model turn.
- **`Task::Build(BuildContext)` → `Outcome::Built(Report)`:** Runs one gated model turn per slice via `target::build(&ctx, PROSE, check)` under `build.md`. The model receives the slice's requirements, owned design types, and brief, and writes code using the `write_files` and `edit_files` tools. Both the SDK report gate and the adapter's `check` closure validate the candidate `Report` and workspace before the turn finishes.
- **`Task::Verify(VerifyContext)` → `Outcome::Verified(Verdict)`:** Runs one gated model turn per wave via `target::verify(&ctx, PROSE, check)` under `verify.md`. The model runs the project's build, lint, and test suite in the integrated workspace, repairs any failures in place, and returns a `Verdict`.

### Scaffolding and guarded markers (`Scaffold`)

Before calling `target::build`, an adapter can generate skeleton files from the typed `Slice` and attach them via `ctx.with_scaffold(Scaffold)`:

- **`brief` and `passages`:** Markdown instructions and reference snippets rendered into the build turn's prompt.
- **`files` (`&[Guarded]`):** Protects scaffolded files from accidental deletion or truncation. Each `Guarded` file specifies `keeps` (substrings that any whole-file `write_files` call must preserve) and `markers` (`Marker { key, block }` stubs that `edit_files` can replace by naming `key` alone in `find`).
- **`types`:** Design type keys already laid into the scaffolded files, allowing the SDK to replace their full definitions in the prompt with a short pointer.

### Merge rules and worker grants (`Metadata` and `Grant`)

`TargetAdapter::metadata()` returns static `target::Metadata`:

- **`rules` (`&[MergeRule]`):** Path-glob strategies (`MergeRule::union`, `MergeRule::ours`, `MergeRule::theirs`) that the engine applies when merging parallel slice branches into the integration worktree.
- **`grants` (`&[Grant]`):** Toolchain permissions attached to the isolated worker sandbox during `build` and `verify` turns:
  - `egress`: Allowed outbound hostnames (`"index.crates.io"`, `"*.example.com"`).
  - `read`: Read-only directories under the operator's home directory (`"~/.rustup"`, `"~/.nvm/versions/node"`).
  - `env`: Static environment variables (`("RUSTUP_HOME", "~/.rustup")`).

---

## Prompt corpus verification (`prose!` and `check`)

Adapters embed their Markdown prompts at compile time with `emery_sdk::prose!` and validate their link graph in native tests with `emery_sdk::check`:

- Source adapters pass `&["extract.md"]` (plus `"survey.md"` when using `survey::surfaces` or `survey::seams`) and `emery_sdk::RUNTIME` (which provides the shared `claims.md` and `reconciliation.md` references).
- Target adapters pass `&["build.md", "verify.md"]` and `&[]`.

---

## Links

- **API documentation:** [docs.rs/emery-sdk](https://docs.rs/emery-sdk) (built for `wasm32-wasip2` so the `export` modules are included)
- **Adapter authoring guide:** [`docs/how-to/author-an-adapter.md`](../../docs/how-to/author-an-adapter.md)
- **First-party adapters:** [augentic/emery-adapters](https://github.com/augentic/emery-adapters)

Licensed under MIT or Apache-2.0, at your option.
