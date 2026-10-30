# Your first specification and build

This step-by-step tutorial walks you through setting up a new Emery project, extracting a specification from Markdown documentation, sealing that specification into Git, and building it with a target adapter. It is for developers using Emery for the first time.

## Contents

- [Prerequisites](#prerequisites)
- [1. Prepare the mock adapters and scratch project](#1-prepare-the-mock-adapters-and-scratch-project)
- [2. Configure `emery.toml`](#2-configure-emerytoml)
- [3. Generate and review a candidate specification](#3-generate-and-review-a-candidate-specification)
- [4. Seal the specification into Git (`emery apply`)](#4-seal-the-specification-into-git-emery-apply)
- [5. Build the plan (`emery build`)](#5-build-the-plan-emery-build)
- [6. Merge the verified build onto your branch](#6-merge-the-verified-build-onto-your-branch)
- [Next steps](#next-steps)

---

## Prerequisites

Before starting, make sure you have the following installed:

- **`emery` binary** (either installed on your `PATH` or built from this repository with `cargo build --release`).
- **[`cursor-sdk-bridge`](https://github.com/cursor/sdk-bridge)** on your `PATH` (see [Installing `cursor-sdk-bridge`](../../examples/README.md#installing-cursor-sdk-bridge)).
- **`CURSOR_API_KEY`** exported in your shell environment (or loaded from a `.env` file).
- **`git` 2.5 or newer** on your `PATH`.

---

## 1. Prepare the mock adapters and scratch project

To make this tutorial self-contained without downloading external packages, we will build the repository's two minimal example adapters (`example:source@0.1.0` and `example:target@0.1.0`) and copy them into the local adapter store (`~/.emery/adapters`).

From the root of the `emery` repository, run:

```bash
# Build the emery binary and the two WebAssembly example adapters
cargo build --release
cargo build --examples --target wasm32-wasip2 --release

# Copy the built .wasm components into the local adapter store
mkdir -p ~/.emery/adapters
cp target/wasm32-wasip2/release/examples/source.wasm ~/.emery/adapters/example_source@0.1.0.wasm
cp target/wasm32-wasip2/release/examples/target.wasm ~/.emery/adapters/example_target@0.1.0.wasm
```

Next, create a clean temporary directory for our tutorial project and initialize it as a Git repository:

```bash
export CURSOR_API_KEY=<your-cursor-api-key>
emery=$PWD/target/release/emery

project=$(mktemp -d)
cd "$project"

# Ignore Emery's local staging and worktree directory
printf '.emery/\n' > .gitignore
```

Create a single Markdown documentation file at `docs/greeting.md`:

```bash
mkdir -p docs
cat > docs/greeting.md <<'EOF'
# Greeting

GET /greeting returns the static string 'hello'.

Acceptance:

- The response body is exactly `hello`.
EOF
```

---

## 2. Configure `emery.toml`

Create an `emery.toml` file in the root of your scratch project. This file tells Emery which source adapters to read from and which target adapter and repository to seal and build into:

```bash
cat > emery.toml <<'EOF'
[[source]]
name = "docs"
adapter = "example:source@0.1.0"
path = "docs/"

[target]
adapter = "example:target@0.1.0"
repository = "."
EOF
```

Here:

- `[[source]]` names the source `"docs"` and points `example:source@0.1.0` at `docs/`.
- `[target]` selects `example:target@0.1.0` and sets `repository = "."`, meaning revisions and builds will be committed onto a branch inside this project's own Git repository.

Commit the initial project files so the repository has a clean `HEAD` commit:

```bash
git init --quiet
git add --all
git commit --quiet --message "greeting docs"
```

---

## 3. Generate and review a candidate specification

Run [`emery specify`](../reference/cli/specify.md) to extract claims from `docs/` and synthesize a specification (`spec`), architectural design (`design`), and build plan (`plan`):

```bash
"$emery" -v specify
```

Emery prints the content-addressed ID of the staged candidate revision:

```text
staged candidate <revision-id>
  plan: 1 slice in 1 wave, widest 1
  nothing sealed; `emery apply` seals it
```

At this stage, nothing has been committed to Git. The candidate sits in the local staging area (`.emery/storage/`). Review the staged Markdown projections using [`emery show --staged`](../reference/cli/show.md):

```bash
"$emery" show spec --staged
"$emery" show design --staged
"$emery" show plan --staged
```

In `show spec --staged`, you will see `REQ-001` (`greeting.behaviour`) with `Status: agreed`, `Sources: docs (docs/greeting.md#L1-L7)`, the requirement statement, and its acceptance scenario.

---

## 4. Seal the specification into Git (`emery apply`)

Once you are satisfied with the staged candidate, seal it into the target repository with [`emery apply`](../reference/cli/apply.md):

```bash
"$emery" apply
```

```text
sealed revision <revision-id> as emery/<revision-id> at <commit-sha>
```

What just happened:

1. Emery committed `spec/spec.json`, `spec/design.json`, and `spec/plan.json` onto a local Git branch named `emery/<revision-id>`.
2. Emery cleared the local staging area in `.emery/storage/`.
3. Your checked-out working tree (`main`) was not modified.

Now bare `emery show` reads directly from the sealed revision in Git:

```bash
"$emery" show spec
git show --stat emery/<revision-id>
```

---

## 5. Build the plan (`emery build`)

Now run [`emery build`](../reference/cli/build.md) to execute the sealed build plan through `example:target@0.1.0`:

```bash
"$emery" -v build
```

```text
built revision <revision-id>
  plan: 1 slice in 1 wave, widest 1
  base <base-sha>
  initialised <init-sha>, 1 file
  wave 1: 1 slice verified at <head-sha>
    SLICE-001 greeting: covered 1/1, written 2 files, merged <head-sha>
  labelled emery/<revision-id> at <head-sha>
```

During the build:

1. **Initialization (`Task::Init`):** The target adapter scaffolded `build/README.md` in an integration worktree (`.emery/vcs/integration`), and Emery committed it as `Initialise`.
2. **Slice build (`Task::Build`):** Emery created an isolated temporary Git worktree at `.emery/vcs/worktrees/SLICE-001`, where the adapter built `build/greeting/index.md` and `build/greeting/REQ-001.md` and merged them into the integration worktree.
3. **Verification (`Task::Verify`):** The adapter verified the integrated tree and advanced the local branch `emery/<revision-id>` to the verified commit.

If you run `"$emery" build` a second time right away, Emery sees that `SLICE-001` is already merged on `emery/<revision-id>` and finishes immediately without repeating any work.

---

## 6. Merge the verified build onto your branch

Because `emery build` commits onto `emery/<revision-id>` without touching your checked-out working tree, you can inspect the commits before merging them onto your current branch:

```bash
# Inspect the commits created by apply and build
git log --oneline --first-parent HEAD..emery/<revision-id>
git diff --stat HEAD emery/<revision-id>

# Fast-forward your branch to include spec/ and build/
git merge --ff-only emery/<revision-id>
ls spec/ build/greeting/
```

---

## Next steps

- **Keep this scratch project open** and continue directly to the next tutorial: [Evolving a specification with `emery amend`](amending-a-spec.md).
- Read [Revision lifecycle](../explanation/revision-lifecycle.md) to learn how `.emery/storage/` and `emery/<id>` branches interact.
- Explore [Configure local, inline, and remote Git sources](../how-to/configure-sources.md) to connect real TypeScript, Python, or Markdown sources.
