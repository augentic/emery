# Configure local, inline, and remote Git sources

This guide shows how to configure local workspace directories, inline text descriptions, and remote Git repositories as inputs for `emery specify` and `emery amend`, and how command-line flags combine with `emery.toml`.

## Contents

- [Read a local directory (`path`)](#read-a-local-directory-path)
- [Pass an inline text description (`description` / `--description`)](#pass-an-inline-text-description-description----description)
- [Read a remote Git repository at a tag, branch, or commit](#read-a-remote-git-repository-at-a-tag-branch-or-commit)
- [Combine CLI flags with `emery.toml` (`--config`)](#combine-cli-flags-with-emerytoml---config)
- [Use a separate change file for `emery amend`](#use-a-separate-change-file-for-emery-amend)
- [See also](#see-also)

---

## Read a local directory (`path`)

To extract claims from files inside your project directory, add a `[[source]]` entry with a relative `path` (or omit `path` to read from `"."`):

```toml
[[source]]
name = "docs"
adapter = "emery:documentation@0.39.0"
path = "docs/"

[[source]]
name = "orders-api"
adapter = "emery:typescript@0.39.0"
path = "services/orders/src"
```

- `name` is the identifier cited in requirement `Sources:` lines. If you omit `name`, it defaults to the adapter's package name (`documentation`, `typescript`).
- If you run two `[[source]]` entries using the same adapter package, give at least one of them a distinct `name` so source names remain unique.
- `path` must be relative and cannot escape above the directory containing `emery.toml` (`..` is rejected).

---

## Pass an inline text description (`description` / `--description`)

When you want to state product intent or an architectural decision without creating a file on disk, use `description` in `emery.toml` or `--description` (`-d`) on the command line:

```toml
[[source]]
name = "intent"
adapter = "emery:intent@0.39.0"
description = "All order mutations must require an Idempotency-Key header. Store idempotency records in Redis."
```

Or directly on the command line:

```bash
emery specify --config \
  --description "emery:intent@0.39.0=All order mutations must require an Idempotency-Key header."
```

> [!NOTE]
> A `[[source]]` entry cannot set both `description` and `path` (or `repository`).

---

## Read a remote Git repository at a tag, branch, or commit

To extract claims from another repository (such as an upstream API service or shared contract repository), set `repository` and `revision` together:

```toml
[[source]]
name = "upstream-contracts"
adapter = "emery:typescript@0.39.0"
repository = "https://github.com/acme/contracts.git"
revision = "v2.3.0"
path = "packages/orders/src"
```

How Emery handles remote sources:

1. Emery normalizes the `repository` URL and clones it once under `.emery/vcs/repos/<key>` (or fetches updates on subsequent runs).
2. It creates a temporary Git worktree at `.emery/vcs/sources/<name>` checked out at `revision` (which can be a branch, tag, or commit SHA).
3. It lends the `path` subdirectory inside that worktree to the adapter (read-only), then removes the temporary worktree as soon as extraction finishes.
4. The resolved commit SHA is printed in terminal output and recorded under `repositories` in `--format json`.

Both `repository` and `revision` are required together—setting one without the other fails with `bad_request`.

---

## Combine CLI flags with `emery.toml` (`--config`)

When you pass positional `<adapter>` arguments or `--description` flags on the command line, Emery disables automatic `emery.toml` discovery unless you also pass `--config`:

```bash
# Reads ONLY the CLI argument (ignores emery.toml completely):
emery specify emery:typescript@0.39.0

# Layers the CLI description over the [[source]] and [target] entries in emery.toml:
emery specify --config --description "emery:intent@0.39.0=Add rate limiting."
```

When `--config` is present alongside CLI sources:

- Each CLI source gets the adapter's package name as its `name` (for example, `intent` for `emery:intent@0.39.0`).
- If `emery.toml` already has an unreplaced `[[source]]` with that `name`, the CLI source **replaces** that entry in place.
- Otherwise, the CLI source is **appended** after the file's `[[source]]` entries.

> [!CAUTION]
> Because a positional `<adapter>` and `--description <adapter>=<text>` for the same adapter both bind the same default source name, passing both on the CLI for the same adapter fails with duplicate source names. Define named `[[source]]` entries in `emery.toml` when using the same adapter twice.

---

## Use a separate change file for `emery amend`

During [`emery amend`](../reference/cli/amend.md), you often want to run only the sources that changed rather than re-extracting every source in `emery.toml`. Create a dedicated TOML file (for example, `changes/cancel-order.toml`):

```toml
[[source]]
name = "cancel-rfc"
adapter = "emery:documentation@0.39.0"
path = "docs/rfcs/014-cancel-order.md"
```

Then run:

```bash
emery amend --config changes/cancel-order.toml
```

Even though `changes/cancel-order.toml` omits a `[target]` table, `emery amend` automatically falls back to reading `[target]` from the project-root `emery.toml`.

---

## See also

- [Configuration (`emery.toml`)](../reference/config.md) — full schema for `[[source]]` and `[target]`.
- [Set authority ranks and resolve source conflicts](resolve-conflicts.md) — control which source wins when sources disagree.
- [Retire requirements explicitly or by scope](retire-requirements.md) — use `scope = "whole"` on behaviour sources during `emery amend`.
