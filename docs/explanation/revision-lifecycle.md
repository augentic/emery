# Revision lifecycle

This page explains how Emery stages, seals, and resolves specification revisions across local storage and your target Git repository. It is for developers and operators who want to understand where specification state lives, how revisions are tracked on Git branches, and how history lineages work.

## Contents

- [Overview: two-tier state](#overview-two-tier-state)
- [The staging area (`.emery/storage`)](#the-staging-area-emerystorage)
  - [Candidates vs. amendments](#candidates-vs-amendments)
  - [Preventing accidental overwrites](#preventing-accidental-overwrites)
- [Target repository modes](#target-repository-modes)
- [How `emery apply` seals a revision](#how-emery-apply-seals-a-revision)
  - [Commit structure and trailers](#commit-structure-and-trailers)
  - [The `emery/<id>` label branch](#the-emeryid-label-branch)
- [How Emery resolves the current revision](#how-emery-resolves-the-current-revision)
  - [1. Cached label branch](#1-cached-label-branch)
  - [2. Direct `HEAD` tree inspection](#2-direct-head-tree-inspection)
  - [Integrity checks and `spec-mismatch`](#integrity-checks-and-spec-mismatch)
- [Revision lineages and `--fresh`](#revision-lineages-and---fresh)
- [See also](#see-also)

---

## Overview: two-tier state

Emery separates unreviewed specification drafts from committed specifications across two distinct locations:

1. **Local staging area (`.emery/storage/`):** Holds at most one unsealed draft (either a candidate revision or an incremental amendment) for local review.
2. **Target Git repository (`spec/` on branch `emery/<id>`):** Holds the sealed, authoritative revision as three JSON files (`spec/spec.json`, `spec/design.json`, and `spec/plan.json`).

Commands that author specifications ([`emery specify`](../reference/cli/specify.md) and [`emery amend`](../reference/cli/amend.md)) write to the staging area first. Commands that consume sealed specifications ([`emery build`](../reference/cli/build.md) and bare [`emery show`](../reference/cli/show.md)) read exclusively from the target Git repository.

---

## The staging area (`.emery/storage`)

When you run `emery specify` or `emery amend` without `--apply`, Emery writes the generated documents into `.emery/storage/` inside your project directory.

### Candidates vs. amendments

The staging area holds one item at a time under the `staged` key:

| Staged kind | Produced by | Stored documents | How to review |
| --- | --- | --- | --- |
| **Candidate** (`revision`) | [`emery specify`](../reference/cli/specify.md) | `spec.json`, `design.json`, `plan.json` | `emery show <spec\|design\|plan> --staged` |
| **Amendment** (`amendment`) | [`emery amend`](../reference/cli/amend.md) | `amendment.json` (containing the change diff plus the updated `spec`, `design`, and `plan`) | `emery show amendment` |

Each staged item is identified by a content-addressed SHA-256 hex digest (`<id>`). Re-running `emery specify` replaces a previously staged candidate and prunes the old candidate's blobs. Similarly, re-running `emery amend` replaces a previously staged amendment.

### Preventing accidental overwrites

A candidate and an amendment cannot silently overwrite each other:

- Running `emery specify` while an amendment is currently staged fails with `amendment-pending`.
- Running `emery amend` while a candidate is currently staged also fails with `amendment-pending`.

To proceed, seal the staged item with [`emery apply`](../reference/cli/apply.md) (which clears the staging area), or re-run the matching command (`amend` or `specify`) that owns the staged item.

---

## Target repository modes

A sealed revision always lives inside the target Git repository configured by the `[target]` table in `emery.toml` (or overridden via `--repository` on `apply` and `build`). Emery supports three target repository modes:

| `[target] repository` | Where revisions and builds live | Base commit rules |
| --- | --- | --- |
| `"."` | Directly inside your current project's Git repository. | The directory must be a Git repository (`repository-required`) with at least one commit (`base-not-sealed`). During `emery build`, the working tree outside `.emery/` must also have no uncommitted changes. |
| `"<url>"` | In a Git clone managed by Emery at `.emery/vcs/repos/<name>`. | Cloned on first use and fetched on subsequent runs. Revisions and builds start from the remote's default branch (`origin/HEAD`). |
| Omitted | In a local Git repository created and managed by Emery at `.emery/vcs/repos/<name>`. | Initialized automatically during the first `emery apply`. The first sealed `Revision <id>` commit becomes the repository's root commit. |

Here `<name>` is `[target] name` in `emery.toml`, or the target adapter's package name if `name` is omitted. Each `.emery/vcs/repos/<name>` directory belongs permanently to the URL or managed repository that first created it.

---

## How `emery apply` seals a revision

Running [`emery apply`](../reference/cli/apply.md) (or passing `--apply` to `specify` or `amend`) promotes the currently staged candidate or amendment into the target repository:

1. **Selects the base commit:**
   - For a **candidate**, Emery starts from the currently sealed revision's commit (or `HEAD` if no revision is sealed yet).
   - For an **amendment**, Emery verifies that the target repository still stands at the exact base revision and commit the amendment was prepared against (`amendment-stale` if it moved), and starts from that commit.
   - For a **fresh candidate** (`specify --fresh`), Emery starts directly from the repository's `HEAD` commit.
2. **Writes `spec/` in a temporary worktree:** Emery creates a temporary Git worktree at `.emery/vcs/revision`, verifies that `spec/` does not contain unrelated user files, and writes `spec/spec.json`, `spec/design.json`, and `spec/plan.json`.
3. **Runs the secret gate:** Emery scans the three JSON documents for accidentally embedded credentials ([Security and sandboxing](security-and-sandbox.md#the-secret-gate-secret-detected)) before creating a commit.
4. **Commits and labels:** Emery creates a single commit, sets the local branch `emery/<id>` to point to that commit, updates the `current-revision` cache pointer in `.emery/storage/`, removes the temporary worktree, and clears the staging area.

Sealing never modifies your project's checked-out working tree or moves your active Git branch.

### Commit structure and trailers

Every sealed revision commit uses a structured commit message:

```text
Revision <id>

Revision: <id>
Parent: <parent-revision-id>
Base: <base-commit-sha>
```

When sealing an amendment, the commit message also records summary trailers after `Base:`:

- `Added: <count>`
- `Changed: <count>`
- `Retired: <count>`
- `Slices: SLICE-002 orders changed, SLICE-004 export removed` (omitted when slice definitions did not change)

### The `emery/<id>` label branch

The local branch `emery/<id>` (where `<id>` is the full 64-character revision digest) marks the revision's tip in the target repository. Immediately after `emery apply`, `emery/<id>` points to the `Revision <id>` commit. When you run [`emery build`](../reference/cli/build.md), Emery advances `emery/<id>` as each wave of slices is merged and verified.

---

## How Emery resolves the current revision

Before running `show`, `build`, `amend`, or `specify`, Emery inspects the target repository to determine its **standing** (`HEAD`, active revision, and branch position). It resolves the current revision in two steps:

### 1. Cached label branch

Emery reads the cached `current-revision` ID from `.emery/storage/` and checks whether the local branch `emery/<id>` exists in the target repository:

- If `emery/<id>` exists and **descends from `HEAD`** (meaning `HEAD` is an ancestor of `emery/<id>`, or they point to the same commit), Emery reads `spec/{spec,design,plan}.json` from the `emery/<id>` commit and verifies that their SHA-256 digest matches `<id>`.
- If `emery/<id>` exists but **does not descend from `HEAD`** (for example, you committed new changes on your main branch after sealing), Emery records the label as `behind` and falls back to inspecting `HEAD`.

### 2. Direct `HEAD` tree inspection

If no cached label branch stands ahead of `HEAD`—for example, in a fresh clone, after merging `emery/<id>` into your main branch, or after deleting an old label branch—Emery inspects `spec/` directly on the `HEAD` commit:

- If `spec/` holds `spec.json`, `design.json`, and `plan.json` that parse as a valid, internally consistent revision, Emery uses that revision as the current revision regardless of whether a local `emery/<id>` branch exists.
- If `spec/` does not exist on `HEAD` and a cached label branch was found behind `HEAD`, commands that require a sealed revision fail with `spec-not-generated` and advise you to merge or rebase `emery/<id>` onto `HEAD` (or seal a fresh revision).

### Integrity checks and `spec-mismatch`

The root-level `spec/` directory in the target repository belongs exclusively to Emery. Emery refuses with `spec-mismatch` in three situations:

- **Non-revision files in `spec/`:** `spec/` contains extra files, subdirectories, or symlinks (such as `spec/notes.md`). Move user files elsewhere or into a nested path like `src/spec/`.
- **Tampered files under a label:** The files in `spec/` at `emery/<id>` do not hash to `<id>`. Restore them with `git checkout emery/<id> -- spec/`.
- **Hand-edited or conflicted merge on `HEAD`:** At an unlabelled `HEAD`, the three JSON files contain Git conflict markers, invalid JSON, or mismatched requirements and slices from two different branches. Keep one branch's `spec/` directory whole (`git checkout <branch> -- spec/`), commit the merge, and re-run the other branch's `emery amend`.

---

## Revision lineages and `--fresh`

When you evolve a specification with [`emery amend`](../reference/cli/amend.md) and seal it with `emery apply`, the new revision records the previous revision's ID in its top-level `parent` field. A revision whose `parent` is non-null forms a **lineage**.

Once a repository holds a lineage, running bare `emery specify` (which generates a brand-new specification from scratch with `parent: null` and renumbers requirements from `REQ-001`) would discard your amendment history and requirement ID stability. To prevent accidental history loss, Emery refuses bare `emery specify` (and `emery apply` of a non-fresh candidate) with `lineage-exists`.

You have two explicit paths forward:

1. **Continue the lineage:** Run [`emery amend`](../reference/cli/amend.md) to stage an incremental change against the current revision, preserving existing `REQ-NNN` and `SLICE-NNN` identifiers.
2. **Start a new lineage:** Run `emery specify --fresh` to intentionally stage a candidate marked `fresh`. When `emery apply` seals a fresh candidate, it commits over the repository's `HEAD` with `parent: null`, starting a new history while leaving all earlier `emery/<id>` branches intact.

---

## See also

- [Glossary](../glossary.md) — definitions of revisions, candidates, amendments, labels, and lineages.
- [Synthesis, authority, and the technology boundary](synthesis-and-authority.md) — how `emery specify` and `emery amend` turn source claims into a revision.
- [Wave-based builds and verification](wave-builds.md) — how `emery build` turns a sealed revision on `emery/<id>` into verified commits.
- [`emery apply` CLI reference](../reference/cli/apply.md) — flags, JSON output, and error codes for sealing.
