# Merge specifications across Git branches

This guide shows how to resolve Git merge conflicts in `spec/` when two developers or branches independently seal amendments against the same base revision.

## Contents

- [Why `spec/` conflicts when branches diverge](#why-spec-conflicts-when-branches-diverge)
- [Step-by-step resolution workflow](#step-by-step-resolution-workflow)
  - [1. Merge the first branch onto `main`](#1-merge-the-first-branch-onto-main)
  - [2. Start the merge of the second branch](#2-start-the-merge-of-the-second-branch)
  - [3. Keep one branch's `spec/` directory whole](#3-keep-one-branchs-spec-directory-whole)
  - [4. Re-run the second branch's `emery amend` over the merge commit](#4-re-run-the-second-branchs-emery-amend-over-the-merge-commit)
- [Why hand-merging `spec/*.json` fails (`spec-mismatch`)](#why-hand-merging-specjson-fails-spec-mismatch)
- [Runnable example with the mock adapters](#runnable-example-with-the-mock-adapters)
- [See also](#see-also)

---

## Why `spec/` conflicts when branches diverge

Suppose Branch A (`emery/<rev-a>`) and Branch B (`emery/<rev-b>`) both start from the same sealed revision and each runs `emery amend --apply`:

- Branch A adds `REQ-010` (`orders.cancel`) and seals `spec/{spec,design,plan}.json`.
- Branch B adds `REQ-010` (`health.ping`) and seals its own `spec/{spec,design,plan}.json`.

Because each sealed revision writes `spec/spec.json` whole and content-addresses the three JSON documents together, running `git merge` between the two branches will produce a Git conflict in `spec/spec.json` (and potentially `spec/design.json` and `spec/plan.json`).

**Never hand-edit or line-merge `spec/*.json`.** Instead, keep one branch's `spec/` directory whole to complete the Git merge, then re-run the other branch's `emery amend` on top of the merged commit.

---

## Step-by-step resolution workflow

### 1. Merge the first branch onto `main`

Fast-forward or merge the first branch (`emery/<rev-a>`) onto your integration branch:

```bash
git merge --ff-only emery/<rev-a>
```

### 2. Start the merge of the second branch

Merge the second branch (`emery/<rev-b>`), which may also contain source file commits alongside its `spec/` changes:

```bash
git merge emery/<rev-b>
# Auto-merging spec/spec.json
# CONFLICT (content): Merge conflict in spec/spec.json
```

All non-`spec/` files (such as your updated documentation or source code from both branches) merge normally in Git.

### 3. Keep one branch's `spec/` directory whole

Check out all three files under `spec/` from the first branch (`emery/<rev-a>` or `HEAD`) as a single unit, and commit the merge:

```bash
# Keep the first branch's spec/{spec,design,plan}.json whole:
git checkout emery/<rev-a> -- spec/

# Complete the Git merge commit:
git commit --no-edit
```

At this point, `HEAD` holds the source files from both branches, and `spec/` holds the valid, internally consistent revision `<rev-a>`. When you run any Emery command now, Emery inspects `spec/` on `HEAD` and recognizes `<rev-a>` as the current revision.

### 4. Re-run the second branch's `emery amend` over the merge commit

Now re-run the second branch's `emery amend` (and `emery apply`) against the merged `HEAD`:

```bash
# Re-apply Branch B's change on top of the kept revision <rev-a>:
emery amend --config <branch-b-sources> --apply

# Verify that both branches' changes are now present under <rev-a> as parent:
emery show spec

# Fast-forward main to the newly sealed revision (or run emery build first):
git merge --ff-only emery/<combined-rev>
```

The new revision `<combined-rev>` cleanly incorporates Branch B's changes on top of `<rev-a>`, assigns non-colliding `REQ-NNN` and `PART-NNN` IDs, and records `Parent: <rev-a>`.

---

## Why hand-merging `spec/*.json` fails (`spec-mismatch`)

Emery verifies cross-document integrity (`Revision::findings`) every time it reads `spec/` from `HEAD`:

- Every requirement in `spec.json` must be placed in exactly one part in `plan.json`, and `plan.json` may not reference any requirement ID absent from `spec.json`.
- Every `data` type key in `design.json` must be owned by exactly one part in `plan.json`.

If you commit conflict markers inside `spec/*.json`, hand-edit a JSON file, or take `spec.json` from Branch A alongside `design.json` and `plan.json` from Branch B, `emery show`, `emery amend`, `emery apply`, and `emery build` will all refuse with `spec-mismatch`:

```text
error spec-mismatch: `spec/` at HEAD was edited or merged by hand ...
  hint: keep one side's `spec/` whole (`git checkout <branch> -- spec/`) and re-run the other side's `emery amend`
```

Checking out one branch's `spec/` whole (`git checkout <branch> -- spec/`) and re-running the other branch's `emery amend` guarantees that requirement IDs, data type ownership, and content digests remain valid.

---

## Runnable example with the mock adapters

Using the `$project` repository from [Evolving a specification with `emery amend`](../tutorials/amending-a-spec.md), you can reproduce a two-branch merge across two local clones:

```bash
# Clone the project to simulate a second operator (reads the current revision from HEAD's tree)
git clone --quiet "$project" "$project-b"

# Each operator amends the current revision with a different inline description and seals it
"$emery" -v amend --config \
  --description "example:source@0.1.0=GET /farewell returns the static string 'so long'." \
  --apply
(
  cd "$project-b" \
    && "$emery" -v amend --config \
      --description "example:source@0.1.0=GET /ping returns the static string 'pong'." \
      --apply \
    && git push --quiet origin 'refs/heads/emery/*:refs/heads/emery/*'
)
git for-each-ref refs/heads/emery/

# Merge the first branch onto main, then merge the second (which conflicts on spec/spec.json)
git merge --ff-only emery/<farewell-rev>
git merge emery/<ping-rev>                  # CONFLICT (content): spec/spec.json
git checkout emery/<farewell-rev> -- spec/  # Keep one side's three documents whole
git commit --quiet --no-edit
"$emery" show spec                          # Holds farewell ('so long'), no ping yet

# Re-run the second branch's amendment on top of the merged revision
"$emery" -v amend --config \
  --description "example:source@0.1.0=GET /ping returns the static string 'pong'." \
  --apply
"$emery" show spec                          # Both farewell and ping, with Parent: <farewell-rev>
git merge --ff-only emery/<combined-rev>
rm -rf "$project-b"
```

---

## See also

- [Revision lifecycle](../explanation/revision-lifecycle.md#how-emery-resolves-the-current-revision) — how Emery resolves revisions across `emery/<id>` labels and merged `HEAD` commits.
- [Evolving a specification with `emery amend`](../tutorials/amending-a-spec.md) — step-by-step amendment tutorial.
