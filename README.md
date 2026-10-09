# Emery

[CI](https://github.com/augentic/emery/actions/workflows/ci.yaml)
[License: MIT OR Apache-2.0](#license)
[Docs](https://emery.augentic.io/)

Emery reconciles intent, documentation, existing code, and captured behaviour into reviewable specifications — durable artifacts, not chat history.

> The v1 delivery workflow (survey/extract, plan/refine/execute/finalize) is archived at git tag `v1`; retrieve it with `git worktree add ../emery-v1 v1`. This tree carries the spec generator and its build step: `emery specify` synthesises the reviewable set from the sources named on the invocation, `emery build` hands its plan to a target adapter wave by wave, `emery show` renders it.

## The live surface

```bash
emery specify <adapter>...  # extract, group, synthesise the spec + design, slice the plan
emery build <adapter>       # build the plan in waves through a target adapter, one merge each under emery/<revision>, verified wave by wave
emery show spec             # render spec.md from the current revision (--format json: the revision)
emery show plan             # render plan.md: the spec sliced into buildable pieces
emery completions <sh>      # shell completions
```

In Cursor, `/emery:specify` wraps `emery specify`. Everything else was deleted from the grammar, not hidden — see the [CLI reference](docs/reference/cli/index.md).

Install from source:

```bash
cargo install --git https://github.com/augentic/emery --locked
emery --version
```

## Documentation

- **CLI reference:** [emery.augentic.io](https://emery.augentic.io/) · [in-tree source](docs/reference/cli/index.md)
- **Contributing:** [AGENTS.md](AGENTS.md) (repository map, invariants, commands) and [CONTRIBUTING.md](CONTRIBUTING.md)

## Developing Emery (contributors)

The repository root is a Rust workspace producing the `emery` binary. The root `Makefile` forwards every goal to [mise](mise.toml), which includes the shared Augentic Rust tasks; install mise first.

```bash
make test    # native integration suite
make ci      # exactly the CI gate: fmt-check, lint (host + wasm32), tests, doctests, docs, vet, deny
make check   # local advisories: audit, fmt (rewrites), lint, outdated, deps
```

Preview the working-tree Cursor skill against a local CLI:

```bash
cursor-agent --plugin-dir plugins/emery
```

Start with [AGENTS.md](AGENTS.md), then [CONTRIBUTING.md](CONTRIBUTING.md). See also [GOVERNANCE.md](GOVERNANCE.md) and [Code of Conduct](CODE_OF_CONDUCT.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
