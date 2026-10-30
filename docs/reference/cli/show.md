# `emery show`

This page is the CLI reference for `emery show`, which renders a Markdown projection (or typed JSON envelope) of `spec`, `design`, `plan`, or `amendment` from the current revision or the local staging area. It is for developers and operators reviewing specifications, designs, build plans, and staged amendments.

## Contents

- [Synopsis](#synopsis)
- [Description](#description)
  - [Reading the current revision vs. `--staged`](#reading-the-current-revision-vs---staged)
  - [Reviewing a staged amendment (`emery show amendment`)](#reviewing-a-staged-amendment-emery-show-amendment)
  - [Output behaviour and front matter](#output-behaviour-and-front-matter)
- [Options](#options)
- [JSON output](#json-output)
- [Errors and recovery](#errors-and-recovery)
- [See also](#see-also)

---

## Synopsis

```bash
emery show spec
emery show design
emery show plan
emery show spec --staged             # read the candidate staged by emery specify
emery show spec --config [<path>]    # specify config file for [target] repository
emery show amendment                 # read the amendment staged by emery amend
```

---

## Description

`emery show` renders a read-only Markdown projection of one of four artifacts:

- **`spec`:** The technology-neutral behavioural specification (`# Specification`), listing each requirement's ID, subject, status (`agreed`, `divergence`, `conflict`), inbound trigger (`Route:` or `Message:`), source citations, statement body, loser notes, and acceptance scenarios.
- **`design`:** The architectural design (`# Design`), including `## Overview`, `## Domain model` (`Type: <key>` and `Type: <key> (mechanism)` blocks), `## APIs and integrations` (where concrete technologies from `call` and `decision` claims appear), and other informed sections.
- **`plan`:** The dependency-ordered build plan (`# Plan`), listing each `## Slice: <name>` block with its `ID:`, `Requirements:`, `Types:`, `Depends on:`, and prose brief.
- **`amendment`:** The incremental change staged by [`emery amend`](amend.md) (`# Amendment`), grouped by change class (`Added`, `Changed`, `Retired`, `Conflict`, `Unchanged`) followed by `## Design` and `## Plan` diff sections.

### Reading the current revision vs. `--staged`

For `spec`, `design`, and `plan`, `emery show` operates in two modes:

1. **Current revision (default, without `--staged`):** Reads the sealed revision from the target Git repository configured in `[target]` of [`emery.toml`](../config.md). Emery checks the cached `emery/<id>` branch first (if it descends from `HEAD`), and otherwise inspects `spec/` on the `HEAD` commit directly (so fresh clones and merged branches work without local `.emery/storage/` state).
2. **Staged candidate (`--staged`):** Reads the unsealed candidate revision staged in `.emery/storage/` by [`emery specify`](specify.md). This mode does not require or read a `[target]` table.

### Reviewing a staged amendment (`emery show amendment`)

`emery show amendment` always reads the staged amendment directly from `.emery/storage/` (it requires no `[target]` table, and `--staged` has no effect). Its Markdown projection includes:

- **Header:** `Base: <revision> at <commit>` and a one-line summary count (`1 added, 1 changed, 0 retired, 0 conflict, 4 unchanged.`).
- **Requirement sections (`## Added`, `## Changed`, `## Retired`, `## Conflict`, `## Unchanged`):**
  - `Changed` entries show both `Before:` and `After:` statements alongside redrafted scenarios.
  - `Retired` entries show the withdrawn statement, plus ``Note: retired by scope: `<source>` was read whole and states nothing under `<stem>`.`` when retired via `scope = "whole"`.
  - `Conflict` entries list each competing statement as a `Note:` line followed by `Note: Operator reconciliation required.` and the drafted scenarios.
- **`## Design` (when design sections changed):** Summarizes changed, added, and removed section headings (`Changed: Domain model; added: APIs and integrations.`) and renders the full text of each changed or added section.
- **`## Plan` (when slices changed):** Renders `### Slice: <name> [added|changed|removed]` blocks. Any `changed` slice whose brief was not redrafted by a slicing turn includes `Note: the brief is the base's; no slicing turn redrafted it.`

### Output behaviour and front matter

In default `text` mode, `emery show` writes **only** the Markdown document to `stdout` with no status prefix line, so you can redirect it cleanly to a file (`emery show spec > spec.md`).

For `spec`, `design`, and `plan`, the Markdown begins with YAML front matter carrying the schema version and revision digest:

```markdown
---
emery: 8
revision: 9f8e7d6c…
---
```

*(A revision's `parent` ID is omitted from the Markdown projection; inspect it via `--format json` under `document.parent` or on the `Revision <id>` commit message with `git log -1 --format=%B emery/<id>`.)*

---

## Options

| Option | Short | Description |
| --- | --- | --- |
| `<spec\|design\|plan\|amendment>` | — | Positional artifact name to render. |
| `--staged` | — | Read `spec`, `design`, or `plan` from the staged candidate in `.emery/storage/` instead of the target repository (ignored for `amendment`). |
| `--config [<path>]` | `-c` | Configuration file containing `[target]` for reading the current revision (defaults to `emery.toml` when `<path>` is omitted; discovered automatically for bare `show`). |
| `--format <text\|json>` | — | Output format (`text` outputs raw Markdown; `json` wraps the Markdown and typed document in a JSON object). |

---

## JSON output

When `--format json` is passed, `emery show` writes a single JSON object to `stdout` (see [CLI output shapes](../cli-output-shapes.md#emery-show-specdesignplanamendment) for complete schemas):

| Field | Type | Description |
| --- | --- | --- |
| `revision` | `string` | SHA-256 ID of the current revision, staged candidate (`--staged`), or staged amendment (`amendment`). |
| `body` | `string` | Complete Markdown projection (identical to `text` mode output). |
| `document` | `object` | The underlying typed JSON document (`Spec`, `Design`, `Plan`, or `Amendment`). |

---

## Errors and recovery

| Error code | Exit | Cause | Recovery |
| --- | --- | --- | --- |
| `build-target-required` | `1` | Bare `emery show <spec\|design\|plan>` was run with no `[target]` table in `--config` or `emery.toml`. | Pass `--staged` to view a staged candidate, or add `[target]` to `emery.toml`. |
| `spec-not-generated` | `2` | No sealed revision exists in the target repository, or `emery/<id>` is behind `HEAD`. | Run `emery apply` if a candidate is staged, `emery specify --apply` to create one, or merge/rebase `emery/<id>` onto `HEAD`. |
| `nothing-staged` | `2` | `--staged` was passed with no staged candidate, or `emery show amendment` was run with no staged amendment. | Run `emery specify` (for `--staged`) or `emery amend` (for `amendment`) first. |
| `amendment-pending` | `1` | `emery show <spec\|design\|plan> --staged` was run while an amendment (not a candidate) is staged. | Run `emery show amendment` to inspect the staged amendment. |
| `spec-mismatch` | `1` | Root-level `spec/` in the target repository was modified by hand, holds extra files, or has unresolved merge conflicts. | Restore `git checkout emery/<revision> -- spec/` or keep one branch's `spec/` whole (`git checkout <branch> -- spec/`). |
| `spec-outdated` | `1` | The revision or staged document uses an older `emery` grammar version. | Regenerate with `emery specify`. |
| `server_error` | `3` | Stored revision blobs are missing, corrupted, or fail digest verification. | Re-run `emery specify` or `emery amend`. |

---

## See also

- [`emery specify`](specify.md) — stage a candidate revision for `emery show <artifact> --staged`.
- [`emery amend`](amend.md) — stage an incremental amendment for `emery show amendment`.
- [`emery apply`](apply.md) — seal the staged item so bare `emery show` and `emery build` read it.
- [CLI output shapes](../cli-output-shapes.md#emery-show-specdesignplanamendment) — full JSON schemas for `Spec`, `Design`, `Plan`, and `Amendment`.
