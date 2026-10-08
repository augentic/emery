# Authority hierarchy

Every source is read under one closed `kind`, declared by its adapter, and carries one authority **rank**: an integer, `1` the highest, equal ranks tied. A source's rank is its kind's default unless the project's config gives that source a rank of its own:

1. **`intent`** — inline operator directives (the `intent` source adapter is the only first-party emitter). Default rank `1`.
2. **`documentation`** — operator-provided written product or technical intent (internal docs, RFCs, product notes). Emitted by the `documentation` source adapter. Default rank `2`.
3. **`behaviour`** — what legacy code actually does. Emitted by behaviour sources such as `typescript` and future code or observation adapters. Default rank `3`.

A configured rank may place a source anywhere on the scale — a wiki beneath the code at `4`, a trusted document level with intent at `1` — so read each contributor's rank, not its kind alone, for where it stood.

The **engine** resolves authority before you are called; the requirements carry the outcome. You never pick winners, derive `Status:`, or order the `Sources:` pairs — you draft honest content for the requirements as they stand.

## Status derivation (engine-computed)

A requirement's contributing claims were grouped into agreeing classes. The engine orders the classes by their leading contributor's rank: one class is `agreed`; a class alone at the top rank winning over lower ones is `divergence`; two classes tied at the top rank are an unresolvable `conflict`. A requirement with no acceptance criterion in evidence is uncovered.

| Contributing classes | `Status:` | Tag |
| -------------------- | --------- | --- |
| 1, covered | `agreed` | (none) |
| 1, uncovered | `unknown` | `[unknown]` |
| ≥2, unique top rank | `divergence` | `[divergence]` |
| ≥2 tied at the top rank | `conflict` | `[conflict]` |

An uncovered `divergence` or `conflict` requirement keeps its tag; the engine adds the gap note beneath its loser notes.

## What the resolution renders

- **`agreed`** / **`unknown`** — the shared statement is the body.
- **`divergence`** — the winning class's statement is the body; the engine renders one `Note:` per losing class from their verbatim statements. Your scenarios follow the winner and never mention the losers.
- **`conflict`** — no body at all. The engine renders one `Note:` per class and a closing note handing the decision to the operator; your scenario must not pick a side.

A `Note:` names the class's sources, then in parentheses the leading contributor's kind, its rank, and its claim id — `Note: code (behaviour, rank 3, session-expiry): …` — so the reader sees why the class lost without knowing the defaults.
