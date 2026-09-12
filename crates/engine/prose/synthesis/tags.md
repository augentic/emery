# Tag grammar

Three review-signal tags render into `spec.md` from each requirement's status, after the heading name. The engine writes them; they document uncertainty inline so the operator can resolve it at the source and re-run `emery specify`.

## Closed tag set

| Tag | Mirrors `Status:` | Meaning | Operator action |
| --- | ----------------- | ------- | --------------- |
| `[unknown]` | `unknown` | Agreed, but no evidenced acceptance behaviour | Bind a source that evidences it; re-run |
| `[conflict]` | `conflict` | Same-class disagreement; no winner | Amend or drop a source; re-run |
| `[divergence]` | `divergence` | Disagreement; one wins by authority (`intent > documentation > behaviour`) | Override the source set if the winner is wrong; otherwise proceed |

One tag per heading, mirroring `Status:`; `conflict` outranks `divergence`, which outranks `unknown`, so an uncovered divergence keeps `[divergence]` and gains the gap note. `Status: agreed` carries no tag.

## What the tag asks of your scenarios

- **`[unknown]`** — a scenario that states what is checked and does not invent the acceptance behaviour.
- **`[conflict]`** — a scenario that names what must be decided without picking a side.
- **`[divergence]`** — scenarios that follow the winning value, with no mention of the loser.

## Anti-patterns

Restating a tag, status, or note in a paragraph; auto-resolving a `[conflict]`; guessing acceptance behaviour for an `[unknown]` requirement.
