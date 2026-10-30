# CLI output shapes

This page defines the canonical JSON output schemas, exit codes, recovery error discriminants, and text-mode formatting rules for every `emery` command. It is for skill authors, CI scripts, and tool integrations consuming `--format json` or parsing error envelopes.

## Contents

- [Conventions](#conventions)
- [Exit codes](#exit-codes)
- [Recovery error codes](#recovery-error-codes)
- [Failure envelope](#failure-envelope)
- [Text-mode style](#text-mode-style)
- [Command output schemas](#command-output-schemas)
  - [`emery specify`](#emery-specify)
  - [`emery amend`](#emery-amend)
  - [`emery apply`](#emery-apply)
  - [`emery build`](#emery-build)
  - [`emery show <spec|design|plan|amendment>`](#emery-show-specdesignplanamendment)
  - [`emery completions <shell>`](#emery-completions-shell)

---

## Conventions

- **Flat JSON objects:** Every `--format json` success response is a single flat JSON object carrying command-specific fields at the top level. There is no `ok` boolean or `data` wrapper.
- **Key naming:** All JSON keys use `kebab-case`.
- **Relative paths:** File paths inside JSON envelopes are `/`‑separated strings relative to the repository root unless noted otherwise.
- **Stream separation:**
  - `stdout`: Semantic command result (text or JSON).
  - `stderr`: Failure envelopes and live tracing logs (`info` by default; adjusted via `-v`, `-q`, or `RUST_LOG`).

---

## Exit codes

`omnia_sdk::Error::exit_code` maps error classes 1:1 to process exit codes:

| Exit code | Error class | Meaning |
| --- | --- | --- |
| `0` | *(success)* | Command succeeded. |
| `1` | `bad_request` | Invalid input, configuration error, precondition failure, or gate rejection. |
| `2` | `not_found` | Missing revision, nothing staged, or Git repository/revision not found. |
| `3` | `server_error` | Claim gate or report gate violation, corrupted storage blob, or local I/O error. |
| `4` | `bad_gateway` | Upstream model backend error, Git remote error, or unreachable adapter registry. |
| `64` | *(usage)* | Clap CLI usage error (unknown flag, missing required argument, or `--jobs 0`). Printed on `stderr` with no JSON envelope (`EX_USAGE`). |

---

## Recovery error codes

When a command fails with exit code `1`–`4`, the `error` field in the failure envelope carries either one of the 19 specific `kebab-case` recovery codes below, a plugin loader code (`refused`, `unavailable`, `internal`), or a generic `snake_case` class (`bad_request`, `not_found`, `server_error`, `bad_gateway`):

| `error` discriminant | Exit | Raised by | When it occurs |
| --- | --- | --- | --- |
| `specify-source-required` | `1` | `specify`, `amend` | No sources passed on the CLI and no `emery.toml` found. |
| `build-target-required` | `1` | `specify --apply`, `amend`, `apply`, `build`, `show` | No `[target]` table available for a command that reads or seals the target repository. |
| `adapter-reference` | `1` | `specify`, `amend`, `build` | Adapter string is not an exact `namespace:name@version` package reference. |
| `unsupported-version` | `1` | `specify`, `amend`, `build` | Adapter component requires a newer `emery-version` than this binary supports. |
| `amendment-pending` | `1` | `specify`, `amend`, `show --staged` | An unsealed amendment blocks `specify` / `show --staged`, or an unsealed candidate blocks `amend`. |
| `amendment-stale` | `1` | `apply`, `amend --apply` | Target repository's current revision or commit moved since the staged amendment was prepared. |
| `lineage-exists` | `1` | `specify`, `apply` | Current revision has a `parent` (is an amendment lineage) and `--fresh` was not set. |
| `retire-unmatched` | `1` | `amend` | A `retire` claim failed to match any existing requirement in the current revision. |
| `spec-outdated` | `1` | All verbs | Stored revision or staged item uses an older `emery` grammar stamp than this binary supports. |
| `spec-mismatch` | `1` | All verbs | Root-level `spec/` contains extra files, tampered JSON, or hand-merged conflict state. |
| `repository-required` | `1` | `specify`, `amend`, `apply`, `build`, `show` | `[target] repository = "."` is set, but the project directory is not a Git repository. |
| `base-not-sealed` | `1` | `specify --apply`, `apply`, `build` | `[target] repository = "."` has no commits yet, or (for `build`) has uncommitted changes outside `.emery/`. |
| `merge-unresolved` | `1` | `build` | Slice merge conflicted on a deleted/non-text path or AI merge resolution exhausted its rounds. |
| `verify-failed` | `1` | `build` | Wave verification (`Task::Verify`) returned `passed: false`. |
| `secret-detected` | `1` | `apply`, `build` | Secret gate caught a credential filename or token pattern in pending changes. |
| `spec-rewritten` | `1` | `build` | A build or verification step modified files under root-level `spec/`. |
| `spec-not-generated` | `2` | `amend`, `build`, `show` | No sealed revision exists in the target repository, or `emery/<id>` is behind `HEAD`. |
| `nothing-staged` | `2` | `apply`, `show --staged`, `show amendment` | No candidate or amendment is currently staged in `.emery/storage/`. |
| `revision-not-found` | `2` | `specify`, `amend`, `apply`, `build`, `show` | Remote Git repository URL or `revision` ref could not be found. |

---

## Failure envelope

On any error (exit codes `1`–`4`), Emery writes a flat JSON object to `stderr` when `--format json` is active:

```json
{
  "error": "specify-source-required",
  "message": "no sources",
  "exit-code": 1,
  "hint": "pass one or more adapters to `emery specify`, or add an `emery.toml` at the project root"
}
```

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `error` | `string` | Yes | Stable error discriminant (from the table above). |
| `message` | `string` | Yes | Human-readable description naming the specific rule, path, or adapter that failed. |
| `exit-code` | `number` | Yes | Process exit code (`1`, `2`, `3`, or `4`). |
| `hint` | `string` | Optional | Actionable recovery command or guidance. |

In `text` mode, the same failure is rendered on `stderr` as:

```text
error[specify-source-required]: no sources
  hint: pass one or more adapters to `emery specify`, or add an `emery.toml` at the project root
```

---

## Text-mode style

All text-mode formatters (`crates/cli/src/text.rs`) follow a uniform layout:

- **Result line first (lowercase, verb-first, no trailing period):** `staged candidate 9f8e7d6c…`, `sealed revision 9f8e7d6c… as emery/9f8e7d6c… at 4e5f6a7b`.
- **Two-space indented detail lines:** `  plan: 3 slices in 2 waves, widest 2`.
- **Explicit empty states:** Every empty state prints a lowercase line rather than silence.
- **Exception (`emery show`):** `emery show` outputs the raw Markdown projection alone with no status header so `emery show spec > spec.md` writes a clean Markdown file.

---

## Command output schemas

### `emery specify`

```json
{
  "revision": "9f8e7d6c…",
  "waves": [["SLICE-001", "SLICE-003"], ["SLICE-002"]],
  "staged": true,
  "fresh": false,
  "diff": {
    "from": "1a2b3c4d…",
    "spec": {
      "preamble": true,
      "added": [{ "id": "REQ-003", "subject": "access.audit" }],
      "removed": [],
      "retired": [],
      "changed": [
        { "id": "REQ-002", "subject": "session.timeout", "fields": ["body", "scenarios"] },
        { "id": "REQ-004", "subject": "orders.create", "was": "REQ-003", "fields": ["id"] }
      ]
    },
    "design": { "preamble": false, "added": [], "removed": [], "changed": ["domain-model"] },
    "plan": {
      "preamble": false,
      "added": [{ "id": "SLICE-002", "name": "access" }],
      "removed": [],
      "changed": [{ "id": "SLICE-001", "name": "authentication", "fields": ["requirements", "brief"] }]
    }
  },
  "repositories": [
    { "source": "upstream-api", "repository": "https://github.com/acme/api.git", "revision": "v2.3.0", "commit": "4e5f6a7b…" }
  ]
}
```

| Field | Type | Presence | Description |
| --- | --- | --- | --- |
| `revision` | `string` | Always | Content-addressed SHA-256 ID of the three candidate documents. |
| `waves` | `string[][]` | Always | Dependency waves of `SLICE-NNN` IDs in build order. |
| `staged` | `boolean` | Always | `true` when staged in `.emery/storage/`; `false` when `--apply` sealed it. |
| `fresh` | `boolean` | Always | `true` when staged with `--fresh`; `false` otherwise. |
| `diff` | `Diff` | Optional | Advisory diff against the current revision in `[target]` (omitted when no `[target]` or no current revision exists). |
| `diff.from` | `string` | In `diff` | SHA-256 ID of the outgoing revision compared against. |
| `diff.spec` | `object` | In `diff` | `{ preamble, added, removed, retired, changed }`. Across distinct histories (`specify`), requirements are matched by shared stem + overlapping citation anchor first and by `id` second (`was` records a changed ID; `retired` is empty). Within one history (`amend`), requirements are matched by `id` alone (`retired` holds withdrawn `{ id, subject, statement }` entries; `removed` is empty). |
| `diff.design` | `object` | In `diff` | `{ preamble, added, removed, changed }` over kebab-case section kinds (`overview`, `domain-model`, `apis`, `technical-logic`, `ui-layout`, `observability`). |
| `diff.plan` | `object` | In `diff` | `{ preamble, added, removed, changed }` over slices (`{ id, name }`, with `fields` on `changed` entries). |
| `repositories` | `object[]` | Optional | Remote Git sources read (`{ source, repository, revision, commit }`). Omitted when empty. |
| `applied` | `ApplyOutput` | Optional | Present only when `--apply` sealed the candidate (see [`emery apply`](#emery-apply)). |

---

### `emery amend`

```json
{
  "amendment": "e5378831…",
  "staged": true,
  "base": { "revision": "9f8e7d6c…", "commit": "4e5f6a7b…" },
  "tally": { "added": 1, "changed": 1, "retired": 1, "conflict": 0, "unchanged": 4, "by-scope": 1 },
  "stems": ["access", "orders", "session"],
  "sections": { "preamble": false, "added": ["apis"], "removed": [], "changed": ["overview"] },
  "slices": [
    { "id": "SLICE-002", "name": "orders", "class": "changed", "brief": "kept" },
    { "id": "SLICE-004", "name": "access", "class": "added", "brief": "redrafted" }
  ]
}
```

| Field | Type | Presence | Description |
| --- | --- | --- | --- |
| `amendment` | `string` | Optional | Content-addressed SHA-256 ID of the staged `Amendment` (omitted when the change is a no-op). |
| `staged` | `boolean` | Always | `true` when the amendment stands staged; `false` when nothing changed or `--apply` sealed it. |
| `base` | `{ revision, commit }` | Always | Revision ID and Git commit SHA the amendment was prepared against. |
| `tally` | `object` | Always | `{ added, changed, retired, conflict, unchanged, by-scope }` counts. |
| `conflicts` | `string[]` | Optional | Subject IDs of requirements in `conflict`, in `ReqId` order (omitted when empty). |
| `stems` | `string[]` | Optional | Sorted unique stems touched by non-`unchanged` requirements (omitted when empty). |
| `sections` | `DesignDiff` | Optional | `{ preamble, added, removed, changed }` over design section kinds (omitted when design is unchanged). |
| `slices` | `SliceChange[]` | Optional | `{ id, name, class, brief }` for each `added`, `changed`, or `removed` slice, where `brief` is `"redrafted"` or `"kept"` (omitted when empty). |
| `repositories` | `object[]` | Optional | Remote Git sources read (`{ source, repository, revision, commit }`). Omitted when empty. |
| `applied` | `ApplyOutput` | Optional | Present only when `--apply` sealed the amendment (see [`emery apply`](#emery-apply)). |

---

### `emery apply`

```json
{
  "revision": "9f8e7d6c…",
  "parent": "1a2b3c4d…",
  "commit": "4e5f6a7b…",
  "label": "emery/9f8e7d6c…",
  "repository": "./.emery/vcs/repos/shop",
  "kind": "amendment",
  "amendment": "e5378831…",
  "diff": {
    "from": "1a2b3c4d…",
    "spec": {
      "preamble": false,
      "added": [],
      "removed": [],
      "retired": [{ "id": "REQ-004", "subject": "orders.export", "statement": "…" }],
      "changed": [{ "id": "REQ-002", "subject": "session.timeout", "fields": ["body", "scenarios"] }]
    },
    "design": { "preamble": false, "added": [], "removed": [], "changed": [] },
    "plan": {
      "preamble": false,
      "added": [],
      "removed": [],
      "changed": [{ "id": "SLICE-002", "name": "orders", "fields": ["requirements"] }]
    }
  }
}
```

| Field | Type | Presence | Description |
| --- | --- | --- | --- |
| `revision` | `string` | Always | SHA-256 ID of the sealed revision. |
| `parent` | `string` | Optional | SHA-256 ID of the parent revision (`null`/omitted on first seal or `--fresh`). |
| `commit` | `string` | Always | Commit SHA of the `Revision <id>` commit. |
| `label` | `string` | Always | Local branch name set on `commit` (`emery/<revision>`). |
| `repository` | `string` | Always | Target repository path (`"."` or `"./.emery/vcs/repos/<name>"`). |
| `kind` | `"revision"` \| `"amendment"` | Always | Which staged item kind was sealed. |
| `amendment` | `string` | Optional | SHA-256 ID of the sealed amendment (present only when `kind == "amendment"`). |
| `diff` | `Diff` | Optional | Diff against the parent/displaced revision (omitted on first seal). |

---

### `emery build`

```json
{
  "revision": "9f8e7d6c…",
  "waves": [["SLICE-001", "SLICE-003"], ["SLICE-002"]],
  "base": "1a2b3c4d…",
  "repository": "./.emery/vcs/repos/shop",
  "initialised": { "commit": "2b3c4d5e…", "paths": [".gitignore", "Cargo.toml", "src/lib.rs"] },
  "resumed": ["SLICE-001"],
  "slices": [
    {
      "id": "SLICE-003",
      "name": "catalogue",
      "wave": 1,
      "covered": ["REQ-005"],
      "uncovered": [],
      "written": ["src/catalogue.rs"],
      "commit": "5e6f7a8b…"
    },
    {
      "id": "SLICE-002",
      "name": "orders",
      "wave": 2,
      "covered": ["REQ-003"],
      "uncovered": ["REQ-004"],
      "written": ["src/lib.rs", "src/orders.rs", "src/orders/create.rs"],
      "resolved": ["src/lib.rs"],
      "commit": "9c0d1e2f…"
    }
  ],
  "verified": [
    { "wave": 1, "slices": ["SLICE-001", "SLICE-003"], "head": "5e6f7a8b…" },
    { "wave": 2, "slices": ["SLICE-002"], "head": "9c0d1e2f…" }
  ],
  "sealed": [{ "wave": 2, "commit": "7a8b9c0d…", "paths": ["Cargo.lock", "src/orders/create.rs"] }],
  "repaired": [2],
  "head": "9c0d1e2f…",
  "label": "emery/9f8e7d6c…"
}
```

| Field | Type | Presence | Description |
| --- | --- | --- | --- |
| `revision` | `string` | Always | SHA-256 ID of the sealed revision that was built. |
| `waves` | `string[][]` | Always | Dependency waves of `SLICE-NNN` IDs. |
| `candidate` | `{ id, kind }` | Optional | Unsealed candidate (`"revision"`) or amendment (`"amendment"`) standing in `.emery/storage/` that was ignored by the build. |
| `base` | `string` | Always | Base commit SHA the build started from. |
| `repository` | `string` | Always | Target repository path (`"."` or `"./.emery/vcs/repos/<name>"`). |
| `initialised` | `{ commit, paths }` | Optional | `Initialise` commit SHA and sorted relative file paths scaffolded during `Task::Init` on this run. |
| `resumed` | `string[]` | Optional | Slice IDs skipped because they were already merged on `emery/<revision>` or staged in `.emery/vcs/integration`. |
| `slices` | `BuiltSlice[]` | Always | Slices built during this run, in merge order (`{ id, name, wave, covered, uncovered, written, resolved, commit }`). `commit` is `null` if the slice changed no files; `resolved` is omitted when no AI merge resolution was needed. |
| `verified` | `object[]` | Optional | Waves verified during this run (`{ wave, slices, head }`). Omitted if all slices were resumed from the label. |
| `sealed` | `object[]` | Optional | Commits created by `Task::Verify` (`{ wave, commit, paths }`). Omitted when verification left no file changes. |
| `repaired` | `number[]` | Optional | 1-based wave numbers where verification modified paths not covered by an `ours` merge rule. |
| `head` | `string` | Always | Final integrated and verified commit SHA. |
| `label` | `string` | Always | Local branch name pointing to `head` (`emery/<revision>`). |

---

### `emery show <spec|design|plan|amendment>`

```json
{
  "revision": "9f8e7d6c…",
  "body": "---\nemery: 8\nrevision: 9f8e7d6c…\n---\n\n# Specification\n…",
  "document": {
    "emery": 8,
    "parent": null,
    "preamble": ["…"],
    "requirements": [
      {
        "id": "REQ-001",
        "subject": "session.timeout",
        "status": "divergence",
        "covered": true,
        "trigger": { "http": { "method": "POST", "path": "/sessions/{id}/refresh" } },
        "sources": [
          { "source": "intent", "claim": "session.timeout", "path": null },
          { "source": "code", "claim": "session-expiry", "path": "src/session.ts#L12-L30" }
        ],
        "body": ["Sessions must expire after 30 minutes of inactivity."],
        "losers": [
          { "sources": ["code"], "kind": "behaviour", "rank": 3, "claim": "session-expiry", "statement": "…" }
        ],
        "scenarios": [
          { "name": "Session expires", "given": [], "when": "…", "then": "…", "and": [] }
        ]
      }
    ]
  }
}
```

#### Top-level `ShowOutput` fields

| Field | Type | Description |
| --- | --- | --- |
| `revision` | `string` | SHA-256 ID of the revision, staged candidate (`--staged`), or staged amendment (`amendment`). |
| `body` | `string` | Rendered Markdown projection (identical to `text` mode output). |
| `document` | `Spec` \| `Design` \| `Plan` \| `Amendment` | Stored JSON document structure, detailed in the tables below. |

#### `document` schema by artifact

| Artifact | Top-level `document` fields | Element structure |
| --- | --- | --- |
| `spec` | `emery` (`u32`), `parent` (`string \| null`), `preamble` (`string[]`), `requirements` (`Requirement[]`) | Each `Requirement` has `id` (`REQ-NNN`), `subject`, `status` (`"agreed"` \| `"divergence"` \| `"conflict"`), `covered` (`bool`), `trigger` (`{ "http": { "method", "path" } }` \| `{ "message": "<name>" }` \| `null`), `sources` (`{ source, claim, path }[]`), `body` (`string[]`), `losers` (`{ sources, kind, rank, claim, statement }[]`), and `scenarios` (`{ name, given, when, then, and }[]`). |
| `design` | `emery` (`u32`), `parent` (`string \| null`), `preamble` (`string[]`), `sections` (`Section[]`) | Each `Section` has `kind` (`"overview"` \| `"domain-model"` \| `"apis"` \| `"technical-logic"` \| `"ui-layout"` \| `"observability"`) and `blocks` (`{ "text": "<paragraph>" }` or `{ "type": { "key", "signature", "role": "data" \| "mechanism" } }`). |
| `plan` | `emery` (`u32`), `parent` (`string \| null`), `preamble` (`string[]`), `slices` (`Slice[]`) | Each `Slice` has `id` (`SLICE-NNN`), `name`, `requirements` (`REQ-NNN[]`), `types` (`string[]` of `data` type keys), `depends-on` (`SLICE-NNN[]`), and `brief` (`string[]`). |
| `amendment` | `emery` (`u32`), `base` (`{ revision, commit }`), `changes` (`Change[]`), `spec` (`Spec`), `design` (`Design`), `plan` (`Plan`), `sections` (`DesignDiff`), `slices` (`SliceChange[]`) | Each `Change` has `id` (`REQ-NNN`), `subject`, `class` (`"added"` \| `"changed"` \| `"retired"` \| `"conflict"` \| `"unchanged"`), `before` (`string \| null`), `after` (`string \| null`), `sources` (`{ source, claim, path }[]`), and `scope` (`string \| null`, populated when retired via `scope = "whole"`). |

---

### `emery completions <shell>`

Writes the raw shell completion script for `<shell>` (`bash`, `zsh`, `fish`, `elvish`, `powershell`) directly to `stdout` without a JSON envelope.
