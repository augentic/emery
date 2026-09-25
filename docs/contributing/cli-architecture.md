# CLI Architecture

The `emery` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary: the runtime invocation

The shipped binary is one domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends — `src/main.rs` is that invocation and nothing else; `src/lib.rs` is the wasm32 guest alone. The engine guest is embedded as static component bytes (`guests: [{ path: env!("EMERY_GUEST") }]`, which the macro reads with `include_bytes!` and names `emery` by the file's stem — the root `build.rs` child-builds the wasm32 engine and emits the artifact's path as `EMERY_GUEST`: the raw component `emery.wasm` in a debug build, JIT-compiled at startup, and the precompiled `emery.cwasm` in a release build, compiled for the binary's own target under omnia's default compile settings; omnia loads either format), the sole `wasi:cli/run` exporter and so the command guest. Deployment policy is CWD-rooted and inline in the invocation: the invocation directory mounts read-only as the guest's `.` (no ancestor walk), and the storage hosts bind engine state to the durable omnia-filesystem store (compiled-in root `.omnia/storage`). Nothing declares the source interface (`emery:adapter/source@0.1.0`) — omnia relays it from the engine's import to the adapter exporting it. What varies per run, the engine names on each `omnia:plugins/loader` load (`omnia_sdk::plugins::Location`), and the deployment's grant bounds it: a project-relative local `.wasm` adapter loads by its path, which the loader reads through the read-only `.` mount — the one root code is read through; a mount a guest could write is never one — and registers under its file's stem, read fresh on every run and never cached; an exact package reference loads by the reference, fetched on every run from the registry the engine names on the load — the project's `emery.toml` `[registries]` table, `augentic.io` for `emery` unless a line re-routes it (no project cache, and no routing compiled into the binary); each load carries its `[[source]] digest` as the pin the loader holds the bytes to, and the loader admits a path or a package as raw wasm alone, since the invocation names the path, the pin, and the routing alike: the only pre-compiled artifact is the engine compiled into the binary, and an adapter is always raw wasm. Statically declared adapter guests remain possible in the same invocation — another `{ path }` entry, whose stem (or `name`) is the bare name a source references. The shipped runtime embeds the engine only, so under it every bare name refuses typed. There is no guest enumeration, no `omnia.toml`, and no `run --manifest` surface.

