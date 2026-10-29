---
name: emery-amend
description: Stage a change against the sealed revision by invoking `emery amend` over the named change sources, relay what it classed, and offer the review and the seal. Use whenever the operator wants to revise, withdraw, or add to a sealed specification without regenerating it.
argument-hint: [<adapter>] --config [<path>]
---

# Amend Skill

`emery amend` is the one amend verb: it reads the named sources as a **change** against the **current revision** — the one [`/emery:apply`](../apply/SKILL.md) sealed in the target's repository — matches each claim to the requirement it revises, withdraws, or adds beside, and stages an **amendment** in `.emery/storage`: every requirement of the revision classed `added`, `changed`, `retired`, `conflict`, or `unchanged`, the specification redrafted for the touched requirements alone under the ids they keep, the design and plan derived from the current ones. `emery apply` seals it as the next revision over the current one, its parent the revision it amends, and `emery build` builds the amended plan under the new label. A sealed revision is amended, never regenerated: `emery specify` over a revision that itself amends an earlier one is refused (`lineage-exists`), and this skill is the way on. This skill installs or refreshes the CLI, elicits the change sources and the target, invokes the verb, relays what it staged, and offers the review and the seal; it edits nothing and commits nothing itself.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling:

```bash
brew tap augentic/tap
brew install emery
# or: cargo install --git https://github.com/augentic/emery --locked
```

Then run `emery --version` and stop on failure. An amend reads the current revision from a repository, so it needs `git` 2.5 or newer on `PATH`.

2. **Elicit the change and pass it as flags** — the CLI has no interactive prompt mode. Gather conversationally what the change *is*: the edited documentation re-read (a positional `<adapter>`, a workspace-backed source), a brief stating one behaviour (`--description <adapter>=<text>`, an inline source), or a code tree the behaviour moved in. An adapter is an exact package reference with a version, `emery:documentation@1.2.0`, never a bare name or a path (`adapter-reference`, exit 1, otherwise). A run that names a source on the command line reads no `emery.toml` unless `--config` says so, and the `[target]` table is where the current revision is read from, so **pass `--config`** — bare for the project-root file, `--config <path>` for a change file of its own — on every run that names a source; without it the run is refused (`build-target-required`, exit 1). `--config` alone reads the file's `[[source]]` entries as the change, the edited documentation under the entries that name it. A positional adapter and a `--description` for the same adapter bind one source name and are refused as a source named twice, so an inline change over an adapter the file names is `--description` alone beside `--config`. A `[[source]]` with `scope = "whole"` names a behaviour source read whole, so a standing requirement under a stem it states and no longer states is retired by its silence; ask for it only when the operator means the code to be the whole statement of its subject. No source may be named `baseline`.
3. **Check what stands** — an amend needs a sealed revision in the `[target]` repository (`spec-not-generated`, exit 2, otherwise: seal the staged candidate with `/emery:apply` first, or run `/emery:specify` then `/emery:apply`), and the staging slot free of a revision candidate (`amendment-pending`, exit 1: seal it with `/emery:apply`, or review it with `emery show <artifact> --staged`). A staged amendment is replaced freely.
4. **Invoke**, in JSON mode, so the relay can read what was classed:

```bash
emery --format json amend --config [<path>]
# or: emery --format json amend --config --description <adapter>=<text>     # an inline change beside the file's sources
# or: emery --format json amend <adapter> --config [<path>]               # the adapter in place of the file's entry of its name
# or: emery --format json amend --config [<path>] --apply                 # stage and seal in one run, when the operator asks for it
```

Amend dispatches model judgment — one extraction per source, one grouping, a draft per touched chunk, and a design or slicing turn where the change informs one — and can take a while. Tracing follows the plugin rule's *Tracing and output* contract (a bare run is `info`; pass `-v` when the operator asks for debug). Pass `--apply` only when the operator asked to seal without reviewing first: it seals the amendment as `/emery:apply` would, and a refused seal leaves it staged.

## Review

After a run that staged an amendment, project it for review:

```bash
emery show amendment > amendment.md
```

