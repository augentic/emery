# Design format

Hard-coded conventions of the rendered `design.md`. These are not configurable.

`design.md` carries the technical shape the non-behavioural claims evidence. The engine renders its front matter (`emery: <grammar>`, `revision: <id>`), `# Design`, your `preamble` paragraphs, then one `## ` section per drafted entry, in this fixed order whatever order you answer in:

1. `overview` → `## Overview` — what the system is and why, from `intent` and top-level `section` claims and the request's requirements. Always required.
2. `domain-model` → `## Domain model` — types and identifiers. Reference every `type` claim exactly once as a `type` block, each with its `role`; the engine renders a `Type: <key>` line — `Type: <key> (mechanism)` for a mechanism — and the claim's `signature` verbatim as a fenced code block beneath it.
3. `apis` → `## APIs and integrations` — external surfaces (`call` / `contract` claims, surface-naming requirements). Name each system the program reaches by its role — a message stream consumed or published, a store, a cache, an upstream service, an identity provider, a monitor — and beside it the technology the `call` claims spell as playing that role, with the topic, URL, entity, and keys as constructed, so a target can map the role onto its own platform and the operator can see what the source ran against. Spell each of these as the claims state it, and say `[unknown]` where they are silent:
   - a stream's topic as the code constructs it — the setting it reads, the default it falls back to, any suffix joined on — and the key each message is published under;
   - an upstream call's method, path, and the fields of its response the code reads, by path (`data.all[].vehicleLabel`), since a target reproduces the response's shape from this section alone;
   - a produced message's wire shape — its field names as sent, nested as sent, with the constants the code fixes (`eventType: "Location"`, `gpsAccuracy: 0`) — where no `type` claim declares it, since a shape the source builds in a function body reaches the design through the `requirement` or `excerpt` that states it, and this is where a target reads it.
4. `technical-logic` → `## Technical logic` — delegation, validation, errors; fold abstracting `excerpt` claims.
5. `ui-layout` → `## UI / layout` — only with spatial claims (`region` / `container` / `leaf`), rendered as one tree.
6. `observability` → `## Observability` — only when claims evidence metrics, traces, or logs.

## Section plan

The request lists every section with its presence — `required`, `permitted`, or `omitted` — computed by the engine from the claim kinds present. Draft every `required` section, never an `omitted` section, and a `permitted` section only when claims inform it. Each drafted section carries at least one block.

## Blocks

- **`{"text": "…"}`** — one Markdown paragraph. Cite decisions inline as `(from <source>)`, where `<source>` is exactly one bound source name; a citation naming an unbound source is refused. Refer to requirements by their `REQ-NNN` id inline — no `### Requirement:` or `#### Scenario:` heading and no provenance line belongs here.
- **`{"type": {"key": "<key>", "role": "data" | "mechanism"}}`** — a `type` claim by its declared name — its `name`, else its id, else its path with any line anchor stripped — under `domain-model` only. Interleave with text blocks to place each signature; every listed type claim is referenced exactly once, or the draft is refused. The `role` is what the type stands for, and it decides what a build does with it:
  - `data` — a shape the program reads, stores, or produces: a message consumed or published, a record, an upstream response, an enumeration, a validation outcome. A build lays it, and the plan gives it to the one slice whose requirements read or produce it.
  - `mechanism` — a class that is part of the source's own machinery rather than its data: a client for an upstream service, a secret retriever, a settings carrier, a bootstrap or main, a converter whose state is its dependencies. A build lays nothing for it — the behaviour its requirements state is built through the target's own platform — and no slice owns it. Decide by what the type holds, not by its name: a class holding a URL and a method that fetches is a mechanism; a class holding the fields a message carries is data, whatever the source called it.

Fold `decision` and `section` claims into the section they inform. Where the claims are silent, say nothing — never pad a section with invented architecture. No timestamps or run identifiers; re-runs must be byte-identical.
