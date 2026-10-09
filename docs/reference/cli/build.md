# emery build

Build every slice of the current plan through a target adapter, into a labelled commit.

## Synopsis

```bash
emery build <adapter>
emery build --config [<path>]
emery build                         # discovers the project-root emery.toml
emery build --jobs 2                # at most two slices of a wave at once
```

## Description

The one build verb. A run names one **target adapter** — an exact package reference, `namespace:name@version` ([Adapters](../adapters.md)) — on the command line, or in the `[target]` table of an operator-owned `emery.toml` selected with `--config [<path>]` (`-c`; the omitted value selects the project-relative `emery.toml`). A run naming no adapter discovers the project-root `emery.toml` as a fallback; one that finds no `[target]` there fails typed with `build-target-required` (exit `1`). Combining `--config` with a positional adapter fails as `bad_request` (exit `1`).

The run reads the current revision first: before any is committed it fails typed with `spec-not-generated` (exit `2`), and a stored revision written under an older grammar fails with `spec-outdated` (exit `1`), both before the adapter loads. The adapter then loads as a source adapter does under [`emery specify`](specify.md) — through the deployment's `omnia:plugins/loader` capability, from the store `~/.emery/adapters` or, when the store lacks the release, the registry the binary routes its namespace to, a `digest` riding the load as its pin — and is held to its axis before any dispatch: a component exporting no `emery:adapter/target` (a source adapter) is `bad_request` (exit `1`) naming the interface it lacks, and its `emery-version` pin is gated the same way (`unsupported-version`, exit `1`).

The build then settles its **base**, the commit it starts from. Without a `[target] repository`, the base is the project checkout's own head: the invocation directory must be a repository (`repository-required`, exit `1`, otherwise) whose checkout holds no pending change — a modified, added, or deleted path outside `.emery/` and `.git/` — and has a commit; a checkout that fails either is `base-not-sealed` (exit `1`), the pending paths listed, and nothing is built. With a `[target] repository`, the base is the commit its `branch` names in a clone of the repository kept under `.emery/vcs/repos/`, cloned when the run has none and fetched when it has one; a branch the repository lacks, or a URL that names no repository, is `revision-not-found` (exit `2`), and a remote that refuses or cannot be reached is `bad_gateway` (exit `4`).

The run then reads what the label `emery/<revision>` already holds over the base, and **resumes** from it. The label is a branch, and it is read back as a local branch alone: a tag of that name, or a branch a remote carries under it, is never the label, and a fetch writes no local branch, so the label a run resumes from is one a build set in the repository it reads, or one whoever can write that repository's branches put there. A label that does not descend from the base is not this base's history and starts the build from the base; one that descends from it is resumed from: the merge commits an earlier run of this revision sealed beneath it name the slices it integrated, and those are not built again, the run starting from the labelled head with the slices it skipped listed as `resumed`; a label beneath which nothing of this revision is sealed is still the head every slice builds over. A revision never built starts from the base. From that head the engine cuts the **integration working copy**, `.emery/vcs/integration`, on no branch; one an earlier run left behind is removed first.

The slices still to build go in **waves**. A wave is every unbuilt slice whose `Depends on:` line names only slices already merged — so a slice waiting on nothing is in the first wave whatever its id, and each slice follows every slice it depends on. The slices of a wave are built **at once**, up to `--jobs` concurrently, each in a working copy of its own, `.emery/vcs/worktrees/<id>`, cut at the wave's head. One `build` dispatch per slice hands the adapter the slice's plan entry, the specification cut to the slice's requirements (the preamble and those requirements, rendered as `emery show spec` renders them), the whole design, the commit its tree sits on, and the working copy as the tree to build into. The adapter returns a **report**: the requirement ids it covered and the files it wrote. The engine holds the report to the slice — a covered id must be one of the slice's requirements, named once; a written path must be a relative `/`-separated path beneath the working copy's root, outside `.emery/` and `.git/`, named once — and a report that breaks a rule is Emery's own finding, `server_error` (exit `3`). A requirement the adapter leaves out of `covered` is reported as uncovered, never invented.

