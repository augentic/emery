# Set authority ranks and resolve source conflicts

This guide shows how to configure source authority ranks (`rank`) so higher-authority sources automatically override lower-authority ones (`[divergence]`), and how to inspect and resolve tied disagreements (`[conflict]`).

## Contents

- [Understand default authority ranks](#understand-default-authority-ranks)
- [Override authority rank in `emery.toml`](#override-authority-rank-in-emerytoml)
- [Identify `[divergence]` vs. `[conflict]` in output](#identify-divergence-vs-conflict-in-output)
- [Resolve a `[conflict]` requirement](#resolve-a-conflict-requirement)
  - [Option 1: Update the contradicting source and run `emery amend`](#option-1-update-the-contradicting-source-and-run-emery-amend)
  - [Option 2: Assign distinct `rank` values in `emery.toml`](#option-2-assign-distinct-rank-values-in-emerytoml)
- [See also](#see-also)

---

## Understand default authority ranks

When two sources make different statements about the same requirement, Emery compares their **authority ranks** (`1` is the highest authority; lower numbers outrank higher numbers).

If you omit `rank` on a `[[source]]` entry, Emery uses the default rank for the adapter's kind:

| Adapter kind | Default `rank` | Typical adapters |
| --- | --- | --- |
| `intent` | `1` (highest) | `emery:intent` |
| `documentation` | `2` | `emery:documentation` |
| `behaviour` | `3` | `emery:typescript`, `emery:python` |

By default, an `intent` prompt (`rank = 1`) overrides `documentation` (`rank = 2`), which in turn overrides existing code (`behaviour`, `rank = 3`).

---

## Override authority rank in `emery.toml`

Set `rank` (a positive integer `>= 1`) on any `[[source]]` entry in `emery.toml` to customize priority.

For example, if you are extracting behaviour from production TypeScript code (`rank = 3`) alongside an older internal wiki (`emery:documentation`, default `rank = 2`), and you want the production code to win whenever they disagree, assign `rank = 4` to the wiki:

```toml
[[source]]
name = "production-api"
adapter = "emery:typescript@0.39.0"
path = "src/"
# Uses default behaviour rank = 3

[[source]]
name = "legacy-wiki"
adapter = "emery:documentation@0.39.0"
path = "wiki/"
rank = 4
```

Now `production-api` (`rank = 3`) outranks `legacy-wiki` (`rank = 4`).

---

## Identify `[divergence]` vs. `[conflict]` in output

When sources disagree on a requirement, one of two statuses results:

- **`divergence` (one winner at the top rank):** One agreement class holds the unique highest authority rank. Emery uses that winning statement as the requirement body and records the overridden lower-ranked statement as a `Note:` in `emery show spec`:

  ```markdown
  ### REQ-003: orders.timeout [divergence]

  Status: divergence
  Sources: intent, production-api (src/orders.ts#L42-L50)

  The service cancels unpaid orders after 15 minutes.

  Note: `production-api` (behaviour, rank 3) states: The service cancels unpaid orders after 30 minutes.
  ```

- **`conflict` (tie at the top rank):** Two or more disagreeing sources share the exact same top authority rank (for example, two `documentation` sources both at `rank = 2`). Emery refuses to guess: it leaves the requirement statement empty, marks it `[conflict]`, and prints `Note: Operator reconciliation required.`:

  ```markdown
  ### REQ-001: greeting.behaviour [conflict]

  Status: conflict
  Sources: docs (docs/greeting.md#L1-L7), rfc (docs/rfc.md#L1-L5)

  Note: `docs` (documentation, rank 2) states: GET /greeting returns the static string 'hello, world'.
  Note: `rfc` (documentation, rank 2) states: GET /greeting returns the static string 'howdy'.
  Note: Operator reconciliation required.
  ```

During `emery amend`, any conflicted requirement is also listed on the `conflicts:` summary line and under `## Conflict` in `emery show amendment`.

---

## Resolve a `[conflict]` requirement

A `[conflict]` does not block `emery apply`—you can seal a conflicted specification into Git so the disagreement is visible in history. However, before running `emery build`, you should resolve the conflict so target adapters have an authoritative requirement statement.

### Option 1: Update the contradicting source and run `emery amend`

If the specification is already sealed with a `[conflict]`:

1. Edit the outdated source file so the sources agree (or remove the contradicting inline `--description`).
2. Run `emery amend` (and `emery apply`):

```bash
emery amend
emery show amendment
# Under ## Changed:
#   Before: [conflict] GET /greeting returns 'hello, world' | GET /greeting returns 'howdy'
#   After: GET /greeting returns the static string 'hello, world'.
emery apply
```

Emery matches the updated source claim against the standing `[conflict]` requirement, transitions it to `changed`, and restores its statement body.

### Option 2: Assign distinct `rank` values in `emery.toml`

If both sources must remain as written (for example, an authoritative RFC vs. an older README, both read by `emery:documentation`), give the authoritative source a higher rank (lower integer) in `emery.toml`:

```toml
[[source]]
name = "rfc"
adapter = "emery:documentation@0.39.0"
path = "docs/rfc.md"
rank = 1

[[source]]
name = "docs"
adapter = "emery:documentation@0.39.0"
path = "docs/greeting.md"
rank = 2
```

Re-run `emery specify` (or `emery amend`). `rfc` (`rank = 1`) now wins over `docs` (`rank = 2`), turning the `[conflict]` into a resolved `[divergence]`.

---

## See also

- [Synthesis, authority, and the technology boundary](../explanation/synthesis-and-authority.md#3-authority-ranks-and-reconciliation) — how agreement classes and authority ranks work.
- [Configuration (`emery.toml`)](../reference/config.md#authority-rank-defaults) — `rank` field reference.
