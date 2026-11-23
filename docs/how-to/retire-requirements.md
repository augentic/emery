# Retire requirements explicitly or by scope

This guide shows the two ways to remove obsolete requirements from a sealed specification during `emery amend`: explicitly via a prose source (`intent` or `documentation`) or automatically by whole-source scope (`scope = "whole"`) on a code adapter.

## Contents

- [Choose the right retirement method](#choose-the-right-retirement-method)
- [Method 1: Retire explicitly with an `intent` or `documentation` source](#method-1-retire-explicitly-with-an-intent-or-documentation-source)
  - [Fix `retire-unmatched` errors](#fix-retire-unmatched-errors)
- [Method 2: Retire automatically with `scope = "whole"` on code sources](#method-2-retire-automatically-with-scope--whole-on-code-sources)
  - [Important rule: stem coverage in `scope = "whole"`](#important-rule-stem-coverage-in-scope--whole)
- [What happens to the design and build plan when requirements retire](#what-happens-to-the-design-and-build-plan-when-requirements-retire)
- [See also](#see-also)

---

## Choose the right retirement method

During [`emery amend`](../reference/cli/amend.md), omitting a requirement from a default (`scope = "delta"`) source does **not** delete it—untouched requirements stay `unchanged`. To withdraw an existing requirement, use one of two mechanisms:

| Method | Best when | Adapter types |
| --- | --- | --- |
| **Explicit withdrawal (`retire` claim)** | You are amending via `--description`, an `emery:intent` prompt, or an RFC/changelog document that states a feature is being removed. | Prose adapters (`emery:intent`, `emery:documentation`) |
| **Whole-source silence (`scope = "whole"`)** | You removed a handler or behaviour from your TypeScript/Python codebase and want the specification synced to what the codebase still implements under that stem. | Behaviour adapters (`emery:typescript`, `emery:python`) |

---

## Method 1: Retire explicitly with an `intent` or `documentation` source

Prose adapters (`emery:intent` and `emery:documentation`) emit a `retire` claim whenever a passage explicitly withdraws a behaviour.

First, check the existing requirement's subject ID in `emery show spec` (for example, `orders.export-csv` under `REQ-004`). Then pass an inline description or Markdown document stating that the behaviour is withdrawn:

```bash
emery amend --config \
  --description "emery:intent@0.39.0=Retire orders.export-csv: CSV order export is removed."
```

Review the staged amendment:

```bash
emery show amendment
```

Under `## Retired`, you will see `REQ-004: orders.export-csv [retired]` with its withdrawn statement. Seal it with `emery apply`.

> [!NOTE]
> If a prose passage withdraws a behaviour *and* replaces it with a new behaviour under the same subject (or if one source emits both a `retire` claim and a `requirement` claim with the same ID), Emery treats it as a restatement (`changed`), never a self-conflict.

### Fix `retire-unmatched` errors

If a `retire` claim does not match any existing requirement in the current revision, `emery amend` fails with `retire-unmatched` (exit `1`):

```text
error retire-unmatched: retire claim `orders.csv` from `intent` matched no requirement in the current specification
  hint: compare the claim's id against `emery show spec` and remove or reword the withdrawal
```

Run `emery show spec` to check the exact `<stem>.<behaviour>` subject ID of the requirement you want to withdraw, and name that subject in your prompt or document.

---

## Method 2: Retire automatically with `scope = "whole"` on code sources

When extracting from a codebase with `emery:typescript` or `emery:python`, deleted code leaves no trace for the adapter to emit a `retire` claim from. Instead, set `scope = "whole"` on that `[[source]]` entry in `emery.toml`:

```toml
[[source]]
name = "orders-service"
adapter = "emery:typescript@0.39.0"
path = "src/"
scope = "whole"

[target]
adapter = "emery:omnia@0.39.0"
repository = "."
```

When you run `emery amend`:

1. `orders-service` extracts all current claims from `src/`.
2. When `.emery/storage/` holds a source snapshot for the base revision, Emery narrows `orders-service` to claims overlapping changed spans or newly added files and puts only affected standing requirements in play. Without a base snapshot (for example, on a fresh clone), Emery evaluates every extracted claim and collects covered stems (`<stem>.*`) and sole-source citations.
3. Any untouched standing requirement citing a changed or deleted span or removed file of `orders-service` or carrying a vanished trigger (or, without a snapshot, falling under a covered stem or citing `orders-service` alone) is automatically retired with the note:
   ``Note: retired by scope: `orders-service` was read whole and states nothing under `orders`.``

### Important rule: snapshot narrowing and fallback in `scope = "whole"`

When a base source snapshot is present in `.emery/storage/`, `scope = "whole"` retires untouched requirements only where they cite a changed or deleted span or removed file of that behaviour source or carry a vanished trigger—leaving untouched routines, untouched files, and other stems untouched by code. On a fresh clone with no snapshot yet in `.emery/storage/`, the first amendment falls back to retiring untouched requirements under stems where that behaviour source emitted **at least one** `requirement` claim during the run, or whose non-empty base `sources` cite that behaviour source alone:

- This retires an entire stem automatically when all of its files are deleted from `orders-service` (when `orders-service` was the sole source behind those requirements), while protecting stems owned by *other* sources (such as `billing.*` or `audit.*` from `documentation` or another service) from being wiped out.
- If a deleted stem's requirements also cited another source (such as `documentation`), `scope = "whole"` leaves them `unchanged`; use Method 1 (`--description` with `emery:intent` or updating `documentation`) to withdraw them explicitly.

*(Setting `scope = "whole"` on a prose source like `emery:documentation` or `emery:intent` has no effect—prose sources only retire requirements through explicit `retire` claims.)*

---

## What happens to the design and build plan when requirements retire

When `emery amend` retires a requirement:

- **Specification (`spec.json`):** The requirement is removed completely. Remaining requirements keep their existing `REQ-NNN` IDs without renumbering.
- **Build plan (`plan.json`):** The retired `REQ-NNN` is removed from its part.
  - If the part still holds other requirements, it stays in the plan (marked `(brief kept)` unless a slicing turn also ran).
  - If the part has no requirements left, the part is removed from `plan.json`, any parts that depended on it are re-wired to its dependencies, and any domain data types owned by that emptied part are removed from `design.json`.
- **Design (`design.json`):** If any paragraph in `design.json` cited the retired `REQ-NNN`, Emery opens a `design-draft` (`DesignAmend`) turn to update or remove the outdated design prose.

---

## See also

- [`emery amend` CLI reference](../reference/cli/amend.md) — change classes, `scope = "whole"`, and limits.
- [Evolving a specification with `emery amend`](../tutorials/amending-a-spec.md) — step-by-step amendment tutorial.