Once the wave is built, each slice is **integrated** in id order, whatever order the builds finished in. Its working copy must still sit on the commit it was cut at: the model's shell reaches the repository, and a commit it sealed or moved there would leave nothing pending and the slice merged as nothing, so a copy whose head moved ends the run as `server_error` (exit `3`), both commits named, with nothing of that slice sealed and the copy left for inspection. Then what it changed is sealed as one commit in its working copy — its message the slice's id and name over trailers naming the slice, the revision, its requirements, the ones covered, the adapter, the base it built over, and the wave — and that commit is **merged** into the integration working copy as a merge commit carrying the same message, under the adapter's **merge rules** (a glob of paths and how both sides are kept there: `union` keeps both sides' lines, `ours` the integrated side, `theirs` the slice's); the slice's working copy is then removed. A slice that changed nothing merges nothing. A merge that **conflicts** at a path no rule resolves is not made: the slice is left unmerged, the paths recorded, and it is built again in the next wave over the merged head; a second conflict ends the run as `slice-conflict` (exit `1`), the paths named.

The adapter then **verifies** the integrated tree: one `verify` dispatch over the integration working copy answers a verdict, `passed` or the checks that failed. A verdict that fails is `verify-failed` (exit `1`), the failures listed, and the run ends. One that passes seals whatever the checks left behind in the working copy, if anything, as a `Wave <k> verified` commit, and the integrated head is **labelled** `emery/<revision>` — a branch in the project repository or the clone, moved to each verified wave's head — so the label always stands at a verified tree, and the next wave is cut from it. Once every slice is merged, the label is **pushed** to the `remote` when the `[target]` table names one, never forced — a remote the repository lacks is `revision-not-found` (exit `2`), one that refuses or cannot be reached `bad_gateway` (exit `4`), and one whose `emery/<revision>` has moved on, holding commits this build does not, `label-diverged` (exit `1`), the label standing locally and nothing written to the remote — and the integration working copy is removed. A push's failure names the remote and where the label stands, so the next `git log --first-parent` has its arguments, and the next run resumes from the local label with nothing rebuilt and pushes it again. The project checkout is never written: the operator reads the build with `git log --first-parent emery/<revision>`, takes it with `git merge` or `git switch`, or opens a pull request from the pushed label. A tag of the label's name shadows that bare spelling in the operator's own git too, so a repository one does not trust is read as `refs/heads/emery/<revision>`.

The first failure ends the run. A slice's failure is returned as the adapter put it — a refusal of the slice is `bad_request` (exit `1`), an upstream failure `bad_gateway` (exit `4`) — with the slice and its wave named in the message and, where earlier slices were merged, the note that their commits stay in `.emery/vcs/integration`; a wave's failure names the wave, the slices merged in it, and where the label stands; a push's names the remote and where the label stands. Either way the integration working copy, and any slice working copy a failed wave left, stay for inspection and are removed by the next run, which resumes from the last verified wave. The engine writes no state of its own: the labelled history is the build's output.

This is the CLI command invoked by [`/emery:build`](../../../plugins/emery/skills/build/SKILL.md). The skill elicits any missing arguments conversationally, passes them as flags, and relays the label, the waves, the per-slice merges, and the push; the CLI itself has no interactive mode.

### The trees a build touches

The runtime mounts the invocation directory as `.`, writable: the tree `specify` reads its sources from and whose `.emery/` a build keeps its clones and working copies under. The store, `~/.emery/adapters`, lies apart from the project and is no mount at all, so a component is never loaded from a directory a run can write (see [Deployment profiles](../deployment-profiles.md)). Version control runs through the deployment's `omnia:vcs` capability, `git` on the host behind it; nothing of it reaches a guest but the operations above.

A build lends each slice's working copy to the adapter's model turn for that slice. The model reads it through the host's workspace tools, which are read-only, and writes it through the SDK's `write_file` tool alone: one file per call, created or replaced whole, at a `/`-separated path relative to the working copy's root. The tool refuses a path outside the root, one under `.emery/` or `.git/`, and one naming `spec.md`, `design.md`, or `plan.md`, so the revision store, the repository's own directory, and the projections are never written by a build; the mount beneath permits the write, the tool does not. Before the turn answers, the SDK holds the report to the tree: a `written` path the tree does not hold, or a file `write_file` wrote that `written` leaves out, is returned to the model as a correction, so a report of files never written does not reach the engine. The verify turn is lent the integration working copy with the shell and no `write_file` tool: it runs the adapter's checks and answers what it saw, and whatever a check leaves behind is sealed as the wave's own commit.

