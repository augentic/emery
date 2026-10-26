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

- Live slides answer, “What should the room remember?”
- Brief slides answer, “What must a reader understand without the presenter?”
- Reference slides answer, “What must a technical reviewer be able to inspect?”
- Proposal slides answer, “What decision is requested, from whom, and on what evidence?”

For side-by-side review, run the live deck on port 3030 and the brief on another port:

```sh
npm run dev:approach -- --port 3030
npm run dev:approach:brief -- --port 3032
```
