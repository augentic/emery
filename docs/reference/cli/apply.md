# emery apply

Seal the staged candidate as the current revision in the repository the specification lives in.

## Synopsis

```bash
emery apply                          # the [target] of the project-root emery.toml
emery apply --config [<path>]
emery apply --repository <url|.>     # in place of the [target] repository
```

## Description

The one seal verb. [`emery specify`](specify.md) drafts a revision and **stages** it as the one candidate; `apply` takes that candidate and **seals** it as the current revision, the one [`emery show`](show.md) renders and [`emery build`](build.md) builds. The revision does not live in the project directory: it lives in the **target repository** — the repository the `[target]` table of `emery.toml` names, where a build lands too — as the three documents `spec/spec.json`, `spec/design.json`, and `spec/plan.json` at one commit, `Revision <id>`, labelled `emery/<id>`. `.emery/storage` keeps only the staged candidate and a pointer to the label, a cache the repository bears out.

Which repository that is follows from the `[target] repository`, or the `--repository` standing in for it, exactly as for `emery build`:

- A URL: the revision is sealed in a clone of it kept under `.emery/vcs/repos/<name>` — `<name>` the table's `name`, or the adapter's package name when the table gives none — cloned when the run has none and fetched when it has one, over the repository's default branch as the origin holds it, `origin/HEAD`. Nothing is pushed. A URL that names no repository, or one with no default branch, is `revision-not-found` (exit `2`); a remote that refuses or cannot be reached is `bad_gateway` (exit `4`).
- `"."`: the revision is sealed in the project's own repository, over its head. The invocation directory must be a repository (`repository-required`, exit `1`) with a commit (`base-not-sealed`, exit `1`); what the checkout holds pending does not matter to a seal, since the checkout is never touched.
- None: the revision is sealed in a repository the engine **creates** at `.emery/vcs/repos/<name>` and keeps, the `Revision` commit its root; every later verb finds it there. The project directory is never read as a repository, so it need not be one.

The seal is one commit. The engine reads where the repository stands for the revision — the commit the current revision is at, else the head — cuts a working copy of it at `.emery/vcs/revision`, writes the three documents under `spec/`, holds what changed to the secret gate, and seals it under a message whose first line is `Revision <id>` and whose trailers are `Revision:`, `Parent:` (the id of the revision this one displaced, when there was one), and `Base:` (the commit it was sealed over). The commit is labelled `emery/<id>`, a branch in that repository; the pointer in `.emery/storage` is swapped to it; the staged candidate is cleared; the working copy is removed. The branch the repository is on, and the checkout in the project directory, are never moved or written: to carry the specification on the branch, merge or rebase the label onto it. A revision the repository already holds — a candidate identical to the current revision — seals nothing and stands labelled as it was.

`spec/` is the engine's. A `spec/` the head already holds that is not exactly the three documents of a revision — a `spec/notes.md`, a file or a link at that path, an empty directory — is the operator's, and the seal is refused `spec-mismatch` (exit `1`) naming what the directory holds, the candidate left staged; move the directory aside and run `apply` again. A `src/spec/` deeper in the tree is the project's own and never read. The same reading refuses `emery show`, `emery build`, and a `specify` that knows the target, since each reads the revision from there first.

A refused seal leaves the candidate staged. `apply` with nothing staged fails typed with `nothing-staged` (exit `2`); run `emery specify` first, and review the candidate with `emery show <artifact> --staged`. `apply` with no `[target]` — no `--config`, and no project-root `emery.toml` carrying the table — fails typed with `build-target-required` (exit `1`): the specification has no repository to live in. A candidate or a current revision under another grammar than this binary's is `spec-outdated` (exit `1`). A document that carries a credential is `secret-detected` (exit `1`), nothing sealed.

`emery specify --apply` stages and seals in one run: the stage lands first, so a seal refused for any of the above leaves the candidate staged for a later `emery apply`.

This is the CLI command invoked by [`/emery:apply`](../../../plugins/emery/skills/apply/SKILL.md), which relays the label and the commit; the CLI itself has no interactive mode.

## The `[target]` table

`emery.toml` is the operator-owned file every verb reads. Its `[target]` table names the repository the specification lives in and the adapter a build runs through; `apply` reads the `name` and `repository` alone and loads nothing. The table is described under [`emery build`](build.md#the-target-table): `name` is the directory a clone or a created repository is kept in, `.emery/vcs/repos/<name>`, which belongs to whatever first cloned or created it; `repository` is one URL, or `"."`, or omitted. A run naming no `--config` discovers the project-root `emery.toml`; `--config` without a value names the project-relative `emery.toml`, and an explicit value names another file.

## Options

| Option                     | Description                                                                                                                                                              |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `--repository <url\|.>`    | The repository the revision is sealed in: a URL, cloned under `.emery/vcs/repos/<name>`, or `.` for the project checkout; beside `--config`, it takes the place of the `[target] repository`. |
| `--config [<path>]`        | Operator-owned config naming the `[target]`; the omitted value selects `emery.toml` (`-c` for short). A run naming no `--config` discovers the project-root file.          |
| `--format`                 | Global output format: `json` for structured automation output.                                                                                                           |

## JSON output

When `--format json` is provided, returns:

- `revision` — the id of the revision sealed, now current
- `parent` — the id of the revision it displaced; omitted on the first seal
- `commit` — the `Revision <id>` commit the three documents stand at
- `label` — the branch set on that commit, `emery/<revision>`
- `repository` — the repository the revision was sealed in: `.` for the project checkout, else `./.emery/vcs/repos/<name>`
- `kind` — what was sealed; `revision`, the one kind
- `diff` — the candidate's diff against the revision it displaced, in the shape [`emery specify`](specify.md#json-output) reports; omitted on the first seal (see [CLI output shapes](../cli-output-shapes.md#emery-apply))

Text mode prints `sealed revision <id> as emery/<id> at <commit8>`, then `  repository ./.emery/vcs/repos/<name>` when the seal landed outside the project checkout, then the one-line diff summary when a revision was displaced.

## See also

- [`emery specify`](specify.md) stages the candidate this verb seals; [`emery show`](show.md) renders the sealed revision, and [`emery build`](build.md) builds its plan; see the [CLI reference](index.md).
