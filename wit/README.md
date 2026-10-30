# `emery:adapter` WIT Contract

This directory contains [`emery.wit`](emery.wit), the WebAssembly Interface Types (WIT) contract between the Emery engine and all source and target adapters, published as the `emery:adapter` package.

## Contract structure

`emery.wit` is self-contained (requiring no external `deps/`) and defines three interfaces and two component worlds:

- **`types` interface:** Shared primitives used by both adapter axes:
  - `adapter-id`: The string identifier routing a call to a registered adapter guest (such as `emery:intent`).
  - `error`: The operation error variant (`invalid-request(string)` when an adapter rejects its input, or `internal(string)` when an adapter fails internally).
- **`source` interface and `source-adapter` world:** The contract for source adapters (`emery specify` and `emery amend`):
  - `metadata(id) -> adapter-metadata`: Returns the adapter's minimum `emery-version` and its `source-kind` (`intent`, `documentation`, or `behaviour`).
  - `extract(id, input) -> result<evidence, error>`: Reads a `workspace` path or inline `value` and returns `evidence` (a list of typed `claim` records with optional `backing` and JSON-encoded `extras`).
- **`target` interface and `target-adapter` world:** The contract for target adapters (`emery build`):
  - `metadata(id) -> target-metadata`: Returns the adapter's minimum `emery-version` and path-glob `merge-rules` (`union`, `ours`, or `theirs`).
  - `run(id, workspace, task) -> result<outcome, error>`: Executes one `task` in the writable `workspace` directory and returns the matching `outcome`:
    - `init` → `initialised`: Deterministically scaffolds missing project layout files before the first slice builds.
    - `build(slice)` → `built(report)`: Implements one `slice` (carrying typed `requirements`, `types`, `brief`, and Markdown projections) and reports the `covered` requirement IDs and `written` file paths.
    - `verify` → `verified(verdict)`: Runs checks and repairs across the integrated workspace after a wave merges, returning `passed` and any remaining `failures`.

### Shared binding generation

Each world (`source-adapter` and `target-adapter`) both imports and exports its axis interface. This allows a single `wit-bindgen` pass per axis in the `emery-adapter` crate to generate:

- The guest export traits (`emery_adapter::source::export` and `emery_adapter::target::export`) implemented by adapters via `emery-sdk`.
- The host import callers (`bindings::import`) used by the Emery engine guest's `Source` and `Target` capabilities.

Because the export types alias the import types, one Rust type represents each WIT record on both sides of the boundary. [augentic/emery-adapters](https://github.com/augentic/emery-adapters) consumes a vendored copy of `emery.wit`.

---

## Publishing

Publishing is manual. Registry versions are immutable: always bump the `package emery:adapter@<ver>;` declaration in `emery.wit` first, and never overwrite an existing version.

1. Install [`wkg`](https://github.com/bytecodealliance/wasm-pkg-tools) and configure the `emery` namespace in `~/.config/wasm-pkg/config.toml` with a GitHub token that has `packages: write` permission:

```toml
# ~/.config/wasm-pkg/config.toml
[namespace_registries]
emery = "augentic.io"

[registry."augentic.io".oci]
auth = { username = "<github-user>", password = "<token>" }
```

2. From the repository root, build and publish the WIT package:

```bash
wkg wit build --wit-dir wit --output emery-adapter.wasm
wkg publish emery-adapter.wasm --package emery:adapter@<ver>
```

---

## Consuming

Pulling a published WIT package requires no authentication:

```bash
wkg get emery:adapter@<semver> --format wit --output emery.wit
```

### `wkg` registry resolution

The `emery` namespace maps to `augentic.io`, whose `/.well-known/wasm-pkg/registry.json` endpoint resolves packages to the backing Open Container Initiative (OCI) registry on GitHub Container Registry (`ghcr.io/augentic`).

See the Bytecode Alliance guide on [Composing and Distributing](https://component-model.bytecodealliance.org/composing-and-distributing/distributing.html) for background on `wasm-pkg` registries.
