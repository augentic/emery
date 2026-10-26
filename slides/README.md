# Slide decks

The decks are separate presentation products with one shared argument:

- `approach-live.md` is the live narrative. Slides carry one claim and enough visual structure for a presenter to speak to.
- `approach-brief.md` is the self-reading approach brief. It carries the explanatory detail omitted from the live deck.
- `reference.md` is the technical leave-behind for architecture, security, platform, and governance review.
- `proposal.md` is the client-specific decision document produced after an engagement.

Use `npm run dev:approach` for the live deck and `npm run dev:approach:brief` for its self-reading companion. The matching `build:*` and `export:*` commands preserve that distinction.

`content/approach.json` is the synchronization contract between those two decks. It fixes their slide ids, order, titles, and shared claims or evidence. Every slide carries the matching `sync` value in its frontmatter. `npm run check:approach` refuses a missing, reordered, renamed, or factually divergent pair; the approach build and export commands run that check first.

When working on the approach:

1. Change the live treatment in `approach-live.md`.
2. Change `content/approach.json` when the shared title, claim, or evidence changes.
3. Carry that change into the corresponding slide of `approach-brief.md`.
4. Run `npm run check:approach`.

Layout and explanatory detail remain specific to each use:

- Live slides answer, “What should the room remember?” — one headline, at most one short claim, and a visual diagram or sparse cards with short labels ($\le 14$ words per card body; never multi-sentence prose).
- Brief slides answer, “What must a reader understand without the presenter?”
- Reference slides answer, “What must a technical reviewer be able to inspect?”
- Proposal slides answer, “What decision is requested, from whom, and on what evidence?”

Guardrails for `approach-live.md` and `approach-brief.md`:

- **Preserve bespoke visual diagrams:** Only edit slides relevant to the request; never replace bespoke visual structures (`.live-runtime-stack`, `.live-portability`, `.live-sdd-flow`, `.live-durability`, `.live-trust-model`, `.live-stat-grid`, `.live-repairs`, etc.) with text-heavy cards.
- **No internal product/crate names:** Clients do not know `Emery` or `Omnia`. Use *Spec-driven delivery* (`spec.md`, `design.md`) and *Sandboxed execution* (`WASI Component`, `Wasmtime sandbox`). Reserve `Emery` and `Omnia` for `reference.md` and `proposal.md`.
- **Talk track vs. slide copy:** Put narrative prose in `<!-- speaker notes -->` and `approach-brief.md`, never inside `.live-card` elements in `approach-live.md`.

For side-by-side review, run the live deck on port 3030 and the brief on another port:

```sh
npm run dev:approach -- --port 3030
npm run dev:approach:brief -- --port 3032
```
