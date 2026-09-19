# CLI Architecture

The `emery` CLI lives in the in-tree Cargo workspace at the repo root. It is a Rust workspace producing a single binary that skills invoke as a subprocess. Adapter-specific deterministic helpers run as in-guest adapter library code inside each adapter's published WebAssembly component.

## One binary: the runtime invocation

The shipped binary is a static deployment expressed as one domain-free `omnia::runtime!` command-mode invocation over the cursor-bound backends (written out in `src/main.rs` with the shipped `locations:` list; `src/lib.rs` is the wasm32 guest alone) — no handwritten `main`. The engine guest is embedded as static component bytes (`include_bytes!` over `$OUT_DIR/emery.cwasm` — the root `build.rs` child-builds the wasm32 engine, then in release builds ahead-of-time compiles it to a serialized wasmtime artifact, so startup deserializes rather than JIT-compiles the engine; debug builds embed the raw component and JIT at startup) and routed as the explicit `command_guest`. Deployment policy is CWD-rooted and inline in the invocation: the invocation directory mounts as the guest's `.` (no ancestor walk), the storage hosts bind engine state to the durable omnia-filesystem store (compiled-in root `.omnia/storage`), the `link:` block declares the source interface (`emery:adapter/source@0.1.0`), and the `plugin:` block the compiled-in declarative `locations:` list (the `.` path root, then the registry policy: `omnia.host` as the default endpoint, with the embedded `src/wasm-pkg.toml` routing namespaces to their registries — no project cache), so a project-relative local `.wasm` adapter loads dynamically at run time — read fresh from the mount on every load — and an exact package reference fetches from its registry on every run. Statically declared adapter guests remain possible in the same invocation — a built `.wasm` path (or `include_bytes!`) plus its adapter id. The shipped runtime embeds the engine only; the journey host in [`examples/runtime.rs`](../../examples/runtime.rs) stays path-only and loads its built mock component by path through the loader. There is no pre-run closure, no guest enumeration, no `omnia.toml`, and no `run --config` surface.

