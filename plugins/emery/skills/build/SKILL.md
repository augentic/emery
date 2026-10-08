---
name: emery-build
description: Build every slice of the current plan by invoking `emery build` through a target adapter, then relay the commits it sealed under the label `emery/<revision>`. Use whenever the operator wants the committed plan built.
argument-hint: [<adapter>]
---

# Build Skill

`emery build` is the one build verb: it reads the current revision, loads the one target adapter the run names — an exact package reference, `namespace:name@version`, read from the store `~/.emery/adapters` and fetched into it on the first run that names it — and builds the plan in waves. The build lands as commits, never in the checkout: starting from the project's sealed head (or from the branch of the repository a `[target] repository` names), the engine builds every slice whose dependencies are merged at once, each in a working copy of its own, seals what each changed as one commit and merges it in id order into an integration working copy under the adapter's merge rules, has the adapter verify the integrated tree, and labels the verified head `emery/<revision>` in the repository the base came from — wave after wave, until every slice is merged — then pushes the label to the `[target] remote` when one is configured and removes the copies. A label the repository already holds for the revision is resumed: the slices it records are not built again. The checkout the operator is working in is read for its head and never written. This skill installs or refreshes the CLI, elicits arguments, invokes the verb, and relays what it committed; it commits nothing itself.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling. Install the latest prebuilt release via Homebrew, or from source; an adapter whose declared minimum `emery-version` outruns the installed binary fails typed later (`unsupported-version`, exit 1) with the same reinstall command as its hint:

```bash
brew tap augentic/tap
brew install emery
# or: cargo install --git https://github.com/augentic/emery --locked
```

Then run `emery --version` and stop on failure. A build also needs `git` 2.5 or newer on `PATH`.

2. **Elicit the target adapter and pass it as a flag** — the CLI has no interactive prompt mode: no adapter at all — and no project-root `emery.toml` carrying a `[target]` table — fails typed (`build-target-required`, exit 1). Gather conversationally: the one target adapter to build through, an exact package reference with a version — `acme:rust@1.4.0`, never `acme:rust`, `rust`, or a path; one without a version or a namespace fails typed (`adapter-reference`, exit 1) before anything loads, so ask for the version when the operator names none. An operator who keeps a config file selects it instead with `--config [<path>]`; omit the value only for the project-relative `emery.toml`, and a run naming no adapter discovers that file on its own. Never combine the file carrier with a positional adapter (mixing fails typed, exit 1). The repository the build lands in, and the remote it pushes to, are the config file's alone (`[target] repository` with `branch`; `remote`): a positional adapter builds into the project's own repository and pushes nowhere. A build needs a committed revision: before any is committed the run fails typed (`spec-not-generated`, exit 2), so run `/emery:specify` first.
3. **Check the base** — without a `[target] repository`, the base is the project checkout's head, and the run is refused (`base-not-sealed`, exit 1) when the checkout holds an uncommitted change outside `.emery/` and `.git/`, or has no commit yet. Run `git status --short` first; if it shows anything, tell the operator what is pending and let them commit or stash it — never commit or stash on their behalf. A directory that is no repository at all is refused too (`repository-required`, exit 1); the engine never runs `git init`, so ask the operator to initialise and seal the project before building.
4. **Invoke**, in JSON mode, so the relay can read what was committed:

```bash
emery --format json build <adapter>
# or: emery --format json build --config [<path>]
# or: emery --format json build --jobs 2 <adapter>   # at most two slices of a wave at once
```

Build dispatches one model turn per slice, the slices of a wave at once, and one verify turn per wave, and can take a while on a deep plan. `--jobs <N>` (`-j`, or `EMERY_JOBS`) caps how many slices of a wave build at once when the operator asks for it; unset builds the whole wave at once, and the cap changes the pace alone, never the history. Tracing follows the plugin rule's *Tracing and output* contract (a bare run is `info`; pass `-v` when the operator asks for debug).

## Relay

A successful run's envelope carries `revision`, `waves`, `base`, `resumed` (when a label was resumed), `slices`, `verified`, `head`, `label`, and, when a remote took it, `pushed`; each slice is `{ id, name, wave, covered, uncovered, written, commit, conflicts }` in merge order, `commit` being the merge commit that brought it in, `null` for a slice that changed nothing, and `conflicts` the paths an earlier build of it conflicted at before it was built again. Relay it, and offer the ways to take it:

- Surface the CLI output verbatim — the revision built, the plan's waves, the base commit, the slices resumed, each wave's slices with their covered and uncovered requirement ids, written files, merge commit, and conflicts, the head each wave was verified at, and the label at its head — then the commands that read it: `git log --oneline --first-parent <base>..<label>` for the merges, one per slice and a `Wave <k> verified` commit where a check left something behind, `git diff --stat <base> <label>` for the files, `git switch <label>` or `git merge <label>` to take them.
- When `pushed` names a remote, offer a pull request from the label onto the branch the base came from, and open it only when the operator asks:

```bash
gh pr create --head emery/<revision> --base <branch> --title "emery build <revision-8>" \
  --body "Revision: <revision>"
```

- A requirement the adapter left out of `covered` is reported `uncovered`, never implemented by hand: relay it and let the operator decide whether to re-run, change a source and re-run `/emery:specify`, or take it up themselves.
- Never commit, amend, rebase, or rewrite anything through this skill: every commit of a build is the engine's, sealed with the revision, the requirements, the adapter, and the base in its message, and `spec.md`, `design.md`, `plan.md`, and `.emery/` stay out of it — the projections belong to `/emery:specify`, and the revision store is never tracked.
- On non-zero exit, surface the structured error and stop. The first failure ends the run, and the label stays at the last verified wave — unset when none verified — so the next `emery build` resumes from there rather than starting over; the message names the failed slice and its wave, or the failed wave, and says the slices merged before it stay committed in `./.emery/vcs/integration`, which the operator may inspect (`git -C .emery/vcs/integration log --first-parent`) and the next build removes. A `verify-failed` failure lists the checks the adapter's verify turn failed over the integrated tree: relay them and the hint — inspect the integration copy, re-run to rebuild the wave from the last verified head, or `git bisect` over `emery/<revision>` to attribute a failure to a slice — and let the operator decide; never fix the tree by hand. A `slice-conflict` failure names a slice whose merge conflicted twice and the paths: two slices rewrite the same files, which is the adapter's merge rules or the plan to settle, so relay the hint (a merge rule in the adapter, or a re-run of `/emery:specify` for a plan that keeps them in one slice) rather than resolving the conflict. A `build-target-required` failure means no adapter was named and no `[target]` table was found: relay the hint and ask for the adapter. A `base-not-sealed` failure lists the pending paths: relay them and let the operator seal the checkout. A `repository-required` failure means the project is no repository: relay the hint. A `spec-not-generated` or `spec-outdated` failure means there is no revision this binary can build: relay the hint (`/emery:specify` first, or again) and let the operator decide. A `revision-not-found` failure means the `[target] branch` is not in the repository: relay the hint. A `refused` failure means the loader rejected the request (an invalid or pre-compiled artifact, a `digest` the release does not resolve to, or a release under a namespace the binary routes nowhere, which the hint says to fetch with `wkg get <reference> -o ~/.emery/adapters/`); relay the hint and let the operator decide.
