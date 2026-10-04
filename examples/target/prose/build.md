# target.build

Build one slice of the plan into the lent project tree, as a Markdown stand-in for the code a real target would write.

## Inputs

- `$WORKSPACE` — the project tree, lent writable. Read it through the workspace tools; write it through this call's `write_file` tool, one file per call, created or replaced whole, at a `/`-separated path relative to `$WORKSPACE`. Every file you write goes beneath it.
- **The slice's plan entry** — its id, name, requirements, the design types it owns, and the slices built before it.
- **The specification, cut to the slice** — the requirements to implement, each with its acceptance scenarios.
- **The design, whole** — the types and sections every slice shares.

Nothing outside `$WORKSPACE` is reachable. Build this slice completely in one pass.

## What to write

Write one directory, `build/<slice-name>/`, holding:

- `index.md` — the slice's name and id, then one line per requirement it implements.
- `<REQ-NNN>.md` per requirement — the requirement's subject, how the behaviour is satisfied, and the scenario that verifies it, each scenario's `WHEN` and `THEN` quoted from the specification.

Write each through `write_file`; the directory is created with the first file. Read what `build/` already holds before writing: a slice built before this one may have written a directory beside yours, which you leave as it is. Never write outside `build/`.

## Report

Answer with one JSON object:

- `covered` — each requirement id the specification above holds that `build/<slice-name>/` now implements, once each. Leave an id out rather than claim what the tree does not hold.
- `written` — each file `write_file` wrote, once each, as a `/`-separated path relative to `$WORKSPACE`, and no file the tree does not hold.

```json
{
  "covered": ["REQ-001"],
  "written": ["build/greeting/index.md", "build/greeting/REQ-001.md"]
}
```