The projection opens with the base and the counts by class, then one section per class — `## Added`, `## Changed` (each with `Before:` and `After:`), `## Retired`, `## Conflict`, `## Unchanged` — and, where the design or the plan changed, `## Design` with each changed or added section whole and `## Plan` with each added, changed, or removed slice. A `## Conflict` entry carries both competing statements with `Note: Operator reconciliation required.`: the amendment can be sealed with it, and the requirement then stands `[conflict]` in the revision until the next change makes the sources agree. A retired entry carrying ``Note: retired by scope: …`` was retired by a source read whole falling silent on it, not by a claim; confirm it is meant. A changed slice noted `Note: the brief is the base's; no slicing turn redrafted it.` was changed by code alone, so its brief is read with care. Never edit `amendment.md`, `.emery/storage`, or `spec/` in the target repository: the amendment is replaced by re-running `emery amend` over changed sources.

## Relay

A successful run's envelope carries `amendment` (the staged amendment's id; absent when the change staged nothing), `staged`, `base` (`{ revision, commit }`), `tally` (`{ added, changed, retired, conflict, unchanged, by-scope }`), `conflicts` and `stems` (when any), `sections` (what the design changes) and `slices` (what the plan changes, each `{ id, name, class, brief }`) when any, `repositories`, and under `--apply` `applied`, the `emery apply` body with `kind: "amendment"`. Relay it, and offer the ways to take it:

- Surface the CLI output verbatim — `staged amendment <id>`, the base, the counts line (`1 added, 1 changed, 0 retired, 0 conflict, 4 unchanged`, ` (N by scope)` after the retired count when any), `conflicts:`, `stems:`, `design:`, and `slices:` when any, and ``nothing sealed; `emery show amendment` reviews it`` — or, under `--apply`, the seal lines `sealed revision <id> as emery/<id> at <commit>` and `amends <parent>`.
- A run printing `nothing staged: no change against the current revision` found every requirement unchanged and nothing of the design or plan different; relay it and stop, since there is nothing to review or seal.
- Once the operator has reviewed the amendment, offer [`/emery:apply`](../apply/SKILL.md) to seal it: it seals the three documents over the commit the base was read at, `Parent:` the base revision and the counts and slices as trailers, and the base's label stands beside the new one. A `conflicts:` line is no reason to withhold the seal, but say what it means: the revision seals with the requirement marked `[conflict]`, a revision to resolve by the next change rather than build.
- Never edit `spec/`, the projections, or `.emery/storage` through this skill, and never commit: the seal is `/emery:apply`'s and the commit the engine's.
- On non-zero exit, surface the structured error and stop. A `specify-source-required` failure means no source and no change file was named: ask for the change. A `build-target-required` failure means the run named its sources on the command line and read no file: re-run with `--config`. A `spec-not-generated` failure means no revision is sealed to amend — none yet, or a candidate staged and not sealed (the message names it): relay the hint (`/emery:apply`, or `/emery:specify` then `/emery:apply`, then amend again) and let the operator decide. An `amendment-pending` failure means a revision candidate holds the slot: relay the hint (`/emery:apply` seals it; `emery show <artifact> --staged` reviews it) and let the operator decide. A `retire-unmatched` failure means a source withdraws a behaviour the revision does not state: relay the named claim and its hint (compare its id against `emery show spec`; reword the source to withdraw a requirement it prints, or drop the withdrawal) and let the operator change the source. A `spec-mismatch` failure means `spec/` at the repository's head is not a sealed revision — a document edited under the label, the operator's own directory, or three documents edited or merged by hand, as a merge of two branches that each sealed one leaves them: relay the hint (restore with `git checkout emery/<revision> -- spec/` or move the directory aside; or keep one side's three files whole with `git checkout <branch> -- spec/`, commit, and amend again) and let the operator decide, never moving or editing it through this skill. An `amendment-stale` failure under `--apply` means the current revision moved between the stage and the seal: relay the hint (re-run `emery amend`, then `emery apply`) and let the operator decide. A `spec-outdated` failure means the sealed revision predates this binary's grammar: relay the hint (`/emery:specify` regenerates it) and let the operator decide. A `refused`, `unavailable`, `adapter-reference`, or source `bad_request` failure is as under `/emery:specify`: relay the hint and let the operator decide.
