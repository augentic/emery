# target.build

Build one slice of the plan into the lent project tree, as a Markdown stand-in for the code a real target would write.

## Inputs

- `$WORKSPACE` — the project tree, lent writable. Read it through the workspace tools; write it through this call's `write_files` tool, one or more files per call, each created or replaced whole, at a `/`-separated path relative to `$WORKSPACE`, and change within a file you wrote through `edit_files`. Every file you write goes beneath it. `build/README.md` is in the tree before you start, scaffolded when the repository was initialised; it is not yours to write.
- **The slice's plan entry** — its id, name, requirements, the design types it owns, and the slices built before it.
- **The specification, cut to the slice** — the requirements to implement, each with its acceptance scenarios.
- **The design, cut to this slice** — the sections every slice shares, with the `Type:` blocks cut to yours: a type another slice owns stands whole where your slice's text names it, and is left out otherwise, listed by key at the end; a `(mechanism)` type always stands whole.

Nothing outside `$WORKSPACE` is reachable. Build this slice completely in one pass.

## What to write

Write one directory, `build/<slice-name>/`, holding:

- `index.md` — the slice's name and id, then one line per requirement it implements.
- `<REQ-NNN>.md` per requirement — the requirement's subject, how the behaviour is satisfied, and the scenario that verifies it, each scenario's `WHEN` and `THEN` quoted from the specification.

Write them through `write_files`, the whole directory in one call; it is created with the first file. Read what `build/` already holds before writing: a slice built before this one may have written a directory beside yours, which you leave as it is. Never write outside `build/`.

## Report

Answer with one JSON object:

- `covered` — each requirement id the specification above holds that `build/<slice-name>/` now implements, once each. Leave an id out rather than claim what the tree does not hold.
- `written` — the files `write_files` wrote, once each, as a `/`-separated path relative to `$WORKSPACE`, and no file the tree does not hold; one you leave out is added for you.

```json
{
  "covered": ["REQ-001"],
  "written": ["build/greeting/index.md", "build/greeting/REQ-001.md"]
}
```
