# `emery amend`

This page is the CLI reference for `emery amend`, which extracts claims from change sources, matches them against the existing requirements in the current revision, and stages an incremental `Amendment` for review and sealing. It is for developers and operators evolving an already-sealed specification.

## Contents

- [Synopsis](#synopsis)
- [Description](#description)
  - [Requirement change classes](#requirement-change-classes)
  - [How `spec`, `design`, and `plan` are updated](#how-spec-design-and-plan-are-updated)
- [The change file and `scope = "whole"`](#the-change-file-and-scope--whole)
- [Options](#options)
- [JSON output](#json-output)
- [Text output](#text-output)
- [Errors and recovery](#errors-and-recovery)
- [Limits](#limits)
- [See also](#see-also)

---

## Synopsis

```bash
emery amend <adapter>... [--description <adapter>=<text>] --config [<path>]
emery amend --config <path>                           # use [[source]] tables from a change file
emery amend --config                                  # use project-root emery.toml sources as the change
emery amend --config --description <adapter>=<text>   # layer an inline change over emery.toml
emery amend --config --apply                          # stage and seal in one run
emery amend                                           # discovers the project-root emery.toml
```

---

## Description

While [`emery specify`](specify.md) generates a revision from scratch and renumbers requirements from `REQ-001`, `emery amend` reads its sources as an incremental **change** against the **current revision** sealed in your target repository. It preserves existing `REQ-NNN` and `SLICE-NNN` identifiers, redrafts only the requirements, design sections, and slices affected by the change, and stages an `Amendment` in `.emery/storage/`.

Before loading any adapters, `emery amend` validates five preconditions:

1. **Valid source names:** No change source may use the reserved name `baseline` (which Emery uses internally for the current revision's existing requirements).
2. **Target configured:** A `[target]` table must be present in the change file or project-root `emery.toml` (`build-target-required` if missing; passing CLI sources without `--config` reads no file and therefore fails).
3. **Current revision exists:** The target repository must hold a readable sealed revision (`spec-not-generated`, `spec-mismatch`, or `spec-outdated` otherwise).
4. **No staged candidate:** No unsealed `specify` candidate may be currently staged in `.emery/storage/` (`amendment-pending` if one is staged; re-running `emery amend` replaces a previously staged *amendment*, never a candidate).

### Requirement change classes

`emery amend` extracts claims from all change sources in parallel and groups them alongside the existing requirements from the current revision (`baseline`, ranked one step below the lowest-ranked change source so new changes always outrank the baseline). Every requirement is classified into one of five classes:

| Class | Meaning | ID & subject behaviour |
| --- | --- | --- |
| `added` | A group of change claims that matched no existing requirement in the current revision. | Assigned a new `REQ-NNN` ID numbered sequentially starting above the highest `ReqId` present in the current base specification (`max_id + 1`). |
| `changed` | An existing requirement matched by a change claim whose statement, trigger, or subject differs from the base (or that resolves a `[conflict]` in the base). | Keeps its existing `REQ-NNN` ID, adopts the change's subject (even if the stem changed), and records `before` and `after` statements. |
| `retired` | An existing requirement withdrawn either explicitly by a matching `retire` claim, or implicitly by `scope = "whole"` on a behaviour source. | Removed from the amended `spec.json` and `plan.json`. |
| `conflict` | Two or more change sources tied at the top authority rank disagree with one another (or one withdraws while another restates at the same rank). | Keeps its `REQ-NNN` ID (or receives a new one if added) and is marked `[conflict]` with competing claims listed in `sources`. |
| `unchanged` | An existing requirement that no change claim touched, or that a change claim restated identically. | Copied byte-for-byte from the current revision (`before`, `after`, and scenarios unchanged). |

> [!NOTE]
> **Resolving a base conflict:** An existing requirement that was sealed in `[conflict]` enters the grouping step with its competing statements formatted as `[conflict] <a> | <b>`. When a change claim matches it without a new tie among change sources, the requirement becomes `changed` (`Before: [conflict] <a> | <b>`, `After: <winning statement>`), resolving the conflict.
>
> **Within-source withdrawal + restatement:** A source cannot conflict with itself. If a single source emits both a `retire` claim and a `requirement` claim with the same ID, Emery ignores the `retire` claim and treats the pair as a restatement.

### How `spec`, `design`, and `plan` are updated

- **Specification (`spec`):** Only `added`, `changed`, and `conflict` requirements are sent to `spec-draft` turns. The base preamble and all `unchanged` requirements are copied byte-for-byte, and `retired` requirements are omitted. If the change contains no drafted requirements (for example, only `retire` or `decision` claims), zero `spec-draft` turns run.
- **Design (`design`):** Type blocks owned by slices that became empty due to retirements are removed first. A `design-draft` (`DesignAmend`) turn runs **only** if the change's claims inform a design section (for example, a `decision`, `call`, `contract`, `type`, or `excerpt` claim) or if a base design paragraph cites a `retired` requirement. When it runs, it redrafts `## Overview` and the informed sections while keeping all other sections verbatim.
- **Build plan (`plan`):** Updated deterministically by code first: existing slices keep their `SLICE-NNN` IDs and names; retired requirements leave their slices; empty slices are removed and their dependants re-wired to their dependencies; restemmed or added requirements join the slice holding their trigger or stem floor; unowned data types join the single touched slice when nothing else is open. A `slicing` (`SliceAmend`) turn runs only if requirements fall under a new stem, push a stem past `SLICE_CAP` (12), or leave data type ownership ambiguous.
- **No-op changes:** If every requirement is `unchanged` and neither the design nor the plan changed, `emery amend` stages nothing and prints `nothing staged: no change against the current revision`.

This is the CLI command invoked by the [`/emery:amend`](../../../plugins/emery/skills/amend/SKILL.md) Cursor skill.

---

## The change file and `scope = "whole"`

A change file is a standard [`emery.toml`](../config.md) passed via `--config [<path>]`. In `emery amend`, each `[[source]]` entry supports the `scope` setting (`"delta"` by default, or `"whole"`):

- **`scope = "delta"` (default):** The source states only what changed. Existing requirements in the current revision that this source does not mention remain `unchanged`.
- **`scope = "whole"` (behaviour sources only):** Tells Emery that a code adapter (`emery:typescript`, `emery:python`) read the complete source tree for its domain. Emery collects every stem (`<stem>.*`) covered by at least one `requirement` claim emitted by that source during the run. Any existing requirement under one of those covered stems that remains `unchanged` (with no change claim matched to it) is automatically classified as **`retired`**, with `scope` set to the source's name and the note:
  ``Note: retired by scope: `<source>` was read whole and states nothing under `<stem>`.``
- **Prose sources (`intent`, `documentation`):** Setting `scope = "whole"` on a prose source never retires requirements by silence; prose sources withdraw requirements only through explicit `retire` claims.

---

## Options

| Option | Short | Description |
| --- | --- | --- |
| `<adapter>...` | — | Workspace-backed change source adapters (`namespace:name@version`). Requires `--config` so `[target]` is known. Replaces a `[[source]]` entry of the same package name or appends a new entry. |
| `--description <adapter>=<text>` | `-d` | Inline change source (`<adapter>=<text>`). Repeatable. Requires `--config` so `[target]` is known. |
| `--config [<path>]` | `-c` | Change configuration file (defaults to `emery.toml` when `<path>` is omitted). If a custom file omits `[target]`, Emery falls back to `[target]` in the project-root `emery.toml`. |
| `--apply` | — | Seal the staged amendment as the next revision in the same run (if sealing fails, the amendment stays staged). |
| `--format <text\|json>` | — | Output format (`text` or `json`; env `EMERY_FORMAT`). |

---

## JSON output

When `--format json` is passed, `emery amend` writes a single JSON object to `stdout` (see [CLI output shapes](../cli-output-shapes.md#emery-amend) for a full example):

| Field | Type | When present | Description |
| --- | --- | --- | --- |
| `amendment` | `string` | When a change was staged | Content-addressed SHA-256 ID of the staged `Amendment` document. |
| `staged` | `boolean` | Always | `true` when the amendment remains staged; `false` when nothing changed or `--apply` sealed it. |
| `base` | `{ revision, commit }` | Always | The current revision ID and Git commit SHA the amendment was prepared against. |
| `tally` | `object` | Always | Counts by class: `{ added, changed, retired, conflict, unchanged, by-scope }`. |
| `conflicts` | `string[]` | When `tally.conflict > 0` | Subject IDs of requirements in `conflict`, in `ReqId` order. |
| `stems` | `string[]` | When any requirement changed | Sorted unique stems touched by non-`unchanged` requirements. |
| `sections` | `object` | When `design` changed | `{ preamble, added, removed, changed }` listing modified design section kinds. |
| `slices` | `object[]` | When `plan` changed | `{ id, name, class, brief }` for each `added`, `changed`, or `removed` slice (`brief` is `"redrafted"` or `"kept"`). |
| `repositories` | `object[]` | When a source uses `repository` | Resolved Git commits for remote change sources (`{ source, repository, revision, commit }`). |
| `applied` | `object` | When `--apply` succeeded | Full [`emery apply`](apply.md#json-output) output object with `kind: "amendment"`. |

---

## Text output

```text
staged amendment e5378831…
  base 9f8e7d6c… at 4e5f6a7b
  1 added, 1 changed, 1 retired (1 by scope), 0 conflict, 4 unchanged
  stems: access, orders, session
  design: overview, apis
  slices: orders (brief kept), access (new)
  nothing sealed; `emery show amendment` reviews it
```

When `--apply` is passed, the final line is replaced by `  sealed revision <id> as emery/<id> at <commit8>` followed by `    amends <parent>`.

---

## Errors and recovery

| Error code | Exit | Cause | Recovery |
| --- | --- | --- | --- |
| `specify-source-required` | `1` | No change sources passed and no `emery.toml` found. | Pass `<adapter>... --config` or `--config <path>`. |
| `build-target-required` | `1` | CLI sources were passed without `--config`, or neither the change file nor `emery.toml` has a `[target]` table. | Pass `--config` or add `[target]` to `emery.toml`. |
| `amendment-pending` | `1` | An unsealed `specify` candidate is currently staged in `.emery/storage/`. | Seal it with `emery apply` or inspect it with `emery show spec --staged` first. |
| `retire-unmatched` | `1` | A `retire` claim did not match any existing requirement in the current revision. | Compare the claim's ID in the error message against `emery show spec` and update or remove the withdrawal. |
| `spec-not-generated` | `2` | No sealed revision exists in the target repository, or the cached `emery/<id>` branch is behind `HEAD`. | Seal an initial revision with `emery specify --apply`, or merge/rebase `emery/<id>` onto `HEAD`. |
| `spec-mismatch` | `1` | `spec/` in the target repository contains non-revision files or hand-edited/conflicted JSON. | Restore `git checkout emery/<revision> -- spec/` or keep one branch's `spec/` whole after a merge. |
| `spec-outdated` | `1` | The current revision uses an older `emery` schema version. | Regenerate with `emery specify --fresh`. |
| `amendment-stale` | `1` | Under `--apply`, the target repository's current revision or commit moved while `amend` was running. | Re-run `emery amend` and `emery apply`. |

---

## Limits

- **Added requirement IDs start after the current base's highest ID (`max_id + 1`).** If the highest-numbered requirement (for example, `REQ-010`) was retired in an earlier amendment so the specification's highest standing ID is now `REQ-009`, the next amendment's added requirement will be assigned `REQ-010`.
- **`scope = "whole"` covers only stems present in the source's extracted claims.** A whole-scope behaviour source retires untouched requirements under stems where that source emitted at least one `requirement` claim. If an entire stem's files were deleted so the adapter emits zero claims under that stem, use an explicit `retire` claim in an `intent` or `documentation` source to withdraw that stem's requirements.
- **Uninformed design sections keep their base prose.** The design turn runs only on `## Overview` and sections informed by the change's claims (or citing a `retired` requirement). If a requirement's behaviour changes without a `decision`, `call`, `contract`, `type`, or `excerpt` claim informing a section, paragraphs citing that requirement in uninformed sections remain as they were in the base design.
- **Slices updated by code alone keep their base brief.** When a slice's requirement list, owned types, or dependencies change deterministically by code without opening a `slicing` turn, its prose brief is not redrafted. Emery flags these slices as `(brief kept)` in terminal output and adds `Note: the brief is the base's; no slicing turn redrafted it.` in `emery show amendment`.
- **One default source name per adapter on the CLI.** Passing both a positional `<adapter>` and `--description <adapter>=<text>` for the same adapter package without custom names in `emery.toml` binds the same source name twice and fails with `bad_request`. Use `--description` alone alongside `--config`, or define named `[[source]]` entries in your config file.
- **Builds after an amendment rebuild all slices.** Because slice merge commits are tied to a specific revision ID, `emery build` after sealing an amendment builds all slices of the new revision over the existing tree rather than skipping unchanged slices.
- **Merging two branches requires keeping one side's `spec/` whole.** If two branches independently seal amendments and merge in Git, `spec/spec.json` will conflict. Keep one branch's `spec/` directory whole (`git checkout <branch> -- spec/`), commit the merge, and re-run the other branch's `emery amend` on top of the merge commit.

---

## See also

- [`emery show`](show.md) — inspect the staged amendment with `emery show amendment`.
- [`emery apply`](apply.md) — seal the staged amendment as the next revision in Git.
- [Configuration (`emery.toml`)](../config.md) — full `[[source]]` and `scope` reference.
- [Revision lifecycle](../../explanation/revision-lifecycle.md) — how candidates, amendments, and lineages interact.
