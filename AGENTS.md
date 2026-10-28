# Emery — Agent Instructions

A Rust workspace producing the `emery` binary:

- `emery specify <adapter>...` extracts typed claims from source adapters, derives requirements under authority rank, drafts a specification, a design, and a build plan, and stages them as one content-addressed candidate. `--apply` also seals it.
- `emery apply` seals the staged candidate as the current revision: one commit in the target's repository holding `spec/`, labelled `emery/<id>`.
- `emery build [<adapter>]` builds the current plan in waves through one target adapter. Each slice is built in its own working copy and merged under the adapter's merge rules, with conflicts resolved by the model. Each wave is verified and labelled `emery/<revision>`, so a re-run resumes.
- `emery show <spec|design|plan|amendment> [--staged]` prints a Markdown projection of the current revision, the staged candidate, or the staged amendment.
- `emery amend <adapter>...` stages an `Amendment` against the current revision. It is hidden from `--help` and the completions while nothing seals what it stages.
- `emery completions <shell>` covers the shown verbs.

`plugins/emery/` is the Cursor plugin wrapping `specify`, `apply`, and `build` as skills. First-party adapters live in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters). The v1 implementation is archived at tag `v1`.

## Git

Never `git commit`, `git push`, open or close a pull request, or delete a branch — in this repository or in any sibling checkout — unless the maintainer lifts this for the session, explicitly and for named work. Leave every change uncommitted in the working tree; the maintainer reviews and commits. No plan or to-do list carries a commit, push, or PR step, and an instruction to complete every step does not override this.

## Vocabulary

- **source adapter** — a WebAssembly component exporting the WIT `source-adapter` world (`metadata` + `extract`): a `SourceInput` in, an `Evidence` document of typed claims out. See [wit/emery.wit](wit/emery.wit).
- **target adapter** — a component exporting the `target-adapter` world (`metadata` + `run`): given the writable tree and a task (`init`, `build(slice)`, or `verify`), it answers the outcome of that kind (`initialised`, `built(report)`, or `verified(verdict)`).
- **engine** — this product: the engine guest and the crates behind it.
- **capability** — an engine-side trait a provider carries: `Model`, `Source`, `Target`, `Plugins`, `StateStore`, `BlobStore`, `Vcs`, and `Files`, which the engine answers itself beneath a working copy and which follows no symbolic link.
- **contract** — a typed agreement: WIT, CLI grammar, JSON envelope.
- **plugin** — the adapter noun in omnia's loader vocabulary. Not the Cursor plugin under `plugins/emery/`.
- **staged candidate** — the revision `specify` stages in the engine's storage, awaiting `emery apply`. The slot holds one item: a candidate, or a **staged amendment** from `amend`. Neither displaces the other (`amendment-pending`).
- **slices staged in the integration copy** — merges an earlier build left in `.emery/vcs/integration`, which the next build resumes. A different store from the staged candidate.

When authoritative inputs are incomplete, preserve the gap as `[unknown]` rather than guessing.

## Map

| Path | Role |
| --- | --- |
| `crates/prose` | Embedded prompt corpora (`prose!`) and `check`, which holds a corpus to its tree and to the prompts code puts |
| `crates/adapter` | The `emery:adapter` WIT contract types for both axes (`source`, `target`): the claim, report, and verdict gates, the path rule `beneath`, the no-follow filesystem helpers, and the `Route` / `Trigger` grammar |
| `crates/lexicon` | `Lexicon`, the technology names specification text is held against, and the `words` splitter |
| `crates/sdk` | The guest-only adapter SDK: `SourceAdapter` / `export_source!`, `TargetAdapter` / `export_target!`, the gated extraction and survey turns, the code survey model, and the build and verify turns with their `write_files` / `edit_files` tools. It re-exports what an adapter needs, so an adapter depends on it and its parser alone. Nothing in production depends on it |
| `crates/engine` | Transport-neutral `specify` / `apply` / `amend` / `build` / `show` over a capability `Provider`; the typed `Revision` (`Spec`, `Design`, `Plan`) and `Amendment` with their Markdown projections; the revision store (`store.rs`), briefs (`brief.rs`), version control (`vcs.rs`), and the secret gate (`secrets.rs`). No clap, toml, terminal text, or exit codes |
| `crates/cli` | The clap grammar, the `emery.toml` carriers (`[[source]]`, `[target]`) and their layering with argv, the text renderers, and the hint table keyed by recovery code. `run(provider, argv)` drives omnia's command façade |
| `crates/precompile` | `emery-precompile`, which `build.rs` runs under `--release` to precompile the engine guest |
| `src/` | `lib.rs` is the wasm32 engine guest (empty natively). `main.rs` is the one runtime binary (`omnia::runtime!`) and its compiled-in policy: the mount `.`, the package store `~/.emery/adapters`, the registry route `emery = "augentic.io"`, and the model backend (Cursor, or `omnia-genai` under the never-released `genai` feature) |
| `build.rs` | Builds the wasm32 guest and the two mocks under `examples/`. A check-only invocation (clippy, rust-analyzer, or `EMERY_SKIP_GUEST` set) builds nothing |
| `examples/` | The mock source and target adapters and the `emery.toml` the live journey runs |
| `tests/` | Root scenario suites over `tests/support/`'s scripted capabilities, `runtime.rs` under the real runtime, and fixtures under `tests/{specify,build,amend}/` |
| `wit/` | The WIT package |
| `docs/` | The mdBook CLI reference deployed to emery.augentic.io: one page per verb under `docs/reference/cli/`, envelopes and recovery codes in `cli-output-shapes.md`. `release.md` is the release runbook |
| `plugins/emery/` | The Cursor plugin: `skills/{specify,apply,build}/SKILL.md` and `rules/emery.mdc`, which `tests/plugin.rs` holds to the clap surface. Its `plugin.json` version moves with the root `.cursor-plugin/marketplace.json`, a patch bump when only skill wording changes |

