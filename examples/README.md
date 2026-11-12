# Adapter Examples

Minimal WebAssembly (Wasm) source and target adapters and a sample project fixture for testing `emery` end to end.

## Contents of this directory

| Path | Description |
| --- | --- |
| [`source/`](source/) | Minimal `emery:adapter/source` guest (`example:source@0.1.0`) that extracts claims from Markdown or inline text under `prose/extract.md`. |
| [`target/`](target/) | Minimal `emery:adapter/target` guest (`example:target@0.1.0`) that scaffolds `build/README.md`, scopes change builds conservatively, builds slice Markdown files under `build/<slice>/`, union-merges `build/*/index.md`, and verifies waves. |
| [`docs/greeting.md`](docs/greeting.md) | Sample Markdown source document used by the tutorials. |
| [`emery.toml`](emery.toml) | Annotated sample configuration wiring `example:source@0.1.0` and `example:target@0.1.0`. |

## Build and install the mock adapters

From the repository root, compile both example adapters for `wasm32-wasip2` and copy them into the local adapter store (`~/.emery/adapters`):

```bash
cargo build --release
cargo build --examples --target wasm32-wasip2 --release

mkdir -p ~/.emery/adapters
cp target/wasm32-wasip2/release/examples/source.wasm ~/.emery/adapters/example_source@0.1.0.wasm
cp target/wasm32-wasip2/release/examples/target.wasm ~/.emery/adapters/example_target@0.1.0.wasm
```

## Walkthroughs and guides

Step-by-step walkthroughs and explanations using these examples live in the documentation book ([`docs/`](../docs/index.md)):

- **[Your first specification and build](../docs/tutorials/quickstart.md):** Install `cursor-sdk-bridge`, configure a scratch project with these mock adapters, and run `emery specify`, `emery show`, `emery apply`, and `emery build`.
- **[Evolving a specification with `emery amend`](../docs/tutorials/amending-a-spec.md):** Amend requirements, layer inline `--description` sources with `--config`, resolve `[conflict]` requirements, and reset a lineage with `--fresh`.
- **[Merge specifications across Git branches](../docs/how-to/merge-branches.md):** Resolve `spec/` conflicts when two branches or clones seal amendments independently.
- **[Author a custom source or target adapter](../docs/how-to/author-an-adapter.md):** How `source/lib.rs`, `target/lib.rs`, `PROSE`, and in-process guest tool calls (`list_docs`, `read_doc`, `write_files`, `edit_files`) work.
