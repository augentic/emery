# Emery

[CI](https://github.com/augentic/emery/actions/workflows/ci.yaml)
[License: MIT OR Apache-2.0](#license)
[Docs](https://emery.augentic.io/)

Emery reconciles intent, documentation, and existing code into reviewable specifications, architectural designs, and dependency-ordered build plans. This page introduces the `emery` command-line interface (CLI) for developers and operators, with a five-command quickstart and links to the full documentation.

## Contents

- [Quickstart in 5 commands](#quickstart-in-5-commands)
- [CLI overview](#cli-overview)
- [Installation](#installation)
- [Documentation](#documentation)
- [Developing Emery](#developing-emery)
- [Historical archive (v1)](#historical-archive-v1)
- [License](#license)

## Quickstart in 5 commands

Emery turns scattered project inputs into three Git-tracked JSON documents (`spec.json`, `design.json`, and `plan.json`) and builds them through WebAssembly (Wasm) adapters:

```bash
# 1. Extract claims from configured sources and stage a candidate revision
emery specify --config

# 2. Review the staged specification (or design / plan) as Markdown
emery show spec --staged

# 3. Seal the staged candidate to the target Git repository under spec/ on branch emery/<id>
emery apply

# 4. Build the sealed plan wave by wave through the target adapter
emery build

# 5. Stage an incremental change against the sealed revision (review with `emery show amendment`)
emery amend --description emery:intent@0.39.0="Add order cancellation" --config
```

## CLI overview

| Command | Purpose |
| --- | --- |
| `emery specify <adapter>...` | Extracts typed claims from source adapters, reconciles requirements by authority rank, synthesises the specification, design, and build plan, and stages the candidate revision. Pass `--apply` to seal in the same run, or `--fresh` to start a new revision history. |
| `emery amend <adapter>...` | Matches change sources against the existing requirements in the current revision and stages an incremental `Amendment` (classifying each requirement as `added`, `changed`, `retired`, `conflict`, or `unchanged`). Pass `--apply` to seal in the same run. |
| `emery apply` | Commits ("seals") the currently staged candidate or amendment to the target Git repository under `spec/` on branch `emery/<id>`, then clears the staging area. |
| `emery build [<adapter>]` | Builds the sealed plan in dependency waves inside isolated Git worktrees, verifies each integrated wave, and advances the `emery/<id>` branch. |
| `emery show <spec\|design\|plan\|amendment>` | Renders an artifact from the current revision, the staged candidate (`--staged`), or the staged amendment (`amendment`) as Markdown or JSON (`--format json`). |
| `emery completions <shell>` | Prints shell auto-completion scripts for `bash`, `elvish`, `fish`, `powershell`, or `zsh`. |

In Cursor, the [`plugins/emery`](plugins/emery/README.md) plugin wraps the core workflow:

- `/emery:specify` runs `emery specify`, renders the staged candidate, and offers to seal it.
- `/emery:amend` runs `emery amend`, reports the change counts, and offers `emery show amendment` and `emery apply`.
- `/emery:apply` seals the staged candidate or amendment and reports the commit, branch label, and parent revision.
- `/emery:build` runs `emery build` and reports the resulting `emery/<id>` branch.

## Installation

Install the `emery` binary from source using Cargo:

```bash
cargo install --git https://github.com/augentic/emery --locked
emery --version
```

## Documentation

- **Documentation site:** [emery.augentic.io](https://emery.augentic.io/) · [in-tree book root](docs/index.md)
- **Core concepts:** [Glossary](docs/glossary.md)
- **CLI & runtime reference:** [CLI Reference](docs/reference/cli/index.md) · [CLI Output Shapes](docs/reference/cli-output-shapes.md) · [Adapters](docs/reference/adapters.md) · [Deployment Profiles](docs/reference/deployment-profiles.md)
- **Live walkthrough:** [examples/README.md](examples/README.md)
- **First-party adapters:** [augentic/emery-adapters](https://github.com/augentic/emery-adapters)

## Developing Emery

The repository root is a Rust workspace producing the `emery` binary. The root `Makefile` forwards every task to [mise](mise.toml), which runs the shared Augentic Rust tasks. Install `mise` before running local checks:

```bash
make test    # native integration suite via cargo nextest
make ci      # full CI gate: fmt-check, lint (host + wasm32), tests, doctests, docs, vet, deny
make check   # local advisories: audit, fmt (rewrites), lint, outdated, deps
```

To build `emery` against a local model backend through [omnia-genai](https://github.com/augentic/omnia-backends/tree/main/crates/genai) instead of Cursor (`GENAI_MODEL` and `GENAI_ENDPOINT`; see [src/main.rs](src/main.rs)), compile with `--features genai` into a separate target directory and copy the binary as `emery-dev`:

```bash
cargo build --release --features genai --target-dir target/genai
cp target/genai/release/emery ~/.cargo/bin/emery-dev
```

To preview the working-tree Cursor plugin skills against a locally installed `emery` binary:

```bash
cursor-agent --plugin-dir plugins/emery
```

For contributor guidelines, architecture invariants, and repository governance, see [AGENTS.md](AGENTS.md), [CONTRIBUTING.md](CONTRIBUTING.md), [GOVERNANCE.md](GOVERNANCE.md), and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Historical archive (v1)

The v1 delivery workflow (`survey`/`extract` and `plan → refine → execute → finalize`) is archived at Git tag `v1`. To check out the `v1` tree alongside this repository:

```bash
git worktree add ../emery-v1 v1
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
