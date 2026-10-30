# `emery specify`

This page is the CLI reference for `emery specify`, which extracts claims from configured source adapters, synthesizes a specification, design, and build plan from scratch, and stages them as a candidate revision. It is for developers and operators generating an initial specification or starting a fresh revision lineage.

## Contents

- [Synopsis](#synopsis)
- [Description](#description)
  - [Every run starts from new](#every-run-starts-from-new)
  - [A lineage is amended or started over](#a-lineage-is-amended-or-started-over)
  - [Where the revision lives](#where-the-revision-lives)
- [The `emery.toml` config](#the-emerytoml-config)
- [Options](#options)
- [JSON output](#json-output)
- [Text output](#text-output)
- [Errors and recovery](#errors-and-recovery)
- [See also](#see-also)

---

## Synopsis

```bash
emery specify <adapter>... [--description <adapter>=<text>]
emery specify --config [<path>]
emery specify <adapter>... --config  # CLI sources layered over emery.toml
emery specify                        # discovers the project-root emery.toml
emery specify --config --apply       # stage and seal in one run
emery specify --config --fresh       # start history over when a lineage exists
```

---

## Description

`emery specify` runs your configured source adapters concurrently, reconciles their claims by authority rank, synthesizes `spec.json`, `design.json`, and `plan.json`, and stages the three documents as a single content-addressed **candidate revision** in `.emery/storage/`.

You can provide sources on the command line, in [`emery.toml`](../config.md), or both:

- **Positional `<adapter>`:** Adds a workspace-backed source that reads files from your project directory (read-only). Its source name defaults to the adapter's package name (`typescript` for `emery:typescript@1.2.0`).
- **`--description <adapter>=<text>` (`-d`):** Adds an inline text source passed directly to the adapter without lending any filesystem directory.
- **`--config [<path>]` (`-c`):** Reads `[[source]]` and `[target]` entries from `<path>` (or `emery.toml` when `<path>` is omitted). If you also pass positional adapters or `--description` flags alongside `--config`, CLI sources replace matching `[[source]]` entries by name and append any new ones in CLI order (see [CLI flag layering rules](../config.md#cli-flag-layering-rules)).
- **Automatic discovery:** Running `emery specify` with no sources and no `--config` discovers `emery.toml` in the project root. Passing sources on the command line without `--config` reads no configuration file.

By default, `emery specify` only stages the candidate in `.emery/storage/`. Review it with [`emery show <spec|design|plan> --staged`](show.md) and commit it to your target repository with [`emery apply`](apply.md). Passing `--apply` stages and seals the candidate in a single run (if sealing fails, the candidate remains staged).

For the full synthesis pipeline, authority rank rules, and technology-neutral `Lexicon` gate, see [Synthesis, authority, and the technology boundary](../../explanation/synthesis-and-authority.md).

### Every run starts from new

`emery specify` always synthesizes a brand-new revision from its sources alone:

- Requirements are numbered `REQ-001` upward in source order, and slices are numbered `SLICE-001` upward.
- Each document's `parent` field is `null`.
- Existing requirements in the sealed revision are not read into synthesis; the current revision is read only to compute the advisory `diff` summary (and as the parent commit when sealing a non-lineage revision under `--apply`).

To update an existing sealed revision while preserving `REQ-NNN` and `SLICE-NNN` identifiers, use [`emery amend`](amend.md) instead.

### A lineage is amended or started over

When the current revision in your target repository was created by an amendment (meaning its `parent` field names an earlier revision), the repository holds a **lineage**. Running bare `emery specify` over a lineage would discard that amendment history, so Emery refuses with `lineage-exists` (exit `1`) before loading any adapters.

You have two choices:

1. **Evolve the existing revision:** Run [`emery amend`](amend.md) to stage an incremental change against the current revision.
2. **Start a new lineage intentionally:** Run `emery specify --fresh`. This marks the staged candidate as `fresh` (`staged candidate <id> (fresh)`). When sealed with `emery apply`, a fresh candidate commits over the repository's `HEAD` with `parent: null`, starting a new history while leaving all earlier `emery/<id>` branches untouched.

If an amendment is currently staged in `.emery/storage/`, `emery specify` refuses with `amendment-pending` (exit `1`) so you do not accidentally overwrite it. See [Revision lifecycle](../../explanation/revision-lifecycle.md#revision-lineages-and---fresh).

### Where the revision lives

When `emery specify` reads a configuration file that includes a `[target]` table, it inspects the target repository before loading any adapters to check for an existing lineage and to compute the `diff` against the current revision. If you run `emery specify` with CLI sources only (and no `--config`), Emery knows no target repository: it stages the candidate without a `diff`, and passing `--apply` fails with `build-target-required` (exit `1`).

This is the CLI command invoked by the [`/emery:specify`](../../../plugins/emery/skills/specify/SKILL.md) Cursor skill.

---

## The `emery.toml` config

`emery specify` reads `[[source]]` entries (`name`, `adapter`, `path`, `description`, `repository`, `revision`, `digest`, `rank`, `scope`) and the `[target]` table (`name`, `adapter`, `digest`, `repository`) from `emery.toml`. Note that `scope` on a `[[source]]` entry is read only by `emery amend` and ignored by `emery specify`.

For the complete schema, field tables, and examples, see [Configuration (`emery.toml`)](../config.md).

```toml
[[source]]
name = "docs"
adapter = "emery:documentation@1.2.0"

[[source]]
name = "api-surface"
adapter = "emery:typescript@1.2.0"
path = "packages/api/src"

[[source]]
name = "intent"
adapter = "emery:intent@1.0.0"
description = "Ship a location-independent spec generator."

[target]
adapter = "emery:omnia@0.39.0"
repository = "."
```

---

## Options

| Option | Short | Description |
| --- | --- | --- |
| `<adapter>...` | — | Workspace-backed source adapter references (`namespace:name@version`). Alongside `--config`, replaces the `[[source]]` entry with the same package name or appends a new entry. |
| `--description <adapter>=<text>` | `-d` | Inline text source (`<adapter>=<text>`). Repeatable. Layers over `--config` by package name just like positional adapters. |
| `--config [<path>]` | `-c` | Path to configuration file (defaults to `emery.toml` when `<path>` is omitted). |
| `--apply` | — | Seal the staged candidate as the current revision in the same run (requires a `[target]` table via `--config` or discovered `emery.toml`). |
| `--fresh` | — | Stage the candidate as a fresh start (`"fresh": true`), allowing `specify` and `apply` to start a new parentless history over an existing lineage. |
| `--format <text\|json>` | — | Output format (`text` or `json`; env `EMERY_FORMAT`). |

---

## JSON output

When `--format json` is passed, `emery specify` writes a single JSON object to `stdout` (see [CLI output shapes](../cli-output-shapes.md#emery-specify) for a full example):

| Field | Type | When present | Description |
| --- | --- | --- | --- |
| `revision` | `string` | Always | Content-addressed SHA-256 ID of the staged candidate revision. |
| `waves` | `string[][]` | Always | Slice IDs (`SLICE-NNN`) grouped into dependency waves in build order. |
| `staged` | ` boolean` | Always | `true` when the candidate remains staged; `false` when `--apply` sealed it. |
| `fresh` | `boolean` | Always | `true` when staged with `--fresh`; `false` otherwise. |
| `diff` | `object` | When `[target]` has a readable current revision | Advisory diff (`from`, `spec`, `design`, `plan`) comparing the candidate against the current revision. |
| `repositories` | `object[]` | When at least one source uses `repository` | Resolved Git commits for remote sources (`{ source, repository, revision, commit }`). |
| `applied` | `object` | When `--apply` succeeded | Full [`emery apply`](apply.md#json-output) output object for the sealed revision. |

---

## Text output

```text
staged candidate 9f8e7d6c…
  plan: 3 slices in 2 waves, widest 2
  upstream-api read from https://github.com/acme/api.git at v2.3.0: 4e5f6a7b…
  diff vs 1a2b3c4d: spec +1 -0 ~1 preamble, design +0 -0 ~1, plan +1 -0 ~1
  nothing sealed; `emery apply` seals it
```

- When `--fresh` is passed, the first line ends with ` (fresh)`.
- When `--apply` is passed, the final line is replaced by `  sealed revision <id> as emery/<id> at <commit8>`.

---

## Errors and recovery

| Error code | Exit | Cause | Recovery |
| --- | --- | --- | --- |
| `specify-source-required` | `1` | No sources passed on the CLI and no `emery.toml` found at the project root. | Pass `<adapter>...` or create `emery.toml`. |
| `lineage-exists` | `1` | The target repository's current revision amends an earlier revision (`parent` is set) and `--fresh` was not passed. | Run `emery amend <adapter>...` to evolve the revision, or pass `emery specify --fresh` to start a new lineage. |
| `amendment-pending` | `1` | An unsealed amendment is currently staged in `.emery/storage/`. | Review it with `emery show amendment`, seal it with `emery apply`, or replace it with `emery amend`. |
| `build-target-required` | `1` | `--apply` was passed without a `[target]` table (for example, CLI sources without `--config`). | Pass `--config` so Emery reads `[target]` from `emery.toml`. |
| `adapter-reference` | `1` | An adapter argument is not an exact `namespace:name@version` reference. | Specify the full reference (for example, `emery:typescript@1.2.0`). |
| `refused` | `1` | Adapter missing from `~/.emery/adapters` under an unrouted namespace, digest mismatch, or invalid Wasm component. | Populate the store with `wkg get <reference> -o ~/.emery/adapters/` or fix the `digest` pin (see [Adapters](../adapters.md)). |
| `unsupported-version` | `1` | The adapter requires a newer `emery` binary version. | Upgrade `emery`. |
| `spec-mismatch` | `1` | Root-level `spec/` in the target repository contains non-revision files or hand-edited/conflicted JSON. | Restore `git checkout emery/<revision> -- spec/` or resolve the branch merge (see [`emery apply`](apply.md#spec-is-the-engines)). |
| `repository-required` | `1` | `[target] repository = "."` is set, but the project directory is not a Git repository. | Run `git init` and create an initial commit, or change `[target] repository`. |
| `base-not-sealed` | `1` | `--apply` was used with `[target] repository = "."`, but the repository has no commits yet. | Create an initial Git commit and re-run `emery apply`. |
| `revision-not-found` | `2` | A source `repository` / `revision` or target `repository` URL could not be found. | Verify the URL and branch/tag/commit in `emery.toml`. |
| `server_error` | `3` | A source adapter emitted claims rejected by the SDK/engine claim gate, or a local I/O failure occurred. | Check `-v` logs for the claim gate findings. |
| `bad_gateway` / `unavailable` | `4` | Upstream model backend error, Git remote failure, or adapter registry unreachable. | Check network connectivity and model credentials, then retry. |

---

## See also

- [Configuration (`emery.toml`)](../config.md) — full `[[source]]` and `[target]` configuration reference.
- [`emery show`](show.md) — review the staged candidate with `emery show <spec|design|plan> --staged`.
- [`emery apply`](apply.md) — seal the staged candidate into the target Git repository.
- [`emery amend`](amend.md) — stage an incremental amendment against a sealed revision.
- [Synthesis, authority, and the technology boundary](../../explanation/synthesis-and-authority.md) — how extraction, grouping, authority ranks, and the `Lexicon` gate work.
