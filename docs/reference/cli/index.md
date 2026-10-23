# CLI Reference

The `emery` CLI owns all deterministic operations. The grammar is four verbs plus the auto-derived `completions` — deleted verbs are gone from the grammar, not hidden.

## Installation

```bash
brew tap augentic/tap
brew install emery

# or: cargo install --git https://github.com/augentic/emery --locked
```

## Conventions

- All commands return structured output on stdout and use exit codes for success/failure; `--format json` selects the JSON envelope (see [CLI output shapes](../cli-output-shapes.md)).
- An adapter is named by an exact package reference, `namespace:name@version`, read from the store `~/.emery/adapters` and fetched into it on the first run that names it (see [Adapters](../adapters.md)).
- The specification lives in the target's repository: `specify` stages a candidate in `.emery/storage`, `apply` seals it under `spec/` as one commit labelled `emery/<revision>`, and `build` and a bare `show` read that revision from the repository the `[target]` table names, never the candidate.
- Commands that modify `.emery/storage` state are idempotent where possible.
- Skills delegate to the CLI for all structural operations — they never hand-edit `.emery/storage` state or `spec/` directly.

## Commands

| Verb | Purpose |
|------|---------|
| [emery specify](specify.md) | Generate the specification, design, and build plan from the sources named on the invocation and stage them as one candidate; `--apply` seals it in the same run |
| [emery apply](apply.md) | Seal the staged candidate as the current revision in the target's repository, under `spec/` at one commit labelled `emery/<revision>` |
| [emery build](build.md) | Build the sealed plan through a target adapter in waves, each slice merged into the label `emery/<revision>` and each wave verified; a re-run resumes from the label |
| [emery show](show.md) | Print a reviewable artifact of the current revision, of the staged candidate under `--staged`, or the staged amendment (`amendment`), to stdout; `--format json` carries the typed document |
| `emery completions <shell>` | Print a shell-completion script; auto-derived from the live clap surface |
