# emery build

Build every slice of the current plan through a target adapter, into a labelled commit.

## Synopsis

```bash
emery build <adapter>
emery build --config [<path>]
emery build                         # discovers the project-root emery.toml
```

## Description

The one build verb. A run names one **target adapter** — an exact package reference, `namespace:name@version` ([Adapters](../adapters.md)) — on the command line, or in the `[target]` table of an operator-owned `emery.toml` selected with `--config [<path>]` (`-c`; the omitted value selects the project-relative `emery.toml`). A run naming no adapter discovers the project-root `emery.toml` as a fallback; one that finds no `[target]` there fails typed with `build-target-required` (exit `1`). Combining `--config` with a positional adapter fails as `bad_request` (exit `1`).

The run reads the current revision first: before any is committed it fails typed with `spec-not-generated` (exit `2`), and a stored revision written under an older grammar fails with `spec-outdated` (exit `1`), both before the adapter loads. The adapter then loads as a source adapter does under [`emery specify`](specify.md) — through the deployment's `omnia:plugins/loader` capability, from the store `~/.emery/adapters` or, when the store lacks the release, the registry the binary routes its namespace to, a `digest` riding the load as its pin — and is held to its axis before any dispatch: a component exporting no `emery:adapter/target` (a source adapter) is `bad_request` (exit `1`) naming the interface it lacks, and its `emery-version` pin is gated the same way (`unsupported-version`, exit `1`).

The build then settles its **base**, the commit it starts from. Without a `[target] repository`, the base is the project checkout's own head: the invocation directory must be a repository (`repository-required`, exit `1`, otherwise) whose checkout holds no pending change — a modified, added, or deleted path outside `.emery/` and `.git/` — and has a commit; a checkout that fails either is `base-not-sealed` (exit `1`), the pending paths listed, and nothing is built. With a `[target] repository`, the base is the commit its `branch` names in a clone of the repository kept under `.emery/vcs/repos/`, fetched when the run has it and cloned when it does not; a branch the repository lacks, or a URL that names no repository, is `revision-not-found` (exit `2`), and a remote that refuses or cannot be reached is `bad_gateway` (exit `4`). From the base the engine cuts the **integration working copy**, `.emery/vcs/integration`, on no branch; one an earlier run left behind is removed first.

Every slice of the plan is then built in turn, in the order of the plan's waves flattened: the first wave is every slice that depends on nothing, each wave after it every slice whose `Depends on:` line names only slices in the waves before, and the slices of one wave come in id order — so a slice waiting on nothing is built in the first wave whatever its id, and each slice follows every slice it depends on. One `build` dispatch per slice hands the adapter the slice's plan entry, the specification cut to the slice's requirements (the preamble and those requirements, rendered as `emery show spec` renders them), the whole design, and the integration working copy as the tree to build into. The adapter returns a **report**: the requirement ids it covered and the files it wrote. The engine holds the report to the slice — a covered id must be one of the slice's requirements, named once; a written path must be a relative `/`-separated path beneath the working copy's root, outside `.emery/` and `.git/`, named once — and a report that breaks a rule is Emery's own finding, `server_error` (exit `3`). A requirement the adapter leaves out of `covered` is reported as uncovered, never invented. What the slice changed is then sealed as **one commit** in the working copy, its message the slice's id and name over trailers naming the revision, the slice's requirements, the ones covered, the adapter, and the base; a slice that changed nothing seals none.

Once every slice is built, the integrated head is **labelled** `emery/<revision>` — a branch in the project repository or the clone, pointing at the last commit, moved if the revision was built before — and, when the `[target]` table names a `remote`, the label is **pushed** there; a remote the repository lacks is `revision-not-found` (exit `2`). The working copy is then removed. The project checkout is never written: the operator reads the build with `git log emery/<revision>`, takes it with `git merge` or `git switch`, or opens a pull request from the pushed label.

The first slice that fails ends the run. Its failure is returned as the adapter put it — a refusal of the slice is `bad_request` (exit `1`), an upstream failure `bad_gateway` (exit `4`) — with the slice named in the message and, where earlier slices were built, the note that their commits stay in `.emery/vcs/integration`, which is left for inspection and removed by the next run; no label is set. The engine writes no state of its own: the labelled history is the build's output. Re-running builds every slice again from the base.

This is the CLI command invoked by [`/emery:build`](../../../plugins/emery/skills/build/SKILL.md). The skill elicits any missing arguments conversationally, passes them as flags, and relays the label, the per-slice commits, and the push; the CLI itself has no interactive mode.

### The trees a build touches

The runtime mounts the invocation directory as `.`, writable: the tree `specify` reads its sources from and whose `.emery/` a build keeps its clones and working copies under. The store, `~/.emery/adapters`, lies apart from the project and is no mount at all, so a component is never loaded from a directory a run can write (see [Deployment profiles](../deployment-profiles.md)). Version control runs through the deployment's `omnia:vcs` capability, `git` on the host behind it; nothing of it reaches a guest but the operations above.

A build lends the integration working copy to the adapter's model turn. The model reads it through the host's workspace tools, which are read-only, and writes it through the SDK's `write_file` tool alone: one file per call, created or replaced whole, at a `/`-separated path relative to the working copy's root. The tool refuses a path outside the root, one under `.emery/` or `.git/`, and one naming `spec.md`, `design.md`, or `plan.md`, so the revision store, the repository's own directory, and the projections are never written by a build; the mount beneath permits the write, the tool does not. Before the turn answers, the SDK holds the report to the tree: a `written` path the tree does not hold, or a file `write_file` wrote that `written` leaves out, is returned to the model as a correction, so a report of files never written does not reach the engine.

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
| `--format`                  | Global output format: `json` for structured automation output.                                                                                                          |

## JSON output

When `--format json` is provided, returns:

- `revision` — the id of the revision whose plan was built
- `waves` — the plan's slices grouped into the sets ready to build at once, a list of lists of slice ids, as [`emery specify`](specify.md#json-output) reported them when the revision was committed; `slices` is these flattened
- `base` — the commit the build started from
- `slices` — every slice in build order, each `{ id, name, covered, uncovered, written, commit }`: the requirement ids the adapter reported implemented, those it did not, in id order, the files it reported written relative to the working copy's root, and the commit that sealed them, `null` when the slice changed nothing (see [CLI output shapes](../cli-output-shapes.md#emery-build))
- `head` — the integrated commit the label points at
- `label` — the label set on the head, `emery/<revision>`
- `pushed` — the remote the label was pushed to; omitted when the target names none

Text mode prints the revision, the plan's shape on one line — `  plan: 2 slices in 2 waves, widest 1` — the base, one line per slice — `  SLICE-001 authentication: covered 2/2, written 3 files, committed 4e5f6a7…` — naming any uncovered ids in parentheses and `nothing to commit` for a slice that changed nothing, then `  labelled emery/<revision> at <head>` and, after a push, `  pushed to <remote>`.

## See also

- [`emery specify`](specify.md) commits the plan this verb builds; [`emery show plan`](show.md) renders it; see the [CLI reference](index.md).
