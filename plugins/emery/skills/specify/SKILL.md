---
name: emery-specify
description: Generate a specification by invoking `emery specify` over the named sources and relaying its output. Use whenever the operator wants to generate or regenerate `spec.md` / `design.md`.
argument-hint: <adapter>
---

# Specify Skill

`emery specify` is the one generate verb: it resolves the named source adapters (a local component loads through the deployment loader, read fresh each run; an exact package reference fetches from its registry; either load's optional `digest` pin is verified host-side), extracts, derives the requirements, synthesises, and commits one revision, swapping the current revision id. Nothing about the source list persists between runs — repeat the sources on every invocation, or keep them in an operator-owned `emery.toml`. Every run starts from its sources alone: nothing of the stored revision reaches the synthesis, and requirements number from `REQ-001` in source order. This skill installs or refreshes the CLI, elicits arguments, invokes the verb, re-projects the committed revision, and relays its output.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling. Install the latest prebuilt release via Homebrew, or from source; an adapter whose declared minimum `emery-version` outruns the installed binary fails typed later (`unsupported-version`, exit 1) with the same reinstall command as its hint:

```bash
brew tap augentic/tap
brew install emery
# or: cargo install --git https://github.com/augentic/emery --locked
```

Then run `emery --version --quiet` and stop on failure.

2. **Elicit every required input and pass it as a flag** — the CLI has no interactive prompt mode: no source at all — and no project-root `emery.toml` to discover — fails typed (`specify-source-required`). Gather conversationally: the source adapters to extract (each positional `<adapter>` is a workspace-backed source; each `--description <adapter>=<text>` is an inline source such as an operator directive). An operator who keeps a config file selects it instead with `--config [<path>]`; omit the value only for the project-relative `emery.toml`, and a run naming no sources at all discovers that file on its own. Never combine the file carrier with positional adapters or `--description` (mixing fails typed, exit 1). Local paths must stay relative to the project and must not escape it.
3. **Invoke**:

```bash
emery specify <adapter>... [--description <adapter>=<text>] --quiet
# or: emery specify --config [<path>] --quiet
```

Specify dispatches model judgment and can take a while on large workspaces; it runs with `--quiet` per the plugin rule's *Tracing and output* contract (`--debug` replaces it when the operator asks for debug).

## Re-project

After every successful run, write the committed revision's Markdown projections beside the code for review:

```bash
emery show spec --quiet > spec.md
emery show design --quiet > design.md
```

Track both files in version control. Never edit them by hand: they are projections of the stored revision, and a hand edit is overwritten by the next run (change a source and re-run instead).

## Relay

- Surface the CLI output verbatim — the success envelope names the committed revision and the re-mine diff against the one it displaced.
- Review is `spec.md` / `design.md` as re-projected, or `emery show spec` / `emery show design` directly — never read or edit `.omnia/storage` state by hand.
- On non-zero exit, surface the structured error and stop — never hand-roll spec documents. A `refused` failure means the loader rejected the request (a pin that no longer matches, a malformed pin, an invalid artifact, or an unserved location); relay the hint and let the operator decide. A `spec-outdated` failure means the stored revision predates this binary's grammar: relay the hint (re-run `emery specify` to regenerate) and let the operator decide.