Every invocation runs in the emery (engine) guest through the shared typed command router — help and version displays and grammar rejections included (the shared clap grammar compiles into the engine, so its renderings are the product's by construction); envelopes and exit codes pass through verbatim. Omnia's direct-command entry forwards argv to the guest verbatim: it reads the `-v` / `-q` verbosity flags to set the run's tracing level, and forwards them with the rest, so the grammar declares them too (see *Tracing* below).

Adapter references need no routes: a judgment over a non-empty embedded corpus declares the `list_docs` / `read_doc` function tools on the completion request, and the model's tool calls stream back to the adapter guest, where the SDK answers them in-process from the adapter's listed `PROSE`. Nothing binds an HTTP listener — the runtime invocation declares guests, mounts, and hosts only.

The engine is versioned by the binary — the binary *contains* its engine, so no store entry, first-launch download, or version-skew window exists for it. Kernels never read the environment: paths are fixed constants relative to the named preopens (the `.` project mount — the same strings resolve against the wasm32 preopen table and the native invocation directory).

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [architecture.md](../standards/architecture.md#workspace-layout). The headline shape: `prose` and `adapter` are the leaves (the embedded prose corpus and the `emery:adapter` contract, one module per axis), `sdk` is the guest-only SDK over them; `engine` owns the domain and the transport-neutral `specify` / `show` operations (path plumbing in `emery_engine::preopen_path`, adapter loading in the engine's `adapter` module) and returns `omnia_sdk::Error` from those operations — no clap, no toml, no terminal text; `cli` (`emery-cli`) is the command façade over the engine: clap grammar, source carriers, `Client` dispatch, the text/JSON projector, and the exit contract; the root package's `src/lib.rs` is wasm32-only: it declares the bare model provider (paths and adapter dispatch are structural, not provider capabilities) and runs `emery_cli::run`; the root binary (`src/main.rs`) owns the native deployment policy — the one `omnia::runtime!` invocation embedding the engine bytes. Architecture standards beyond the graph (the deployment, adapter resolution, the `.omnia/storage` layout boundary) live there too.

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs   →  omnia::runtime! (command mode; embedded engine bytes, static guests and mounts)
              →  emery guest  →  typed command router  →  adapter dispatches route by adapter id
```

The deployment projects nothing out of argv: no pre-boot fact depends on the parsed grammar — the invocation directory is the project root, and everything else, displays and rejections included, renders in the guest.

The operator grammar is assembled in `crates/cli/src/lib.rs` on façade-side `SpecifyArgs` / `ShowArgs` types (`clap::Args`), each decoding into its engine input (`emery_engine::specify::SpecifyInput`, `emery_engine::show::ShowInput` — serde DTOs handled by the engine's `specify` / `show` fns, `omnia_sdk::api::Handler<P, I>` through omnia's blanket impl) by exhaustive struct literal, so grammar/input drift is a compile error. `emery_cli` owns clap behavior, the source carriers (argv, `--config`, root discovery), the per-output text render fns, and the hint table; `emery_cli::run(provider, argv)` is the whole entry, and it runs on omnia's command façade (`omnia_sdk::api::command`): `parse::<App>` classifies argv, `Command::new(&client, &metadata, format).hints(hint).call(handler, decode, render)` projects each verb, `completions::<App>` answers the completions verb, and the buffered `Response` comes back. The WASI shim in `src/lib.rs` constructs the provider and passes the response through `execute_wasi`; that boundary owns telemetry initialization and flushing, writes both channels, and exits with the response's exact status. The handler contract is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

## JSON envelope contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** — `app-name`, `project-dir` (never `app_name` or `projectDir`)
- **Flat bodies** — every success body is the typed `*Output` encoded directly; every failure is the flat `{error, message, exit-code}` envelope (optional `hint`). There is no top-level envelope-version stamp.
- **Error discriminants** — the four kebab recovery codes (`specify-source-required`, `unsupported-version`, `spec-not-generated`, `spec-outdated`), the loader's kebab refusals (`refused`, `unavailable`, `internal`), plus the four snake_case Omnia defaults (`bad_request`, `not_found`, `server_error`, `bad_gateway`); skills and tests grep on the `error` field, so renaming one is a breaking change.

The `--format text|json` flag controls output shape; `EMERY_FORMAT=json` is the environment equivalent. Invocation metadata rides the environment too: `EMERY_REQUEST_ID`, `EMERY_CORRELATION_ID`, and `EMERY_CAUSATION_ID` (`Metadata::from_env("EMERY")`); a missing request id is minted from `wasi:random`, and `Client::call` runs the handler in a `handler` tracing span carrying the request and correlation ids.

## Tracing

Progress is `tracing`, never stdout: the engine emits a handful of INFO events at its slow steps (each source extraction, each synthesis pass) and DEBUG detail at each step it completes (the sources decoded, each adapter loaded, each source extracted, each answer accepted, the revision committed or read), rendered on stderr by the guest subscriber `execute_wasi` installs. That subscriber reads the guest environment's `RUST_LOG`, which omnia sets from the run's one tracing level — the same level its host console opens at. In command mode that level is `info` on a bare run, so the engine's progress, an adapter's `source_adapter_extract` span, and the SDK's INFO progress all reach stderr. The global `-v` / `--verbose` and `-q` / `--quiet` flags move it: omnia's direct-command entry reads them from argv before the guest runs — each `-v` one step up (`debug`, `trace`), each `-q` one step down (`warn`, `error`, `off`), clamped — and forwards argv verbatim, so the grammar flattens `omnia_sdk::api::command::Verbosity` into `App` to accept them, list them in help and completions, and refuse `-v` beside `-q` as a usage error (exit `64`); nothing in `emery_cli` reads their counts. A flag overrides a process `RUST_LOG`; a bare run keeps one that is set, so `RUST_LOG=emery_sdk=debug` admits the SDK's detail alone and `RUST_LOG=off` silences every guest. The host never writes the process environment. `Client::call` opens the `handler` span carrying request and correlation ids; each concurrent source extraction and model judgment adds one child span with explicit identifiers. No span records its error as an event: the failure body on stderr is the one print of a description, so tracing names a failed step and its class and never repeats the text. `execute_wasi` flushes the guest telemetry before any non-zero exit. `emery_cli` installs no subscriber. Adapters are fresh guest instances whose extraction boundary installs and flushes its own subscriber from the same environment. Under that filter the SDK follows the engine's shape: INFO as each model turn opens (`surveying`, `mining` per seam, with the file count where a `Files` seam knows it), DEBUG for what it yielded (`surveyed` with the surfaces found, `mined` with the claim count), WARN when a turn is put once more (carrying the upstream error, which nothing else reports) or fails (`failed`, carrying the class alone — the join reports each description once), and DEBUG for every reference-tool call (`answered`, with the arguments as the model sent them) and every `candidate rejected` (with its findings). Every event names the source key and, within a seam, its index, because the guest `fmt` layer filters spans and so prints no span fields. The semantic result stays the buffered `Response`; no engine code writes a process stream.

## Exit codes

The exit-code contract is part of the public interface for operators and skill wrappers; `omnia_sdk::Error::exit_code` maps the variants and is the single source of truth, applied by omnia's `Command` projector. The one table lives in [cli-contract.md § Exit codes](../standards/cli-contract.md#exit-codes).

Guest commands inherit the same contract: omnia's command façade projects parser, decoder, and handler outcomes into a buffered command response; the WASI run export forwards its exit and the binary passes it through verbatim.

## Error handling

Commands return `omnia_sdk::Error`. Construct the Omnia class that matches: `BadRequest` for operator or input refusals, `NotFound` for missing resources, `BadGateway` for upstream or model failures; everything else is `ServerError`. Do not introduce a house error type.

The pattern for a command operation:

1. Call into a library crate function that returns `Result<T, omnia_sdk::Error>`
2. Return a typed `Serialize` body; its render fn in `crates/cli/src/text.rs` is its text mode
3. Let omnia's command projector render success or apply the shared error contract

## Public Rust API

The root `emery` package is the Omnia deployment unit. It does not expose a public Rust library surface for consumers. Code that needs Rust APIs imports the member crates directly, for example `emery_engine::specify::SpecifyInput` or `emery_cli::run`.
