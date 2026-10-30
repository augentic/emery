# Evolving a specification with `emery amend`

This tutorial walks you through updating an existing sealed specification using `emery amend`, reviewing the staged amendment diff, sealing the new revision into a lineage, and rebuilding your project. It continues directly from the scratch project created in [Your first specification and build](quickstart.md).

## Contents

- [Why `emery amend` instead of `emery specify`?](#why-emery-amend-instead-of-emery-specify)
- [1. Modify an existing source document](#1-modify-an-existing-source-document)
- [2. Stage and review the amendment](#2-stage-and-review-the-amendment)
- [3. Seal the amendment and rebuild](#3-seal-the-amendment-and-rebuild)
- [4. Add a new requirement and technology decision](#4-add-a-new-requirement-and-technology-decision)
- [5. Understand revision lineages and `--fresh`](#5-understand-revision-lineages-and---fresh)
- [Next steps](#next-steps)

---

## Why `emery amend` instead of `emery specify`?

Once a specification is sealed and built, requirements (`REQ-001`) and build slices (`SLICE-001`) are referenced across commits, tests, and code. Running `emery specify` again would synthesize a brand-new specification from scratch and renumber requirements from `REQ-001`.

[`emery amend`](../reference/cli/amend.md) instead treats your inputs as an incremental **change** against the currently sealed revision:

- Existing requirements that match a change claim keep their `REQ-NNN` ID and become `changed`.
- Untouched requirements are copied byte-for-byte as `unchanged`.
- Brand-new behaviours receive new `REQ-NNN` IDs starting above the current specification's highest ID.
- Only affected requirements, design sections, and build slices are redrafted.

---

## 1. Modify an existing source document

In your scratch project from the previous tutorial (where the first revision and its build are merged onto your branch), update `docs/greeting.md` so the greeting returns `'hello, world'` instead of `'hello'`:

```bash
cat > docs/greeting.md <<'EOF'
# Greeting

GET /greeting returns the static string 'hello, world'.

Acceptance:

- The response body is exactly `hello, world`.
EOF

git commit --quiet --all --message "greeting says hello, world"
```

---

## 2. Stage and review the amendment

Run `emery amend` (which automatically discovers `emery.toml` in the project root):

```bash
"$emery" -v amend
```

```text
staged amendment <amendment-id>
  base <first-revision-id> at <base-commit>
  0 added, 1 changed, 0 retired, 0 conflict, 0 unchanged
  stems: greeting
  nothing sealed; `emery show amendment` reviews it
```

Inspect the staged amendment with [`emery show amendment`](../reference/cli/show.md#reviewing-a-staged-amendment-emery-show-amendment):

```bash
"$emery" show amendment
```

Under `## Changed`, you will see `REQ-001` (`greeting.behaviour`) showing both `Before:` (the previous `'hello'` statement) and `After:` (the updated `'hello, world'` statement), along with its redrafted acceptance scenario. Because the requirement kept its stem (`greeting`) and no technology decision changed, the design and slice structure needed no extra model turns.

---

## 3. Seal the amendment and rebuild

Seal the staged amendment into the repository with [`emery apply`](../reference/cli/apply.md#sealing-an-amendment):

```bash
"$emery" apply
```

```text
sealed revision <second-revision-id> as emery/<second-revision-id> at <commit-sha>
  amends <first-revision-id>
  diff vs <first-revision-id>: spec +0 -0 ~1, design +0 -0 ~0, plan +0 -0 ~0
```

Notice the `amends <first-revision-id>` line. Inspect the commit message on `emery/<second-revision-id>`:

```bash
git log -1 --format=%B emery/<second-revision-id>
```

The commit carries `Parent: <first-revision-id>`, `Base:`, and the summary trailers `Added: 0`, `Changed: 1`, `Retired: 0`.

Now build the amended revision and fast-forward your branch:

```bash
"$emery" -v build
git merge --ff-only emery/<second-revision-id>
```

---

## 4. Add a new requirement and technology decision

You can also pass an inline `--description` on the command line alongside `--config` to layer an ad-hoc change over your `emery.toml` sources.

> [!IMPORTANT]
> Always include `--config` when passing `--description` or `<adapter>` to `emery amend` so Emery still reads `[target]` and `[[source]]` from `emery.toml`.

Let's add a `GET /farewell` endpoint and specify that its phrasebook is stored in PostgreSQL, sealing in the same run with `--apply`:

```bash
"$emery" -v amend --config \
  --description "example:source@0.1.0=GET /farewell returns the static string 'bye'. Keep the farewell's phrasebook in PostgreSQL." \
  --apply
```

Emery extracts both the `docs` directory (which restates `greeting.behaviour`, leaving `REQ-001` `unchanged`) and the inline description (which adds `farewell.behaviour` as `REQ-002` and emits a `decision` claim for PostgreSQL):

```text
staged amendment <amendment-id>
  base <second-revision-id> at <base-commit>
  1 added, 0 changed, 0 retired, 0 conflict, 1 unchanged
  stems: farewell
  design: overview, apis
  slices: farewell (new)
  sealed revision <third-revision-id> as emery/<third-revision-id> at <commit-sha>
    amends <second-revision-id>
```

Inspect the updated specification, design, and build plan:

```bash
# REQ-002 appears alongside REQ-001, and names no database technology
"$emery" show spec

# PostgreSQL appears in ## APIs and integrations inside the design
"$emery" show design

# SLICE-002 (farewell) is added alongside SLICE-001 (greeting)
"$emery" show plan
```

Build the updated plan and merge the result:

```bash
"$emery" -v build
git merge --ff-only emery/<third-revision-id>
```

---

## 5. Understand revision lineages and `--fresh`

Because the current revision in your repository now has a `parent` (`amends <second-revision-id>`), the repository holds a **lineage**. If you try to run bare `emery specify` now, Emery refuses before loading any adapters so you do not accidentally wipe out your amendment history:

```bash
"$emery" specify
# Error: lineage-exists (exit 1)
# Hint: emery amend <adapter>..., or emery specify --fresh <adapter>...
```

If you ever want to intentionally discard the existing lineage and regenerate the specification from scratch over `HEAD`, pass `--fresh`:

```bash
"$emery" -v specify --fresh --apply
```

This starts a new parentless revision (`parent: null`) over `HEAD` while leaving all earlier `emery/<id>` branches intact in Git.

---

## Next steps

- [Set authority ranks and resolve source conflicts](../how-to/resolve-conflicts.md) — learn how to handle `[conflict]` requirements when sources disagree.
- [Retire requirements explicitly or by scope](../how-to/retire-requirements.md) — learn how to remove obsolete requirements during an amendment.
- [Merge specifications across Git branches](../how-to/merge-branches.md) — learn how two developers amending a specification on separate branches merge their work.
