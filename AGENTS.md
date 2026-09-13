# Emery — Agent Instructions

A Rust workspace producing the `emery` binary. `emery specify <adapter>... [--description <adapter>=<text>] [--config [<path>]]` extracts typed claims from source adapters, derives requirements under authority precedence, synthesises a specification and a design, and commits the pair as one content-addressed revision; `emery show <spec|design>` prints a Markdown projection of the current revision; `emery completions <shell>` is derived from the clap surface. `plugins/emery/` is the Cursor plugin carrying the `/emery:specify` skill wrapper. First-party adapters live in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters). The v1 implementation is archived at git tag `v1`.

## Vocabulary

- **source adapter** — a WebAssembly component exporting the WIT `source-adapter` world (`metadata` + `extract`): given a `SourceInput` (a key and a workspace or inline value) it returns an `Evidence` document of typed claims. See [wit/emery.wit](wit/emery.wit).
- **engine** — this product: the engine guest and the crates behind it.
- **capability** — an engine-side trait a provider carries: `Model`, `Source`, `Plugins`, `StateStore`, `BlobStore`.
- **contract** — a typed agreement: WIT, CLI grammar, JSON envelope.
- **plugin** — the adapter noun in omnia's loader vocabulary. Not to be confused with the Cursor plugin under `plugins/emery/`, which the `emery` CLI never sees.

When authoritative inputs are incomplete, preserve the gap as `[unknown]` rather than guessing.

## Map

| Path | Role |
| --- | --- |
| `crates/prose` | Embedded prompt corpora: the `Doc` registry. Feature `emit` (build-dependencies only) is the embed-time walker and link check |
| `crates/adapter` | The `emery:adapter` WIT contract, both sides, one module per axis (`emery_adapter::source`): the `Source` capability, `SourceInput`, `Evidence` / `Claim`, and the claim gate `Evidence::findings` |
| `crates/sdk` | The guest-only adapter SDK: `SourceAdapter` and the `source!` export macro. No production crate depends on it |
| `crates/engine` | Transport-neutral `specify` / `show` operations over a capability `Provider`; the typed `Revision`, its Markdown projection, and the revision store. No clap, toml, terminal text, or exit codes |
| `crates/cli` | The clap grammar, the source carriers (argv, `--description`, `--config` / project-root `emery.toml`), the text render fns, and the hint table. `run(provider, argv)` drives omnia's command façade |
| `src/` | `lib.rs`: the wasm32 engine guest. `main.rs`: the shipped runtime — one `omnia::runtime!` block; the invocation directory mounts read-only as `.`, revision state lives in `.omnia/storage`, Cursor answers the model, adapters load from local `.wasm` paths or the `omnia.host` registry |
| `examples/` | `adapter/` is the one mock source adapter; `runtime.rs` is a path-only journey host; `emery.toml` binds them |
| `tests/` | Root scenario suites (`specify.rs`, `command.rs`, `plugin.rs`) over `tests/support/` |
| `wit/`, `docs/`, `plugins/emery/` | The WIT package; the Developer Guide (mdBook; house standards under `docs/standards/`); the Cursor plugin |

Dependency direction, leaf to root: `prose`, `adapter` → `sdk`; `prose`, `adapter` → `engine` → `cli` → root. Never `engine → cli`; nothing in production depends on `sdk`. Details: [docs/standards/architecture.md](docs/standards/architecture.md).

## Invariants

- Failures are `omnia_guest::Error`, built with `bad_request!` and siblings. Explicit variants only for the recovery codes `specify-source-required`, `unsupported-version`, `spec-not-generated`, `spec-outdated`. `anyhow::Context` carries the unclassified tail to `server_error`. Production code never panics — a guest panic is a trap with no envelope. Exit codes are omnia's 1:1 map (`Error::exit_code`); the table is in [docs/standards/cli-contract.md](docs/standards/cli-contract.md#exit-codes).
- The model never authors a document. Synthesis is typed `Question`s put as briefs (`crates/engine/src/specify/{basis,spec,design}.rs`) and verified against engine facts; the stored revision is the truth and Markdown is a `Display` projection. Nothing parses Markdown back, and nothing of a stored revision reaches a run. A revision-shape change re-ids every revision and changes the JSON fixtures under `tests/specify/`; a projection change touches only the `.md` fixtures; a prompt change (`crates/engine/prose/`) touches none.
- Adapters named by a project-relative `.wasm` path or an exact package reference load fresh on every run through the `omnia:plugins/loader` capability, optionally pinned by the source's `digest`. Bare names dispatch only guests declared in the deployment. GitHub URLs are refused.
- Deleted verbs and flags are deleted from the grammar, not aliased. Pre-1.0, a major bump means regenerating with a fresh `emery specify`.
- Engine state is written only through the storage capabilities. Never hand-edit `.omnia/storage`. `spec.md` / `design.md` beside code are `emery show` output, never sources.
- When you remove a symbol: `rg <Symbol> -- AGENTS.md docs/` and fix every hit in the same PR.

## Code style

Baseline: the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html). House deltas win: [style.md](docs/standards/style.md), [coding-standards.md](docs/standards/coding-standards.md), [handler-shape.md](docs/standards/handler-shape.md). In short:

- Short names that lean on the module path: `adapter::Loader`, `registry::show` — never `SourceAdapterLoader` or `show_registry`. Handler DTOs are `<Verb>Input` / `<Verb>Output`. There are no length caps.
- Comments say what and why, never how. `//!` module docs: a title line, then plain paragraphs. `///` on exported items, with `# Errors` where it applies; an example when it helps. Private items carry a `//` only when it explains something non-obvious. No history in comments.
- `<module>.rs` plus `<module>/<child>.rs`; `mod.rs` only under `tests/support/`.
- Name the capability at the dispatch site (`Source::extract(provider, ..)`, `BlobStore::put(store, ..)`). Prefer `strum`, `anyhow`, and `derive_more` to hand-rolled impls. Suppress a lint with `#[expect(lint, reason = "…")]` at the smallest scope, never `#[allow]`.
- Formatting is nightly rustfmt (`make fmt`); never hand-format.

## Testing

Root-led: every CLI-reachable behaviour lives in `tests/` and drives `emery_cli::run` in-process over scripted capabilities (`tests/support/`), asserting the exit code, the envelope, and scripted storage — never the filesystem. Crate suites survive only for independent library contracts (`sdk`, `prose`); unit tests only for CLI-unreachable branches. A test fn names the scenario (`gen_spec`, `no_sources`), never the outcome; the `//` comment above it carries the why. Scripted doubles are strict: script exactly the turns a run consumes. Placement rules: [docs/standards/testing.md](docs/standards/testing.md).

## Commands

All from the repository root through `make` ([`Makefile`](Makefile) → mise):

```bash
make ci          # check + vet + deny — run before committing
make check       # fmt + lint + test + test-docs + doc
make test        # cargo nextest run --locked --workspace --all-features, under -Dwarnings
make lint        # cargo clippy --workspace --all-targets --all-features -- -D warnings
make fmt         # cargo +nightly fmt --all
make cov         # cargo llvm-cov nextest --workspace
make sweep       # drop target/ artifacts untouched for a week
mdbook build docs   # Developer Guide + link check
```

The live journey (`cargo run --example runtime -- specify --config examples/emery.toml`) is in [examples/README.md](examples/README.md); the skill preview (`cursor-agent --plugin-dir plugins/emery`) in [docs/contributing/operator-plugins.md](docs/contributing/operator-plugins.md).

If `make ci` cannot run, say exactly why and which checks ran instead.
