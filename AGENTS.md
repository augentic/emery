# Emery — Agent Instructions

A Rust workspace producing the `emery` binary. `emery specify <adapter>... [--description <adapter>=<text>] [--config [<path>]]` extracts typed claims from source adapters, derives requirements under authority precedence, synthesises a specification and a design, slices the specification into a build plan, and commits the three as one content-addressed revision; `emery show <spec|design|plan>` prints a Markdown projection of the current revision; `emery completions <shell>` is derived from the clap surface. `plugins/emery/` is the Cursor plugin carrying the `/emery:specify` skill wrapper. First-party adapters live in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters). The v1 implementation is archived at git tag `v1`.

<!-- conventions:begin agents/git -->
## Git

Never `git commit`, `git push`, open or close a pull request, or delete a branch — in this repository or in any sibling checkout — unless the maintainer lifts this for the session, explicitly and for named work. Leave every change uncommitted in the working tree; the maintainer reviews and commits. No plan or to-do list carries a commit, push, or PR step, and an instruction to complete every step does not override this.
<!-- conventions:end agents/git -->

## Vocabulary

- **source adapter** — a WebAssembly component exporting the WIT `source-adapter` world (`metadata` + `extract`): given a `SourceInput` (a name and a workspace or inline value) it returns an `Evidence` document of typed claims. See [wit/emery.wit](wit/emery.wit).
- **engine** — this product: the engine guest and the crates behind it.
- **capability** — an engine-side trait a provider carries: `Model`, `Source`, `Plugins`, `StateStore`, `BlobStore`.
- **contract** — a typed agreement: WIT, CLI grammar, JSON envelope.
- **plugin** — the adapter noun in omnia's loader vocabulary. Not to be confused with the Cursor plugin under `plugins/emery/`, which the `emery` CLI never sees.

When authoritative inputs are incomplete, preserve the gap as `[unknown]` rather than guessing.

## Map