Dependency direction, leaf to root: `prose`, `adapter`, `lexicon` → `sdk`; `prose`, `adapter`, `lexicon` → `engine` → `cli` → root. Never `engine → cli`; nothing in production depends on `sdk`.

## Invariants

- **Errors.** Failures are `omnia_sdk::Error`, built with `bad_request!` and siblings. Explicit variants only for the recovery codes `specify-source-required`, `build-target-required`, `adapter-reference`, `unsupported-version`, `spec-not-generated`, `nothing-staged`, `amendment-pending`, `retire-unmatched`, `spec-outdated`, `spec-mismatch`, `repository-required`, `base-not-sealed`, `revision-not-found`, `merge-unresolved`, `verify-failed`, `secret-detected`, `spec-rewritten`. Each code's hint lives in `crates/cli`, and the codes a shown verb raises are documented in `docs/reference/cli-output-shapes.md`. `anyhow::Context` carries the unclassified tail to `server_error`. Production code never panics: a guest panic is a trap with no envelope. Exit codes are omnia's map (`bad_request` 1, `not_found` 2, `server_error` 3, `bad_gateway` 4; clap usage errors 64, with no envelope).
- **The model never authors a document.** Synthesis is typed `Question`s put as briefs (`brief.rs`) and verified against engine facts. The stored revision is the truth. Markdown is a `Display` projection, and nothing parses it back.
- **Code decides what code can.** A requirement's trigger (a route or a message) is derived by the survey at the registration and bound by the engine, never asked of the model. The `Trigger` variant is closed: a case is added only for a surface kind a recogniser reads at a registration and a target reads. Ids, stems, and plan placement where a rule decides are code's too.
- **Specification is behaviour by role; the design holds the technology.** One `Lexicon` gates every turn that authors specification text, each hit a finding to correct rather than a refusal. Technology reaches the design through `call` and `excerpt` claims from code and `decision` claims from prose.
- **Amendments keep what they don't touch.** Unchanged requirements and untouched design sections stay byte for byte the base's, and slices keep their ids, names, and briefs. A reworded base paragraph is the model's declared choice (named in `revised`), never a silent loss.
- **Plan floors.** Requirements sharing a stem are one slice at least, and a stem is never split; past `SLICE_CAP` the floor is the sub-stem. One trigger is one slice. Each data type key is owned by exactly one slice, and a mechanism type by none.
- **Revision shape.** A revision-shape change moves the grammar stamp (`revision::EMERY`), re-ids every revision, and changes the JSON fixtures under `tests/specify/` and `tests/build/`. A projection change touches only the `.md` fixtures. A prompt change (`crates/engine/prose/`) touches none.
- **W^X.** What a run can write it never loads code from. The one mount is the invocation directory `.`; the package store `~/.emery/adapters` is no mount. No guest holds a host path, a credential, or a git process. The model's worker is confined to its lent tree plus the target adapter's `metadata().grants`, and nothing in the environment widens it.
- **Adapter references** are exact `namespace:name@version` and nothing else (`adapter-reference`). A load goes through the `omnia:plugins/loader` capability: the store first, else the registry the deployment routes the namespace to, held to any configured `digest`. A project file chooses the package and the bytes, never the registry. Adapters are raw wasm only, and a handle is held to its axis before dispatch.
- **Build** lands as commits in one repository (the project's, a clone, or one `apply` created), never in the checkout's working tree. The engine never initialises the project directory and never pushes. The store's seal is the one writer of `spec/`: a build that changes it is `spec-rewritten`, and every seal point passes the secret gate (`secret-detected`). A build reads the current revision from the repository, never the staged candidate, and resumes from its label and the integration copy.
- **State** is written only through the storage and version-control capabilities. Never hand-edit `.emery/storage` or `spec/`. `spec.md`, `design.md`, and `plan.md` beside code are `emery show` output, never sources.
- Deleted verbs and flags are deleted from the grammar, not aliased. Pre-1.0, a major bump means regenerating with a fresh `emery specify`.
- When you remove a symbol: `rg <Symbol> -- AGENTS.md README.md docs/` and fix every hit in the same change.

## Code style

clippy (`make lint`) and nightly rustfmt (`make fmt`) are the style gate; beyond them and the rules below, match the surrounding code.

- Suppress a lint with `#[expect(lint, reason = "…")]` at the smallest scope, never `#[allow]`. The one exception is a `#![allow]` with its `reason` around generated bindings.
- `<module>.rs` plus `<module>/<child>.rs`; `mod.rs` only under `tests/support/`.
- Visibility is an item's reach and nothing wider. An item is private unless another module reads it, and then `pub`, which a private module bounds. `pub(crate)` only where `pub` would export the item from the crate; `clippy::redundant_pub_crate` refuses it elsewhere. Never `pub(super)` or `pub(in …)`.
- A fn over a type is that type's method where the type's module declares it, or where it is a plain lookup or predicate on the type. A constructor is an associated fn. A policy `const` sits beside the type whose method reads it. Values several fns thread through every call become one struct. A fn stays free when it is pure over primitives and iterators, or when it is one module's rule applied to another module's type.

Comments follow the conventions `std`, `serde`, and `tokio` converge on: docs state the observable contract, never the body's mechanics.

- `///` goes on the public API only, the items a user of the crate can reach, never on an `impl` block or a trait-impl method. A clap field's `///` is its `--help` text. A doc opens with one summary sentence of about fifteen words, then a blank line, then short sentences and lists. `# Examples` holds compiled doctests for non-obvious usage only; `# Errors` names each class the caller matches on, linked; `# Panics` the rest. Every item mentioned is an intra-doc link. No mechanics, history, or migration notes. A `//!` says what a module is for, in the same shape.
- A private item takes a `//` only for what a senior developer would not see from its name and signature: a constraint, a why, an invariant.
- Inside a body, a `//` is a lowercase section header above a blank-line-separated block, naming what the block achieves. The one in-body explanation is `// HACK: …`.
- A test fn takes `//`, never `///`, and only for rationale its name and assertions do not show.
- No commented-out code.
- Short plain sentences, one idea each. Three or more things are a list. Each fact has one home across `//!`, `///`, and `//`.

## Testing

- Tests drive the public boundary over strict scripted doubles, scripting exactly the exchanges a run consumes. Never private internals, the operator's filesystem, a network, or a live model. A test fn names the scenario (`gen_spec`, `no_sources`), never the outcome.
- Every CLI-reachable behaviour lives in root `tests/`, driving `emery_cli::run` in-process over `tests/support/` and asserting the exit code, the envelope, scripted storage, and the recorded version-control calls.
- The adapter boundary (store, fetch, pin, loader refusals, real components) and the repository boundary (commits, labels, clones, dirty checkouts) are asserted in `tests/runtime.rs` under the real runtime, with a hermetic git as the oracle.
- Crate suites survive only for independent library contracts (`sdk`, `prose`); unit tests only for CLI-unreachable branches.

## Commands

All from the repository root through `make` ([`Makefile`](Makefile) → mise). The tasks come from [`augentic/toolkit`](https://github.com/augentic/toolkit), pinned in [`mise.toml`](mise.toml) to the tag `.github/workflows/` uses; bump both together.

```bash
make ci # exactly the CI jobs — run before handing over
make check # local advisories: audit, fmt, lint, outdated, deps
make test # cargo nextest run --locked --workspace --all-features
make lint # clippy natively and per feature, then the wasm32-wasip2 guest pass
make fmt # cargo +nightly fmt --all
make vet-regen # regenerate cargo-vet inputs, then vet
make cov # coverage summary
make sweep # drop target/ artifacts untouched for a week
```

If `make ci` cannot run, say exactly why and which checks ran instead. `mdbook build docs` builds the CLI reference and checks its links. The live journey over the mock adapters is in [examples/README.md](examples/README.md). Preview the skills with `cursor-agent --plugin-dir plugins/emery` over a `cargo install --path . --locked` binary.
