---
name: emery-build
description: Build every slice of the current plan by invoking `emery build` through a target adapter, then commit what the build wrote. Use whenever the operator wants the committed plan built into the project tree.
argument-hint: [<adapter>]
---

# Build Skill

`emery build` is the one build verb: it reads the current revision, loads the one target adapter the run names — an exact package reference, `namespace:name@version`, read from the store `~/.emery/adapters` and fetched into it on the first run that names it — and hands every slice of the plan to it in build order, the plan's waves flattened, each slice built into the project tree and reported as the requirement ids it covered and the files it wrote. The tree is the build's output: the engine writes no state of its own, and re-running builds every slice again over whatever the tree then holds. This skill installs or refreshes the CLI, elicits arguments, invokes the verb, commits what the build wrote, and relays its output.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling. Install the latest prebuilt release via Homebrew, or from source; an adapter whose declared minimum `emery-version` outruns the installed binary fails typed later (`unsupported-version`, exit 1) with the same reinstall command as its hint:

```bash
brew tap augentic/tap
brew install emery
# or: cargo install --git https://github.com/augentic/emery --locked
```

Then run `emery --version` and stop on failure.

2. **Elicit the target adapter and pass it as a flag** — the CLI has no interactive prompt mode: no adapter at all — and no project-root `emery.toml` carrying a `[target]` table — fails typed (`build-target-required`, exit 1). Gather conversationally: the one target adapter to build through, an exact package reference with a version — `acme:rust@1.4.0`, never `acme:rust`, `rust`, or a path; one without a version or a namespace fails typed (`adapter-reference`, exit 1) before anything loads, so ask for the version when the operator names none. An operator who keeps a config file selects it instead with `--config [<path>]`; omit the value only for the project-relative `emery.toml`, and a run naming no adapter discovers that file on its own. Never combine the file carrier with a positional adapter (mixing fails typed, exit 1). A build needs a committed revision: before any is committed the run fails typed (`spec-not-generated`, exit 2), so run `/emery:specify` first.
3. **Invoke**, in JSON mode, so the commit step can read what was written:

```bash
emery --format json build <adapter>
# or: emery --format json build --config [<path>]
```

Build dispatches one model turn per slice and can take a while on a wide plan. Tracing follows the plugin rule's *Tracing and output* contract (a bare run is `info`; pass `-v` when the operator asks for debug).

## Commit

A successful run's envelope carries `revision`, `waves`, and `slices`, each slice `{ id, name, covered, uncovered, written }` in build order. Commit what the build wrote, and only that:

1. Collect every path under `slices[].written`. A build that wrote nothing — every `written` list empty — commits nothing; relay the output and stop.
2. Stage those paths alone — `git add -- <path>...` — never `git add -A`, `git add .`, or a pathspec wider than the written files, so the operator's own work in progress is never swept into the build's commit. A written path that is ignored by `.gitignore` is reported to the operator rather than force-added.
3. Commit onto the current branch with the revision and the slices in the message: the subject is `emery build <rev-8>: <slices>`, the first eight characters of `revision` and each slice as `<id> <name>`, comma-separated; the body carries one trailer per line — `Revision:` the full revision id, `Slices:` the slice ids in build order, `Covered:` every covered requirement id, `Uncovered:` every uncovered one (omit the trailer when none), `Adapter:` the target adapter reference the run named.

```bash
git add -- src/auth.rs src/orders.rs src/orders/create.rs
git commit -m "emery build 9f8e7d6c: SLICE-001 authentication, SLICE-002 orders" \
  -m "Revision: 9f8e7d6c…
Slices: SLICE-001, SLICE-002
Covered: REQ-001, REQ-002, REQ-003
Uncovered: REQ-004
Adapter: acme:rust@1.4.0"
```

The skill creates no branch and opens no pull request: the commit lands where the operator's checkout is. Never amend or rewrite an earlier commit, and never commit `spec.md`, `design.md`, `plan.md`, or anything under `.emery/` through this skill — the projections belong to `/emery:specify`, and the revision store is never tracked.

## Relay

- Surface the CLI output verbatim — the envelope names the revision built, the plan's waves, and each slice's covered and uncovered requirement ids and written files — then the commit's hash and subject.
- A requirement the adapter left out of `covered` is reported `uncovered`, never implemented by hand: relay it and let the operator decide whether to re-run, change a source and re-run `/emery:specify`, or take it up themselves.
- On non-zero exit, surface the structured error and stop — commit nothing, and leave what earlier slices wrote in the tree for the operator (the message names the failed slice and the slices built before it). A `build-target-required` failure means no adapter was named and no `[target]` table was found: relay the hint and ask for the adapter. A `spec-not-generated` or `spec-outdated` failure means there is no revision this binary can build: relay the hint (`/emery:specify` first, or again) and let the operator decide. A `refused` failure means the loader rejected the request (an invalid or pre-compiled artifact, a `digest` the release does not resolve to, or a release under a namespace the binary routes nowhere, which the hint says to fetch with `wkg get <reference> -o ~/.emery/adapters/`); relay the hint and let the operator decide.
