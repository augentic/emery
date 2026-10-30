# `emery build`

This page is the CLI reference for `emery build`, which executes the sealed build plan in dependency waves through a target adapter, merging and verifying each wave onto branch `emery/<id>`. It is for developers and operators building and verifying code from a sealed revision.

## Contents

- [Synopsis](#synopsis)
- [Description](#description)
  - [Repository resolution and cleanliness](#repository-resolution-and-cleanliness)
  - [Waves, worktrees, and resume behaviour](#waves-worktrees-and-resume-behaviour)
  - [Worker sandbox and pre-commit gates](#worker-sandbox-and-pre-commit-gates)
- [The `[target]` table](#the-target-table)
- [Options](#options)
- [JSON output](#json-output)
- [Text output](#text-output)
- [Errors and recovery](#errors-and-recovery)
- [See also](#see-also)

---

## Synopsis

```bash
emery build <adapter>
emery build --config [<path>]
emery build <adapter> --config       # CLI adapter replaces [target] adapter
emery build <adapter> --repository . # build into the project repository
emery build                          # discovers the project-root emery.toml
emery build --jobs 2                 # build at most 2 slices concurrently per wave
```

---

## Description

`emery build` reads the **currently sealed revision** from your target Git repository (never an unsealed candidate or amendment in `.emery/storage/`), loads the configured **target adapter** (`namespace:name@version`), and builds every slice of `plan.json` in dependency waves.

### Repository resolution and cleanliness

Before loading the target adapter, `emery build` resolves the target repository from `[target]` in [`emery.toml`](../config.md) (or `--repository`) and reads the sealed revision:

- **`repository = "."` (project repository):** Builds from your project repository's `HEAD`. The directory must be a Git repository (`repository-required`) with at least one commit and **no uncommitted changes outside `.emery/` and `.git/`** (`base-not-sealed`, listing any dirty paths). Add `.emery/` to your `.gitignore` so local staging state does not clutter `git status`.
- **`repository = "<url>"` (remote clone):** Clones `<url>` into `.emery/vcs/repos/<name>` on first use and fetches `origin/HEAD` on subsequent runs.
- **Omitted (engine-managed repository):** Uses the repository created by `emery apply` at `.emery/vcs/repos/<name>`. `emery build` never creates a repository itself (`spec-not-generated` if none exists).

If a candidate or amendment is currently staged in `.emery/storage/` and differs from the sealed revision being built, `emery build` proceeds with the sealed revision and reports the unsealed item in `candidate` (and prints a reminder line in text mode).

### Waves, worktrees, and resume behaviour

1. **Resume check:** Emery inspects the local branch `emery/<revision>` (where it descends from the repository base) and `.emery/vcs/integration`. Any slice already merged on `emery/<revision>` is skipped (`resumed`). Any slice merged into `.emery/vcs/integration` during an earlier run that stopped before wave verification is also skipped (`resumed`, printed as `staged by an earlier run, not built again`) and verified in its wave.
2. **Initialization (`Task::Init`):** If no slices of this revision have been merged yet, the target adapter scaffolds any missing project layout files in `.emery/vcs/integration` without a model turn, and Emery commits them as `Initialise`.
3. **Parallel slice builds (`Task::Build`):** Ready slices (whose `depends-on` slices are all merged) build concurrently up to `--jobs <N>` (and within the backend's `CURSOR_MAX_AGENTS`, default `8`), each in an isolated temporary Git worktree at `.emery/vcs/worktrees/<slice-id>`. Transient model failures (`bad_gateway` or time-budget exhaustion) are retried once in a fresh worktree.
4. **Deterministic integration:** Completed slices in a wave are committed and merged into `.emery/vcs/integration` in ascending slice ID order (`SLICE-001`, `SLICE-002`, ...) using the adapter's merge rules (`union`, `ours`, `theirs`), falling back to a single AI merge turn (`MergeBrief`) when a text file conflicts outside the rules.
5. **Wave verification (`Task::Verify`) and labelling:** After a wave's slices merge, the target adapter runs workspace checks and repairs on `.emery/vcs/integration`. Once verification passes, any changes are committed as `Wave <k> verified` and the local branch **`emery/<revision>`** advances to the verified head.

Your checked-out working tree is never modified during `emery build`. After the build finishes, inspect `emery/<revision>` with `git log --first-parent emery/<revision>` or merge it onto your branch with `git merge --ff-only emery/<revision>`.

For a deep architectural explanation of wave scheduling, merge resolution, and resume mechanics, see [Wave-based builds and verification](../../explanation/wave-builds.md).

### Worker sandbox and pre-commit gates

Every build and verification turn runs inside an OS kernel sandbox (`omnia-confine`) with shell access restricted to `.emery/vcs/worktrees` and `.emery/vcs/integration`, network egress restricted to the target adapter's declared worker grants, and tool calls capped by `CURSOR_MAX_TOOL_CALLS` (default `128`). Before any commit is created, Emery checks all modified paths against the **specification guard** (`spec-rewritten`, blocking changes to `spec/`) and the **secret gate** (`secret-detected`, blocking credential files and tokens). See [Security, sandboxing, and gates](../../explanation/security-and-sandbox.md).

This is the CLI command invoked by the [`/emery:build`](../../../plugins/emery/skills/build/SKILL.md) Cursor skill.

---

## The `[target]` table

`emery build` reads `[target]` (`name`, `adapter`, `digest`, `repository`) from `emery.toml`. Passing a positional `<adapter>` alongside `--config` overrides `adapter` (and drops `digest`), and passing `--repository` overrides `repository`.

For full `[target]` field definitions and layering rules, see [Configuration (`emery.toml`)](../config.md#the-target-table).

```toml
[target]
name = "shop"                                # optional directory under .emery/vcs/repos/
adapter = "emery:omnia@0.39.0"
digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
repository = "git@github.com:acme/shop.git"  # URL clone, "." for project checkout, or omitted
```

---

## Options

| Option | Short | Description |
| --- | --- | --- |
| `<adapter>` | — | Target adapter reference (`namespace:name@version`). Alongside `--config`, replaces `[target] adapter` and clears `[target] digest`. |
| `--repository <url\|.>` | — | Target Git repository (`"."` for project checkout, or a Git URL cloned under `.emery/vcs/repos/<name>`). Overrides `[target] repository`. |
| `--config [<path>]` | `-c` | Path to configuration file (defaults to `emery.toml` when `<path>` is omitted; discovered automatically when no `<adapter>` is passed). |
| `--jobs <N>` | `-j` | Maximum number of slices in a wave to build concurrently (positive integer; `0` is exit `64`; env `EMERY_JOBS`). Unset builds all ready slices in a wave concurrently within the backend's agent limit. |
| `--format <text\|json>` | — | Output format (`text` or `json`; env `EMERY_FORMAT`). |

---

## JSON output

When `--format json` is passed, `emery build` writes a single JSON object to `stdout` (see [CLI output shapes](../cli-output-shapes.md#emery-build) for a full example):

| Field | Type | When present | Description |
| --- | --- | --- | --- |
| `revision` | `string` | Always | SHA-256 ID of the sealed revision that was built. |
| `waves` | `string[][]` | Always | Slice IDs grouped into dependency waves. |
| `candidate` | `{ id, kind }` | When an unsealed candidate or amendment is staged | Identifies the staged item (`kind` is `"revision"` or `"amendment"`) that was not built. |
| `base` | `string` | Always | Base Git commit SHA the build started from. |
| `repository` | `string` | Always | Target repository path (`"."` or `"./.emery/vcs/repos/<name>"`). |
| `initialised` | `{ commit, paths }` | When `Task::Init` created an `Initialise` commit this run | Commit SHA and sorted relative file paths scaffolded during initialization. |
| `resumed` | `string[]` | When slices were skipped | Slice IDs skipped because they were already merged on `emery/<revision>` or staged in `.emery/vcs/integration`. |
| `slices` | `object[]` | Always | Built slices in merge order: `{ id, name, wave, covered, uncovered, written, resolved, commit }`. |
| `verified` | `object[]` | When at least one wave was verified this run | Verified waves: `{ wave, slices, head }`. |
| `sealed` | `object[]` | When wave verification committed changes | Verification commits: `{ wave, commit, paths }`. |
| `repaired` | `number[]` | When verification modified files outside `ours` merge rules | 1-based wave numbers where verification repaired code beyond `ours`-ruled lockfiles. |
| `head` | `string` | Always | Final verified Git commit SHA. |
| `label` | `string` | Always | Local Git branch updated to `head` (`emery/<revision>`). |

---

## Text output

```text
built revision 9f8e7d6c…
  plan: 3 slices in 2 waves, widest 2
  base 1a2b3c4d…
  repository ./.emery/vcs/repos/shop
  initialised 2b3c4d5e, 3 files
  resumed: SLICE-001
  wave 1: 2 slices verified at 5e6f7a8b
    SLICE-001: staged by an earlier run, not built again
    SLICE-003 catalogue: covered 1/1, written 1 file, merged 5e6f7a8b…
  wave 2: 1 slice verified at 9c0d1e2f, repaired
    SLICE-002 orders: covered 1/2 (uncovered REQ-004), written 3 files, merged 9c0d1e2f…, resolved (src/lib.rs)
  labelled emery/9f8e7d6c… at 9c0d1e2f…
```

---

## Errors and recovery

Whenever a slice build, merge, or wave verification fails, Emery integrates all earlier successful slices in the wave into `.emery/vcs/integration` and leaves `.emery/vcs/integration` and the failed `.emery/vcs/worktrees/<slice>` on disk for inspection.

| Error code | Exit | Cause | Recovery |
| --- | --- | --- | --- |
| `build-target-required` | `1` | No target adapter passed and no `[target]` table found in `emery.toml`. | Pass `<adapter>` or configure `[target]` in `emery.toml`. |
| `spec-not-generated` | `2` | No revision is sealed in the target repository, or `emery/<revision>` is behind `HEAD`. | Run `emery apply` to seal a staged candidate, or merge/rebase `emery/<revision>` onto `HEAD`. |
| `base-not-sealed` | `1` | Under `repository = "."`, the project working tree has uncommitted changes outside `.emery/`, or has no commits. | Commit or stash pending changes (and add `.emery/` to `.gitignore`), then re-run `emery build`. |
| `repository-required` | `1` | Under `repository = "."`, the project directory is not a Git repository. | Initialize Git (`git init`) and commit, or remove `repository = "."`. |
| `spec-mismatch` | `1` | Root-level `spec/` was modified or contains non-revision files. | Restore `git checkout emery/<revision> -- spec/`. |
| `spec-outdated` | `1` | The sealed revision uses an older `emery` grammar stamp. | Regenerate with `emery specify` and `emery apply`. |
| `merge-unresolved` | `1` | Two slices in a wave conflicted on a deleted/non-text file, or AI merge resolution could not resolve the conflict. | Inspect `.emery/vcs/worktrees/<slice>` and `.emery/vcs/integration`, then re-run `emery build` (which rebuilds over the already-merged slices) or add a merge rule to the adapter. |
| `verify-failed` | `1` | `Task::Verify` reported failing checks after repair attempts. | Inspect `.emery/vcs/integration`; re-run `emery build` to re-verify without rebuilding the wave's slices, or remove `.emery/vcs/integration` (`git worktree remove --force`) to rebuild the wave from scratch. |
| `secret-detected` | `1` | A slice build, merge resolution, initialization, or wave verification wrote a credential file or token. | Inspect the named path in the worktree, remove the secret source, and re-run `emery build`. |
| `spec-rewritten` | `1` | A shell command or adapter modified files under root-level `spec/`. | Fix the command or adapter layout and re-run `emery build`. |
| `bad_gateway` | `4` | The model backend failed twice on one turn, or the remote Git repository failed. | Re-run `emery build`; completed waves on `emery/<revision>` and merged slices in `.emery/vcs/integration` will resume automatically. |

---

## See also

- [Wave-based builds and verification](../../explanation/wave-builds.md) — how slicing, parallel worktrees, merge resolution, and resume work.
- [Security, sandboxing, and gates](../../explanation/security-and-sandbox.md) — worker grants, kernel sandboxing, `spec-rewritten`, and `secret-detected`.
- [Configuration (`emery.toml`)](../config.md#the-target-table) — `[target]` configuration reference.
- [`emery apply`](apply.md) — seal a revision before building.
