---
name: code-review
description: >-
  Careful code-quality sweep of the Emery repository (or a given crate/directory)
  against AGENTS.md. Use when the user invokes /code-review or asks for a
  quality review of Rust workspace code.
disable-model-invocation: true
---

# code-review

Do a careful code-quality sweep of the current repository (or the crate/directory
given as an argument, if any).

Before reviewing, read `AGENTS.md`. Its Invariants and Testing sections are the
contract to review against; everything else is idiomatic Rust as clippy and the
surrounding code have it, not a house rule.

Look for:

1. Code that can be simplified, rationalised, or removed outright.
2. Non-idiomatic Rust that could use recognisable patterns and idioms.
3. Names longer than needed. Heuristic: >15 chars is suspect, >25 needs
   justification. Sharper rule: the module path is context — flag
   `show_registry` in `registry.rs`, which should be `registry::show`.
4. Tests that violate the root-led integration policy: any `src`
   `#[cfg(test)]` test — or crate integration test — whose behavior is
   reachable through the CLI or MCP entry points and already owned (or
   ownable) by a root scenario in `tests/`.
5. YAGNI — abstractions, flags, or generality with no current consumer.
6. Latent bugs and footguns.

Process:
- Partition by workspace crate; use one explore subagent per crate if helpful.
- Each area returns at most its top 5 findings — prioritize, don't enumerate.

Rules of evidence:
- Every finding cites file and line.
- "Unused / can be removed" claims require a search showing no callers,
  including prose (`docs/`, `AGENTS.md`, adapter repos where relevant).
- Skip purely stylistic preferences; a finding names an invariant, a bug, or a
  simplification, not a taste.
- If something might be a contract-locked boundary rather than YAGNI, flag the
  uncertainty instead of asserting.

Output: report only — make no edits. Rank findings by value. For each:
location, one-line problem, proposed change, estimated effort and risk.
