# target.verify

Verify the integrated project tree after a wave of slices has merged into it, and answer a verdict.

## Inputs

- `$WORKSPACE` — the integrated tree, lent writable with the shell. Read it through the workspace tools, run the checks below in it, and repair what they find through this call's `write_files` tool, then run them again. A repair stays within `build/`: an `index.md` line added for a requirement file it leaves out, a requirement file removed through `delete` when nothing names it. What you write is sealed as the wave's own commit by the engine, never by you.

Nothing outside `$WORKSPACE` is reachable.

## Checks

The tree holds a Markdown stand-in for code: one directory `build/<slice-name>/` per slice built, each with an `index.md` and one `<REQ-NNN>.md` per requirement it implements. Run each check and read what it prints:

1. **Every slice directory has an index.** For each directory under `build/`, `index.md` exists and is not empty.
2. **Every index lists files the directory holds.** Each `REQ-NNN` an `index.md` names has a `REQ-NNN.md` beside it.
3. **No requirement file is orphaned.** Each `REQ-NNN.md` under `build/<slice-name>/` is named in that directory's `index.md`.
4. **Nothing was written outside `build/`.** `git status --porcelain` in `$WORKSPACE` shows no path outside `build/`.

A shell loop over `build/*/` with `test`, `grep`, and `ls` is enough for the first three; the fourth is one `git status` call.

## Verdict

Answer with one JSON object:

- `passed` — true when every check passed on its last run, false otherwise.
- `failures` — one entry per check that still failed, naming the check and quoting the tail of what it printed; empty when `passed` is true.

```json
{
  "passed": false,
  "failures": ["check 2: build/greeting/index.md names REQ-002, but build/greeting/REQ-002.md does not exist"]
}
```

Report what the checks found, never what the tree should hold.
