# Authority hierarchy

Every Evidence document carries one closed `authority` class. Highest wins:

1. **`intent`** — inline operator directives (the `intent` source adapter is the only first-party emitter).
2. **`documentation`** — operator-provided written product or technical intent (internal docs, RFCs, product notes). Emitted by the `documentation` source adapter.
3. **`behaviour`** — what legacy code actually does. Emitted by behaviour sources such as `typescript` and future code or observation adapters.

The **engine** resolves authority before you are called; the requirements carry the outcome. You never pick winners, derive `Status:`, or order the `Sources:` pairs — you draft honest content for the requirements as they stand.

## Status derivation (engine-computed)

A requirement's contributing claims were grouped into agreeing classes. The engine ranks the classes by authority: one class is `agreed`; a unique highest-authority class winning over lower ones is `divergence`; two classes tied at the top authority are an unresolvable `conflict`. A requirement with no acceptance criterion in evidence is uncovered.

| Contributing classes | `Status:` | Tag |
| -------------------- | --------- | --- |
| 1, covered | `agreed` | (none) |
| 1, uncovered | `unknown` | `[unknown]` |
| ≥2, unique top authority | `divergence` | `[divergence]` |
| ≥2 at the same top authority | `conflict` | `[conflict]` |

An uncovered `divergence` or `conflict` requirement keeps its tag; the engine adds the gap note beneath its loser notes.

## What the resolution renders

- **`agreed`** / **`unknown`** — the shared statement is the body.
- **`divergence`** — the winning class's statement is the body; the engine renders one `Note:` per losing class from their verbatim statements. Your scenarios follow the winner and never mention the losers.
- **`conflict`** — no body at all. The engine renders one `Note:` per class and a closing note handing the decision to the operator; your scenario must not pick a side.
