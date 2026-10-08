# Adapter Examples

Live `specify` and `build` journey via [omnia-cursor](https://github.com/augentic/omnia-backends/tree/main/crates/cursor): the mock source adapter extracts greeting claims from [docs/](docs/) through the host model, the engine synthesises `spec.md` / `design.md` and slices `plan.md`, the revision commits, and the mock target adapter builds each slice of the plan into a working copy cut from the project's sealed head, one commit per slice under the label `emery/<revision>`.

The adapters live at [source/](source/) and [target/](target/) — the same anatomy as a first-party adapter. The shipped `emery` binary hosts them: [emery.toml](emery.toml) names each as an exact package reference of its own, `example:source@0.1.0` and `example:target@0.1.0`, which the binary reads from its store, `~/.emery/adapters`, as `example_source@0.1.0.wasm` and `example_target@0.1.0.wasm` — the names `cp` writes below. The store lies apart from the project, so a component is never loaded from a tree a run can write, and nothing under the `example` namespace is fetched: a reference the store holds is read from it before any registry is asked. The source input is [docs/](docs/); the build lands as commits in the project repository, which is why the journey runs from a scratch project of its own rather than this checkout.

## Prerequisites

- [cursor-sdk-bridge](https://github.com/cursor/sdk-bridge). See [below](#installing-cursor-sdk-bridge) for installation.
- `CURSOR_API_KEY` in a `.env` file
- `git` 2.5 or newer on `PATH` (`GIT_BINARY` names another)

## Build and run

```bash
# build the binary and the mock adapters
cargo build --release
cargo build --examples --target wasm32-wasip2 --release

# copy the adapters to the store under their references
mkdir -p ~/.emery/adapters
cp target/wasm32-wasip2/release/examples/source.wasm ~/.emery/adapters/example_source@0.1.0.wasm
cp target/wasm32-wasip2/release/examples/target.wasm ~/.emery/adapters/example_target@0.1.0.wasm

# a scratch project: the fixture tree and the config, sealed as one commit
set -a; source .env; set +a
emery=$PWD/target/release/emery
project=$(mktemp -d)
cp -R examples/docs examples/emery.toml "$project"
cd "$project"
printf '.emery/\n' > .gitignore
git init --quiet && git add --all && git commit --quiet --message "greeting docs"

# generate the specification set (the project-root emery.toml is discovered)
"$emery" -v specify

# review the committed spec and its build plan
"$emery" show spec
"$emery" show plan

# build every slice of the plan: one commit each, labelled emery/<revision>
"$emery" -v build

# read the build; the checkout itself is untouched
git log --oneline HEAD..emery/<revision>
git diff --stat HEAD emery/<revision>
git switch emery/<revision>          # or: git merge emery/<revision>
```

Without `.env`:

```bash
export CURSOR_API_KEY=<Cursor API key>
```

The mock target writes a Markdown stand-in for code: `build/<slice-name>/index.md` and one `REQ-NNN.md` per requirement, where a real target writes the implementation. Each slice's files are its commit under the label; the `.emery/` beneath the project holds the revision store and the clones and working copies a run cuts, and is ignored so the checkout stays sealed. A `build` over a checkout with an uncommitted change is refused as `base-not-sealed`, naming the paths; commit or stash them and run it again. A stored release is final until removed, so after rebuilding a mock copy it in again; `rm ~/.emery/adapters/example_*` takes both out of the store.

To build into another repository instead — a brownfield project, or a remote the label should land on — uncomment the `repository`, `branch`, and `remote` keys of the `[target]` table in [emery.toml](emery.toml): the build then starts from that branch's commit in a clone kept under `.emery/vcs/repos/`, and the label is pushed to the remote once every slice is sealed.

### Tracing

The examples above run with the `-v` flag, which sets tracing level to `debug`. Each additional `v` raises the level, while `-q[q]` lowers it.

### Backend callbacks

*Extract*, *synthesis*, and *build* all run through the Cursor backend. The guest answers reference-tool calls in-process, just like the [omnia-cursor example](https://github.com/augentic/omnia-backends/tree/main/examples/cursor) does.

See [#host-to-guest-tool-calls](#host-to-guest-tool-calls) below for more detail.

## Host-to-guest tool calls

In Emery, the tools a completion session declares are the reference tools — `list_docs` and `read_doc` — over the adapter's embedded prose corpus, and, on a build turn alone, `write_file` over the lent tree. `wasi-model` delivers them as two streams rather than direct callbacks: the host writes each `ToolCall` to the session's `calls` stream, and the guest answers with a `ToolResult` on a second stream it created and passed to `create`, carrying the same correlation ID so the host can resume the completion.

Every answer is served in-process by the SDK from the adapter's listed `PROSE`: `list_docs` returns the adapter's reference paths and Emery's `reconciliation.md` — never a system document (`extract.md`, `claims.md`, `build.md`), which a turn either carries already or has nothing to learn from — `read_doc` returns one document body by adapter-relative path, and anything else — an unknown tool, malformed arguments, an unembedded path — comes back as a repairable error. No HTTP shelf, no MCP callback, and no access to the source input or the revision store crosses this boundary; the model reaches nothing but the adapter's own reference documents and, on a build turn, the one write beneath the lent tree. The lent tree is read through the host's workspace tools; a build turn is lent the integration working copy, never the checkout, and writes it through `write_file`, served the same way from the guest, one file per call beneath the lent root, refused under `.emery/` or `.git/` and at the projections.

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
