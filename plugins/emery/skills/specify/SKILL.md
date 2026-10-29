---
name: emery-specify
description: Generate a specification by invoking `emery specify` over the named sources, stage it as a candidate for `emery apply`, and relay its output. Use whenever the operator wants to generate or regenerate `spec.md` / `design.md` / `plan.md`.
argument-hint: <adapter>
---

# Specify Skill

`emery specify` is the one generate verb: it resolves the named source adapters — each an exact package reference, `namespace:name@version`, read from the store `~/.emery/adapters` and fetched into it on the first run that names it — extracts, derives the requirements, synthesises, slices the specification into a build plan, and stages the three documents as one **candidate** in `.emery/storage`, which [`/emery:apply`](../apply/SKILL.md) seals as the current revision in the target's repository; `--apply` does both in one run. Nothing about the source list persists between runs — repeat the sources on every invocation, or keep them in an operator-owned `emery.toml`. Every run starts from its sources alone: nothing of the sealed revision reaches the synthesis, requirements number from `REQ-001` in source order, and a second run replaces the candidate. A sealed revision is revised with [`/emery:amend`](../amend/SKILL.md), not by a second specify: once the current revision amends an earlier one, a plain specify is refused (`lineage-exists`), and `--fresh` is the deliberate way to start the history over. This skill installs or refreshes the CLI, elicits arguments, invokes the verb, re-projects the staged candidate, relays its output, and offers the seal.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling. Install the latest prebuilt release via Homebrew, or from source; an adapter whose declared minimum `emery-version` outruns the installed binary fails typed later (`unsupported-version`, exit 1) with the same reinstall command as its hint:

```bash
brew tap augentic/tap
brew install emery
# or: cargo install --git https://github.com/augentic/emery --locked
```

Then run `emery --version` and stop on failure.

2. **Elicit every required input and pass it as a flag** — the CLI has no interactive prompt mode: no source at all — and no project-root `emery.toml` to discover — fails typed (`specify-source-required`). Gather conversationally: the source adapters to extract (each positional `<adapter>` is a workspace-backed source; each `--description <adapter>=<text>` is an inline source such as an operator directive). An adapter is an exact package reference with a version — `emery:typescript@0.13.0`, never `emery:typescript`, `typescript`, or a path; one without a version or a namespace fails typed (`adapter-reference`, exit 1) before anything loads, so ask for the version when the operator names none. An operator who keeps a config file selects it with `--config [<path>]`; omit the value only for the project-relative `emery.toml`, and a run naming no sources at all discovers that file on its own. Positional adapters and `--description` beside `--config` layer over the file: one named as a `[[source]]` is — by its adapter's package name — takes that entry's place whole, and any other is appended, so pass both when the operator wants one entry swapped for another adapter or a source added for the run and the rest kept as the file has them. Local paths must stay relative to the project and must not escape it. The `[target]` table of the same file names the repository the specification lives in; a run that knows one reads the current revision from it first, for the diff and for `--apply`, and an argv-only run knows none, so it stages and cannot seal.
3. **Invoke**:

```bash
emery specify <adapter>... [--description <adapter>=<text>]
# or: emery specify --config [<path>]
# or: emery specify <adapter>... --config [<path>]   # layered over the file's sources
# or: emery specify --config [<path>] --apply        # stage and seal in one run, when the operator asks for it
# or: emery specify --config [<path>] --fresh        # start the history over, when the operator asks for it
```

Specify dispatches model judgment and can take a while on large workspaces. Tracing follows the plugin rule's *Tracing and output* contract (a bare run is `info`; pass `-v` when the operator asks for debug). Pass `--apply` only when the operator asked to seal without reviewing first: it seals the candidate in the `[target]` repository as `/emery:apply` would, and a refused seal leaves the candidate staged for it. Pass `--fresh` only when the operator asked to start over: it stages the candidate marked fresh, so the seal writes a revision with no parent over the repository's head whatever revision stands there, every earlier label left standing; the earlier history is then unreachable through `parent` and a later amendment reads the fresh revision as its base. One thing to say when it happens: unchanged sources re-derive the same id, so a fresh seal of documents the repository already holds under an earlier label moves that label onto the new commit.

## Re-project

After every successful run, write the staged candidate's Markdown projections beside the code for review:

```bash
emery show spec --staged > spec.md
emery show design --staged > design.md
emery show plan --staged > plan.md
```

Under `--apply` the candidate is the current revision once sealed, so drop `--staged`. Track all three files in version control. Never edit them by hand: they are projections of the staged candidate, and a hand edit is overwritten by the next run (change a source and re-run instead). Never edit `spec/` in the target repository either: its three documents are the sealed revision, held to the id their label names.

## Relay

- Surface the CLI output verbatim — text names the staged candidate, the plan's shape (`  plan: 3 slices in 2 waves, widest 2`), a one-line diff summary against the current revision where the `[target]` repository holds one, and ends ``nothing sealed; `emery apply` seals it`` or, under `--apply`, `sealed revision <id> as emery/<id> at <commit>`; the full per-requirement and per-slice diff, and the waves themselves, ride `--format json`.
- Review is `spec.md` / `design.md` / `plan.md` as re-projected, or `emery show spec --staged` / `emery show design --staged` / `emery show plan --staged` directly — never read or edit `.emery/storage` state by hand.
- Once the operator has reviewed the candidate, offer [`/emery:apply`](../apply/SKILL.md) to seal it: `emery build` reads the sealed revision alone and refuses while only the candidate stands (`spec-not-generated`, naming it).
- On non-zero exit, surface the structured error and stop — never hand-roll spec documents. A `refused` failure means the loader rejected the request (an invalid or pre-compiled artifact, a `digest` the release does not resolve to, an export signature that does not match this binary's contract, or a release under a namespace the binary routes nowhere, which the hint says to fetch with `wkg get <reference> -o ~/.emery/adapters/`); relay the hint and let the operator decide. A `spec-mismatch` failure means `spec/` in the target repository is not a sealed revision — a document edited there, the operator's own directory, or three documents edited or merged by hand: relay the hint (restore with `git checkout emery/<revision> -- spec/`, or move the directory aside; or keep one side's three files whole with `git checkout <branch> -- spec/` and commit) and let the operator decide, never editing or moving it through this skill. A `lineage-exists` failure means the current revision amends an earlier one: relay the hint ([`/emery:amend`](../amend/SKILL.md) to stage a change against it, or `emery specify --fresh` to start over) and let the operator decide. An `amendment-pending` failure means an amendment is staged, which a specify never displaces: relay the hint (`emery show amendment` reviews it, `/emery:apply` seals it, `/emery:amend` replaces it) and let the operator decide. A `build-target-required` failure under `--apply` means the run knows no `[target]`: relay the hint and ask for the table. A `repository-required` or `base-not-sealed` failure means the `[target] repository = "."` checkout is no repository or has no commit: relay the hint and let the operator seal it. A `spec-outdated` failure means the stored revision predates this binary's grammar: relay the hint (re-run `emery specify` to regenerate) and let the operator decide.