Add `.emery/` to the project's `.gitignore`: the revision store and the working copies beneath it are never counted against the base, but an un-ignored `.emery/` is noise in every `git status`.

## The `[target]` table

`emery.toml` is the same operator-owned file [`emery specify`](specify.md) reads. Its `[target]` table names the one adapter a build runs through — an exact package reference — and, optionally, the `sha256:` digest its release must resolve to, the repository the plan is built into, and the remote the label is pushed to. A run naming its adapter on the command line reads no file at all and builds into the project repository, pushing nowhere. The file is read whole, so an unknown key under any table is a parse error naming the key (`bad_request`), whichever verb reads the file. The adapter is read from the store and fetched into it as under `specify` ([Adapters](../adapters.md)); the project file names which package and which bytes, never where from.

```toml
[target]
adapter = "acme:rust@1.4.0"
digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
repository = "git@github.com:acme/shop.git" # optional: build into a clone of this repository
branch = "main"                              # required with `repository`: the commit to build from
remote = "origin"                            # optional: push the label here once the build integrates
```

- `repository` and `branch` go together: a repository is built from its branch's commit, resolved on every run, so a `repository` without a `branch`, or a `branch` without a `repository`, is refused at the file (`bad_request`). The clone is keyed by the URL, normalised — `https://github.com/acme/shop.git`, `https://github.com/acme/shop/`, and `git@github.com:acme/shop` share one — and lives under `.emery/vcs/repos/`, shared with any `[[source]]` naming the same repository. Credentials are the host's `git`'s: whatever lets `git clone <url>` run where `emery` runs.
- `remote` stands alone: without a `repository` it names a remote of the project repository, so a greenfield project with an `origin` pushes its label there.

## Options

| Option                      | Description                                                                                                                                                             |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<adapter>` (positional)    | The target adapter, an exact package reference `namespace:name@version` ([Adapters](../adapters.md)).                                                                   |
| `--config [<path>]`         | Operator-owned config; the omitted value selects `emery.toml` (`-c` for short). Mutually exclusive with the positional adapter.                                        |
| `--jobs <N>`                | How many slices of a wave to build at once (`-j` for short; the environment's `EMERY_JOBS`). Unset builds every slice of a wave at once; `0` is a usage error (exit `64`). The cap bounds concurrency alone: every slice of a wave builds over the wave's head whatever it is, so a cap of one yields the same history. |
| `--format`                  | Global output format: `json` for structured automation output.                                                                                                          |

## JSON output

When `--format json` is provided, returns:

- `revision` — the id of the revision whose plan was built
- `waves` — the plan's slices grouped into the sets ready to build at once, a list of lists of slice ids, as [`emery specify`](specify.md#json-output) reported them when the revision was committed: the plan's own projection, which a conflicted slice's rebuild in a later wave departs from
- `base` — the commit the build started from
- `resumed` — the slice ids the label already held, not built again, in id order; omitted when none
- `slices` — every slice this run built, in merge order, each `{ id, name, wave, covered, uncovered, written, commit, conflicts }`: the wave it merged in, from one; the requirement ids the adapter reported implemented, those it did not, in id order; the files it reported written relative to its working copy's root; the merge commit that brought them into the integrated head, `null` when the slice changed nothing; and the paths an earlier build of the slice conflicted at, omitted when none (see [CLI output shapes](../cli-output-shapes.md#emery-build))
- `verified` — the integrated head each wave was verified and labelled at, in wave order; omitted when every slice was resumed
- `head` — the integrated commit the label points at
- `label` — the label set on the head, `emery/<revision>`
- `pushed` — the remote the label was pushed to; omitted when the target names none

Text mode prints the revision, the plan's shape on one line — `  plan: 2 slices in 2 waves, widest 1` — the base, `  resumed: SLICE-001, SLICE-003` when any slice was skipped, then one block per wave — `  wave 1: 2 slices verified at 4e5f6a7b` over one line per slice, `    SLICE-001 authentication: covered 2/2, written 3 files, merged 4e5f6a7…` — naming any uncovered ids in parentheses, `nothing to merge` for a slice that changed nothing, and `conflicted (<paths>)` for one built again after a conflict, then `  labelled emery/<revision> at <head>` and, after a push, `  pushed to <remote>`.

## See also

- [`emery specify`](specify.md) commits the plan this verb builds; [`emery show plan`](show.md) renders it; see the [CLI reference](index.md).
