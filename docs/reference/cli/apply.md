# `emery apply`

This page is the CLI reference for `emery apply`, which commits the currently staged candidate revision or amendment into your target Git repository under `spec/` on branch `emery/<id>` and clears the staging area. It is for developers and operators sealing reviewed specifications.

## Contents

- [Synopsis](#synopsis)
- [Description](#description)
  - [Sealing a candidate](#sealing-a-candidate)
  - [Sealing an amendment](#sealing-an-amendment)
  - [`spec/` is the engine's](#spec-is-the-engines)
  - [Refusals](#refusals)
- [Options](#options)
- [JSON output](#json-output)
- [Text output](#text-output)
- [See also](#see-also)

---

## Synopsis

```bash
emery apply                          # uses [target] from project-root emery.toml
emery apply --config [<path>]        # uses [target] from the specified config file
emery apply --repository <url|.>     # overrides [target] repository
```

---

## Description

`emery apply` takes the candidate revision staged by [`emery specify`](specify.md) or the amendment staged by [`emery amend`](amend.md) and **seals** it as the current revision in the target Git repository configured under `[target]` in [`emery.toml`](../config.md).

When sealing, Emery:

1. Resolves the target repository (`"."` for the project repository, a clone of a Git URL at `.emery/vcs/repos/<name>`, or an engine-created repository at `.emery/vcs/repos/<name>` when `repository` is omitted).
2. Creates a temporary Git worktree at `.emery/vcs/revision` starting from the base commit.
3. Writes `spec/spec.json`, `spec/design.json`, and `spec/plan.json` and scans them with the [secret gate](../../explanation/security-and-sandbox.md#the-secret-gate-secret-detected).
4. Commits the three files with a structured `Revision <id>` message, sets the local branch `emery/<id>` to that commit, updates the cached pointer in `.emery/storage/`, removes the temporary worktree, and clears the staging area.

Sealing never modifies your checked-out working tree or moves your active branch. If the staged revision is byte-for-byte identical to the current revision in the repository, `emery apply` seals no new commit and simply clears the staging area.

For a conceptual walkthrough of target repository modes, commit trailers, and revision resolution, see [Revision lifecycle](../../explanation/revision-lifecycle.md#how-emery-apply-seals-a-revision).

### Sealing a candidate

A candidate staged by `emery specify` is sealed over the current revision's commit (or `HEAD` on the first seal):

- **First or single-step revision:** If the repository holds no revision, or holds a revision whose `parent` is `null`, the candidate seals cleanly (recording `Parent: <previous-id>` if a parentless revision was displaced).
- **Refusal over a lineage (`lineage-exists`):** If the current revision in the repository has a non-null `parent` (meaning it was sealed from an amendment), sealing a regular `specify` candidate is refused with `lineage-exists` (exit `1`), leaving the candidate staged.
- **Sealing a fresh candidate (`--fresh`):** A candidate staged with `emery specify --fresh` seals directly over the repository's `HEAD` commit with no `Parent:` trailer (`parent: null`), starting a new lineage while leaving all earlier `emery/<id>` branches intact.

### Sealing an amendment

An amendment staged by `emery amend` carries its amended `spec`, `design`, and `plan` (each with `parent` set to `base.revision`) alongside the exact `base` (`{ revision, commit }`) it was prepared against:

- **Base verification (`amendment-stale`):** Before writing anything, Emery verifies that the target repository's current revision is still `base.revision` at `base.commit`. If another revision was sealed or `HEAD` moved past the label in the meantime, `emery apply` fails with `amendment-stale` (exit `1`) and leaves the amendment staged so you can re-run `emery amend`.
- **Commit trailers:** The sealed commit is created on top of `base.commit` with `Revision:`, `Parent:`, and `Base:` trailers, followed by summary trailers:
  - `Added: <count>`
  - `Changed: <count>`
  - `Retired: <count>`
  - `Slices: SLICE-002 orders changed, SLICE-004 export removed` *(omitted when no slices changed)*
- **Conflicts allowed:** Requirements in `[conflict]` do not block `emery apply`; they are sealed into `spec/spec.json` with `[conflict]` status and competing statements recorded as notes so you can reconcile them in a subsequent amendment.

### `spec/` is the engine's

The root-level `spec/` directory in the target repository is reserved exclusively for Emery's `spec.json`, `design.json`, and `plan.json` files (nested directories like `src/spec/` are untouched). `emery apply` fails with `spec-mismatch` (exit `1`) if:

- **Extra files exist in `spec/`:** `spec/` holds any file, symlink, or subdirectory other than the three JSON documents. Restore the labelled files (`git checkout emery/<revision> -- spec/`) or move your files elsewhere.
- **`spec/` was edited or merged by hand:** At an unlabelled `HEAD` (such as after a `git merge` of two branches that each sealed a revision), the three JSON files fail to parse or fail cross-document consistency (`Revision::findings`, such as a slice referencing a missing requirement ID or data type). Keep one branch's three files whole (`git checkout <branch> -- spec/`), commit the merge, and re-run the other branch's `emery amend`.

### Refusals

Whenever `emery apply` refuses to seal, the staged candidate or amendment remains untouched in `.emery/storage/`.

| Error code | Exit | Cause | Recovery |
| --- | --- | --- | --- |
| `nothing-staged` | `2` | Nothing is currently staged in `.emery/storage/`. | Run `emery specify` or `emery amend` first, or pass `--apply` to either command. |
| `build-target-required` | `1` | No `[target]` table found in `--config` or the project-root `emery.toml`. | Add a `[target]` table to `emery.toml` or pass `--config <path>`. |
| `lineage-exists` | `1` | A non-fresh `specify` candidate is being sealed over a revision that has a `parent`. | Run `emery specify --fresh` then `emery apply`, or use `emery amend`. |
| `amendment-stale` | `1` | The target repository's current revision or commit no longer matches the staged amendment's `base`. | Re-run `emery amend` against the updated revision, then `emery apply`. |
| `spec-mismatch` | `1` | Root-level `spec/` contains non-revision files, tampered files, or hand-merged conflict state. | Restore `git checkout emery/<revision> -- spec/` or keep one branch's `spec/` whole (`git checkout <branch> -- spec/`). |
| `spec-outdated` | `1` | The staged item or current revision uses an older `emery` grammar stamp. | Regenerate with `emery specify`. |
| `secret-detected` | `1` | One of the staged specification documents contains a credential pattern. | Remove the secret from your source input, re-run `specify` or `amend`, and `apply` again. |
| `repository-required` | `1` | `[target] repository = "."` is set, but the project directory is not a Git repository. | Initialize a Git repository (`git init`) and create a commit, or change `[target] repository`. |
| `base-not-sealed` | `1` | `[target] repository = "."` is set, but the repository has no commits yet. | Create an initial commit in the repository and re-run `emery apply`. |
| `revision-not-found` | `2` | The `[target] repository` URL does not exist, has no default branch, or conflicts with an engine-created repository of the same `name`. | Check the URL or remove `.emery/vcs/repos/<name>`. |
| `bad_gateway` | `4` | The remote `[target] repository` could not be reached. | Check network and Git credentials and retry. |

This is the CLI command invoked by the [`/emery:apply`](../../../plugins/emery/skills/apply/SKILL.md) Cursor skill.

---

## Options

| Option | Short | Description |
| --- | --- | --- |
| `--repository <url\|.>` | — | Target Git repository where the revision is sealed (`"."` for the project repository, or a Git URL cloned under `.emery/vcs/repos/<name>`). Overrides `[target] repository` in `emery.toml`. |
| `--config [<path>]` | `-c` | Path to configuration file containing `[target]` (defaults to `emery.toml` when `<path>` is omitted; discovered automatically when `--config` is not passed). |
| `--format <text\|json>` | — | Output format (`text` or `json`; env `EMERY_FORMAT`). |

---

## JSON output

When `--format json` is passed, `emery apply` writes a single JSON object to `stdout` (see [CLI output shapes](../cli-output-shapes.md#emery-apply) for a full example):

| Field | Type | When present | Description |
| --- | --- | --- | --- |
| `revision` | `string` | Always | SHA-256 ID of the newly sealed revision. |
| `parent` | `string` | When sealing an amendment or displacing an initial revision | SHA-256 ID of the parent revision (`null`/omitted on first seal or `--fresh`). |
| `commit` | `string` | Always | Git commit SHA where `spec/{spec,design,plan}.json` were sealed. |
| `label` | `string` | Always | Local Git branch name (`emery/<revision>`). |
| `repository` | `string` | Always | Target repository path (`"."` or `"./.emery/vcs/repos/<name>"`). |
| `kind` | `"revision"` \| `"amendment"` | Always | Whether a `specify` candidate (`"revision"`) or an `amend` change (`"amendment"`) was sealed. |
| `amendment` | `string` | When `kind == "amendment"` | SHA-256 ID of the staged amendment that was sealed. |
| `diff` | `object` | When a previous revision existed | Diff against the parent/displaced revision (`spec.retired` populated for amendments). |

---

## Text output

```text
sealed revision 9f8e7d6c… as emery/9f8e7d6c… at 4e5f6a7b
  amends 1a2b3c4d…
  repository ./.emery/vcs/repos/shop
  diff vs 1a2b3c4d: spec +0 -0 ~1 retired 1, design +0 -0 ~0, plan +0 -0 ~1
```

- `  amends <parent>` is printed only when `kind` is `"amendment"`.
- `  repository <path>` is printed only when `repository` is not `"."`.

---

## See also

- [`emery specify`](specify.md) — stage a candidate revision from scratch.
- [`emery amend`](amend.md) — stage an incremental amendment against the current revision.
- [`emery show`](show.md) — inspect the sealed revision in Markdown or JSON.
- [`emery build`](build.md) — build the sealed revision's plan wave by wave.
- [Revision lifecycle](../../explanation/revision-lifecycle.md) — how staging, sealing, labels, and lineages work.
