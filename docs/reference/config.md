# Configuration (`emery.toml`)

This page is the complete reference for the `emery.toml` configuration file, including all `[[source]]` and `[target]` fields, discovery rules, and how CLI flags layer over configuration entries. It is for developers and operators configuring Emery projects or change files.

## Contents

- [File discovery and `--config`](#file-discovery-and---config)
- [CLI flag layering rules](#cli-flag-layering-rules)
  - [Source layering (`specify` and `amend`)](#source-layering-specify-and-amend)
  - [Target layering (`build` and `apply`)](#target-layering-build-and-apply)
- [The `[[source]]` table](#the-source-table)
  - [Source content modes](#source-content-modes)
  - [Authority rank defaults](#authority-rank-defaults)
- [The `[target]` table](#the-target-table)
  - [Target repository modes](#target-repository-modes)
- [Validation and error conditions](#validation-and-error-conditions)
- [Complete example](#complete-example)
- [See also](#see-also)

---

## File discovery and `--config`

`emery.toml` is an operator-owned TOML file. Emery never modifies or generates `emery.toml`.

How a command locates `emery.toml` depends on the flags you pass:

| Invocation style | Which file Emery reads |
| --- | --- |
| `--config <path>` (or `-c <path>`) | Reads the explicit project-relative file at `<path>`. If the file does not exist, the command fails (`server_error`, exit `3`). |
| `--config` (or `-c` with no value) | Explicitly selects `emery.toml` in the project root. |
| No `--config` and **no** CLI adapters or `--description` flags | Automatically discovers `emery.toml` in the project root if it exists. |
| No `--config`, **with** positional `<adapter>` or `--description` flags | Reads **no** configuration file. Only the command-line arguments are used. |

> [!IMPORTANT]
> Because passing adapters or `--description` on the command line disables automatic `emery.toml` discovery, add `--config` whenever you want command-line sources to combine with your project's `[[source]]` or `[target]` configuration.

For [`emery amend`](cli/amend.md), if you pass `--config <change-file.toml>` and that file omits a `[target]` table, Emery falls back to reading `[target]` from the project-root `emery.toml`.

---

## CLI flag layering rules

When you combine `--config` with command-line arguments, the configuration file forms the base and CLI flags override or append to it.

### Source layering (`specify` and `amend`)

Every positional `<adapter>` and `--description <adapter>=<text>` flag on the command line creates a source entry whose `name` is the adapter's package name (for example, `typescript` for `emery:typescript@1.2.0`), with default authority rank and `scope = "delta"`:

1. **Base list:** Emery loads the `[[source]]` entries from the configuration file in declaration order.
2. **Replace matching name:** If a CLI source has the same `name` as an unreplaced `[[source]]` entry in the file, the CLI source **replaces that file entry completely** in place (dropping the file entry's `path`, `digest`, `rank`, `scope`, and `repository`).
3. **Append remaining:** Any CLI source whose `name` does not match an unreplaced file entry is appended to the end of the source list in CLI argument order.

### Target layering (`build` and `apply`)

1. **Positional `<adapter>` (`emery build <adapter> --config`):** Replaces `[target] adapter` and clears `[target] digest`, while preserving `[target] name` and `[target] repository` from the file.
2. **`--repository <url|.>` (`emery build` and `emery apply`):** Replaces `[target] repository`, while preserving `[target] name`, `[target] adapter`, and `[target] digest`.

---

## The `[[source]]` table

Each `[[source]]` entry configures one input for [`emery specify`](cli/specify.md) or [`emery amend`](cli/amend.md). Entries are evaluated in declaration order.

| Key | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `adapter` | String (`namespace:name@version`) | Yes | — | Exact WebAssembly adapter package reference (see [Adapters](adapters.md)). |
| `name` | String (kebab-case) | No | Adapter package name | Unique identifier cited in specification requirements (`Sources:`). Must not be `baseline` during `emery amend`. |
| `path` | String (relative path) | No | `"."` | Relative directory or file path for workspace-backed sources. Resolved relative to the directory containing the config file (or relative to the repository root when `repository` is set). Cannot be combined with `description`. |
| `description` | String | No | — | Inline text passed directly to the adapter (equivalent to `--description`). Cannot be combined with `path` or `repository`. |
| `repository` | String (Git URL) | No | — | Remote or local Git repository URL to clone/fetch under `.emery/vcs/repos/<key>`. Requires `revision`. Cannot be combined with `description`. |
| `revision` | String | Conditional | — | Branch, tag, or commit SHA to check out when `repository` is set. Required when `repository` is set; forbidden otherwise. |
| `digest` | String (`sha256:<64-hex>`) | No | — | Pins the adapter `.wasm` binary to an exact SHA-256 content hash. Verified by the host loader before execution. |
| `rank` | Positive integer (`>= 1`) | No | Adapter kind default | Source authority rank (`1` is highest authority). Used to resolve disagreements between sources. |
| `scope` | `"delta"` \| `"whole"` | No | `"delta"` | Read by [`emery amend`](cli/amend.md) only (ignored by `specify`). When set to `"whole"` on a behaviour source, untouched existing requirements under stems covered by at least one `requirement` claim from this source are retired automatically. |

### Source content modes

Each `[[source]]` entry operates in one of three mutually exclusive content modes:

- **Local workspace (`path` or omitted):** Reads files from your project directory (read-only), rooted at `path` (or `"."` when omitted).
- **Inline text (`description`):** Passes the inline string directly to the adapter without lending any filesystem directory.
- **Remote Git repository (`repository` + `revision`, optional `path`):** Clones or fetches the repository into `.emery/vcs/repos/<key>`, creates a temporary Git worktree at `.emery/vcs/sources/<name>` checked out at `revision`, lends `<worktree>/<path>` to the adapter, and removes the worktree when extraction finishes.

### Authority rank defaults

When `rank` is omitted, Emery assigns the default rank for the source adapter's declared kind:

| Adapter kind | Default `rank` | Typical adapters |
| --- | --- | --- |
| `intent` | `1` (highest) | `emery:intent` |
| `documentation` | `2` | `emery:documentation` |
| `behaviour` | `3` | `emery:typescript`, `emery:python` |

Lower integers have higher authority (`1` outranks `2`). Equal ranks tie: two disagreeing claims tied at the top rank produce a `[conflict]`, while a single claim alone at the top rank wins as a `[divergence]`. See [Synthesis, authority, and the technology boundary](../explanation/synthesis-and-authority.md#3-authority-ranks-and-reconciliation).

---

## The `[target]` table

The single `[target]` table configures where specifications are sealed ([`emery apply`](cli/apply.md)), where [`emery show`](cli/show.md) and [`emery amend`](cli/amend.md) read the current revision from, and which target adapter [`emery build`](cli/build.md) uses.

| Key | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `adapter` | String (`namespace:name@version`) | Yes | — | Exact WebAssembly target adapter package reference (see [Adapters](adapters.md)). |
| `name` | String (kebab-case) | No | Adapter package name | Name of the directory under `.emery/vcs/repos/<name>` when using a cloned or engine-managed target repository. |
| `digest` | String (`sha256:<64-hex>`) | No | — | Pins the target adapter `.wasm` binary to an exact SHA-256 content hash. |
| `repository` | String (`"."` or Git URL) | No | Omitted (managed repo) | Target Git repository where `spec/` is sealed and builds are committed. |

### Target repository modes

| `repository` value | Target repository location | Behaviour |
| --- | --- | --- |
| `"."` | Project root directory | Seals revisions and commits builds onto branch `emery/<id>` in your project's own Git repository. Requires `.` to be a Git repository with at least one commit. |
| `"<url>"` | `.emery/vcs/repos/<name>` | Clones `<url>` on first use and fetches `origin/HEAD` on subsequent runs. Seals and builds onto `emery/<id>` inside the clone. Never pushes automatically. |
| Omitted | `.emery/vcs/repos/<name>` | Creates a local Git repository at `.emery/vcs/repos/<name>` on the first `emery apply`, with `Revision <id>` as its root commit. Your project directory does not need to be a Git repository. |

See [Revision lifecycle](../explanation/revision-lifecycle.md#target-repository-modes) for details on how Emery manages target repositories.

---

## Validation and error conditions

Emery parses the entire configuration file strictly (`#[serde(deny_unknown_fields)]`) before loading any adapters:

| Condition | Error code | Exit code |
| --- | --- | --- |
| Unknown TOML key (such as `git`, `url`, `branch`, `remote`, or `registry`) | `bad_request` | `1` |
| Invalid `adapter` format (missing namespace or `@version`) | `adapter-reference` / `bad_request` | `1` |
| Duplicate `[[source]] name` entries, or `name = "baseline"` in `amend` | `bad_request` | `1` |
| `rank` is `0`, negative, or non-integer | `bad_request` | `1` |
| `path` is absolute or escapes above the project root (`..`) | `bad_request` | `1` |
| Both `path` and `description` set on one `[[source]]` | `bad_request` | `1` |
| `repository` set without `revision`, `revision` set without `repository`, or `repository` combined with `description` | `bad_request` | `1` |
| Two different versions of the same adapter package in one run, or one package pinned to two different `digest` values | `bad_request` | `1` |
| `[target] name` is not kebab-case | `bad_request` | `1` |
| Missing `[target]` table when required by `apply`, `build`, `amend`, or bare `show` | `build-target-required` | `1` |

---

## Complete example

```toml
# 1. Read Markdown docs from the invocation directory (default path = ".").
[[source]]
name = "docs"
adapter = "emery:documentation@1.2.0"

# 2. Extract behaviour from a local subdirectory.
# In `emery amend`, scope = "whole" retires untouched requirements under stems
# covered by at least one requirement claim from this source.
[[source]]
name = "api-surface"
adapter = "emery:typescript@1.2.0"
path = "packages/api/src"
scope = "whole"

# 3. Inline intent description (the file equivalent of --description).
[[source]]
name = "intent"
adapter = "emery:intent@1.0.0"
description = "Orders must be idempotent by client request key."

# 4. Lower-ranked wiki (rank 4 sits below behaviour's default rank 3).
[[source]]
name = "wiki"
adapter = "emery:documentation@1.2.0"
path = "wiki"
rank = 4

# 5. Remote Git repository read at a pinned tag, with a pinned adapter digest.
[[source]]
name = "upstream-contracts"
adapter = "emery:typescript@1.2.0"
repository = "https://github.com/acme/contracts.git"
revision = "v2.3.0"
path = "src"
digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"

# Target adapter and repository where revisions are sealed and built.
[target]
name = "orders-service"
adapter = "emery:omnia@0.39.0"
repository = "."
```

---

## See also

- [`emery specify`](cli/specify.md) — generating and staging a candidate revision.
- [`emery amend`](cli/amend.md) — staging an incremental amendment using a change file.
- [`emery apply`](cli/apply.md) — sealing a staged candidate or amendment into the `[target]` repository.
- [`emery build`](cli/build.md) — building a sealed revision through the `[target]` adapter.
- [Adapters](adapters.md) — package reference syntax and `~/.emery/adapters` store layout.
