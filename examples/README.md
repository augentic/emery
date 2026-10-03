# Adapter Examples

Live `specify` and `build` journey via [omnia-cursor](https://github.com/augentic/omnia-backends/tree/main/crates/cursor): the mock source adapter extracts greeting claims from [docs/](docs/) through the host model, the engine synthesises `spec.md` / `design.md` and slices `plan.md`, the revision commits, and the mock target adapter builds each slice of the plan into the invocation directory.

The adapters live at [source/](source/) and [target/](target/) — the same anatomy as a first-party adapter. The shipped `emery` binary hosts them: [emery.toml](emery.toml) names each built component by path beneath the adapters root, `~/.emery/adapters`, which the runtime mounts read-only and apart from the project, so a component is never loaded from a tree a run can write. The source input is [docs/](docs/); the build writes `build/` beneath the invocation directory.

## Prerequisites

- [cursor-sdk-bridge](https://github.com/cursor/sdk-bridge). See [below](#installing-cursor-sdk-bridge) for installation.
- `CURSOR_API_KEY` (optionally in `.env` file)



## Build and run

```bash
# build the mock adapters
cargo build --example source --example target --target wasm32-wasip2 --release

# install them beneath the adapters root the runtime loads local components from
mkdir -p ~/.emery/adapters
install target/wasm32-wasip2/release/examples/source.wasm ~/.emery/adapters/
install target/wasm32-wasip2/release/examples/target.wasm ~/.emery/adapters/

# generate the specification set
set -a; source .env; set +a
cargo run -- -v specify --config examples/emery.toml

# review the committed spec and its build plan
cargo run -- show spec
cargo run -- show plan

# build every slice of the plan into the invocation directory
cargo run -- -v build --config examples/emery.toml
```

Without `.env`:

```bash
export CURSOR_API_KEY=<Cursor API key>
```

The mock target writes a Markdown stand-in for code: `build/<slice-name>/index.md` and one `REQ-NNN.md` per requirement, where a real target writes the implementation. The `build/` tree is the journey's output and is ignored by git.

### Tracing

The examples above run with the `-v` flag, which sets tracing level to `debug`. Each additional `v` raises the level, while `-q[q]` lowers it.

### Backend callbacks

*Extract*, *synthesis*, and *build* all run through the Cursor backend. The guest answers reference-tool calls in-process, just like the [omnia-cursor example](https://github.com/augentic/omnia-backends/tree/main/examples/cursor) does.

See [#host-to-guest-tool-calls](#host-to-guest-tool-calls) below for more detail.

## Host-to-guest tool calls

In Emery, the only tools a completion session declares are the reference tools — `list_docs` and `read_doc` — over the adapter's embedded prose corpus. `wasi-model` delivers them as two streams rather than direct callbacks: the host writes each `ToolCall` to the session's `calls` stream, and the guest answers with a `ToolResult` on a second stream it created and passed to `create`, carrying the same correlation ID so the host can resume the completion.

Every answer is served in-process by the SDK from the adapter's listed `PROSE`: `list_docs` returns the adapter's reference paths and Emery's `reconciliation.md` — never a system document (`extract.md`, `claims.md`, `build.md`), which a turn either carries already or has nothing to learn from — `read_doc` returns one document body by adapter-relative path, and anything else — an unknown tool, malformed arguments, an unembedded path — comes back as a repairable error. No HTTP shelf, no MCP callback, and no access to the source input or the revision store crosses this boundary; the model reaches nothing but the adapter's own reference documents. The project tree itself is lent through the host's workspace tools: read-only to an extraction turn, writable to a build turn.

## Installing cursor-sdk-bridge

```bash
# download and install
curl -fsSL -o /tmp/cursor-sdk-bridge.tar.gz \
  https://github.com/cursor/sdk-bridge/releases/latest/download/cursor-sdk-bridge-standalone-darwin-arm64.tar.gz \
  && tar -xzf /tmp/cursor-sdk-bridge.tar.gz -C /tmp \
  && install /tmp/bin/cursor-sdk-bridge ~/.local/bin/cursor-sdk-bridge

# verify
cursor-sdk-bridge --help
```

See [bridge docs](https://cursor.com/docs/sdk/bridge) for more information.