| Path | Role |
| --- | --- |
| `crates/prose` | Embedded prompt corpora: `prose!["../prose/extract.md", ..]` embeds the listed documents as a `&'static [Doc]` — each named from the invoking file the way `include_str!` names one, embedded as it embeds one, and tabled by what follows its `prose/` segment (`extract.md`, `references/ids.md`); `find` / `body` look a document up by that path; `check(docs, tree, prompts, imports)` is the native test that holds a list to its tree and to the prompts code puts to the model — every document listed once, every relative link a document in the table or an import, every prompt in the table, every other document reached from a prompt through those links, no listed document shadowing an import |
| `crates/adapter` | The `emery:adapter` WIT contract, both sides, one module per axis (`emery_adapter::source`): the `Source` capability, `SourceInput`, `Evidence` / `Claim`, and the claim gate `Evidence::findings` |
| `crates/sdk` | The guest-only adapter SDK, types and functions in omnia's helper shape: `extract` (one gated model turn per seam, joined), `workspace::list` (physical workspace traversal under an adapter's `keep` over each offered `Entry`), `metadata` (the `metadata` answer for a kind of source), and `source_adapter!(metadata, extract)` — the one macro, in omnia's `command!` shape, that exports the `source-adapter` world over an adapter's two plain fns on `wasm32`, lifting the WIT input and the host's model onto the `Context` and lowering the outcome; the `export` module beneath it stays public for a guest written by hand; `RUNTIME` is the runtime references every adapter shares (`crates/sdk/prose/`: `claims.md`, `reconciliation.md`), which a prompt at its own `prose/` root links as `claims.md` and never lists — `read_doc` answers them after the adapter's own table, and `check(docs, tree, prompts, RUNTIME)` — the prose crate's `check`, re-exported — holds an adapter's list to its tree with them as the imports; it re-exports the contract types (`Anchor` / `BadAnchor` for the `path` grammar and `is_kebab` for stems among them), the embedded prose (`Doc`, `prose!`, the `body` / `find` lookups), `serde_json`, and `tracing`, so an adapter's `[dependencies]` is `emery-sdk` and nothing more than the pure-Rust parser its source demands. A survey is a plain fn over the input where the source alone decides the cut; where its surfaces are the model's to name, `survey::surfaces(ctx, docs, facts, check)` is the one turn it puts: the adapter's code renders what it read of the workspace as `survey::Facts` (every production module, the `text` of what locates the surfaces, the `files` to lay whole), the SDK asks under the adapter's `survey.md` — its system prompt, a system document `list_docs` never lists beside `extract.md` and `claims.md` — for a `survey::Inventory` (each surface a `name`, an `anchor` in the claim `path` grammar, a `stem`; the `unreached` modules), holds it to the tree (an anchor names a listed module and lines its file holds, a stem is kebab-case, a name is unique, an `unreached` module is listed and no surface's entry) and to the adapter's `check`, returns every finding for correction, spelling every anchor and `unreached` path root-relative before the gate and the `check` read it and in what it hands back, so one inventory is what every round and the adapter see. What an adapter derives from the source itself — the closure each anchor reaches, the stems code can read there, the ids, the claims it copies from declarations — is code's, not the model's. No production crate depends on it |
| `crates/engine` | Transport-neutral `specify` / `show` operations over a capability `Provider`; the typed `Revision` (`Spec`, `Design`, `Plan`), its Markdown projection, and the revision store. No clap, toml, terminal text, or exit codes |
| `crates/cli` | The clap grammar, the source carriers (argv, `--description`, `--config` / project-root `emery.toml`), the text render fns, and the hint table. `run(provider, argv)` drives omnia's command façade |
| `src/` | `lib.rs`: the wasm32 engine guest. `main.rs`: the shipped runtime — one `omnia::runtime!` invocation and nothing hand-written (on wasm32 the bin is an empty `main`, so the workspace-wide wasm32 clippy pass can include it): the compiled-in policy is the invocation directory mounted read-only as `.` (the guest's data view and the one root a local adapter loads through), the engine embedded as the guest `emery` from `EMERY_GUEST`, revision state in `.omnia/storage`, and Cursor answering the model; it declares no adapter and no registry routing — where a package is fetched from is the project's `emery.toml` `[registries]` table (`emery` is `augentic.io` unless a line re-routes it), named by the engine on the load. One tracing level governs the host and every guest — `info` bare, one step up per `-v` and down per `-q`, a process `RUST_LOG` refining a bare run — read by omnia's direct entry from the flags the grammar declares through `omnia_sdk::api::command::Verbosity` |
| `examples/` | `adapter/` is the one mock source adapter; `emery.toml` names its built component for the shipped binary to load by path |
| `tests/` | Root scenario suites (`specify.rs`, `command.rs`, `plugin.rs`) over `tests/support/` |
| `wit/`, `docs/`, `plugins/emery/` | The WIT package; the CLI reference (mdBook, deployed to emery.augentic.io) and the release runbook; the Cursor plugin |

Dependency direction, leaf to root: `prose`, `adapter` → `sdk`; `prose`, `adapter` → `engine` → `cli` → root. Never `engine → cli`; nothing in production depends on `sdk`.

## Invariants

- Failures are `omnia_sdk::Error`, built with `bad_request!` and siblings. Explicit variants only for the recovery codes `specify-source-required`, `unsupported-version`, `spec-not-generated`, `spec-outdated`. `anyhow::Context` carries the unclassified tail to `server_error`. Production code never panics — a guest panic is a trap with no envelope. Exit codes are omnia's 1:1 map (`Error::exit_code`: `bad_request` 1, `not_found` 2, `server_error` 3, `bad_gateway` 4; clap usage errors exit 64 with no envelope).
- The model never authors a document. Synthesis is typed `Question`s put as briefs (`crates/engine/src/specify/{basis,spec,design,plan}.rs`) and verified against engine facts; the stored revision is the truth and Markdown is a `Display` projection. The specification is drafted per stem group: `SpecBrief::chunked` cuts the bases into chunks of at most `SPEC_CHUNK` (25) requirements — a stem past the cap split, smaller stems merged up to it — one `spec-draft` turn each beside the design and slicing turns, the first chunk alone carrying the preamble, and `SpecBrief::assemble` orders the one `Spec` by `ReqId`. The plan's floor is the stem — the first segment of a requirement subject's dotted id: requirements sharing one are one slice at the least, the model may merge stems and never splits one, each design type key is owned by exactly one slice, and a specification under one stem is one slice with no turn spent. Nothing parses Markdown back, and nothing of a stored revision reaches a run. A revision-shape change re-ids every revision and changes the JSON fixtures under `tests/specify/`; a projection change touches only the `.md` fixtures; a prompt change (`crates/engine/prose/`) touches none.
- Every adapter reference goes through the `omnia:plugins/loader` capability at the location it names (`omnia_sdk::plugins::Location`), under the guest name that location registers (`Location::name`), and the deployment's grant bounds every load: a project-relative `.wasm` path loads through the read-only `.` mount, read fresh on every run, as the guest its file's stem names; an exact package reference loads from the registry the project's `emery.toml` `[registries]` table routes its namespace to (`emery` is `augentic.io` unless a line re-routes it; an unrouted namespace is `bad_request` before any load), which the engine names on the load, as the guest its reference without the version names (`emery:intent@1.0.0` is `emery:intent`); a bare name is a guest the deployment declares at boot — the loader attests it or refuses it typed, so an undeclared name never reaches a dispatch trap. A `[[source]] digest` rides the load as its pin and the loader holds the resolved bytes to it; a digest on a bare name is `bad_request`. Two references that name one guest (two components sharing a file stem, two versions of one package) are refused before any load, as is one that names the engine's own guest (`emery_engine::ENGINE`, `emery`: the bare name or a component of that stem), which the loader would attest in the adapter's place. GitHub URLs are refused. The only pre-compiled artifact is the engine compiled into the binary; adapters are always raw wasm — the loader admits a path or a package as raw wasm alone, so a pre-compiled artifact under an adapter's path is refused typed however it hashes, since the path, its pin, and its registry routing come from the same project the run is asked to trust.
- Deleted verbs and flags are deleted from the grammar, not aliased. Pre-1.0, a major bump means regenerating with a fresh `emery specify`.
- Engine state is written only through the storage capabilities. Never hand-edit `.omnia/storage`. `spec.md` / `design.md` / `plan.md` beside code are `emery show` output, never sources.
- When you remove a symbol: `rg <Symbol> -- AGENTS.md README.md docs/` and fix every hit in the same PR.

<!-- conventions:begin agents/code-style -->
## Code style

clippy (`make lint`) and nightly rustfmt (`make fmt`) are the style gate; beyond them and the rules below, match the surrounding code.

- Suppress a lint with `#[expect(lint, reason = "…")]` at the smallest scope, never `#[allow]`.
- `<module>.rs` plus `<module>/<child>.rs`; `mod.rs` only under `tests/support/`.
- A fn over a type is that type's method, not a free fn taking it as its first argument, where the type's module declares the fn or the fn is a plain lookup or predicate on the type. A constructor is an associated fn. A policy `const` sits beside the type whose method reads it. Values several fns thread through every call become one struct whose methods they are. A fn stays free when it is pure over primitives and iterators, or when it is one module's rule applied to another module's type.
<!-- conventions:end agents/code-style -->

<!-- conventions:begin agents/comments -->
Comments follow the conventions `std`, `serde`, and `tokio` converge on: docs state the observable contract for the crate's user, never the body's mechanics.

- `///` goes on the public API only — the `pub` types, fns, fields, variants, and re-exports a user of the crate can reach — never on a private or `pub(crate)` item, an `impl` block, or a trait-impl method. A clap field's `///` is its `--help` text. A doc opens with one summary sentence (about fifteen words, full stop), then a blank line, then short sentences and bullet lists. `# Examples` holds compiled doctests, for non-obvious usage only; `# Errors` names each class the caller matches on, linked; `# Panics` the rest. Every item mentioned is an intra-doc link. No mechanics, history, or migration notes. A `//!` says what a module is for, in the same shape.
- A private item takes a `//` only for what a senior developer would not see from its name and signature: a constraint, a why, an invariant. Most carry nothing. No restatements, match-arm labels, or body paraphrases.
- Inside a body, a `//` is a section header: lowercase, no full stop, above a blank-line-separated block, naming what the block achieves, so the headers read together outline the fn. A fn readable at a glance carries none, and a header never narrates the line beneath it. The one in-body explanation is `// HACK: …`, for a trick a senior would not see through.
- A test fn takes `//`, never `///`, and only for rationale its scenario name and assertions do not expose.
- No commented-out code.
- Every sentence earns its place and reads once: short plain sentences, one idea each; three or more things are a bullet list, not a colon-and-dash clause; no chained em-dashes, nested parentheticals, or semicolon runs; each fact has one home across `//!`, `///`, and `//`. A comment is as long as its why takes and no longer — concise is not dense, and readable is not verbose.
<!-- conventions:end agents/comments -->

<!-- conventions:begin agents/testing -->
## Testing

Tests drive the public boundary: a behaviour is asserted through what a user of the product or crate can reach, over scripted doubles rather than a live filesystem, network, or model, never through private internals. A suite below the root survives only for an independent library contract; a unit test only for a branch no public boundary reaches. A test fn names the scenario (`gen_spec`, `no_sources`), never the outcome. Scripted doubles are strict: script exactly the exchanges a run consumes.
<!-- conventions:end agents/testing -->

Root-led: every CLI-reachable behaviour lives in `tests/` and drives `emery_cli::run` in-process over scripted capabilities (`tests/support/`), asserting the exit code, the envelope, and scripted storage — never the filesystem. Crate suites survive only for independent library contracts (`sdk`, `prose`); unit tests only for CLI-unreachable branches.

<!-- conventions:begin agents/commands -->
## Commands

All from the repository root through `make` ([`Makefile`](Makefile) → mise). The tasks are the shared `mise/rust.toml` of [`augentic/toolkit`](https://github.com/augentic/toolkit), pinned in [`mise.toml`](mise.toml) to the tag every `uses:` under `.github/workflows/` names; a bump is one pull request over both, and `make conventions-check` holds them together.

```bash
make ci # exactly the CI jobs: fmt-check + lint + test + test-docs + docs + vet + deny + conventions-check — run before handing over
make check # local advisories: audit + fmt (rewrites) + lint + outdated + deps
make test # cargo nextest run --locked --workspace --all-features, under -Dwarnings
make lint # lint-host (cargo clippy --workspace --all-targets --all-features, then cargo hack --each-feature), then lint-wasm (the same over every lib, bin and example for wasm32-wasip2 — never tests)
make fmt # cargo +nightly fmt --all
make vet-regen # regenerate cargo-vet imports/exemptions/unpublished, then vet
make conventions-sync # write the shared conventions at the pinned toolkit tag
make cov # cargo llvm-cov nextest --workspace --all-features --summary-only
make sweep # drop target/ artifacts untouched for a week
```

A file with a `Managed by augentic/toolkit` header, and everything between a `conventions:begin` and `conventions:end` marker pair, is written by `make conventions-sync`: never edit it here. Change it in [`augentic/toolkit`](https://github.com/augentic/toolkit) instead. If `make ci` cannot run, say exactly why and which checks ran instead.
<!-- conventions:end agents/commands -->

`mdbook build docs` builds the CLI reference and checks its links. The live journey (`cargo run -- specify --config examples/emery.toml`) is in [examples/README.md](examples/README.md). The skill preview is `cursor-agent --plugin-dir plugins/emery` over a `cargo install --path . --locked` binary; reinstall when the CLI changes.