Every invocation runs in the emery (engine) guest through the shared typed command router — help and version displays and grammar rejections included (the shared clap grammar compiles into the engine, so its renderings are the product's by construction); envelopes and exit codes pass through verbatim. Omnia's direct-command entry forwards argv to the guest verbatim: nothing is reserved for the host, so the verbosity flags `-v` / `--verbose` and `-q` / `--quiet` are the grammar's own global options (see *Tracing* below).

Adapter references need no routes: a judgment over a non-empty embedded corpus declares the `list_docs` / `read_doc` function tools on the completion request, and the model's tool calls stream back to the adapter guest, where the SDK answers them in-process from the adapter's listed `PROSE`. Nothing binds an HTTP listener — the runtime invocation declares guests, mounts, and hosts only.

The engine is versioned by the binary — the binary *contains* its engine, so no store entry, first-launch download, or version-skew window exists for it. Kernels never read the environment: paths are fixed constants relative to the named preopens (the `.` project mount — the same strings resolve against the wasm32 preopen table and the native invocation directory).

## Core crate dependency graph

The authoritative crate graph (leaf → root, with per-crate roles) lives in [architecture.md](../standards/architecture.md#workspace-layout). The headline shape: `prose` and `adapter` are the leaves (the embedded prose corpus and the `emery:adapter` contract, one module per axis), `sdk` is the guest-only SDK over them; `engine` owns the domain and the transport-neutral `specify` / `show` operations (path plumbing in `emery_engine::preopen_path`, adapter loading in the engine's `adapter` module) and returns `omnia_sdk::Error` from those operations — no clap, no toml, no terminal text; `cli` (`emery-cli`) is the command façade over the engine: clap grammar, source carriers, `Client` dispatch, the text/JSON projector, and the exit contract; the root package's `src/lib.rs` is wasm32-only: it declares the bare model provider (paths and adapter dispatch are structural, not provider capabilities) and runs `emery_cli::run`; the root binary (`src/main.rs`) owns the native deployment policy inline as one `omnia::runtime!` invocation embedding the engine bytes. Architecture standards beyond the graph (the deployment, adapter resolution, the `.omnia/storage` layout boundary) live there too.

## Dispatch pattern

The binary entry point is thin:

```text
src/main.rs   →  omnia::runtime! (command mode; embedded engine bytes, static guests and mounts)
              →  emery guest  →  typed command router  →  adapter dispatches route by adapter id
```

The deployment projects nothing out of argv: no pre-boot fact depends on the parsed grammar — the invocation directory is the project root, and everything else, displays and rejections included, renders in the guest.

The operator grammar is assembled in `crates/cli/src/lib.rs` on façade-side `SpecifyArgs` / `ShowArgs` types (`clap::Args`), each decoding into its engine input (`emery_engine::specify::SpecifyInput`, `emery_engine::show::ShowInput` — serde DTOs handled by the engine's `specify` / `show` fns, `omnia_sdk::api::Handler<P, I>` through omnia's blanket impl) by exhaustive struct literal, so grammar/input drift is a compile error. `emery_cli` owns clap behavior, the source carriers (argv, `--config`, root discovery), the per-output text render fns, and the hint table; `emery_cli::run(provider, argv, verbosity)` is the whole entry, and it runs on omnia's command façade (`omnia_sdk::api::command`): `parse::<App>` classifies argv, the `verbosity` callback receives the `Verbosity` the global flags select (once, before the verb runs), `Command::new(&client, &metadata, format).hints(hint).call(handler, decode, render)` projects each verb, `completions::<App>` answers the completions verb, and the buffered `Response` comes back. The WASI shim in `src/lib.rs` constructs the provider, applies the reported verbosity to the guest's tracing filter, and passes the response through `execute_wasi`; that boundary owns telemetry initialization and flushing, writes both channels, and exits with the response's exact status. The handler contract is documented in [docs/standards/handler-shape.md](../standards/handler-shape.md).

## JSON envelope contract

All JSON output follows the shared envelope contract:

- **Kebab-case keys** — `app-name`, `project-dir` (never `app_name` or `projectDir`)
- **Flat bodies** — every success body is the typed `*Output` encoded directly; every failure is the flat `{error, message, exit-code}` envelope (optional `hint`). There is no top-level envelope-version stamp.
- **Error discriminants** — the four kebab recovery codes (`specify-source-required`, `unsupported-version`, `spec-not-generated`, `spec-outdated`), the loader's kebab refusals (`refused`, `already-active`, `unavailable`, `internal`), plus the four snake_case Omnia defaults (`bad_request`, `not_found`, `server_error`, `bad_gateway`); skills and tests grep on the `error` field, so renaming one is a breaking change.

The `--format text|json` flag controls output shape; `EMERY_FORMAT=json` is the environment equivalent. Invocation metadata rides the environment too: `EMERY_REQUEST_ID`, `EMERY_CORRELATION_ID`, and `EMERY_CAUSATION_ID` (`Metadata::from_env("EMERY")`); a missing request id is minted from `wasi:random`, and `Client::call` runs the handler in a `handler` tracing span carrying the request and correlation ids.

## Tracing

Progress is `tracing`, never stdout: the engine emits a handful of INFO events at its slow steps (each source extraction, each synthesis pass) and DEBUG detail at each step it completes (the sources decoded, each adapter loaded, each source extracted, each answer accepted, the revision committed or read), rendered on stderr by the guest subscriber `execute_wasi` installs. That subscriber is installed before clap sees argv, so its filter is reloadable: `emery_cli::run` reports the `Verbosity` the global `-v` / `--verbose` (counted) and `-q` / `--quiet` flags select through its callback, and the shim in `src/lib.rs` hands `Verbosity::into_filter()` — the level's preset composed with the ambient `RUST_LOG` — to `omnia_wasi_otel::set_filter`. The CLI opens its `command` span only after that callback, so the selected filter governs the whole dispatch and `execute_wasi` flushes the closed span before any non-zero exit. A bare invocation is `info`; `-v` is `info` plus `emery_cli`, `emery_engine`, and `omnia_sdk` at `debug`; `-vv` (or more) is `debug` plus those three at `trace`; `-q` is `off`. An ambient `RUST_LOG` is appended to every preset but `-q`'s, so its directive wins for a target both name (`RUST_LOG=omnia_sdk=trace emery -v specify …` traces the SDK) and a bare level in it replaces the preset's; one that does not parse fails the reload, and the run keeps the filter telemetry started with. `-q` ignores it. `-v` and `-q` together are a clap usage error, wherever in argv the two sit. `emery_cli` installs no subscriber, so the root suites record the callback instead — and `specify.rs` drives the scripted journey under a subscriber the callback reloads, asserting the DEBUG detail each level shows or hides. Adapters are separate guest instances whose subscribers the flags cannot reach; `RUST_LOG` on the process is the one knob every guest and the host read. The semantic result stays the buffered `Response`; no engine code writes a process stream.

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
